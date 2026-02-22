<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { compareStore } from "./lib/stores/compare.svelte";
  import TopBar from "./lib/components/TopBar.svelte";
  import FilePane from "./lib/components/FilePane.svelte";
  import DiffDetails from "./lib/components/DiffDetails.svelte";
  import BottomBar from "./lib/components/BottomBar.svelte";
  import ProgressIndicator from "./lib/components/ProgressIndicator.svelte";

  onMount(() => {
    compareStore.init();
  });

  onDestroy(() => {
    compareStore.destroy();
  });

  function handleKeydown(e: KeyboardEvent) {
    const meta = e.metaKey || e.ctrlKey;

    if (e.key === "Tab" && !meta) {
      e.preventDefault();
      compareStore.switchPane();
    } else if (meta && e.key === "d") {
      e.preventDefault();
      compareStore.toggleDetails();
    } else if (meta && e.key === "e") {
      e.preventDefault();
      if (compareStore.phase === "done") {
        compareStore.exportReport();
      }
    } else if (meta && e.key === "r") {
      e.preventDefault();
      if (compareStore.canCompare) {
        compareStore.startCompare();
      }
    } else if (meta && e.key === ".") {
      e.preventDefault();
      if (compareStore.isRunning) {
        compareStore.cancelCompare();
      }
    } else if (meta && e.key === "f") {
      e.preventDefault();
      // Focus search — future enhancement
    }
  }

  let leftItems = $derived(
    compareStore.filteredDiffs.filter(
      (d) => d.diffKind !== "onlyRight",
    ),
  );

  let rightItems = $derived(
    compareStore.filteredDiffs.filter(
      (d) => d.diffKind !== "onlyLeft",
    ),
  );
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="app">
  <TopBar />

  <div class="pane-area">
    {#if compareStore.phase === "idle" && compareStore.diffs.length === 0}
      <div class="empty-state">
        <div class="empty-content">
          <h2>SplitCommander</h2>
          <p>Select two folders to compare their contents.</p>
          <div class="empty-actions">
            <button class="empty-btn" onclick={() => compareStore.selectRoot("left")}>
              Select Left Folder
            </button>
            <button class="empty-btn" onclick={() => compareStore.selectRoot("right")}>
              Select Right Folder
            </button>
          </div>
        </div>
      </div>
    {:else}
      <FilePane
        side="left"
        items={leftItems}
        isActive={compareStore.activePane === "left"}
      />
      <FilePane
        side="right"
        items={rightItems}
        isActive={compareStore.activePane === "right"}
      />
      {#if compareStore.showDetails && compareStore.selectedItem}
        <DiffDetails item={compareStore.selectedItem} />
      {/if}
    {/if}

    <ProgressIndicator />
  </div>

  <BottomBar />
</div>

<style>
  :global(:root) {
    --surface-0: #1a1a2e;
    --surface-1: #16213e;
    --surface-2: #0f3460;
    --border: #2a2a4a;
    --accent: #4fc3f7;
    --accent-dim: rgba(79, 195, 247, 0.15);
    --danger: #f44336;
    --text-primary: #e0e0e0;
    --text-secondary: #8888aa;
    --font-mono: "SF Mono", "Menlo", "Monaco", "Consolas", monospace;
    --font-sans: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
  }

  :global(*) {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
  }

  :global(body) {
    font-family: var(--font-sans);
    background: var(--surface-0);
    color: var(--text-primary);
    overflow: hidden;
    -webkit-font-smoothing: antialiased;
  }

  .app {
    display: flex;
    flex-direction: column;
    height: 100vh;
    width: 100vw;
  }

  .pane-area {
    display: flex;
    flex: 1;
    min-height: 0;
    position: relative;
  }

  .empty-state {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .empty-content {
    text-align: center;
  }

  .empty-content h2 {
    font-size: 24px;
    font-weight: 700;
    color: var(--accent);
    margin-bottom: 8px;
  }

  .empty-content p {
    color: var(--text-secondary);
    font-size: 14px;
    margin-bottom: 20px;
  }

  .empty-actions {
    display: flex;
    gap: 12px;
    justify-content: center;
  }

  .empty-btn {
    padding: 8px 20px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 8px;
    color: var(--text-primary);
    cursor: pointer;
    font-size: 13px;
    font-weight: 500;
    transition: all 0.15s;
  }

  .empty-btn:hover {
    border-color: var(--accent);
    background: var(--accent-dim);
  }
</style>
