<script lang="ts">
  import type { DiffItem } from "../types";
  import DiffBadge from "./DiffBadge.svelte";
  import { compareStore } from "../stores/compare.svelte";

  let {
    side,
    items,
    isActive,
  }: {
    side: "left" | "right";
    items: DiffItem[];
    isActive: boolean;
  } = $props();

  const ROW_HEIGHT = 28;
  const OVERSCAN = 10;

  let containerEl: HTMLDivElement | undefined = $state();
  let scrollTop = $state(0);
  let containerHeight = $state(0);

  let visibleRange = $derived.by(() => {
    const start = Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - OVERSCAN);
    const visibleCount = Math.ceil(containerHeight / ROW_HEIGHT) + OVERSCAN * 2;
    const end = Math.min(items.length, start + visibleCount);
    return { start, end };
  });

  let visibleItems = $derived(items.slice(visibleRange.start, visibleRange.end));
  let totalHeight = $derived(items.length * ROW_HEIGHT);
  let offsetY = $derived(visibleRange.start * ROW_HEIGHT);

  function onScroll() {
    if (containerEl) {
      scrollTop = containerEl.scrollTop;
    }
  }

  function onResize() {
    if (containerEl) {
      containerHeight = containerEl.clientHeight;
    }
  }

  $effect(() => {
    if (containerEl) {
      containerHeight = containerEl.clientHeight;
      const observer = new ResizeObserver(() => onResize());
      observer.observe(containerEl);
      return () => observer.disconnect();
    }
  });

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

  function getEntryMeta(item: DiffItem) {
    return side === "left" ? item.left : item.right;
  }

  function kindIcon(item: DiffItem): string {
    const meta = getEntryMeta(item);
    if (!meta) {
      // Entry only exists on other side
      const otherMeta = side === "left" ? item.right : item.left;
      if (otherMeta?.kind === "dir") return "\ud83d\udcc1";
      return "\ud83d\udcc4";
    }
    if (meta.kind === "dir") return "\ud83d\udcc2";
    if (meta.kind === "symlink") return "\ud83d\udd17";
    return "\ud83d\udcc4";
  }

  function fileName(relPath: string): string {
    const parts = relPath.split("/");
    return parts[parts.length - 1];
  }

  function handleRowClick(item: DiffItem) {
    compareStore.selectItem(item);
  }
</script>

<section
  class="file-pane"
  class:active={isActive}
  aria-label="{side} file pane"
  onfocusin={() => (compareStore.activePane = side)}
>
  <div class="pane-header">
    <span class="pane-label">{side === "left" ? "Left" : "Right"}</span>
    <span class="item-count">{items.length} items</span>
  </div>

  <div class="column-headers">
    <span class="col-icon"></span>
    <span class="col-name">Name</span>
    <span class="col-size">Size</span>
    <span class="col-date">Modified</span>
    <span class="col-status">Status</span>
  </div>

  <div class="scroll-container" bind:this={containerEl} onscroll={onScroll}>
    <div class="scroll-spacer" style:height="{totalHeight}px">
      <div class="visible-rows" style:transform="translateY({offsetY}px)">
        {#each visibleItems as item (item.relPath)}
          {@const meta = getEntryMeta(item)}
          <div
            class="row"
            class:selected={compareStore.selectedItem?.relPath === item.relPath}
            class:ghost={!meta}
            onclick={() => handleRowClick(item)}
            onkeydown={(e) => e.key === "Enter" && handleRowClick(item)}
            role="button"
            tabindex="-1"
          >
            <span class="col-icon">{kindIcon(item)}</span>
            <span class="col-name" title={item.relPath}>{fileName(item.relPath)}</span>
            <span class="col-size">{meta ? formatSize(meta.size) : "\u2014"}</span>
            <span class="col-date">{meta ? formatDate(meta.modified) : "\u2014"}</span>
            <span class="col-status"><DiffBadge kind={item.diffKind} /></span>
          </div>
        {/each}
      </div>
    </div>
  </div>
</section>

<style>
  .file-pane {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    border-right: 1px solid var(--border);
    background: var(--surface-0);
  }

  .file-pane:last-child {
    border-right: none;
  }

  .file-pane.active {
    box-shadow: inset 0 0 0 1px var(--accent-dim);
  }

  .pane-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 6px 10px;
    background: var(--surface-1);
    border-bottom: 1px solid var(--border);
    font-size: 11px;
  }

  .pane-label {
    font-weight: 700;
    color: var(--accent);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .item-count {
    color: var(--text-secondary);
  }

  .column-headers {
    display: flex;
    padding: 4px 10px;
    background: var(--surface-1);
    border-bottom: 1px solid var(--border);
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
    transition: background 0.1s;
    font-size: 12px;
  }

  .row:hover {
    background: var(--surface-2);
  }

  .row.selected {
    background: var(--accent-dim);
  }

  .row.ghost {
    opacity: 0.4;
  }

  .col-icon {
    width: 24px;
    flex-shrink: 0;
    font-size: 13px;
  }

  .col-name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--font-mono);
    color: var(--text-primary);
  }

  .col-size {
    width: 70px;
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

  .col-status {
    width: 80px;
    text-align: right;
    flex-shrink: 0;
  }
</style>
