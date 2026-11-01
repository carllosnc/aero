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

        // Prioritize PowerShell 7 (pwsh.exe)
        let pwsh_full_path = r"C:\Program Files\PowerShell\7\pwsh.exe";
        let exe = if std::path::Path::new(pwsh_full_path).exists() {
            pwsh_full_path
        } else {
            "pwsh.exe"
        };

        let mut cmd = CommandBuilder::new(exe);
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        cmd.env("WT_SESSION", "1");

        let child = match pair.slave.spawn_command(cmd) {
            Ok(c) => {
                tracing::info!("PTY session spawned successfully with {}", exe);
                c
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to spawn pwsh ({}): {:?}. Falling back to powershell.exe",
                    exe,
                    e
                );
                let mut fallback = CommandBuilder::new("powershell.exe");
                fallback.env("TERM", "xterm-256color");
                fallback.env("COLORTERM", "truecolor");
                fallback.env("WT_SESSION", "1");
                pair.slave.spawn_command(fallback)?
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
