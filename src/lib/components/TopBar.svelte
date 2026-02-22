<script lang="ts">
  import { compareStore } from "../stores/compare.svelte";

  function truncatePath(path: string | null, maxLen = 40): string {
    if (!path) return "Select folder\u2026";
    if (path.length <= maxLen) return path;
    return "\u2026" + path.slice(-maxLen + 1);
  }
</script>

<header class="top-bar">
  <div class="root-selectors">
    <button
      class="root-btn"
      class:selected={compareStore.leftRoot !== null}
      onclick={() => compareStore.selectRoot("left")}
      title={compareStore.leftRoot ?? "Select left folder"}
    >
      <span class="side-label">L</span>
      <span class="path-text">{truncatePath(compareStore.leftRoot)}</span>
    </button>
    <button
      class="root-btn"
      class:selected={compareStore.rightRoot !== null}
      onclick={() => compareStore.selectRoot("right")}
      title={compareStore.rightRoot ?? "Select right folder"}
    >
      <span class="side-label">R</span>
      <span class="path-text">{truncatePath(compareStore.rightRoot)}</span>
    </button>
  </div>

  <div class="controls">
    <div class="mode-toggle">
      <button
        class="mode-btn"
        class:active={compareStore.mode === "structure"}
        onclick={() => compareStore.setMode("structure")}
      >
        Structure
      </button>
      <button
        class="mode-btn"
        class:active={compareStore.mode === "smart"}
        onclick={() => compareStore.setMode("smart")}
      >
        Smart
      </button>
    </div>

    {#if compareStore.isRunning}
      <button class="action-btn cancel" onclick={() => compareStore.cancelCompare()}>
        Cancel
      </button>
    {:else}
      <button
        class="action-btn compare"
        disabled={!compareStore.canCompare}
        onclick={() => compareStore.startCompare()}
      >
        Compare
      </button>
    {/if}
  </div>
</header>

<style>
  .top-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 48px;
    padding: 0 12px;
    background: var(--surface-1);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    gap: 12px;
  }

  .root-selectors {
    display: flex;
    gap: 8px;
    flex: 1;
    min-width: 0;
  }

  .root-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 12px;
    font-family: var(--font-mono);
    max-width: 50%;
    min-width: 0;
    overflow: hidden;
    transition: border-color 0.15s;
  }

  .root-btn:hover {
    border-color: var(--accent);
  }

  .root-btn.selected {
    color: var(--text-primary);
    border-color: var(--accent-dim);
  }

  .side-label {
    font-weight: 700;
    color: var(--accent);
    flex-shrink: 0;
  }

  .path-text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .controls {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }

  .mode-toggle {
    display: flex;
    border: 1px solid var(--border);
    border-radius: 6px;
    overflow: hidden;
  }

  .mode-btn {
    padding: 4px 10px;
    background: var(--surface-2);
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 11px;
    font-weight: 500;
    transition: all 0.15s;
  }

  .mode-btn:first-child {
    border-right: 1px solid var(--border);
  }

  .mode-btn.active {
    background: var(--accent);
    color: var(--surface-0);
  }

  .action-btn {
    padding: 5px 14px;
    border: none;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    transition: opacity 0.15s;
  }

  .action-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .action-btn.compare {
    background: var(--accent);
    color: var(--surface-0);
  }

  .action-btn.cancel {
    background: var(--danger);
    color: white;
  }
</style>
