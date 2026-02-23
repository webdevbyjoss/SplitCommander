<script lang="ts">
  import { tick } from "svelte";
  import type { CompareEntry, SyncAction } from "../types";
  import { compareStore } from "../stores/compare.svelte";

  const ROW_HEIGHT = 28;
  const OVERSCAN = 10;

  let activeSide = $state<"left" | "right">("left");

  // Action menu state
  let actionMenuOpen = $state(false);
  let actionMenuActions = $state<SyncAction[]>([]);
  let actionMenuSelectedIndex = $state(0);
  let confirmAction = $state<SyncAction | null>(null);
  let executing = $state(false);

  let containerEl: HTMLDivElement | undefined = $state();
  let scrollTop = $state(0);
  let containerHeight = $state(0);

  // Navigation history: remembers selection + scroll per compareRelPath
  let navHistory = new Map<string, { selectedIndex: number; scrollTop: number }>();

  let entries = $derived(compareStore.filteredCompareEntries);

  // +1 for the ".." go-up row
  let totalItems = $derived(entries.length + 1);
  let totalHeight = $derived(totalItems * ROW_HEIGHT);

  let visibleRange = $derived.by(() => {
    const start = Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - OVERSCAN);
    const visibleCount = Math.ceil(containerHeight / ROW_HEIGHT) + OVERSCAN * 2;
    const end = Math.min(totalItems, start + visibleCount);
    return { start, end };
  });

  let offsetY = $derived(visibleRange.start * ROW_HEIGHT);

  let visibleRows = $derived.by(() => {
    const rows: Array<{ index: number; entry: CompareEntry | null }> = [];
    for (let i = visibleRange.start; i < visibleRange.end; i++) {
      if (i === 0) {
        rows.push({ index: -1, entry: null }); // ".." row
      } else {
        rows.push({ index: i - 1, entry: entries[i - 1] });
      }
    }
    return rows;
  });

  function onScroll() {
    if (containerEl) scrollTop = containerEl.scrollTop;
  }

  $effect(() => {
    if (containerEl) {
      containerHeight = containerEl.clientHeight;
      const observer = new ResizeObserver(() => {
        if (containerEl) containerHeight = containerEl.clientHeight;
      });
      observer.observe(containerEl);
      return () => observer.disconnect();
    }
  });

  function ensureVisible(index: number) {
    if (!containerEl) return;
    const rowIndex = index + 1; // +1 because ".." is row 0
    const rowTop = rowIndex * ROW_HEIGHT;
    const rowBottom = rowTop + ROW_HEIGHT;
    const viewTop = containerEl.scrollTop;
    const viewBottom = viewTop + containerEl.clientHeight;

    if (rowTop < viewTop) {
      containerEl.scrollTop = rowTop;
    } else if (rowBottom > viewBottom) {
      containerEl.scrollTop = rowBottom - containerEl.clientHeight;
    }
  }

  async function handleKeydown(e: KeyboardEvent) {
    const inInput =
      e.target instanceof HTMLInputElement ||
      e.target instanceof HTMLTextAreaElement;
    if (inInput) return;

    // Confirmation prompt handling
    if (confirmAction) {
      e.preventDefault();
      if (e.key === "y" || e.key === "Y") {
        const action = confirmAction;
        confirmAction = null;
        executing = true;
        await compareStore.executeSyncAction(action);
        executing = false;
      } else if (e.key === "n" || e.key === "N" || e.key === "Escape") {
        confirmAction = null;
      }
      return;
    }

    // Action menu navigation
    if (actionMenuOpen) {
      e.preventDefault();
      if (e.key === "Escape") {
        actionMenuOpen = false;
        return;
      }
      if (e.key === "ArrowDown") {
        actionMenuSelectedIndex = Math.min(actionMenuActions.length - 1, actionMenuSelectedIndex + 1);
        return;
      }
      if (e.key === "ArrowUp") {
        actionMenuSelectedIndex = Math.max(0, actionMenuSelectedIndex - 1);
        return;
      }
      if (e.key === "Enter") {
        const action = actionMenuActions[actionMenuSelectedIndex];
        if (action) {
          actionMenuOpen = false;
          confirmAction = action;
        }
        return;
      }
      // Number key selection (1-based)
      const num = parseInt(e.key);
      if (num >= 1 && num <= actionMenuActions.length) {
        const action = actionMenuActions[num - 1];
        actionMenuOpen = false;
        confirmAction = action;
        return;
      }
      return;
    }

    if (e.key === "ArrowDown") {
      e.preventDefault();
      if (compareStore.compareSelectedIndex < entries.length - 1) {
        compareStore.compareSelectedIndex++;
        ensureVisible(compareStore.compareSelectedIndex);
      }
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      if (compareStore.compareSelectedIndex > -1) {
        compareStore.compareSelectedIndex--;
        ensureVisible(compareStore.compareSelectedIndex);
      }
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (compareStore.compareSelectedIndex === -1) {
        await navigateUp();
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
    } else if (e.key === "Backspace") {
      e.preventDefault();
      await navigateUp();
    } else if (e.key === "Tab" && !e.metaKey && !e.ctrlKey) {
      activeSide = activeSide === "left" ? "right" : "left";
    } else if (e.key === "Home") {
      e.preventDefault();
      compareStore.compareSelectedIndex = -1;
      ensureVisible(-1);
    } else if (e.key === "End") {
      e.preventDefault();
      compareStore.compareSelectedIndex = entries.length - 1;
      ensureVisible(compareStore.compareSelectedIndex);
    } else if (e.key === "PageDown") {
      e.preventDefault();
      const pageSize = Math.floor((containerHeight || 400) / ROW_HEIGHT);
      compareStore.compareSelectedIndex = Math.min(entries.length - 1, compareStore.compareSelectedIndex + pageSize);
      ensureVisible(compareStore.compareSelectedIndex);
    } else if (e.key === "PageUp") {
      e.preventDefault();
      const pageSize = Math.floor((containerHeight || 400) / ROW_HEIGHT);
      compareStore.compareSelectedIndex = Math.max(-1, compareStore.compareSelectedIndex - pageSize);
      ensureVisible(compareStore.compareSelectedIndex);
    }
  }

  async function navigateUp() {
    if (compareStore.isAtCompareRoot) return;
    // Compute parent relPath before navigating
    const parts = compareStore.compareRelPath.split("/");
    parts.pop();
    const parentRelPath = parts.join("/");

    await compareStore.navigateCompareUp();

    // Restore saved parent state
    const saved = navHistory.get(parentRelPath);
    if (saved) {
      compareStore.compareSelectedIndex = saved.selectedIndex;
      // Update reactive scrollTop so virtual scroll renders correct rows
      scrollTop = saved.scrollTop;
      await tick();
      // Now set actual DOM scroll position
      if (containerEl) containerEl.scrollTop = saved.scrollTop;
    }
  }

  function buildFilePath(name: string): string {
    return compareStore.compareRelPath ? compareStore.compareRelPath + "/" + name : name;
  }

  function handleRowClick(index: number) {
    compareStore.compareSelectedIndex = index;
  }

  async function handleRowDblClick(index: number) {
    if (index === -1) {
      await navigateUp();
    } else {
      const entry = entries[index];
      if (entry && entry.kind === "dir") {
        navHistory.set(compareStore.compareRelPath, {
          selectedIndex: compareStore.compareSelectedIndex,
          scrollTop: containerEl?.scrollTop ?? 0,
        });
        compareStore.navigateCompareDir(entry.name);
      } else if (entry && entry.kind === "file") {
        compareStore.openFile(activeSide, buildFilePath(entry.name));
      }
    }
  }

  function formatSize(bytes: number | null): string {
    if (bytes === null || bytes === 0) return "\u2014";
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
  }

  function statusLabel(status: string): string {
    switch (status) {
      case "same": return "same";
      case "modified": return "diff";
      case "onlyLeft": return "\u2190 only";
      case "onlyRight": return "only \u2192";
      case "typeMismatch": return "type!";
      case "pending": return "";
      default: return "";
    }
  }

  function pathSegments(relPath: string): { name: string; relPath: string }[] {
    if (!relPath) return [];
    const parts = relPath.split("/");
    const segs: { name: string; relPath: string }[] = [];
    for (let i = 0; i < parts.length; i++) {
      segs.push({
        name: parts[i],
        relPath: parts.slice(0, i + 1).join("/"),
      });
    }
    return segs;
  }

  function navigateToRelPath(relPath: string) {
    compareStore.compareRelPath = relPath;
    compareStore.compareSelectedIndex = -1;
    compareStore.loadCompareDirectory();
  }

  function navigateToRoot() {
    compareStore.compareRelPath = "";
    compareStore.compareSelectedIndex = -1;
    compareStore.loadCompareDirectory();
  }

  // Close action menu when navigating away
  $effect(() => {
    compareStore.compareRelPath;
    actionMenuOpen = false;
    confirmAction = null;
  });
</script>

<svelte:window onkeydown={handleKeydown} />

<section class="compare-pane" data-testid="compare-pane">
  <div class="breadcrumb" data-testid="compare-breadcrumb">
    <span class="root-label left" title={compareStore.leftRoot ?? ""}>L: {compareStore.leftRoot}</span>
    <span class="root-sep">&harr;</span>
    <span class="root-label right" title={compareStore.rightRoot ?? ""}>R: {compareStore.rightRoot}</span>
    {#if compareStore.compareRelPath}
      <span class="crumb-sep">/</span>
      {#each pathSegments(compareStore.compareRelPath) as seg, i}
        {#if i > 0}<span class="crumb-sep">/</span>{/if}
        <button class="crumb" onclick={() => navigateToRelPath(seg.relPath)}>{seg.name}</button>
      {/each}
    {/if}
    <span class="item-count">{entries.length} items</span>
    <span class="side-indicator">{activeSide === "left" ? "L" : "R"}</span>
  </div>

  <div class="column-headers">
    <span class="col-icon"></span>
    <span class="col-left-name">Name</span>
    <span class="col-left-size">Size</span>
    <span class="col-status">Status</span>
    <span class="col-right-name">Name</span>
    <span class="col-right-size">Size</span>
  </div>

  <div class="scroll-container" bind:this={containerEl} onscroll={onScroll}>
    <div class="scroll-spacer" style:height="{totalHeight}px">
      <div class="visible-rows" style:transform="translateY({offsetY}px)">
        {#each visibleRows as row (row.index)}
          {#if row.entry === null}
            <!-- ".." go-up row -->
            <div
              class="row go-up"
              class:selected={compareStore.compareSelectedIndex === -1}
              onclick={() => handleRowClick(-1)}
              ondblclick={() => handleRowDblClick(-1)}
              role="button"
              tabindex="-1"
              data-testid="compare-row-parent"
            >
              <span class="col-icon">
                {#if !compareStore.isAtCompareRoot}
                  <svg class="kind-icon up" viewBox="0 0 16 16"><path d="M8 12V3.5m0 0L4 7.5m4-4 4 4" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>
                {/if}
              </span>
              <span class="col-left-name">{compareStore.isAtCompareRoot ? "." : ".."}</span>
              <span class="col-left-size"></span>
              <span class="col-status"></span>
              <span class="col-right-name">{compareStore.isAtCompareRoot ? "." : ".."}</span>
              <span class="col-right-size"></span>
            </div>
          {:else}
            {@const entry = row.entry}
            <div
              class="row status-{entry.status}"
              class:selected={compareStore.compareSelectedIndex === row.index}
              class:dir={entry.kind === "dir"}
              onclick={() => handleRowClick(row.index)}
              ondblclick={() => handleRowDblClick(row.index)}
              role="button"
              tabindex="-1"
              data-testid="compare-row-{entry.name}"
            >
              <span class="col-icon">
                {#if entry.kind === "dir"}
                  <svg class="kind-icon folder" viewBox="0 0 16 16"><path d="M1 3.5A1.5 1.5 0 0 1 2.5 2h2.764c.397 0 .779.158 1.06.44l1.116 1.12c.281.28.663.44 1.06.44H13.5A1.5 1.5 0 0 1 15 5.5v7a1.5 1.5 0 0 1-1.5 1.5h-11A1.5 1.5 0 0 1 1 12.5z"/></svg>
                {:else if entry.kind === "symlink"}
                  <svg class="kind-icon symlink" viewBox="0 0 16 16"><path d="M4.715 6.542 3.343 7.914a3 3 0 1 0 4.243 4.243l1.828-1.829A3 3 0 0 0 8.586 5.5L8 6.086a1 1 0 0 0-.154.199 2 2 0 0 1 .861 3.337L6.88 11.45a2 2 0 1 1-2.83-2.83l.793-.792a4.018 4.018 0 0 1-.128-1.287z"/><path d="M11.285 9.458l1.372-1.372a3 3 0 1 0-4.243-4.243L6.586 5.671A3 3 0 0 0 7.414 10.5l.586-.586a1 1 0 0 0 .154-.199 2 2 0 0 1-.861-3.337L9.12 4.55a2 2 0 1 1 2.83 2.83l-.793.792c.112.42.155.855.128 1.287z"/></svg>
                {:else}
                  <svg class="kind-icon file" viewBox="0 0 16 16"><path d="M4 0a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h8a2 2 0 0 0 2-2V4.707A1 1 0 0 0 13.707 4L10 .293A1 1 0 0 0 9.293 0H4zm5.5 1.5v2a1 1 0 0 0 1 1h2l-3-3z"/></svg>
                {/if}
              </span>

              <!-- Left side -->
              {#if entry.status === "onlyRight"}
                <span class="col-left-name placeholder">&mdash;</span>
                <span class="col-left-size placeholder">&mdash;</span>
              {:else}
                <span class="col-left-name" title={entry.name}>{entry.name}</span>
                <span class="col-left-size">{entry.kind === "dir" ? (entry.dirInfo ? formatSize(entry.dirInfo.totalSize) : "\u2014") : formatSize(entry.leftSize)}</span>
              {/if}

              <!-- Status badge -->
              <span class="col-status">
                {#if entry.status === "pending"}
                  <span class="mini-spinner" data-testid="status-pending"></span>
                {:else}
                  <span class="status-badge status-{entry.status}" data-testid="status-{entry.status}">{statusLabel(entry.status)}</span>
                {/if}
              </span>

              <!-- Right side -->
              {#if entry.status === "onlyLeft"}
                <span class="col-right-name placeholder">&mdash;</span>
                <span class="col-right-size placeholder">&mdash;</span>
              {:else}
                <span class="col-right-name" title={entry.name}>{entry.name}</span>
                <span class="col-right-size">{entry.kind === "dir" ? (entry.dirInfo ? formatSize(entry.dirInfo.totalSize) : "\u2014") : formatSize(entry.rightSize)}</span>
              {/if}
            </div>
          {/if}
        {/each}
      </div>
    </div>
  </div>

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
</section>

<style>
  .compare-pane {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    background: var(--surface-0);
  }

  .breadcrumb {
    display: flex;
    align-items: center;
    padding: 4px 10px;
    background: var(--surface-1);
    border-bottom: 1px solid var(--border);
    font-size: 12px;
    overflow-x: auto;
    white-space: nowrap;
    gap: 4px;
  }

  .root-label {
    font-weight: 600;
    color: var(--accent);
    max-width: 40%;
    overflow: hidden;
    text-overflow: ellipsis;
    flex-shrink: 1;
    min-width: 60px;
  }

  .root-sep {
    color: var(--text-secondary);
    flex-shrink: 0;
  }

  .crumb {
    background: none;
    border: none;
    color: var(--toolbar-action);
    cursor: pointer;
    font-size: 12px;
    font-family: var(--font-sans);
    padding: 1px 2px;
    border-radius: 3px;
  }

  .crumb:hover {
    background: var(--toolbar-action-hover);
  }

  .crumb-sep {
    color: var(--text-secondary);
  }

  .item-count {
    margin-left: auto;
    flex-shrink: 0;
    color: var(--text-secondary);
    font-size: 10px;
  }

  .side-indicator {
    flex-shrink: 0;
    font-size: 10px;
    font-weight: 700;
    color: var(--accent);
    background: var(--accent-dim);
    padding: 1px 5px;
    border-radius: 3px;
    margin-left: 4px;
  }

  .column-headers {
    display: flex;
    padding: 0 10px;
    background: var(--surface-1);
    border-bottom: 1px solid var(--border);
    height: 24px;
    align-items: center;
    font-size: 10px;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.4px;
  }

  .scroll-container {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
  }

  .scroll-spacer {
    position: relative;
  }

  .visible-rows {
    position: absolute;
    left: 0;
    right: 0;
  }

  .row {
    display: flex;
    align-items: center;
    height: 28px;
    padding: 0 10px;
    cursor: pointer;
    font-size: 12px;
  }

  .row:hover {
    background: var(--hover-bg);
  }

  .row.selected {
    background: var(--accent-dim);
    box-shadow: inset 2px 0 0 var(--accent);
  }

  .row.selected:hover {
    background: var(--accent-dim);
  }

  .row.dir .col-left-name,
  .row.dir .col-right-name {
    font-weight: 600;
  }

  .row.go-up {
    opacity: 0.7;
  }

  .row.go-up:hover,
  .row.go-up.selected {
    opacity: 1;
  }

  /* Status-based row styling */
  .row.status-same {
    opacity: 0.5;
  }

  .row.status-same:hover,
  .row.status-same.selected {
    opacity: 0.8;
  }

  .row.status-modified .col-left-name,
  .row.status-modified .col-right-name {
    color: var(--diff-modified);
  }

  .row.status-onlyLeft .col-right-name,
  .row.status-onlyLeft .col-right-size,
  .row.status-onlyRight .col-left-name,
  .row.status-onlyRight .col-left-size {
    opacity: 0.3;
  }

  .row.status-typeMismatch .col-left-name,
  .row.status-typeMismatch .col-right-name {
    color: var(--diff-error);
  }

  .col-icon {
    width: 24px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .kind-icon {
    width: 14px;
    height: 14px;
  }

  .kind-icon.folder { fill: #3B9CF7; }
  .kind-icon.file { fill: var(--text-secondary); }
  .kind-icon.symlink { fill: var(--text-secondary); }
  .kind-icon.up { color: var(--text-secondary); }

  .col-left-name,
  .col-right-name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--font-sans);
    color: var(--text-primary);
  }

  .col-left-size,
  .col-right-size {
    width: 65px;
    text-align: right;
    flex-shrink: 0;
    color: var(--text-secondary);
    font-family: var(--font-mono);
    font-size: 11px;
    padding: 0 4px;
  }

  .col-status {
    width: 70px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .status-badge {
    font-size: 9px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.3px;
    padding: 1px 5px;
    border-radius: 3px;
  }

  .status-badge.status-same {
    color: var(--diff-same);
    background: var(--diff-same-bg);
  }

  .status-badge.status-modified {
    color: var(--diff-modified);
    background: var(--diff-modified-bg);
  }

  .status-badge.status-onlyLeft {
    color: var(--diff-only-left);
    background: var(--diff-only-left-bg);
  }

  .status-badge.status-onlyRight {
    color: var(--diff-only-right);
    background: var(--diff-only-right-bg);
  }

  .status-badge.status-typeMismatch {
    color: var(--diff-error);
    background: var(--diff-error-bg);
  }

  .row.status-pending {
    opacity: 0.7;
  }

  .row.status-pending:hover,
  .row.status-pending.selected {
    opacity: 0.9;
  }

  .mini-spinner {
    display: inline-block;
    width: 10px;
    height: 10px;
    border: 1.5px solid var(--border);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .placeholder {
    font-style: italic;
    opacity: 0.3;
  }

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
</style>
