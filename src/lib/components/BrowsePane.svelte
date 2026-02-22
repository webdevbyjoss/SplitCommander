<script lang="ts">
  import { untrack } from "svelte";
  import type { BrowseEntry } from "../types";
  import { compareStore } from "../stores/compare.svelte";

  let {
    side,
    path,
    entries,
    isActive,
    initialState,
  }: {
    side: "left" | "right";
    path: string;
    entries: BrowseEntry[];
    isActive: boolean;
    initialState?: { selectedIndex: number; scrollTop: number } | null;
  } = $props();

  const ROW_HEIGHT = 28;
  const OVERSCAN = 10;

  // Navigation history: remembers selection + scroll per directory path
  let navHistory = new Map<string, { selectedIndex: number; scrollTop: number }>();
  // Pending scroll restoration (set before entries change, applied after)
  let pendingScrollRestore: number | null = null;
  // Whether we've consumed the initial state from app restart
  let initialStateConsumed = false;

  let isRoot = $derived(path === "/");

  // Sort state
  type SortColumn = "name" | "type" | "size" | "modified";
  let sortColumn = $state<SortColumn>("name");
  let sortAsc = $state(true);

  function toggleSort(col: SortColumn) {
    if (sortColumn === col) {
      sortAsc = !sortAsc;
    } else {
      sortColumn = col;
      sortAsc = true;
    }
  }

  function sortIndicator(col: SortColumn): string {
    if (sortColumn !== col) return "";
    return sortAsc ? " \u25B2" : " \u25BC";
  }

  // Extension helper — truncate to 4 chars max
  function getExtension(entry: BrowseEntry): string {
    if (entry.kind === "dir") return "dir";
    if (entry.kind === "symlink") return "link";
    const dot = entry.name.lastIndexOf(".");
    if (dot <= 0) return "\u2014";
    const ext = entry.name.slice(dot + 1).toLowerCase();
    return ext.length > 4 ? ext.slice(0, 4) + "\u2026" : ext;
  }

  // Middle-truncate: keep start + end visible, ellipsis in center
  const NAME_END_LEN = 8;
  function nameParts(name: string): { start: string; end: string } {
    // For short names the CSS flex won't overflow, so no visible truncation
    const endLen = Math.min(NAME_END_LEN, Math.floor(name.length / 3));
    if (endLen === 0) return { start: name, end: "" };
    return { start: name.slice(0, -endLen), end: name.slice(-endLen) };
  }

  // Filter hidden files (dotfiles) per-pane
  let showHidden = $derived(side === "left" ? compareStore.leftShowHidden : compareStore.rightShowHidden);
  let visibleEntries = $derived(
    showHidden ? entries : entries.filter((e) => !e.name.startsWith(".")),
  );

  // Sorted entries — dirs always first, then sort within groups
  let sortedEntries = $derived.by(() => {
    const sorted = [...visibleEntries];
    sorted.sort((a, b) => {
      // Dirs before files
      const aIsDir = a.kind === "dir" ? 0 : 1;
      const bIsDir = b.kind === "dir" ? 0 : 1;
      if (aIsDir !== bIsDir) return aIsDir - bIsDir;

      // Within same group, sort by column
      let cmp = 0;
      switch (sortColumn) {
        case "name":
          cmp = a.name.toLowerCase().localeCompare(b.name.toLowerCase());
          break;
        case "type":
          cmp = getExtension(a).localeCompare(getExtension(b));
          if (cmp === 0) cmp = a.name.toLowerCase().localeCompare(b.name.toLowerCase());
          break;
        case "size":
          cmp = a.size - b.size;
          if (cmp === 0) cmp = a.name.toLowerCase().localeCompare(b.name.toLowerCase());
          break;
        case "modified": {
          const am = a.modified ?? -1;
          const bm = b.modified ?? -1;
          cmp = am - bm;
          if (cmp === 0) cmp = a.name.toLowerCase().localeCompare(b.name.toLowerCase());
          break;
        }
      }
      return sortAsc ? cmp : -cmp;
    });
    return sorted;
  });

  // -1 = ".." row, 0..n = sortedEntries
  let selectedIndex = $state(-1);

  // When navigating up, remember the dir we came from to re-select it
  let selectAfterLoad: string | null = $state(null);

  let containerEl: HTMLDivElement | undefined = $state();
  let scrollTop = $state(0);
  let containerHeight = $state(0);

  // Total items = 1 (go-up row) + entries.length
  let totalItems = $derived(sortedEntries.length + 1);

  let visibleRange = $derived.by(() => {
    const start = Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - OVERSCAN);
    const visibleCount = Math.ceil(containerHeight / ROW_HEIGHT) + OVERSCAN * 2;
    const end = Math.min(totalItems, start + visibleCount);
    return { start, end };
  });

  let totalHeight = $derived(totalItems * ROW_HEIGHT);
  let offsetY = $derived(visibleRange.start * ROW_HEIGHT);

  // When entries change, restore selection + scroll from navHistory, initialState, or selectAfterLoad
  $effect(() => {
    const current = sortedEntries; // track — fires when entries change
    const pending = untrack(() => selectAfterLoad);
    const pendingScroll = untrack(() => pendingScrollRestore);
    const initState = untrack(() => (!initialStateConsumed && initialState) ? initialState : null);

    if (initState) {
      // First load after app restart — restore persisted state
      initialStateConsumed = true;
      selectedIndex = Math.min(initState.selectedIndex, current.length - 1);
      queueMicrotask(() => {
        if (containerEl) containerEl.scrollTop = initState.scrollTop;
        compareStore.reportPaneState(side, selectedIndex, initState.scrollTop);
      });
    } else if (pending) {
      // Navigating up — re-select the directory we came from
      const idx = current.findIndex((e) => e.name === pending);
      selectedIndex = idx >= 0 ? idx : -1;
      selectAfterLoad = null;
      queueMicrotask(() => {
        if (pendingScroll !== null && containerEl) {
          containerEl.scrollTop = pendingScroll;
          pendingScrollRestore = null;
        } else if (idx >= 0) {
          ensureVisible(idx);
        }
        compareStore.reportPaneState(side, selectedIndex, containerEl?.scrollTop ?? 0);
      });
    } else {
      // Normal navigation into a new directory
      selectedIndex = -1;
      if (containerEl) containerEl.scrollTop = 0;
      compareStore.reportPaneState(side, -1, 0);
    }
  });

  function onScroll() {
    if (containerEl) {
      scrollTop = containerEl.scrollTop;
      compareStore.reportPaneState(side, selectedIndex, scrollTop);
    }
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
    const rowIndex = index + 1;
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

  function handlePaneKeydown(e: KeyboardEvent) {
    if (!isActive) return;

    if (e.key === "ArrowDown") {
      e.preventDefault();
      if (selectedIndex < sortedEntries.length - 1) {
        selectedIndex++;
        ensureVisible(selectedIndex);
        compareStore.reportPaneState(side, selectedIndex, containerEl?.scrollTop ?? 0);
      }
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      if (selectedIndex > -1) {
        selectedIndex--;
        ensureVisible(selectedIndex);
        compareStore.reportPaneState(side, selectedIndex, containerEl?.scrollTop ?? 0);
      }
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (selectedIndex === -1) {
        if (!isRoot) goUp();
      } else {
        const entry = sortedEntries[selectedIndex];
        if (entry && (entry.kind === "dir" || entry.kind === "symlink")) {
          const currentPath = side === "left" ? compareStore.leftPath : compareStore.rightPath;
          navHistory.set(currentPath, { selectedIndex, scrollTop: containerEl?.scrollTop ?? 0 });
          compareStore.navigateTo(side, entry.name);
        } else if (entry && entry.kind === "file") {
          compareStore.openFile(side, entry.name);
        }
      }
    } else if (e.key === "Backspace") {
      e.preventDefault();
      if (!isRoot) goUp();
    } else if (e.key === "Home") {
      e.preventDefault();
      selectedIndex = -1;
      ensureVisible(-1);
      compareStore.reportPaneState(side, selectedIndex, containerEl?.scrollTop ?? 0);
    } else if (e.key === "End") {
      e.preventDefault();
      selectedIndex = sortedEntries.length - 1;
      ensureVisible(selectedIndex);
      compareStore.reportPaneState(side, selectedIndex, containerEl?.scrollTop ?? 0);
    } else if (e.key === "PageDown") {
      e.preventDefault();
      const pageSize = Math.floor((containerHeight || 400) / ROW_HEIGHT);
      selectedIndex = Math.min(sortedEntries.length - 1, selectedIndex + pageSize);
      ensureVisible(selectedIndex);
      compareStore.reportPaneState(side, selectedIndex, containerEl?.scrollTop ?? 0);
    } else if (e.key === "PageUp") {
      e.preventDefault();
      const pageSize = Math.floor((containerHeight || 400) / ROW_HEIGHT);
      selectedIndex = Math.max(-1, selectedIndex - pageSize);
      ensureVisible(selectedIndex);
      compareStore.reportPaneState(side, selectedIndex, containerEl?.scrollTop ?? 0);
    }
  }

  function formatSize(bytes: number): string {
    if (bytes === 0) return "\u2014";
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
  }

  function formatDate(epoch: number | null): string {
    if (epoch === null) return "\u2014";
    return new Date(epoch).toLocaleDateString(undefined, {
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  }

  // Icon kind is used in the template with inline SVGs instead of emoji

  function goUp() {
    const currentPath = side === "left" ? compareStore.leftPath : compareStore.rightPath;
    // Save current state so we can restore if we come back
    navHistory.set(currentPath, { selectedIndex, scrollTop: containerEl?.scrollTop ?? 0 });
    const parts = currentPath.split("/");
    selectAfterLoad = parts[parts.length - 1] || null;
    // Look up saved state for parent directory
    const parentPath = parts.slice(0, -1).join("/") || "/";
    const saved = navHistory.get(parentPath);
    pendingScrollRestore = saved?.scrollTop ?? null;
    compareStore.navigateUp(side);
  }

  function handleRowClick(index: number) {
    selectedIndex = index;
    compareStore.reportPaneState(side, index, containerEl?.scrollTop ?? 0);
  }

  function handleRowDblClick(index: number) {
    if (index === -1) {
      if (!isRoot) goUp();
    } else {
      const entry = sortedEntries[index];
      if (entry && (entry.kind === "dir" || entry.kind === "symlink")) {
        const currentPath = side === "left" ? compareStore.leftPath : compareStore.rightPath;
        navHistory.set(currentPath, { selectedIndex, scrollTop: containerEl?.scrollTop ?? 0 });
        compareStore.navigateTo(side, entry.name);
      } else if (entry && entry.kind === "file") {
        compareStore.openFile(side, entry.name);
      }
    }
  }

  function pathSegments(p: string): { name: string; fullPath: string }[] {
    const parts = p.split("/").filter(Boolean);
    const segments: { name: string; fullPath: string }[] = [];
    for (let i = 0; i < parts.length; i++) {
      segments.push({
        name: parts[i],
        fullPath: "/" + parts.slice(0, i + 1).join("/"),
      });
    }
    return segments;
  }

  async function navigateToPath(fullPath: string) {
    const currentPath = side === "left" ? compareStore.leftPath : compareStore.rightPath;
    navHistory.set(currentPath, { selectedIndex, scrollTop: containerEl?.scrollTop ?? 0 });
    if (side === "left") {
      compareStore.leftPath = fullPath;
    } else {
      compareStore.rightPath = fullPath;
    }
    await compareStore.loadDirectory(side);
  }

  let visibleRows = $derived.by(() => {
    const rows: Array<{ index: number; entry: BrowseEntry | null }> = [];
    for (let i = visibleRange.start; i < visibleRange.end; i++) {
      if (i === 0) {
        rows.push({ index: -1, entry: null });
      } else {
        rows.push({ index: i - 1, entry: sortedEntries[i - 1] });
      }
    }
    return rows;
  });
</script>

<svelte:window onkeydown={handlePaneKeydown} />

<section
  class="browse-pane"
  class:active={isActive}
  aria-label="{side} browse pane"
  onfocusin={() => (compareStore.activePane = side)}
  data-testid="pane-{side}"
>
  <div class="breadcrumb" data-testid="breadcrumb-{side}">
    <button
      class="picker-btn"
      onclick={() => compareStore.selectRoot(side)}
      title="Open folder"
      aria-label="Pick {side} directory"
    ><svg class="picker-icon" viewBox="0 0 16 16"><path d="M1 3.5A1.5 1.5 0 0 1 2.5 2h2.764c.397 0 .779.158 1.06.44l1.116 1.12c.281.28.663.44 1.06.44H13.5A1.5 1.5 0 0 1 15 5.5v7a1.5 1.5 0 0 1-1.5 1.5h-11A1.5 1.5 0 0 1 1 12.5z"/></svg></button>
    <button class="crumb" onclick={() => navigateToPath("/")} aria-label="Navigate to root">/</button>
    {#each pathSegments(path) as seg, i}
      {#if i > 0}<span class="crumb-sep">/</span>{/if}
      <button class="crumb" onclick={() => navigateToPath(seg.fullPath)}>{seg.name}</button>
    {/each}
    <span class="item-count">{sortedEntries.length} items</span>
  </div>

  <div class="column-headers">
    <span class="col-icon"></span>
    <button class="col-header col-name" class:active={sortColumn === "name"} onclick={() => toggleSort("name")}>
      Name{sortIndicator("name")}
    </button>
    <button class="col-header col-type" class:active={sortColumn === "type"} onclick={() => toggleSort("type")}>
      Type{sortIndicator("type")}
    </button>
    <button class="col-header col-size" class:active={sortColumn === "size"} onclick={() => toggleSort("size")}>
      Size{sortIndicator("size")}
    </button>
    <button class="col-header col-date" class:active={sortColumn === "modified"} onclick={() => toggleSort("modified")}>
      Modified{sortIndicator("modified")}
    </button>
  </div>

  <div class="scroll-container" bind:this={containerEl} onscroll={onScroll}>
    <div class="scroll-spacer" style:height="{totalHeight}px">
      <div class="visible-rows" style:transform="translateY({offsetY}px)">
        {#each visibleRows as row (row.index)}
          {#if row.entry === null}
            <div
              class="row go-up"
              class:selected={selectedIndex === -1 && isActive}
              onclick={() => handleRowClick(-1)}
              ondblclick={() => handleRowDblClick(-1)}
              onkeydown={(e) => e.key === "Enter" && handleRowDblClick(-1)}
              role="button"
              tabindex="-1"
              aria-label={isRoot ? "Current directory" : "Go to parent directory"}
              data-testid="row-parent"
            >
              <span class="col-icon">
                {#if !isRoot}
                  <svg class="kind-icon up" viewBox="0 0 16 16"><path d="M8 12V3.5m0 0L4 7.5m4-4 4 4" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>
                {/if}
              </span>
              <span class="col-name">{isRoot ? "." : ".."}</span>
              <span class="col-type"></span>
              <span class="col-size"></span>
              <span class="col-date"></span>
            </div>
          {:else}
            <div
              class="row"
              class:selected={selectedIndex === row.index && isActive}
              class:dir={row.entry.kind === "dir"}
              onclick={() => handleRowClick(row.index)}
              ondblclick={() => handleRowDblClick(row.index)}
              onkeydown={(e) => e.key === "Enter" && handleRowDblClick(row.index)}
              role="button"
              tabindex="-1"
              data-testid="row-{row.entry.name}"
            >
              <span class="col-icon">
                {#if row.entry.kind === "dir"}
                  <svg class="kind-icon folder" viewBox="0 0 16 16"><path d="M1 3.5A1.5 1.5 0 0 1 2.5 2h2.764c.397 0 .779.158 1.06.44l1.116 1.12c.281.28.663.44 1.06.44H13.5A1.5 1.5 0 0 1 15 5.5v7a1.5 1.5 0 0 1-1.5 1.5h-11A1.5 1.5 0 0 1 1 12.5z"/></svg>
                {:else if row.entry.kind === "symlink"}
                  <svg class="kind-icon symlink" viewBox="0 0 16 16"><path d="M4.715 6.542 3.343 7.914a3 3 0 1 0 4.243 4.243l1.828-1.829A3 3 0 0 0 8.586 5.5L8 6.086a1 1 0 0 0-.154.199 2 2 0 0 1 .861 3.337L6.88 11.45a2 2 0 1 1-2.83-2.83l.793-.792a4.018 4.018 0 0 1-.128-1.287z"/><path d="M11.285 9.458l1.372-1.372a3 3 0 1 0-4.243-4.243L6.586 5.671A3 3 0 0 0 7.414 10.5l.586-.586a1 1 0 0 0 .154-.199 2 2 0 0 1-.861-3.337L9.12 4.55a2 2 0 1 1 2.83 2.83l-.793.792c.112.42.155.855.128 1.287z"/></svg>
                {:else}
                  <svg class="kind-icon file" viewBox="0 0 16 16"><path d="M4 0a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h8a2 2 0 0 0 2-2V4.707A1 1 0 0 0 13.707 4L10 .293A1 1 0 0 0 9.293 0H4zm5.5 1.5v2a1 1 0 0 0 1 1h2l-3-3z"/></svg>
                {/if}
              </span>
              <span class="col-name" title={row.entry.name}><span class="name-start">{nameParts(row.entry.name).start}</span><span class="name-end">{nameParts(row.entry.name).end}</span></span>
              <span class="col-type">{getExtension(row.entry)}</span>
              <span class="col-size">{row.entry.kind === "dir" ? "\u2014" : formatSize(row.entry.size)}</span>
              <span class="col-date">{formatDate(row.entry.modified)}</span>
            </div>
          {/if}
        {/each}
      </div>
    </div>
  </div>
</section>

<style>
  .browse-pane {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    border-right: 1px solid var(--border);
    background: var(--surface-0);
  }

  .browse-pane:last-of-type {
    border-right: none;
  }

  .browse-pane.active {
    box-shadow: inset 0 0 0 1px var(--accent-dim);
  }

  .breadcrumb {
    display: flex;
    align-items: center;
    padding: 4px 10px;
    background: var(--surface-1);
    border-bottom: 1px solid var(--border);
    font-size: 12px;
    font-family: var(--font-sans);
    overflow-x: auto;
    white-space: nowrap;
  }

  .picker-btn {
    background: none;
    border: none;
    cursor: pointer;
    padding: 2px 4px;
    border-radius: 3px;
    margin-right: 2px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .picker-icon {
    width: 13px;
    height: 13px;
    fill: var(--text-secondary);
    display: block;
  }

  .picker-btn:hover .picker-icon {
    fill: var(--toolbar-action);
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
    margin: 0 1px;
  }

  .item-count {
    margin-left: auto;
    flex-shrink: 0;
    color: var(--text-secondary);
    font-size: 10px;
  }

  .column-headers {
    display: flex;
    padding: 0 10px;
    background: var(--surface-1);
    border-bottom: 1px solid var(--border);
    height: 24px;
    align-items: center;
  }

  .col-header {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 10px;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.4px;
    padding: 0;
    text-align: left;
    white-space: nowrap;
  }

  .col-header:hover {
    color: var(--text-primary);
  }

  .col-header.active {
    color: var(--toolbar-action);
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

  .row.dir .col-name {
    font-weight: 600;
  }

  .row.go-up {
    opacity: 0.7;
  }

  .row.go-up:hover,
  .row.go-up.selected {
    opacity: 1;
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

  .kind-icon.folder {
    fill: #3B9CF7;
  }

  .kind-icon.file {
    fill: var(--text-secondary);
  }

  .kind-icon.symlink {
    fill: var(--text-secondary);
  }

  .kind-icon.up {
    color: var(--text-secondary);
  }

  .col-name {
    flex: 1;
    min-width: 0;
    display: flex;
    overflow: hidden;
    white-space: nowrap;
    font-family: var(--font-sans);
    color: var(--text-primary);
  }

  .name-start {
    overflow: hidden;
    text-overflow: ellipsis;
    flex-shrink: 1;
    min-width: 20px;
  }

  .name-end {
    flex-shrink: 0;
    white-space: nowrap;
  }

  .col-type {
    width: 50px;
    flex-shrink: 0;
    color: var(--text-secondary);
    font-size: 11px;
    text-align: right;
    padding-right: 8px;
  }

  .col-size {
    width: 65px;
    text-align: right;
    flex-shrink: 0;
    color: var(--text-secondary);
    font-family: var(--font-mono);
    font-size: 11px;
  }

  .col-date {
    width: 110px;
    text-align: right;
    flex-shrink: 0;
    color: var(--text-secondary);
    font-size: 11px;
  }
</style>
