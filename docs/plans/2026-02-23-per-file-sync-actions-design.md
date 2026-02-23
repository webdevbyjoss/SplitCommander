# Per-File Sync Actions in Compare Mode

## Overview

Add the ability to resolve individual file differences directly from the compare screen. Users press Enter on a non-same entry to open an action menu with context-appropriate operations (copy, delete, overwrite). All actions happen within the existing compare screen.

## Action Matrix

| Status | Entry Type | Menu Options |
|--------|-----------|-------------|
| onlyLeft | file/dir | 1. Copy to right, 2. Delete from left |
| onlyRight | file/dir | 1. Copy to left, 2. Delete from right |
| metaDiff | file | 1. Copy left->right, 2. Copy right->left, 3. Delete left, 4. Delete right |
| metaDiff | directory | No menu - Enter navigates inside for manual resolution |
| same | any | No menu - Enter navigates (dir) or opens (file) |
| typeMismatch | any | No menu |

## Interaction Flow

1. User navigates to a non-same entry in compare mode
2. Press Enter opens action menu popup anchored to the selected row
3. User picks action via number key (1-4) or arrow keys + Enter
4. Confirmation prompt: "Copy file.txt to right? (y/n)"
5. On y: execute operation, re-compare current directory
6. On n/Esc: dismiss, return to list
7. Entry status updates naturally (e.g. onlyLeft becomes same after copy)

## Execution Details

- **Copy (no overwrite):** Use existing `copy_entry` Tauri command
- **Copy (overwrite for metaDiff):** New `copy_entry_overwrite` command - deletes destination then copies
- **Delete:** Use existing `delete_entry` Tauri command
- **Post-action:** Call `loadCompareDirectory()` to refresh. Status updates naturally. Hidden by showIdentical toggle if now "same".
- **Errors:** Display in footer status area (existing error mechanism)

## Rust Changes

- Add `copy_entry_overwrite` command (delete destination + copy source) or add `force` boolean flag to existing `copy_entry`

## Frontend Changes

- **ComparePane.svelte:** Action menu popup (positioned at selected row), confirmation prompt, Enter key handling branching (actionable entries open menu, dirs navigate, files open)
- **Compare store:** `executeSyncAction()` method - calls Tauri commands then refreshes directory
- **No new screens or flows**
