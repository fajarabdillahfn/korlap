use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClaudeMessage {
    pub role: String,
    pub content: Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PendingToolCall {
    pub id: String,
    pub name: String,
    pub input: Value,
}

#[derive(Debug, Serialize, Clone)]
pub struct ChatResult {
    pub text: String,
    pub tool_uses: Vec<PendingToolCall>,
    pub full_content: Value,
}

#[derive(Debug, Serialize)]
struct ApiRequest {
    model: String,
    max_tokens: u32,
    stream: bool,
    system: String,
    messages: Vec<ClaudeMessage>,
    tools: Vec<Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StreamChunk {
    pub task_id: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct StreamDone {
    pub task_id: String,
    pub full_text: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct StreamError {
    pub task_id: String,
    pub error: String,
}

const MODEL: &str = "claude-sonnet-4-6";
const MAX_TOKENS: u32 = 8192;
const SYSTEM_PROMPT: &str = "You are an expert software engineering assistant helping with coding tasks. \
You have access to tools for reading files, applying code changes, and running commands. \
Always explain your reasoning before taking actions.";

fn tool_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "execute_command",
            "description": "Run a shell command in the task's worktree. Use for running tests, builds, git operations, or any terminal command.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "The shell command to execute" },
                    "rationale": { "type": "string", "description": "Why this command is needed" }
                },
                "required": ["command", "rationale"]
            }
        }),
        json!({
            "name": "apply_diff",
            "description": "Apply a search-and-replace edit to a file in the worktree. Replaces exact old_content with new_content in the specified file.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "file_path": { "type": "string", "description": "Relative path to the file within the worktree" },
                    "old_content": { "type": "string", "description": "Exact existing content to replace (must match exactly)" },
                    "new_content": { "type": "string", "description": "Replacement content" },
                    "rationale": { "type": "string", "description": "Why this change is needed" }
                },
                "required": ["file_path", "old_content", "new_content", "rationale"]
            }
        }),
        json!({
            "name": "read_file",
            "description": "Read a file from the task's worktree. Returns file contents.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "file_path": { "type": "string", "description": "Relative path to the file within the worktree" },
                    "full": { "type": "boolean", "description": "If true, return full file. Default false truncates at 200 lines." }
                },
                "required": ["file_path"]
            }
        }),
    ]
}

/// Estimate token count (rough: 1 token ≈ 4 chars)
pub fn estimate_tokens(messages: &[ClaudeMessage]) -> usize {
    messages.iter().map(|m| m.content.to_string().len() / 4).sum()
}

/// Trim oldest messages to stay within `max_tokens`.
/// Returns a new Vec (immutable pattern). Guarantees the result starts with a
/// plain `user` text message (not assistant, not a JSON tool_result array) to
/// satisfy the Claude API's alternating-role requirement.
pub fn trim_messages(messages: Vec<ClaudeMessage>, max_tokens: usize) -> Vec<ClaudeMessage> {
    let total = messages.len();

    // Find the earliest index whose suffix fits within the token budget.
    let mut start = 0;
    while start < total.saturating_sub(1) && estimate_tokens(&messages[start..]) > max_tokens {
        start += 1;
    }

    // Advance past any leading non-plain-user messages (assistant turns or
    // tool_result JSON arrays) — those are invalid as the first API turn.
    while start < total {
        let msg = &messages[start];
        let is_plain_user = msg.role == "user"
            && matches!(&msg.content, Value::String(s) if !s.starts_with('['));
        if is_plain_user {
            break;
        }
        start += 1;
    }

    // Collect from `start` — O(n), no in-place mutation of the input.
    messages.into_iter().skip(start).collect()
}

struct BlockAccumulator {
    block_type: String,
    id: String,
    name: String,
    partial_json: String,
    text: String,
}

pub async fn stream_chat(
    api_key: String,
    task_id: String,
    messages: Vec<ClaudeMessage>,
    app: tauri::AppHandle,
) -> Result<ChatResult, String> {
    use futures_util::StreamExt;
    use tauri::Emitter;

    let trimmed = trim_messages(messages, 150_000);

    let request = ApiRequest {
        model: MODEL.to_string(),
        max_tokens: MAX_TOKENS,
        stream: true,
        system: SYSTEM_PROMPT.to_string(),
        messages: trimmed,
        tools: tool_definitions(),
    };

    let client = reqwest::Client::new();
    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&request)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        eprintln!("Claude API error {}: {}", status, body); // full detail server-side only
        return Err(format!("Claude API returned an error (HTTP {}). Check the server log for details.", status.as_u16()));
    }

    let mut stream = response.bytes_stream();
    let mut full_text = String::new();
    let mut buffer = String::new();
    let mut current_block: Option<BlockAccumulator> = None;
    let mut tool_uses: Vec<PendingToolCall> = Vec::new();
    let mut content_blocks: Vec<Value> = Vec::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(pos) = buffer.find('\n') {
            let line = buffer[..pos].trim().to_string();
            buffer = buffer[pos + 1..].to_string();

            if let Some(data) = line.strip_prefix("data: ") {
                if data == "[DONE]" {
                    break;
                }
                if let Ok(json) = serde_json::from_str::<Value>(data) {
                    let event_type = json["type"].as_str().unwrap_or("");

                    match event_type {
                        "content_block_start" => {
                            let cb = &json["content_block"];
                            let block_type = cb["type"].as_str().unwrap_or("").to_string();
                            let id = cb["id"].as_str().unwrap_or("").to_string();
                            let name = cb["name"].as_str().unwrap_or("").to_string();
                            current_block = Some(BlockAccumulator {
                                block_type,
                                id,
                                name,
                                partial_json: String::new(),
                                text: String::new(),
                            });
                        }
                        "content_block_delta" => {
                            let delta = &json["delta"];
                            let delta_type = delta["type"].as_str().unwrap_or("");

                            match delta_type {
                                "text_delta" => {
                                    if let Some(text) = delta["text"].as_str() {
                                        full_text.push_str(text);
                                        if let Some(ref mut block) = current_block {
                                            block.text.push_str(text);
                                        }
                                        let _ = app.emit(
                                            "chat:stream_chunk",
                                            StreamChunk {
                                                task_id: task_id.clone(),
                                                text: text.to_string(),
                                            },
                                        );
                                    }
                                }
                                "input_json_delta" => {
                                    if let Some(partial) = delta["partial_json"].as_str() {
                                        if let Some(ref mut block) = current_block {
                                            block.partial_json.push_str(partial);
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                        "content_block_stop" => {
                            if let Some(block) = current_block.take() {
                                match block.block_type.as_str() {
                                    "tool_use" => {
                                        let input: Value = serde_json::from_str(&block.partial_json)
                                            .unwrap_or(Value::Object(serde_json::Map::new()));
                                        let tool_call = PendingToolCall {
                                            id: block.id.clone(),
                                            name: block.name.clone(),
                                            input: input.clone(),
                                        };
                                        tool_uses.push(tool_call);
                                        content_blocks.push(json!({
                                            "type": "tool_use",
                                            "id": block.id,
                                            "name": block.name,
                                            "input": input
                                        }));
                                    }
                                    "text" => {
                                        content_blocks.push(json!({
                                            "type": "text",
                                            "text": block.text
                                        }));
                                    }
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    let full_content = Value::Array(content_blocks);

    Ok(ChatResult {
        text: full_text,
        tool_uses,
        full_content,
    })
}
