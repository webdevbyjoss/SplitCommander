use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::io::{Read, Write};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

pub struct PtyState {
    pub master: Arc<Mutex<Box<dyn MasterPty + Send>>>,
    pub writer: Arc<Mutex<Box<dyn Write + Send>>>,
    pub child: Arc<Mutex<Box<dyn portable_pty::Child + Send + Sync>>>,
    pub reader_active: Arc<AtomicBool>,
}

/// Spawns a PTY running the user's shell in the given working directory.
/// Returns the PTY state (for writing/resizing) and a reader (for reading output).
pub fn spawn_pty(
    cwd: &str,
    rows: u16,
    cols: u16,
) -> Result<(PtyState, Box<dyn Read + Send>), String> {
    let pty_system = native_pty_system();

    let pair = pty_system
        .openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| format!("Failed to open PTY: {}", e))?;

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
    let mut cmd = CommandBuilder::new(&shell);
    cmd.cwd(cwd);
    cmd.env("TERM", "xterm-256color");

    let child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| format!("Failed to spawn shell: {}", e))?;
    drop(pair.slave);

    let reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| format!("Failed to clone reader: {}", e))?;
    let writer = pair
        .master
        .take_writer()
        .map_err(|e| format!("Failed to take writer: {}", e))?;

    let state = PtyState {
        master: Arc::new(Mutex::new(pair.master)),
        writer: Arc::new(Mutex::new(writer)),
        child: Arc::new(Mutex::new(child)),
        reader_active: Arc::new(AtomicBool::new(true)),
    };

    Ok((state, reader))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawn_pty_creates_running_child() {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        let (state, _reader) = spawn_pty(&home, 24, 80).expect("should spawn PTY");

        let mut child = state.child.lock().unwrap();
        assert!(
            child.try_wait().unwrap().is_none(),
            "child should be running"
        );

        let _ = child.kill();
        let _ = child.wait();
    }

    #[test]
    fn test_pty_resize() {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        let (state, _reader) = spawn_pty(&home, 24, 80).expect("should spawn PTY");

        let master = state.master.lock().unwrap();
        let result = master.resize(PtySize {
            rows: 40,
            cols: 120,
            pixel_width: 0,
            pixel_height: 0,
        });
        assert!(result.is_ok(), "resize should succeed");

        drop(master);
        let mut child = state.child.lock().unwrap();
        let _ = child.kill();
        let _ = child.wait();
    }

    #[test]
    fn test_pty_write_and_read() {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        let (state, mut reader) = spawn_pty(&home, 24, 80).expect("should spawn PTY");

        // Write an echo command
        {
            let mut writer = state.writer.lock().unwrap();
            writer.write_all(b"echo hello_pty_test\r").unwrap();
            writer.flush().unwrap();
        }

        // Read output with timeout
        let mut output = Vec::new();
        let mut buf = [0u8; 4096];
        let start = std::time::Instant::now();
        while start.elapsed() < std::time::Duration::from_secs(3) {
            match reader.read(&mut buf) {
                Ok(n) if n > 0 => {
                    output.extend_from_slice(&buf[..n]);
                    let text = String::from_utf8_lossy(&output);
                    if text.contains("hello_pty_test") {
                        break;
                    }
                }
                _ => {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
            }
        }

        let text = String::from_utf8_lossy(&output);
        assert!(
            text.contains("hello_pty_test"),
            "should see echo output in PTY, got: {}",
            text
        );

        let mut child = state.child.lock().unwrap();
        let _ = child.kill();
        let _ = child.wait();
    }
}
