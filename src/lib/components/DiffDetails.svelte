<script lang="ts">
  import type { DiffItem, EntryMeta } from "../types";
  import DiffBadge from "./DiffBadge.svelte";
  import { compareStore } from "../stores/compare.svelte";

  let { item }: { item: DiffItem } = $props();

  function formatSize(bytes: number): string {
    if (bytes === 0) return "0 B";
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }

  function formatDate(epoch: number | null): string {
    if (epoch === null) return "\u2014";
    return new Date(epoch).toLocaleString();
  }

  function kindLabel(meta: EntryMeta | null): string {
    if (!meta) return "\u2014";
    return meta.kind.charAt(0).toUpperCase() + meta.kind.slice(1);
  }
</script>

<aside class="details-panel">
  <div class="details-header">
    <h3>Details</h3>
    <button class="close-btn" onclick={() => compareStore.toggleDetails()}>&times;</button>
  </div>

  <div class="details-path">{item.relPath}</div>
  <div class="details-status">
    <DiffBadge kind={item.diffKind} />
  </div>

  {#if item.errorMessage}
    <div class="error-msg">{item.errorMessage}</div>
  {/if}

  <table class="meta-table">
    <thead>
      <tr>
        <th></th>
        <th>Left</th>
        <th>Right</th>
      </tr>
    </thead>
    <tbody>
      <tr>
        <td class="label">Type</td>
        <td class:diff={item.left?.kind !== item.right?.kind}>{kindLabel(item.left)}</td>
        <td class:diff={item.left?.kind !== item.right?.kind}>{kindLabel(item.right)}</td>
      </tr>
      <tr>
        <td class="label">Size</td>
        <td class:diff={item.left?.size !== item.right?.size}>
          {item.left ? formatSize(item.left.size) : "\u2014"}
        </td>
        <td class:diff={item.left?.size !== item.right?.size}>
          {item.right ? formatSize(item.right.size) : "\u2014"}
        </td>
      </tr>
      <tr>
        <td class="label">Modified</td>
        <td class:diff={item.left?.modified !== item.right?.modified}>
          {item.left ? formatDate(item.left.modified) : "\u2014"}
        </td>
        <td class:diff={item.left?.modified !== item.right?.modified}>
          {item.right ? formatDate(item.right.modified) : "\u2014"}
        </td>
      </tr>
      {#if item.left?.symlinkTarget || item.right?.symlinkTarget}
        <tr>
          <td class="label">Link target</td>
          <td class:diff={item.left?.symlinkTarget !== item.right?.symlinkTarget}>
            {item.left?.symlinkTarget ?? "\u2014"}
          </td>
          <td class:diff={item.left?.symlinkTarget !== item.right?.symlinkTarget}>
            {item.right?.symlinkTarget ?? "\u2014"}
          </td>
        </tr>
      {/if}
    </tbody>
  </table>
</aside>

<style>
  .details-panel {
    width: 280px;
    flex-shrink: 0;
    background: var(--surface-1);
    border-left: 1px solid var(--border);
    padding: 12px;
    overflow-y: auto;
    font-size: 12px;
  }

  .details-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 10px;
  }

  .details-header h3 {
    margin: 0;
    font-size: 13px;
    font-weight: 700;
    color: var(--text-primary);
  }

  .close-btn {
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 18px;
    padding: 0 4px;
    line-height: 1;
  }

  .close-btn:hover {
    color: var(--text-primary);
  }

  .details-path {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--text-primary);
    word-break: break-all;
    margin-bottom: 8px;
    padding: 6px 8px;
    background: var(--surface-2);
    border-radius: 4px;
  }

  .details-status {
    margin-bottom: 12px;
  }

  .error-msg {
    color: var(--danger);
    font-size: 11px;
    margin-bottom: 12px;
    padding: 6px 8px;
    background: var(--diff-error-bg);
    border-radius: 4px;
  }

  .meta-table {
    width: 100%;
    border-collapse: collapse;
  }

  .meta-table th,
  .meta-table td {
    padding: 4px 6px;
    text-align: left;
    border-bottom: 1px solid var(--border);
    font-size: 11px;
  }

  .meta-table th {
    color: var(--text-secondary);
    font-weight: 600;
    text-transform: uppercase;
    font-size: 10px;
    letter-spacing: 0.3px;
  }

  .label {
    color: var(--text-secondary);
    font-weight: 500;
    width: 70px;
  }

  .diff {
    color: var(--accent);
    font-weight: 600;
  }
</style>
