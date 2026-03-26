use base64::{engine::general_purpose::STANDARD, Engine as _};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};

pub struct PtySession {
    pub writer: Arc<Mutex<Box<dyn Write + Send>>>,
    pub child: Box<dyn portable_pty::Child + Send>,
}

pub type PtyMap = Arc<Mutex<HashMap<String, PtySession>>>;

#[derive(Clone, serde::Serialize)]
struct StdoutPayload {
    terminal_id: String,
    data: String, // base64-encoded bytes
}

#[derive(Clone, serde::Serialize)]
struct ExitPayload {
    terminal_id: String,
    code: Option<u32>,
}

pub fn spawn_pty(
    app: AppHandle,
    pty_map: tauri::State<PtyMap>,
    terminal_id: String,
    cwd: String,
) -> Result<(), String> {
    let pty_system = native_pty_system();

    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| e.to_string())?;

    let shell = if cfg!(windows) {
        std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string())
    } else {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string())
    };
    let mut cmd = CommandBuilder::new(shell);
    cmd.cwd(cwd);

    let child = pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;
    let writer = Arc::new(Mutex::new(
        pair.master.take_writer().map_err(|e| e.to_string())? as Box<dyn Write + Send>,
    ));
    let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;

    // Store session
    {
        let mut map = pty_map.lock().map_err(|e| e.to_string())?;
        map.insert(terminal_id.clone(), PtySession { writer, child });
    }

    // Clone the Arc so the reader thread can access the map for exit code
    let pty_map_arc: PtyMap = (*pty_map).clone();

    // Spawn reader thread
    let tid = terminal_id.clone();
    let app_clone = app.clone();
    std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let data = STANDARD.encode(&buf[..n]);
                    let _ = app_clone.emit(
                        "terminal:stdout",
                        StdoutPayload {
                            terminal_id: tid.clone(),
                            data,
                        },
                    );
                }
            }
        }

        // Collect exit code from child (process has already exited at this point)
        let code = pty_map_arc
            .lock()
            .ok()
            .and_then(|mut map| {
                map.get_mut(&tid).and_then(|s| {
                    s.child
                        .wait()
                        .ok()
                        .map(|status| if status.success() { 0u32 } else { 1u32 })
                })
            });

        let _ = app_clone.emit(
            "terminal:exit",
            ExitPayload {
                terminal_id: tid,
                code,
            },
        );
    });

    Ok(())
}
