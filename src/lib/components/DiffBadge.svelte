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
    background: rgba(80, 200, 120, 0.15);
    color: #50c878;
  }

  .onlyLeft {
    background: rgba(255, 193, 7, 0.15);
    color: #ffc107;
  }

  .onlyRight {
    background: rgba(255, 152, 0, 0.15);
    color: #ff9800;
  }

  .metaDiff {
    background: rgba(33, 150, 243, 0.15);
    color: #2196f3;
  }

  .typeMismatch {
    background: rgba(244, 67, 54, 0.15);
    color: #f44336;
  }

  .error {
    background: rgba(244, 67, 54, 0.25);
    color: #f44336;
  }
</style>
