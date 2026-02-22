<script lang="ts">
  import type { DiffKind } from "../types";

  let { kind }: { kind: DiffKind } = $props();

  const labels: Record<DiffKind, string> = {
    same: "Same",
    onlyLeft: "Left only",
    onlyRight: "Right only",
    metaDiff: "Modified",
    typeMismatch: "Type \u2260",
    error: "Error",
  };
</script>

<span class="badge {kind}">{labels[kind]}</span>

<style>
  .badge {
    display: inline-block;
    padding: 1px 6px;
    border-radius: 3px;
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.3px;
    white-space: nowrap;
  }

  .same {
    background: var(--diff-same-bg);
    color: var(--diff-same);
  }

  .onlyLeft {
    background: var(--diff-only-left-bg);
    color: var(--diff-only-left);
  }

  .onlyRight {
    background: var(--diff-only-right-bg);
    color: var(--diff-only-right);
  }

  .metaDiff {
    background: var(--diff-modified-bg);
    color: var(--diff-modified);
  }

  .typeMismatch {
    background: var(--diff-error-bg);
    color: var(--diff-error);
  }

  .error {
    background: var(--diff-error-strong-bg);
    color: var(--diff-error);
  }
</style>
