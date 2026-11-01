mod app;
mod config;
mod dwm;
mod hotkey;
mod input;
mod pty;
mod renderer;
mod settings_ui;
mod terminal;

use anyhow::Result;
use app::{AppEvent, DropApp};
use tracing_subscriber::EnvFilter;
use winit::event_loop::EventLoop;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    println!("===========================================================");
    println!("  DROP // GPU Drop-Down Terminal (Windows)");
    println!("  Phase 2: PTY & Terminal State Machine Active");
    println!("  * Shell: PowerShell (powershell.exe via ConPTY)");
    println!("  * State: alacritty_terminal ANSI Parser & Grid");
    println!("  * Rendering: wgpu + glyphon (V-Sync, Hardware Accelerated)");
    println!("  * Global Hotkey: Press [Ctrl + '] to Show/Hide");
    println!("  * Exit: Press [Shift + ESC] or close window");
    println!("===========================================================");

    let event_loop: EventLoop<AppEvent> = EventLoop::<AppEvent>::with_user_event().build()?;
    let proxy = event_loop.create_proxy();
    let mut app = DropApp::new(proxy);

    event_loop.run_app(&mut app)?;

    Ok(())
}
