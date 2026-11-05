use anyhow::Result;
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtyPair, PtySize};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use winit::event_loop::EventLoopProxy;

pub struct PtyManager {
    master: Box<dyn MasterPty + Send>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    _child: Box<dyn Child + Send + Sync>,
}

impl PtyManager {
    pub fn new<T: 'static + Send>(
        tab_id: usize,
        cols: u16,
        rows: u16,
        shell: &str,
        proxy: EventLoopProxy<T>,
        on_output: impl Fn(usize, Vec<u8>) -> T + Send + 'static,
    ) -> Result<Self> {
        let pty_system = native_pty_system();
        let pair: PtyPair = pty_system.openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let shell_lower = shell.trim().to_lowercase();
        let (exe, fallback_exe) = match shell_lower.as_str() {
            "cmd" | "cmd.exe" => ("cmd.exe".to_string(), None),
            "powershell" | "powershell.exe" => ("powershell.exe".to_string(), None),
            "wsl" | "wsl.exe" => ("wsl.exe".to_string(), Some("powershell.exe".to_string())),
            "bash" | "git-bash" => {
                let git_bash = r"C:\Program Files\Git\bin\bash.exe";
                if std::path::Path::new(git_bash).exists() {
                    (git_bash.to_string(), Some("powershell.exe".to_string()))
                } else {
                    ("bash.exe".to_string(), Some("powershell.exe".to_string()))
                }
            }
            "pwsh" | "pwsh.exe" | "" => {
                let pwsh_full_path = r"C:\Program Files\PowerShell\7\pwsh.exe";
                if std::path::Path::new(pwsh_full_path).exists() {
                    (pwsh_full_path.to_string(), Some("powershell.exe".to_string()))
                } else {
                    ("pwsh.exe".to_string(), Some("powershell.exe".to_string()))
                }
            }
            custom => (custom.to_string(), Some("powershell.exe".to_string())),
        };

        let mut cmd = CommandBuilder::new(&exe);
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        cmd.env("COLORFGBG", "15;0");
        cmd.env("TERM_PROGRAM", "Aero");
        cmd.env("WT_SESSION", "e0a0a520-2ef6-4df6-8800-000000000001");

        let child = match pair.slave.spawn_command(cmd) {
            Ok(c) => {
                tracing::info!("PTY session spawned successfully with {}", exe);
                c
            }
            Err(e) => {
                if let Some(ref fb) = fallback_exe {
                    tracing::warn!(
                        "Failed to spawn primary shell ({}): {:?}. Falling back to {}",
                        exe,
                        e,
                        fb
                    );
                    let mut fallback = CommandBuilder::new(fb);
                    fallback.env("TERM", "xterm-256color");
                    fallback.env("COLORTERM", "truecolor");
                    fallback.env("COLORFGBG", "15;0");
                    fallback.env("TERM_PROGRAM", "Aero");
                    fallback.env("WT_SESSION", "e0a0a520-2ef6-4df6-8800-000000000001");
                    pair.slave.spawn_command(fallback)?
                } else {
                    return Err(e.into());
                }
            }
        };

        let mut reader = pair.master.try_clone_reader()?;
        let writer = Arc::new(Mutex::new(pair.master.take_writer()?));

        // Background reader thread
        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => {
                        tracing::info!("PTY stream tab {} reached EOF", tab_id);
                        break;
                    }
                    Ok(n) => {
                        let bytes = buf[..n].to_vec();
                        let event = on_output(tab_id, bytes);
                        if proxy.send_event(event).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        tracing::error!("Error reading from PTY tab {}: {:?}", tab_id, e);
                        break;
                    }
                }
            }
        });

        Ok(Self {
            master: pair.master,
            writer,
            _child: child,
        })
    }

    pub fn write(&self, data: &[u8]) -> Result<()> {
        let mut writer = self.writer.lock().unwrap();
        writer.write_all(data)?;
        writer.flush()?;
        Ok(())
    }

    pub fn resize(&mut self, cols: u16, rows: u16) -> Result<()> {
        self.master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        Ok(())
    }
}
