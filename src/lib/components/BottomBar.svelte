<script lang="ts">
  import { compareStore } from "../stores/compare.svelte";
  import type { Snippet } from "svelte";

  let { children }: { children?: Snippet } = $props();

  function formatNumber(n: number): string {
    return n.toLocaleString();
  }

  let pendingCount = $derived(
    compareStore.compareEntries.filter((e) => e.status === "pending").length
  );
</script>

<footer class="bottom-bar">
  <div class="status">
    {#if compareStore.loading}
      <span class="loading-indicator" aria-label="Loading directory"><span class="mini-spinner"></span> Loading…</span>
    {/if}

    {#if compareStore.isRunning}
      <span class="phase running">
        {compareStore.phase === "scanning-left" ? "Scanning left\u2026" : compareStore.phase === "scanning-right" ? "Scanning right\u2026" : "Comparing\u2026"}
      </span>
      <span class="progress">
        L: {formatNumber(compareStore.scanProgress.left)} &middot; R: {formatNumber(compareStore.scanProgress.right)}
      </span>
    {/if}

    {#if pendingCount > 0}
      <span class="loading-indicator"><span class="mini-spinner"></span> {pendingCount} resolving</span>
    {/if}

    {#if compareStore.compareSummary}
      <span class="summary">
        <span class="sum-item same">{formatNumber(compareStore.compareSummary.same)} same</span>
        <span class="sum-item only-left">{formatNumber(compareStore.compareSummary.onlyLeft)} left only</span>
        <span class="sum-item only-right">{formatNumber(compareStore.compareSummary.onlyRight)} right only</span>
        {#if compareStore.compareSummary.metaDiff > 0}
          <span class="sum-item meta-diff">{formatNumber(compareStore.compareSummary.metaDiff)} modified</span>
        {/if}
        {#if compareStore.compareSummary.typeMismatch > 0}
          <span class="sum-item type-mismatch">{formatNumber(compareStore.compareSummary.typeMismatch)} type mismatch</span>
        {/if}
      </span>
    {:else if compareStore.phase === "done" && compareStore.summary}
      <span class="summary">
        <span class="sum-item same">{formatNumber(compareStore.summary.same)} same</span>
        <span class="sum-item only-left">{formatNumber(compareStore.summary.onlyLeft)} left only</span>
        <span class="sum-item only-right">{formatNumber(compareStore.summary.onlyRight)} right only</span>
        {#if compareStore.summary.metaDiff > 0}
          <span class="sum-item meta-diff">{formatNumber(compareStore.summary.metaDiff)} modified</span>
        {/if}
      </span>
    {/if}

    {#if compareStore.error}
      <span class="error-text">{compareStore.error}</span>
    {/if}
  </div>

  <div class="shortcuts">
    {#if children}
      {@render children()}
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

  .loading-indicator {
    display: flex;
    align-items: center;
    gap: 4px;
    color: var(--accent);
    font-weight: 500;
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

  .sum-item.same { color: var(--diff-same); }
  .sum-item.only-left { color: var(--diff-only-left); }
  .sum-item.only-right { color: var(--diff-only-right); }
  .sum-item.meta-diff { color: var(--diff-modified); }
  .sum-item.type-mismatch { color: var(--diff-error); }
  .sum-item.errors { color: var(--diff-error); }

  .error-text {
    color: var(--danger);
    animation: toast-fade 20s ease-in forwards;
  }

  @keyframes toast-fade {
    0%, 90% { opacity: 1; }
    100% { opacity: 0; }
  }

  .shortcuts {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-shrink: 0;
  }

  .shortcuts :global(.key-hint) {
    color: var(--text-secondary);
    font-size: 10px;
  }

  .shortcuts :global(kbd) {
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
