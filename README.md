# SplitCommander

A keyboard-first, two-pane file manager for macOS with directory comparison.

<img src="docs/screenshots/browse.png" alt="Browse Mode" width="100%">

## Features

- **Dual-pane file browser** — navigate two directories side by side with keyboard or mouse
- **Directory comparison** — diff two directories by structure (presence + type) or smart mode (presence + type + size + mtime)
- **Color-coded diffs** — instantly see what's identical, left-only, right-only, modified, or mismatched
- **Built-in terminal** — dual-pane terminal panel that follows your current directory
- **Fast** — parallel scanning with jwalk, virtual scrolling for 200k+ file lists
- **Dark & light themes** — follows your system preference

## Install

### Prerequisites

- [Node.js](https://nodejs.org/) 18+
- [Rust](https://rustup.rs/) (stable)
- Xcode Command Line Tools (`xcode-select --install`)

### Build from source

```bash
git clone https://github.com/webdevbyjoss/SplitCommander.git
cd SplitCommander
npm install
npm run tauri build
```

The built `.app` bundle will be in `src-tauri/target/release/bundle/macos/`.

### Development

```bash
npm run tauri dev
```

## Usage

SplitCommander is keyboard-driven. The footer bar shows available commands for the current screen.

### Browse mode

| Key | Action |
|-----|--------|
| `Tab` | Switch pane |
| `Up/Down` | Navigate files |
| `Enter` | Open file or enter directory |
| `Backspace` | Go up one level |
| `i` | Toggle hidden files |
| `t` | Create directory |
| `g` | Compare directories |
| `` ` `` | Toggle terminal |
| `q` | Quit |

### Compare mode

| Key | Action |
|-----|--------|
| `Up/Down` | Navigate diffs |
| `Enter` | Drill into subdirectory |
| `Backspace` | Go up one level |
| `s` | Toggle identical files |
| `Esc` | Back to browse |
| `q` | Quit |

### Terminal

| Key | Action |
|-----|--------|
| `Tab` | Switch terminal pane |
| `Esc Esc` | Close terminal |

## Testing

```bash
# Rust tests (42 tests covering scanner, comparator, file ops, ignore rules, security, export)
cd src-tauri && cargo test

# E2E tests (Playwright)
npx playwright test
```

## Contributing

Contributions welcome. Please open an issue first to discuss what you'd like to change.
