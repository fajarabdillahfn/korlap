<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { onMount, onDestroy } from 'svelte';
  import { showToast } from '$lib/toast.svelte';

  let { taskId }: { taskId: string } = $props();

  interface Message {
    id: string;
    role: 'user' | 'assistant' | 'system';
    content: string;
    created_at: string;
  }

  interface ToolCall {
    id: string;
    name: 'execute_command' | 'apply_diff' | 'read_file';
    input: Record<string, unknown>;
  }

  interface ToolCallsPendingPayload {
    task_id: string;
    tool_calls: ToolCall[];
  }

  let messages = $state<Message[]>([]);
  let input = $state('');
  let streaming = $state(false);
  let streamingText = $state('');
  let messagesEl = $state<HTMLDivElement | undefined>(undefined);
  let pendingToolCalls = $state<ToolCall[]>([]);
  let executingToolId = $state<string | null>(null);

  // @filename autocomplete
  let fileSuggestions = $state<string[]>([]);
  let suggestionIndex = $state(0);
  let atQuery = $state(''); // the query after @
  let atStart = $state(-1); // cursor position where @ was typed
  let inputEl = $state<HTMLTextAreaElement | undefined>(undefined);
  let debounceTimer: ReturnType<typeof setTimeout> | undefined;

  let unlistenChunk: (() => void) | undefined;
  let unlistenComplete: (() => void) | undefined;
  let unlistenToolCalls: (() => void) | undefined;
  let unlistenError: (() => void) | undefined;

  // Focus chat input when Ctrl+L is pressed globally
  function handleFocusChat() {
    inputEl?.focus();
  }

  // Approve/reject first tool call with A / X when tool calls are pending
  $effect(() => {
    if (pendingToolCalls.length === 0) return;

    function handleToolCallKeys(e: KeyboardEvent) {
      const target = e.target as HTMLElement;
      if (target.tagName === 'TEXTAREA' || target.tagName === 'INPUT') return;
      if (executingToolId !== null) return;
      const first = pendingToolCalls[0];
      if (!first) return;
      if (e.key === 'a' || e.key === 'A') { e.preventDefault(); approveToolCall(first); }
      else if (e.key === 'x' || e.key === 'X') { e.preventDefault(); rejectToolCall(first); }
    }

    document.addEventListener('keydown', handleToolCallKeys);
    return () => document.removeEventListener('keydown', handleToolCallKeys);
  });

  onMount(async () => {
    messages = await invoke<Message[]>('list_messages', { task_id: taskId });
    document.addEventListener('korlap:focus-chat', handleFocusChat);

    unlistenChunk = await listen<{ task_id: string; text: string }>('chat:stream_chunk', (e) => {
      if (e.payload.task_id !== taskId) return;
      streamingText += e.payload.text;
      scrollToBottom();
    });

    unlistenComplete = await listen<{ task_id: string; full_text: string }>('chat:complete', async (e) => {
      if (e.payload.task_id !== taskId) return;
      streaming = false;
      streamingText = '';
      messages = await invoke<Message[]>('list_messages', { task_id: taskId });
      scrollToBottom();
    });

    unlistenToolCalls = await listen<ToolCallsPendingPayload>('chat:tool_calls_pending', (e) => {
      if (e.payload.task_id !== taskId) return;
      streaming = false;
      streamingText = '';
      pendingToolCalls = e.payload.tool_calls;
      scrollToBottom();
    });

    unlistenError = await listen<{ task_id: string; error: string }>('chat:error', (e) => {
      if (e.payload.task_id !== taskId) return;
      streaming = false;
      streamingText = '';
      pendingToolCalls = [];
      showToast(`Claude error: ${e.payload.error}`);
    });
  });

  onDestroy(() => {
    unlistenChunk?.();
    unlistenComplete?.();
    unlistenToolCalls?.();
    unlistenError?.();
    document.removeEventListener('korlap:focus-chat', handleFocusChat);
  });

  async function sendMessage() {
    const text = input.trim();
    if (!text || streaming) return;

    input = '';
    streaming = true;
    streamingText = '';

    try {
      await invoke('send_chat_message', { task_id: taskId, content: text });
    } catch (e) {
      streaming = false;
      streamingText = '';
      showToast(String(e));
    }
  }

  async function approveToolCall(call: ToolCall) {
    executingToolId = call.id;
    try {
      await invoke('approve_tool_call', { task_id: taskId, tool_use_id: call.id });
      pendingToolCalls = pendingToolCalls.filter(c => c.id !== call.id);
      if (pendingToolCalls.length === 0) {
        streaming = true;
        streamingText = '';
      }
    } catch (e) {
      showToast(String(e));
    } finally {
      executingToolId = null;
    }
  }

  async function rejectToolCall(call: ToolCall) {
    executingToolId = call.id;
    try {
      await invoke('reject_tool_call', { task_id: taskId, tool_use_id: call.id, reason: 'User rejected' });
      pendingToolCalls = pendingToolCalls.filter(c => c.id !== call.id);
      if (pendingToolCalls.length === 0) {
        streaming = true;
        streamingText = '';
      }
    } catch (e) {
      showToast(String(e));
    } finally {
      executingToolId = null;
    }
  }

  function closeSuggestions() {
    fileSuggestions = [];
    suggestionIndex = 0;
    atQuery = '';
    atStart = -1;
  }

  function selectSuggestion(file: string) {
    if (atStart < 0) return;
    const before = input.slice(0, atStart);
    const after = input.slice(atStart + 1 + atQuery.length); // after the @query
    input = `${before}@${file}${after}`;
    closeSuggestions();
    // refocus textarea
    requestAnimationFrame(() => inputEl?.focus());
  }

  function handleInput() {
    const pos = inputEl?.selectionStart ?? input.length;
    // Find the last @ before cursor that starts the query
    const textToCursor = input.slice(0, pos);
    const atIdx = textToCursor.lastIndexOf('@');
    if (atIdx < 0) { closeSuggestions(); return; }
    const queryPart = textToCursor.slice(atIdx + 1);
    // Only show if no space in query
    if (queryPart.includes(' ') || queryPart.includes('\n')) { closeSuggestions(); return; }

    atStart = atIdx;
    atQuery = queryPart;

    clearTimeout(debounceTimer);
    debounceTimer = setTimeout(async () => {
      try {
        const results = await invoke<string[]>('list_worktree_files', {
          task_id: taskId,
          query: queryPart,
        });
        fileSuggestions = results;
        suggestionIndex = 0;
      } catch {
        fileSuggestions = [];
      }
    }, 150);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (fileSuggestions.length > 0) {
      if (e.key === 'ArrowDown') { e.preventDefault(); suggestionIndex = (suggestionIndex + 1) % fileSuggestions.length; return; }
      if (e.key === 'ArrowUp') { e.preventDefault(); suggestionIndex = (suggestionIndex - 1 + fileSuggestions.length) % fileSuggestions.length; return; }
      if (e.key === 'Tab' || e.key === 'Enter') { e.preventDefault(); selectSuggestion(fileSuggestions[suggestionIndex]); return; }
      if (e.key === 'Escape') { e.preventDefault(); closeSuggestions(); return; }
    }
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      sendMessage();
    }
  }

  function scrollToBottom() {
    requestAnimationFrame(() => {
      if (messagesEl) messagesEl.scrollTop = messagesEl.scrollHeight;
    });
  }

  function isDisplayableMessage(msg: Message): boolean {
    if (msg.role === 'system') return false;
    if (msg.content.startsWith('[')) return false;
    return true;
  }

  function getToolLabel(call: ToolCall): string {
    if (call.name === 'execute_command') return '🔧 Execute Command';
    if (call.name === 'apply_diff') return '✏ Edit File';
    if (call.name === 'read_file') return '📄 Read File';
    return call.name;
  }
</script>

<div class="chat-panel">
  <div class="messages" bind:this={messagesEl}>
    {#each messages as msg (msg.id)}
      {#if isDisplayableMessage(msg)}
        <div class="message" class:user={msg.role === 'user'} class:assistant={msg.role === 'assistant'}>
          <div class="message-role">{msg.role === 'user' ? 'You' : 'Claude'}</div>
          <div class="message-content">{msg.content}</div>
        </div>
      {/if}
    {/each}

    {#each pendingToolCalls as call (call.id)}
      <div class="tool-call-card" class:executing={executingToolId === call.id}>
        <div class="tool-call-header">
          {#if call.name === 'execute_command'}
            <span class="tool-label command">{getToolLabel(call)}</span>
          {:else if call.name === 'apply_diff'}
            <span class="tool-label diff">{getToolLabel(call)}{call.input.file_path ? ': ' + String(call.input.file_path) : ''}</span>
          {:else if call.name === 'read_file'}
            <span class="tool-label read">{getToolLabel(call)}{call.input.file_path ? ': ' + String(call.input.file_path) : ''}</span>
          {/if}
        </div>

        <div class="tool-call-details">
          {#if call.name === 'execute_command'}
            {#if call.input.command}
              <div class="detail-row">
                <span class="detail-label">Command:</span>
                <code class="detail-code">{String(call.input.command)}</code>
              </div>
            {/if}
            {#if call.input.rationale}
              <div class="detail-row">
                <span class="detail-label">Reason:</span>
                <span class="detail-text">{String(call.input.rationale)}</span>
              </div>
            {/if}
          {:else if call.name === 'apply_diff'}
            {#if call.input.old_content !== undefined && call.input.new_content !== undefined}
              <div class="detail-row">
                <span class="detail-text">
                  Replace {String(call.input.old_content).split('\n').length} lines with {String(call.input.new_content).split('\n').length} lines
                </span>
              </div>
            {/if}
            {#if call.input.rationale}
              <div class="detail-row">
                <span class="detail-label">Reason:</span>
                <span class="detail-text">{String(call.input.rationale)}</span>
              </div>
            {/if}
          {:else if call.name === 'read_file'}
            {#if call.input.rationale}
              <div class="detail-row">
                <span class="detail-label">Reason:</span>
                <span class="detail-text">{String(call.input.rationale)}</span>
              </div>
            {/if}
          {/if}
        </div>

        <div class="tool-call-actions">
          <button
            class="btn-approve"
            onclick={() => approveToolCall(call)}
            disabled={executingToolId !== null}
          >
            {executingToolId === call.id ? 'Running…' : 'Approve & Run'}
          </button>
          <button
            class="btn-reject"
            onclick={() => rejectToolCall(call)}
            disabled={executingToolId !== null}
          >
            Reject
          </button>
        </div>
      </div>
    {/each}

    {#if streaming}
      <div class="message assistant">
        <div class="message-role">
          Claude
          <span class="stream-indicator" aria-label="Streaming"></span>
        </div>
        <div class="message-content streaming">{streamingText || '…'}</div>
      </div>
    {/if}
  </div>

  <div class="input-area">
    <div class="input-wrapper">
      {#if fileSuggestions.length > 0}
        <ul class="file-suggestions" role="listbox">
          {#each fileSuggestions as file, i (file)}
            <li
              class="suggestion-item"
              class:active={i === suggestionIndex}
              role="option"
              aria-selected={i === suggestionIndex}
              onmousedown={(e) => { e.preventDefault(); selectSuggestion(file); }}
            >{file}</li>
          {/each}
        </ul>
      {/if}
      <textarea
        class="chat-input"
        placeholder="Ask Claude… (Enter to send, Shift+Enter for newline, @ for files)"
        bind:value={input}
        bind:this={inputEl}
        onkeydown={handleKeydown}
        oninput={handleInput}
        disabled={streaming || pendingToolCalls.length > 0}
        rows="3"
      ></textarea>
    </div>
    <button
      class="send-btn"
      onclick={sendMessage}
      disabled={streaming || !input.trim() || pendingToolCalls.length > 0}
    >
      {streaming ? '…' : 'Send'}
    </button>
  </div>
</div>

<style>
  .chat-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-base);
  }

  .messages {
    flex: 1;
    overflow-y: auto;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .message {
    display: flex;
    flex-direction: column;
    gap: 4px;
    max-width: 85%;
  }

  .message.user {
    align-self: flex-end;
    align-items: flex-end;
  }

  .message.assistant {
    align-self: flex-start;
  }

  .message-role {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
    display: flex;
    align-items: center;
    gap: 6px;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; transform: scale(1); }
    50% { opacity: 0.4; transform: scale(0.7); }
  }

  .stream-indicator {
    display: inline-block;
    width: 6px;
    height: 6px;
    background: var(--accent);
    border-radius: 50%;
    animation: pulse 1s ease-in-out infinite;
  }

  .message-content {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 10px 14px;
    font-size: 13px;
    color: var(--text-primary);
    line-height: 1.5;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .message.user .message-content {
    background: var(--bg-active);
    border-color: var(--border-light);
  }

  .message-content.streaming {
    opacity: 0.85;
  }

  /* Tool call cards */
  .tool-call-card {
    background: var(--bg-card);
    border: 1px solid var(--border-light);
    border-radius: 8px;
    padding: 14px 16px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    align-self: flex-start;
    max-width: 90%;
  }

  .tool-call-card.executing {
    opacity: 0.7;
  }

  .tool-call-header {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .tool-label {
    font-size: 13px;
    font-weight: 700;
    color: var(--text-primary);
  }

  .tool-label.command {
    color: var(--accent);
  }

  .tool-label.diff {
    color: var(--status-ok);
  }

  .tool-label.read {
    color: var(--text-secondary);
  }

  .tool-call-details {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .detail-row {
    display: flex;
    align-items: baseline;
    gap: 6px;
    font-size: 13px;
    flex-wrap: wrap;
  }

  .detail-label {
    font-weight: 600;
    color: var(--text-secondary);
    flex-shrink: 0;
  }

  .detail-text {
    color: var(--text-primary);
    line-height: 1.4;
  }

  .detail-code {
    font-family: monospace;
    font-size: 12px;
    background: var(--bg-active);
    color: var(--text-primary);
    border-radius: 4px;
    padding: 2px 6px;
    word-break: break-all;
  }

  .tool-call-actions {
    display: flex;
    gap: 8px;
    margin-top: 2px;
  }

  .btn-approve {
    background: var(--status-ok);
    border: none;
    border-radius: 6px;
    color: var(--text-primary);
    font-size: 13px;
    font-weight: 600;
    padding: 7px 16px;
    cursor: pointer;
    transition: opacity 0.1s;
  }

  .btn-approve:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .btn-reject {
    background: var(--error);
    border: none;
    border-radius: 6px;
    color: white;
    font-size: 13px;
    font-weight: 600;
    padding: 7px 16px;
    cursor: pointer;
    transition: opacity 0.1s;
  }

  .btn-reject:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  /* Input area */
  .input-area {
    display: flex;
    gap: 8px;
    padding: 12px;
    border-top: 1px solid var(--border);
    flex-shrink: 0;
  }

  .input-wrapper {
    flex: 1;
    position: relative;
    display: flex;
    flex-direction: column;
  }

  .file-suggestions {
    position: absolute;
    bottom: 100%;
    left: 0;
    right: 0;
    background: var(--bg-card);
    border: 1px solid var(--border-light);
    border-radius: 6px;
    list-style: none;
    margin: 0 0 4px;
    padding: 4px 0;
    max-height: 200px;
    overflow-y: auto;
    z-index: 10;
    box-shadow: 0 -4px 12px rgba(0,0,0,0.3);
  }

  .suggestion-item {
    cursor: pointer;
    font-family: 'JetBrains Mono', monospace;
    font-size: 12px;
    padding: 5px 12px;
    color: var(--text-primary);
    transition: background 0.08s;
  }

  .suggestion-item:hover,
  .suggestion-item.active {
    background: var(--bg-active);
    color: var(--accent);
  }

  .chat-input {
    width: 100%;
    background: var(--bg-card);
    border: 1px solid var(--border-light);
    border-radius: 8px;
    color: var(--text-primary);
    font-size: 13px;
    font-family: inherit;
    padding: 10px 12px;
    resize: none;
    outline: none;
    transition: border-color 0.1s;
  }

  .chat-input::placeholder {
    color: var(--text-muted);
  }

  .chat-input:focus {
    border-color: var(--accent);
  }

  .send-btn {
    background: var(--accent);
    border: none;
    border-radius: 8px;
    color: var(--bg-base);
    font-size: 13px;
    font-weight: 600;
    padding: 10px 20px;
    cursor: pointer;
    align-self: flex-end;
    transition: opacity 0.1s;
  }

  .send-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
</style>
