<script lang="ts">
  import { compareStore } from "../stores/compare.svelte";
</script>

<header class="top-bar">
  <div class="controls">
    {#if compareStore.appMode === "compare"}
      <button class="action-btn back" onclick={() => compareStore.backToBrowse()}>
        Browse
      </button>
    {/if}

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
    justify-content: flex-end;
    height: 36px;
    padding: 0 12px;
    background: var(--surface-1);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    gap: 8px;
  }

  .controls {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .mode-toggle {
    display: flex;
    border: 1px solid var(--border);
    border-radius: 6px;
    overflow: hidden;
  }

  .mode-btn {
    padding: 3px 10px;
    background: var(--surface-2);
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 11px;
    font-weight: 500;
    transition: all 0.05s;
  }

  .mode-btn:first-child {
    border-right: 1px solid var(--border);
  }

  .mode-btn.active {
    background: var(--accent);
    color: var(--surface-0);
  }

  .action-btn {
    padding: 4px 12px;
    border: none;
    border-radius: 6px;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    transition: opacity 0.05s;
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

  .action-btn.back {
    background: var(--surface-2);
    border: 1px solid var(--border);
    color: var(--text-primary);
  }

  .action-btn.back:hover {
    border-color: var(--accent);
  }
</style>
