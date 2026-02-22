<script lang="ts">
  import { compareStore } from "../stores/compare.svelte";

  const phaseLabels: Record<string, string> = {
    idle: "Ready",
    "scanning-left": "Scanning left\u2026",
    "scanning-right": "Scanning right\u2026",
    comparing: "Comparing\u2026",
    done: "Done",
    error: "Error",
    cancelled: "Cancelled",
  };

  function formatNumber(n: number): string {
    return n.toLocaleString();
  }
</script>

<footer class="bottom-bar">
  <div class="status">
    <span
      class="phase"
      class:running={compareStore.isRunning}
      class:error={compareStore.phase === "error"}
    >
      {phaseLabels[compareStore.phase]}
    </span>

    {#if compareStore.isRunning}
      <span class="progress">
        L: {formatNumber(compareStore.scanProgress.left)} &middot; R: {formatNumber(compareStore.scanProgress.right)}
      </span>
    {/if}

    {#if compareStore.summary}
      <span class="summary">
        <span class="sum-item same">{formatNumber(compareStore.summary.same)} same</span>
        <span class="sum-item only-left">{formatNumber(compareStore.summary.onlyLeft)} left only</span>
        <span class="sum-item only-right">{formatNumber(compareStore.summary.onlyRight)} right only</span>
        {#if compareStore.summary.metaDiff > 0}
          <span class="sum-item meta-diff">{formatNumber(compareStore.summary.metaDiff)} modified</span>
        {/if}
        {#if compareStore.summary.typeMismatch > 0}
          <span class="sum-item type-mismatch">{formatNumber(compareStore.summary.typeMismatch)} type mismatch</span>
        {/if}
        {#if compareStore.summary.errors > 0}
          <span class="sum-item errors">{formatNumber(compareStore.summary.errors)} errors</span>
        {/if}
      </span>
    {/if}

    {#if compareStore.error}
      <span class="error-text">{compareStore.error}</span>
    {/if}
  </div>

  <div class="shortcuts">
    <span class="key-hint"><kbd>Tab</kbd> switch pane</span>
    <span class="key-hint"><kbd>{"\u2318"}D</kbd> details</span>
    <span class="key-hint"><kbd>{"\u2318"}E</kbd> export</span>
    {#if compareStore.phase === "done"}
      <button class="export-btn" onclick={() => compareStore.exportReport()}>Export JSON</button>
    {/if}
  </div>
</footer>

<style>
  .bottom-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 32px;
    padding: 0 12px;
    background: var(--surface-1);
    border-top: 1px solid var(--border);
    flex-shrink: 0;
    font-size: 11px;
  }

  .status {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
  }

  .phase {
    font-weight: 600;
    color: var(--text-secondary);
  }

  .phase.running {
    color: var(--accent);
  }

  .phase.error {
    color: var(--danger);
  }

  .progress {
    color: var(--text-secondary);
    font-family: var(--font-mono);
    font-size: 10px;
  }

  .summary {
    display: flex;
    gap: 8px;
  }

  .sum-item {
    font-weight: 500;
  }

  .sum-item.same { color: #50c878; }
  .sum-item.only-left { color: #ffc107; }
  .sum-item.only-right { color: #ff9800; }
  .sum-item.meta-diff { color: #2196f3; }
  .sum-item.type-mismatch { color: #f44336; }
  .sum-item.errors { color: #f44336; }

  .error-text {
    color: var(--danger);
  }

  .shortcuts {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-shrink: 0;
  }

  .key-hint {
    color: var(--text-secondary);
    font-size: 10px;
  }

  kbd {
    display: inline-block;
    padding: 0 4px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 3px;
    font-family: var(--font-mono);
    font-size: 10px;
    line-height: 1.4;
  }

  .export-btn {
    padding: 2px 8px;
    background: var(--accent-dim);
    border: 1px solid var(--accent);
    border-radius: 4px;
    color: var(--accent);
    cursor: pointer;
    font-size: 10px;
    font-weight: 600;
  }

  .export-btn:hover {
    background: var(--accent);
    color: var(--surface-0);
  }
</style>
