# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

SplitCommander is a keyboard-first, two-pane file manager for macOS with best-in-class directory comparison. Built with Tauri v2 (Rust backend) and React + TypeScript (frontend). Currently read-only (comparison only, no write operations).

## Build & Development Commands

```bash
# Install frontend dependencies
npm install

# Run in development mode (launches Tauri dev window)
npm run tauri dev

# Build for production
npm run tauri build

# Run Rust tests
cd src-tauri && cargo test

# Run a single Rust test
cd src-tauri && cargo test test_name

# Run frontend tests
npm test

# Lint
npm run lint
cd src-tauri && cargo clippy
```

## Architecture

### Two-process model (Tauri v2)
- **Rust backend** (`src-tauri/`): filesystem scanning, comparison engine, hashing, event emission
- **React frontend** (`src/`): UI rendering, receives events from Rust via Tauri event system

### Communication pattern
- **Commands** (UI → Rust): `select_roots`, `start_compare`, `cancel_compare`, `export_report`
- **Events** (Rust → UI): `ScanProgress`, `DiffBatch`, `HashProgress`, `CompareDone`

### Rust core engine (`src-tauri/src/core/`)
Three-phase comparison pipeline:
1. **Scan** (`scan.rs`): Walk both roots, build `Map<RelPath, EntryMeta>`, emit early diffs (OnlyLeft, OnlyRight, TypeMismatch)
2. **Metadata compare** (`compare.rs`): Size, mtime, symlink targets → MetaDiff or MetaSameCandidate
3. **Deep verify** (`hash.rs`): BLAKE3 streaming hash, parallel, cancellable → Same or ContentDiff

Key modules:
- `model.rs` — core types: `EntryKind`, `EntryMeta`, `Signature`, `DiffKind`, `DiffItem`, `CompareSummary`
- `ignore.rs` — glob rules + macOS noise preset (`.DS_Store`, `._*`, `.Spotlight-V100`, `.Trashes`)
- `security.rs` — root confinement: reject paths escaping selected roots via `..` or symlink tricks
- `export.rs` — JSON/CSV diff report generation
- `commands.rs` — Tauri command handlers
- `events.rs` — event types and emit helpers

### Compare modes
- **Structure**: presence + type only
- **Smart** (default): presence + type + metadata (size, mtime with optional tolerance)
- **Deep**: Smart + BLAKE3 content hashing

### Diff categories
`OnlyLeft` | `OnlyRight` | `TypeMismatch` | `Same` | `MetaDiff` | `ContentDiff` | `Error`

## Key Design Decisions

- **Read-only until explicit write mode**: no file modification in v0.1–v0.2
- **Filesystem access in Rust only**: UI stays unprivileged; no Tauri FS plugin exposure
- **Roots chosen via OS file picker**: Rust enforces path confinement to selected roots
- **Case-insensitive path keys by default** (macOS): detect collisions (e.g., `a.txt` vs `A.txt`)
- **Symlinks**: not followed; compared by target text (`read_link`); cycle detection deferred
- **Parallelism**: bounded worker pool (Rayon or tokio tasks), hash queue with backpressure
- **Every long operation accepts a cancellation token**: scan, metadata, hashing
- **Progressive results**: structure diffs appear immediately, metadata diffs next, content diffs stream in as hashing completes

## Performance Constraints

- Handle 200k+ files without UI freeze
- Structure diffs: <1s typical repos, <5s huge trees
- Hash workers throttled to avoid thermal issues
- Compact metadata storage with path interning (avoid full file lists in memory)
