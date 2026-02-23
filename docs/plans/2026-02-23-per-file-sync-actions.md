# Per-File Sync Actions Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add per-file sync actions (copy, delete, overwrite) in compare mode via an Enter-triggered action menu with confirmation.

**Architecture:** Enter key on actionable entries opens an inline action menu popup. User picks an action by number key, confirms with y/n. Actions use existing Tauri commands (`copy_entry`, `delete_entry`) plus a new `copy_entry_overwrite` command. After execution, the current compare directory is refreshed.

**Tech Stack:** Rust (fileops + commands), Svelte 5 (ComparePane + compare store), TypeScript, Tauri IPC

---

### Task 1: Add `copy_entry_overwrite` to Rust fileops

**Files:**
- Modify: `src-tauri/src/core/fileops.rs:7-25` (add new function after `copy_entry`)

**Step 1: Write the failing test**

Add to the existing test module in `src-tauri/src/core/fileops.rs`:

```rust
#[test]
fn test_copy_overwrite_file() {
    let dir = test_dir("copy_overwrite");
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::create_dir_all(dir.join("dst")).unwrap();
    fs::write(dir.join("src/test.txt"), "new content").unwrap();
    fs::write(dir.join("dst/test.txt"), "old content").unwrap();

    let result = copy_entry_overwrite(&dir.join("src/test.txt"), &dir.join("dst"));
    assert!(result.is_ok());
    assert_eq!(fs::read_to_string(dir.join("dst/test.txt")).unwrap(), "new content");
    // Source still exists
    assert!(dir.join("src/test.txt").exists());

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_copy_overwrite_no_existing() {
    let dir = test_dir("copy_overwrite_new");
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::create_dir_all(dir.join("dst")).unwrap();
    fs::write(dir.join("src/test.txt"), "hello").unwrap();

    let result = copy_entry_overwrite(&dir.join("src/test.txt"), &dir.join("dst"));
    assert!(result.is_ok());
    assert_eq!(fs::read_to_string(dir.join("dst/test.txt")).unwrap(), "hello");

    let _ = fs::remove_dir_all(&dir);
}
```

**Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test test_copy_overwrite`
Expected: FAIL — `copy_entry_overwrite` not found

**Step 3: Write the implementation**

Add to `src-tauri/src/core/fileops.rs` after the `copy_entry` function (after line 25):

```rust
/// Copies a file or directory from `src` to `dest_dir/<src_name>`, overwriting if destination exists.
pub fn copy_entry_overwrite(src: &Path, dest_dir: &Path) -> Result<PathBuf, String> {
    let name = src
        .file_name()
        .ok_or_else(|| "Invalid source path".to_string())?;
    let dest = dest_dir.join(name);

    // Remove existing destination if present
    if dest.exists() {
        if dest.is_dir() {
            fs::remove_dir_all(&dest).map_err(|e| format!("Cannot remove existing: {}", e))?;
        } else {
            fs::remove_file(&dest).map_err(|e| format!("Cannot remove existing: {}", e))?;
        }
    }

    if src.is_dir() {
        copy_dir_recursive(src, &dest)?;
    } else {
        fs::copy(src, &dest).map_err(|e| format!("Copy failed: {}", e))?;
    }

    Ok(dest)
}
```

**Step 4: Run test to verify it passes**

Run: `cd src-tauri && cargo test test_copy_overwrite`
Expected: PASS (both tests)

**Step 5: Commit**

```bash
git add src-tauri/src/core/fileops.rs
git commit -m "feat: add copy_entry_overwrite for file sync in compare mode"
```

---

### Task 2: Add `copy_entry_overwrite` Tauri command

**Files:**
- Modify: `src-tauri/src/core/commands.rs:288-305` (add new command after `copy_entry`)
- Modify: `src-tauri/src/lib.rs:10-34` (register new command)

**Step 1: Add the Tauri command**

Add to `src-tauri/src/core/commands.rs` after the `copy_entry` command (after line 305):

```rust
/// Copies a file or directory, overwriting destination if it exists.
#[tauri::command]
pub async fn copy_entry_overwrite(source_path: String, dest_dir: String) -> Result<(), String> {
    let src = PathBuf::from(&source_path);
    let dst = PathBuf::from(&dest_dir);

    if !src.exists() {
        return Err(format!("Source does not exist: {}", source_path));
    }
    if !dst.is_dir() {
        return Err(format!("Destination is not a directory: {}", dest_dir));
    }

    tokio::task::spawn_blocking(move || fileops::copy_entry_overwrite(&src, &dst))
        .await
        .map_err(|e| format!("Task failed: {}", e))?
        .map(|_| ())
}
```

**Step 2: Register in lib.rs**

Add `core::commands::copy_entry_overwrite,` to the `invoke_handler` list in `src-tauri/src/lib.rs` after `core::commands::copy_entry,` (after line 20).

**Step 3: Verify compilation**

Run: `cd src-tauri && cargo check`
Expected: compiles with no new errors

**Step 4: Run all tests**

Run: `cd src-tauri && cargo test`
Expected: all tests pass (42 total now)

**Step 5: Commit**

```bash
git add src-tauri/src/core/commands.rs src-tauri/src/lib.rs
git commit -m "feat: expose copy_entry_overwrite as Tauri command"
```

---

### Task 3: Add `SyncAction` type and `executeSyncAction` to compare store

**Files:**
- Modify: `src/lib/types.ts` (add SyncAction type)
- Modify: `src/lib/stores/compare.svelte.ts:456-530` (add sync action methods)

**Step 1: Add the SyncAction type**

Add to `src/lib/types.ts`:

```typescript
export type SyncActionKind = "copyToRight" | "copyToLeft" | "deleteLeft" | "deleteRight" | "overwriteToRight" | "overwriteToLeft";

export interface SyncAction {
  kind: SyncActionKind;
  label: string;
  entryName: string;
}
```

**Step 2: Add helper to get available actions for an entry**

Add to `src/lib/stores/compare.svelte.ts` before the closing `}` of the class (before line 578):

```typescript
  getSyncActions(entry: CompareEntry): SyncAction[] {
    const name = entry.name;
    if (entry.status === "same" || entry.status === "pending" || entry.status === "typeMismatch") {
      return [];
    }
    // metaDiff directories: no actions (navigate inside instead)
    if (entry.status === "modified" && entry.kind === "dir") {
      return [];
    }
    const actions: SyncAction[] = [];
    if (entry.status === "onlyLeft") {
      actions.push({ kind: "copyToRight", label: `Copy to right`, entryName: name });
      actions.push({ kind: "deleteLeft", label: `Delete from left`, entryName: name });
    } else if (entry.status === "onlyRight") {
      actions.push({ kind: "copyToLeft", label: `Copy to left`, entryName: name });
      actions.push({ kind: "deleteRight", label: `Delete from right`, entryName: name });
    } else if (entry.status === "modified") {
      // metaDiff file: both directions + delete each
      actions.push({ kind: "overwriteToRight", label: `Copy left → right`, entryName: name });
      actions.push({ kind: "overwriteToLeft", label: `Copy right → left`, entryName: name });
      actions.push({ kind: "deleteLeft", label: `Delete from left`, entryName: name });
      actions.push({ kind: "deleteRight", label: `Delete from right`, entryName: name });
    }
    return actions;
  }

  async executeSyncAction(action: SyncAction): Promise<boolean> {
    const leftDir = this.compareRelPath
      ? this.leftRoot + "/" + this.compareRelPath
      : this.leftRoot!;
    const rightDir = this.compareRelPath
      ? this.rightRoot + "/" + this.compareRelPath
      : this.rightRoot!;
    const leftPath = leftDir + "/" + action.entryName;
    const rightPath = rightDir + "/" + action.entryName;

    try {
      switch (action.kind) {
        case "copyToRight":
          await invoke("copy_entry", { sourcePath: leftPath, destDir: rightDir });
          break;
        case "copyToLeft":
          await invoke("copy_entry", { sourcePath: rightPath, destDir: leftDir });
          break;
        case "overwriteToRight":
          await invoke("copy_entry_overwrite", { sourcePath: leftPath, destDir: rightDir });
          break;
        case "overwriteToLeft":
          await invoke("copy_entry_overwrite", { sourcePath: rightPath, destDir: leftDir });
          break;
        case "deleteLeft":
          await invoke("delete_entry", { targetPath: leftPath });
          break;
        case "deleteRight":
          await invoke("delete_entry", { targetPath: rightPath });
          break;
      }
      // Refresh the current compare directory
      await this.loadCompareDirectory();
      return true;
    } catch (e) {
      this.setError(`Sync failed: ${e}`);
      return false;
    }
  }
```

**Step 3: Add SyncAction imports to types.ts**

Ensure the new types are exported from `src/lib/types.ts`.

**Step 4: Verify type-checking**

Run: `npm run check`
Expected: no new errors from these changes

**Step 5: Commit**

```bash
git add src/lib/types.ts src/lib/stores/compare.svelte.ts
git commit -m "feat: add sync action types and executeSyncAction to compare store"
```

---

### Task 4: Build the action menu popup in ComparePane

**Files:**
- Modify: `src/lib/components/ComparePane.svelte` (add action menu UI + confirmation + keyboard handling)

**Step 1: Add action menu state and imports**

At the top of the `<script>` block in `ComparePane.svelte`, add imports and state:

```typescript
import type { CompareEntry, SyncAction } from "../types";
// ... existing imports ...

// Action menu state
let actionMenuOpen = $state(false);
let actionMenuActions = $state<SyncAction[]>([]);
let actionMenuSelectedIndex = $state(0);
let confirmAction = $state<SyncAction | null>(null);
let executing = $state(false);
```

**Step 2: Modify Enter key handling**

Replace the existing Enter key handler (lines 92-121) in `handleKeydown` to branch based on whether the entry has sync actions:

```typescript
} else if (e.key === "Enter") {
  e.preventDefault();
  if (confirmAction) {
    // Confirmation is showing — do nothing (y/n handled separately)
    return;
  }
  if (actionMenuOpen) {
    // Pick the selected action from the menu
    const action = actionMenuActions[actionMenuSelectedIndex];
    if (action) {
      actionMenuOpen = false;
      confirmAction = action;
    }
    return;
  }
  if (compareStore.compareSelectedIndex === -1) {
    if (!compareStore.isAtCompareRoot) {
      const relPath = compareStore.compareRelPath;
      navHistory.set(relPath, {
        selectedIndex: compareStore.compareSelectedIndex,
        scrollTop: containerEl?.scrollTop ?? 0,
      });
      compareStore.navigateCompareUp();
    }
  } else {
    const entry = entries[compareStore.compareSelectedIndex];
    if (!entry) return;
    const actions = compareStore.getSyncActions(entry);
    if (actions.length > 0) {
      // Open action menu
      actionMenuActions = actions;
      actionMenuSelectedIndex = 0;
      actionMenuOpen = true;
    } else if (entry.kind === "dir") {
      navHistory.set(compareStore.compareRelPath, {
        selectedIndex: compareStore.compareSelectedIndex,
        scrollTop: containerEl?.scrollTop ?? 0,
      });
      compareStore.navigateCompareDir(entry.name);
    } else if (entry.kind === "file") {
      const side = activeSide;
      if (entry.status === "onlyRight" && side === "left") {
        compareStore.openFile("right", buildFilePath(entry.name));
      } else if (entry.status === "onlyLeft" && side === "right") {
        compareStore.openFile("left", buildFilePath(entry.name));
      } else {
        compareStore.openFile(side, buildFilePath(entry.name));
      }
    }
  }
```

**Step 3: Add action menu and confirmation keyboard handling**

Add additional key handlers inside `handleKeydown`, before the existing ArrowDown/ArrowUp handlers:

```typescript
// Action menu navigation
if (actionMenuOpen) {
  if (e.key === "Escape") {
    e.preventDefault();
    actionMenuOpen = false;
    return;
  }
  if (e.key === "ArrowDown") {
    e.preventDefault();
    actionMenuSelectedIndex = Math.min(actionMenuActions.length - 1, actionMenuSelectedIndex + 1);
    return;
  }
  if (e.key === "ArrowUp") {
    e.preventDefault();
    actionMenuSelectedIndex = Math.max(0, actionMenuSelectedIndex - 1);
    return;
  }
  // Number key selection (1-based)
  const num = parseInt(e.key);
  if (num >= 1 && num <= actionMenuActions.length) {
    e.preventDefault();
    const action = actionMenuActions[num - 1];
    actionMenuOpen = false;
    confirmAction = action;
    return;
  }
  e.preventDefault();
  return;
}

// Confirmation prompt handling
if (confirmAction) {
  if (e.key === "y" || e.key === "Y") {
    e.preventDefault();
    const action = confirmAction;
    confirmAction = null;
    executing = true;
    await compareStore.executeSyncAction(action);
    executing = false;
    return;
  }
  if (e.key === "n" || e.key === "N" || e.key === "Escape") {
    e.preventDefault();
    confirmAction = null;
    return;
  }
  e.preventDefault();
  return;
}
```

Note: The `handleKeydown` function needs to become `async` for the `await` call.

**Step 4: Add action menu and confirmation popup HTML**

Add after the `.scroll-container` div closing tag (before `</section>`):

```svelte
{#if actionMenuOpen}
  <div class="action-menu-overlay" role="dialog">
    <div class="action-menu">
      <div class="action-menu-title">{actionMenuActions[0]?.entryName}</div>
      {#each actionMenuActions as action, i}
        <button
          class="action-menu-item"
          class:selected={i === actionMenuSelectedIndex}
          onclick={() => { actionMenuOpen = false; confirmAction = action; }}
        >
          <span class="action-num">{i + 1}</span>
          {action.label}
        </button>
      {/each}
      <div class="action-menu-hint">Esc cancel</div>
    </div>
  </div>
{/if}

{#if confirmAction}
  <div class="action-menu-overlay" role="dialog">
    <div class="action-menu confirm">
      <div class="action-menu-title">{confirmAction.label}</div>
      <div class="confirm-target">{confirmAction.entryName}</div>
      <div class="confirm-prompt">
        {#if executing}
          <span class="mini-spinner"></span> Executing...
        {:else}
          Press <kbd>y</kbd> to confirm, <kbd>n</kbd> to cancel
        {/if}
      </div>
    </div>
  </div>
{/if}
```

**Step 5: Add CSS for action menu and confirmation**

Add to the `<style>` block:

```css
.action-menu-overlay {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--overlay-bg);
  z-index: 100;
}

.action-menu {
  background: var(--surface-1);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 8px 0;
  min-width: 220px;
  box-shadow: 0 8px 24px var(--shadow);
}

.action-menu-title {
  padding: 4px 16px 8px;
  font-size: 12px;
  font-weight: 600;
  color: var(--text-primary);
  border-bottom: 1px solid var(--border);
  margin-bottom: 4px;
}

.action-menu-item {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 6px 16px;
  border: none;
  background: none;
  color: var(--text-primary);
  font-size: 12px;
  font-family: var(--font-sans);
  cursor: pointer;
  text-align: left;
}

.action-menu-item:hover,
.action-menu-item.selected {
  background: var(--accent-dim);
}

.action-num {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  background: var(--surface-2);
  border: 1px solid var(--border);
  border-radius: 3px;
  font-family: var(--font-mono);
  font-size: 10px;
  font-weight: 600;
  flex-shrink: 0;
}

.action-menu-hint {
  padding: 6px 16px 2px;
  font-size: 10px;
  color: var(--text-secondary);
  border-top: 1px solid var(--border);
  margin-top: 4px;
}

.action-menu.confirm {
  padding: 16px;
  text-align: center;
}

.action-menu.confirm .action-menu-title {
  border-bottom: none;
  padding: 0 0 4px;
  text-align: center;
}

.confirm-target {
  font-family: var(--font-mono);
  font-size: 12px;
  color: var(--accent);
  padding: 4px 0 12px;
}

.confirm-prompt {
  font-size: 11px;
  color: var(--text-secondary);
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 4px;
}

.confirm-prompt kbd {
  display: inline-block;
  padding: 0 4px;
  background: var(--surface-2);
  border: 1px solid var(--border);
  border-radius: 3px;
  font-family: var(--font-mono);
  font-size: 10px;
  line-height: 1.4;
}
```

**Step 6: Close action menu on navigation away**

Add an `$effect` that closes the menu when selected index or relPath changes:

```typescript
$effect(() => {
  // Close action menu when navigating away
  compareStore.compareRelPath;
  actionMenuOpen = false;
  confirmAction = null;
});
```

**Step 7: Verify type-checking and visual test**

Run: `npm run check`
Expected: no new errors

Run: `npm run tauri dev`
Expected: Action menu appears on Enter for onlyLeft/onlyRight/modified file entries. Confirmation works with y/n. After action completes, directory refreshes.

**Step 8: Commit**

```bash
git add src/lib/components/ComparePane.svelte
git commit -m "feat: add action menu and confirmation UI for per-file sync"
```

---

### Task 5: Integration test — manual verification

**Step 1: Create test directories**

```bash
mkdir -p /tmp/sc-test-left /tmp/sc-test-right
echo "shared" > /tmp/sc-test-left/shared.txt
echo "shared" > /tmp/sc-test-right/shared.txt
echo "left only" > /tmp/sc-test-left/only-left.txt
echo "right only" > /tmp/sc-test-right/only-right.txt
echo "version 1" > /tmp/sc-test-left/modified.txt
echo "version 2" > /tmp/sc-test-right/modified.txt
mkdir -p /tmp/sc-test-left/only-left-dir
echo "in dir" > /tmp/sc-test-left/only-left-dir/nested.txt
```

**Step 2: Run the app and test**

Run: `npm run tauri dev`

Test matrix:
1. Navigate left pane to `/tmp/sc-test-left`, right to `/tmp/sc-test-right`
2. Press `g` to compare
3. Select `only-left.txt` → press Enter → menu shows "Copy to right" + "Delete from left"
4. Press `1` → confirmation shows → press `y` → file copied, status becomes "same"
5. Select `only-right.txt` → press Enter → menu shows "Copy to left" + "Delete from right"
6. Press `2` → confirmation → press `y` → file deleted from right, row disappears
7. Select `modified.txt` → press Enter → menu shows 4 options (copy L→R, copy R→L, delete left, delete right)
8. Press `1` → confirm → left version overwrites right, status becomes "same"
9. Press Esc during menu → menu closes, back to list
10. Press `n` during confirmation → cancels, back to list

**Step 3: Verify edge cases**
- Enter on "same" entry (file) → opens with default app (no menu)
- Enter on "same" entry (dir) → navigates into directory
- Enter on "modified" directory → navigates into directory (no menu)
- Enter on ".." → navigates up
- Number keys 1-4 select actions correctly
- Arrow keys navigate action menu

**Step 4: Clean up**

```bash
rm -rf /tmp/sc-test-left /tmp/sc-test-right
```

**Step 5: Final commit (if any fixes needed)**

```bash
git add -A
git commit -m "fix: address issues found during manual sync action testing"
```
