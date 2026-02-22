<script lang="ts">
  import { compareStore } from "../stores/compare.svelte";
</script>

{#if compareStore.isRunning}
  <div class="progress-overlay">
    <div class="progress-card">
      <div class="spinner"></div>
      <div class="progress-text">
        {#if compareStore.phase === "scanning-left"}
          <p class="label">Scanning left directory...</p>
          <p class="count">{compareStore.scanProgress.left.toLocaleString()} entries</p>
        {:else if compareStore.phase === "scanning-right"}
          <p class="label">Scanning right directory...</p>
          <p class="count">{compareStore.scanProgress.right.toLocaleString()} entries</p>
        {:else}
          <p class="label">Comparing...</p>
        {/if}
      </div>
      <button class="cancel-btn" onclick={() => compareStore.cancelCompare()}>
        Cancel
      </button>
    </div>
  </div>
{/if}

<style>
  .progress-overlay {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.5);
    z-index: 10;
  }

  .progress-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    padding: 24px 32px;
    background: var(--surface-1);
    border: 1px solid var(--border);
    border-radius: 12px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
  }

  .spinner {
    width: 32px;
    height: 32px;
    border: 3px solid var(--border);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .progress-text {
    text-align: center;
  }

  .label {
    margin: 0;
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .count {
    margin: 4px 0 0;
    font-size: 12px;
    font-family: var(--font-mono);
    color: var(--text-secondary);
  }

  .cancel-btn {
    padding: 4px 16px;
    background: transparent;
    border: 1px solid var(--danger);
    border-radius: 6px;
    color: var(--danger);
    cursor: pointer;
    font-size: 12px;
    font-weight: 500;
    transition: all 0.15s;
  }

  .cancel-btn:hover {
    background: var(--danger);
    color: white;
  }
</style>
