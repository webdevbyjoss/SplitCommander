# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

SplitCommander is a keyboard-first, two-pane file manager for macOS with directory comparison. Built with Tauri v2 (Rust backend) and Svelte 5 + TypeScript (frontend). Currently read-only (comparison only, no write operations).

## Build & Development Commands

```bash
# Install frontend dependencies
npm install

# Run in development mode (launches Tauri dev window)
npm run tauri dev

# Build for production
npm run tauri build

# Run Rust tests (27 tests covering model, scanner, comparator, ignore, security, export)
cd src-tauri && cargo test

# Run a single Rust test
cd src-tauri && cargo test test_name

# Check Rust compilation
cd src-tauri && cargo check

# Lint Rust
cd src-tauri && cargo clippy

# Build frontend only
npx vite build

# Type-check Svelte/TS
npm run check
```

## Architecture

### Two-process model (Tauri v2)
- **Rust backend** (`src-tauri/src/`): filesystem scanning, comparison engine, event emission, JSON export
- **Svelte 5 frontend** (`src/`): UI rendering, receives events from Rust via Tauri event system

### Communication pattern
- **Commands** (UI → Rust): `set_root`, `start_compare`, `cancel_compare`, `get_diffs`, `get_summary`, `export_report`
- **Events** (Rust → UI): `scan-progress`, `compare-done`, `compare-error`

### Rust core engine (`src-tauri/src/core/`)
Three-phase comparison pipeline:
1. **Scan** (`scan.rs`): Walk both roots with jwalk (parallel), build `HashMap<String, EntryMeta>`, emit progress events
2. **Metadata compare** (`compare.rs`): Diff classification → OnlyLeft, OnlyRight, TypeMismatch, Same, MetaDiff
3. **Deep verify** (`hash.rs`): BLAKE3 hashing (planned for v0.2)

Key modules:
- `model.rs` — EntryKind, EntryMeta, DiffKind, DiffItem, CompareMode, CompareSummary (all `serde(rename_all = "camelCase")`)
- `scan.rs` — parallel directory walking with jwalk, cancellation via AtomicBool, progress callbacks
- `compare.rs` — Structure mode (presence+type) and Smart mode (presence+type+size+mtime)
- `ignore.rs` — glob rules + macOS noise preset (.DS_Store, ._, .Spotlight-V100, etc.)
- `security.rs` — root confinement via canonicalize + starts_with
- `export.rs` — JSON report generation with chrono timestamps
- `commands.rs` — Tauri command handlers, AppState with Mutex-protected fields
- `events.rs` — event payload types and name constants

### Frontend (`src/`)
- **Svelte 5 runes** for reactive state (`$state`, `$derived`, `$effect`)
- `lib/stores/compare.svelte.ts` — central CompareStore class with all state and actions
- `lib/types.ts` — TypeScript mirrors of Rust model types
- Components: TopBar, FilePane (with manual virtual scrolling), DiffBadge, DiffDetails, BottomBar, ProgressIndicator
- FilePane implements manual windowing (ROW_HEIGHT-based) for 200k+ file lists

### Key design decisions
- **Read-only** until explicit write mode toggle
- **Filesystem access in Rust only**: UI stays unprivileged
- **Case-insensitive path keys** (macOS): lowercased HashMap keys, original case preserved separately
- **Symlinks**: not followed; compared by target text
- **Directories always Same in Smart mode** (size/mtime not meaningful for dirs)
- **Every long operation accepts a cancellation token** (AtomicBool)
- **Progressive results**: scan progress streams via events, diffs retrieved after compare-done
- **Dark theme**: CSS custom properties for consistent theming
