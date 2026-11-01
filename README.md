# DROP (Aero) ⚡

> **A hardware-accelerated, Quake-style drop-down terminal for Windows built with Rust, WGPU, and modern DWM backdrop effects.**

Drop slides smoothly from the top of your screen at the touch of a global hotkey, delivering a blazingly fast, seamless command-line experience with Windows 11 Acrylic styling, GPU-rendered typography, and integrated multi-tab management.

---

## ✨ Features

- **Smooth Drop-Down Animation**: Fluid quartic/cubic easing transitions that slide in and out from the top of your monitor.
- **Global Hotkey Integration**: Instant toggle from anywhere via low-level Windows keyboard hook (`Ctrl + '` / `Ctrl + ` `).
- **Modern Windows Backdrop Effects**: Native Windows 11 Acrylic / Mica system backdrops with automatic fallback to Windows 10 DWM blur-behind.
- **Hardware-Accelerated Text & Graphics**: Built on **WGPU** (DirectX 12 / Vulkan) and **Glyphon** for crisp, sub-pixel text rasterization locked to V-Sync.
- **Windows Pseudo Console (ConPTY)**: Powered by `portable-pty` running PowerShell or custom shells with full ANSI escape sequence support via `alacritty_terminal`.
- **Tabbed Experience**: Built-in tab bar supporting multiple concurrent shell sessions (`Ctrl+Shift+T`, `Ctrl+Shift+W`, `Ctrl+Tab`, `Alt+1..9`).
- **Interactive In-Terminal Settings**: Powered by `egui`, allowing real-time customization of fonts, padding, themes, and cursor styles saved to `drop_config.json`.
- **Dynamic Resizing**: Draggable bottom border grip or keyboard shortcuts (`Ctrl+Shift+Up/Down`, `Ctrl+Shift+Enter`) to adjust height on the fly.
- **Rich Themes**: Ships with curated themes including Dark Slate, Catppuccin, Dracula, Tokyo Night, One Dark, Nord, Cyberpunk, and Solarized Dark.

---

## ⌨️ Default Shortcuts

| Shortcut | Action |
|---|---|
| `Ctrl + '` or `Ctrl + ` ` | **Toggle Drop** (Global hotkey to show / hide terminal) |
| `Ctrl + ,` | **Toggle Settings UI** |
| `Ctrl + Shift + T` | Open new tab |
| `Ctrl + Shift + W` | Close current tab |
| `Ctrl + Tab` / `Ctrl + PageDown` | Switch to next tab |
| `Ctrl + Shift + Tab` / `Ctrl + PageUp` | Switch to previous tab |
| `Alt + 1` .. `Alt + 9` | Direct switch to tab 1 through 9 |
| `Shift + PageUp` / `Shift + PageDown` | Scroll history buffer |
| `Ctrl + V` / `Shift + Insert` | Paste from clipboard |
| `Ctrl + Shift + Down` | Increase terminal height (+60px) |
| `Ctrl + Shift + Up` | Decrease terminal height (-60px) |
| `Ctrl + Shift + Enter` | Toggle between compact (40%) and expanded (85%) height |
| `Shift + Escape` | Exit / close application |

---

## 🚀 Getting Started

### Prerequisites

- **Windows 10 (1903+)** or **Windows 11** (recommended for Acrylic backdrop)
- **Rust toolchain** (1.75+ recommended): [rustup.rs](https://rustup.rs)
- A font with programming ligatures or monospaced glyphs (e.g., *Fira Code*, *Cascadia Code*, *JetBrains Mono*)

### Building from Source

```powershell
# Clone the repository
git clone https://github.com/carllosnc/aero.git
cd aero

# Build in release mode
cargo build --release

# Run Drop
cargo run --release
```

---

## ⚙️ Configuration

Drop stores its runtime settings in `drop_config.json`:

```json
{
  "padding_x": 24.0,
  "padding_y": 18.0,
  "font_family": "Fira Code",
  "font_size": 19.0,
  "theme": "Dark Slate",
  "cursor_style": "bar"
}
```

You can adjust these settings directly in the JSON file or interactively via the built-in Settings UI modal (`Ctrl + ,`).

---

## 🏗️ Architecture

```
src/
├── main.rs         # Application entry point & logger initialization
├── app.rs          # Event loop, tab state, sliding animations, and window management
├── dwm.rs          # Windows 11 Acrylic & Windows 10 DWM blur-behind API integration
├── hotkey.rs       # Low-level Windows keyboard hook (WH_KEYBOARD_LL) for global toggle
├── terminal.rs     # Terminal emulator state machine (alacritty_terminal parser & grid)
├── pty.rs          # ConPTY backend process management (portable-pty)
├── input.rs        # Keyboard translation into ANSI/VT escape sequences
├── renderer.rs     # GPU rendering engine (wgpu + glyphon + custom rect shader)
├── settings_ui.rs  # Settings overlay UI modal implemented with egui
└── config.rs       # Theme definitions, config file persistence, and defaults
```

---

## 📜 License

Licensed under the [MIT License](LICENSE).
