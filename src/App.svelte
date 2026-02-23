<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { compareStore } from "./lib/stores/compare.svelte";
  import { terminalStore } from "./lib/stores/terminal.svelte";
  import BrowsePane from "./lib/components/BrowsePane.svelte";
  import ComparePane from "./lib/components/ComparePane.svelte";
  import BottomBar from "./lib/components/BottomBar.svelte";
  import ProgressIndicator from "./lib/components/ProgressIndicator.svelte";
  import TerminalPanel from "./lib/components/TerminalPanel.svelte";

  onMount(() => {
    compareStore.init();
    terminalStore.initListeners();
  });

  onDestroy(() => {
    compareStore.destroy();
    terminalStore.destroy();
  });

  function handleKeydown(e: KeyboardEvent) {
    // Backtick toggles terminal (browse mode only, not in terminal panel)
    if (e.key === "`" && !e.ctrlKey && !e.metaKey && !e.altKey) {
      if (compareStore.appMode === "compare") return;
      const inTerminal = (e.target as HTMLElement)?.closest?.(".terminal-panel");
      if (!inTerminal) {
        e.preventDefault();
        const side = compareStore.activePane;
        const cwd = side === "left" ? compareStore.leftPath : compareStore.rightPath;
        terminalStore.toggle(side, cwd);
        return;
      }
    }

    // Skip single-letter shortcuts when typing in an input
    const inInput =
      e.target instanceof HTMLInputElement ||
      e.target instanceof HTMLTextAreaElement;

    if (e.key === "Tab" && !e.metaKey && !e.ctrlKey) {
      e.preventDefault();
      if (compareStore.appMode === "compare") return;
      compareStore.switchPane();
      // Sync terminal active side and spawn if needed
      if (terminalStore.visible) {
        const newSide = compareStore.activePane;
        terminalStore.activeSide = newSide;
        const alive = newSide === "left" ? terminalStore.leftAlive : terminalStore.rightAlive;
        if (!alive) {
          const cwd = newSide === "left" ? compareStore.leftPath : compareStore.rightPath;
          terminalStore.spawn(newSide, cwd);
        }
      }
      return;
    }

    if (e.key === "Escape") {
      // Let terminal panel handle Escape when focused there (double-Escape to close)
      const inTerminal = (e.target as HTMLElement)?.closest?.(".terminal-panel");
      if (inTerminal) return;

      if (compareStore.isRunning) {
        compareStore.cancelCompare();
      } else if (compareStore.appMode === "compare") {
        compareStore.backToBrowse();
      }
      return;
    }

    // Vim-style single letter shortcuts (only when not in an input)
    if (!inInput && !e.metaKey && !e.ctrlKey && !e.altKey) {
      if (e.key === "g" && compareStore.appMode === "browse" && compareStore.canCompare) {
        e.preventDefault();
        compareStore.startCompare();
      } else if (e.key === "s" && compareStore.appMode === "compare") {
        e.preventDefault();
        compareStore.toggleIdentical();
      } else if (e.key === "q") {
        e.preventDefault();
        compareStore.quitApp();
      } else if (e.key === "r" && compareStore.appMode === "browse") {
        e.preventDefault();
        compareStore.refresh();
      } else if (e.key === "i" && compareStore.appMode === "browse") {
        e.preventDefault();
        compareStore.toggleHidden();
      } else if (e.key === "t" && compareStore.appMode === "browse" && !compareStore.mkdirPromptActive) {
        e.preventDefault();
        compareStore.startMkdirPrompt();
      }
    }
  }


</script>

<svelte:window onkeydown={handleKeydown} />

<div class="app">
  <div class="pane-area">
    {#if compareStore.appMode === "browse"}
      <BrowsePane
        side="left"
        path={compareStore.leftPath}
        entries={compareStore.leftEntries}
        isActive={compareStore.activePane === "left"}
        initialState={compareStore.leftInitState}
      />
      <BrowsePane
        side="right"
        path={compareStore.rightPath}
        entries={compareStore.rightEntries}
        isActive={compareStore.activePane === "right"}
        initialState={compareStore.rightInitState}
      />
    {:else}
      <ComparePane />
    {/if}

    <ProgressIndicator />
  </div>

  <TerminalPanel />
  <BottomBar>
    {#if terminalStore.visible}
      <span class="key-hint"><kbd>Tab</kbd> pane</span>
      <span class="key-hint"><kbd>Esc Esc</kbd> close terminal</span>
    {:else if compareStore.appMode === "browse"}
      <span class="key-hint"><kbd>Tab</kbd> pane</span>
      <span class="key-hint"><kbd>c</kbd> copy</span>
      <span class="key-hint"><kbd>m</kbd> move</span>
      <span class="key-hint"><kbd>d</kbd> delete</span>
      <span class="key-hint"><kbd>t</kbd> mkdir</span>
      <span class="key-hint"><kbd>g</kbd> compare</span>
      <span class="key-hint"><kbd>`</kbd> terminal</span>
      <span class="key-hint"><kbd>q</kbd> quit</span>
    {:else}
      <span class="key-hint"><kbd>↑↓</kbd> move</span>
      <span class="key-hint"><kbd>Enter</kbd> open</span>
      <span class="key-hint"><kbd>Bksp</kbd> up</span>
      <span class="key-hint"><kbd>s</kbd> {compareStore.showIdentical ? "hide" : "show"} same</span>
      <span class="key-hint"><kbd>Esc</kbd> browse</span>
      <span class="key-hint"><kbd>q</kbd> quit</span>
    {/if}
  </BottomBar>
</div>

<style>
  :global(:root) {
    /* Dark theme (default) */
    --surface-0: #1a1a2e;
    --surface-1: #16213e;
    --surface-2: #0f3460;
    --border: #2a2a4a;
    --accent: #4fc3f7;
    --accent-dim: rgba(79, 195, 247, 0.15);
    --danger: #f44336;
    --text-primary: #e0e0e0;
    --text-secondary: #8888aa;
    --toolbar-action: var(--accent);
    --toolbar-action-hover: var(--accent-dim);
    --hover-bg: rgba(255, 255, 255, 0.02);
    --overlay-bg: rgba(0, 0, 0, 0.5);
    --shadow: rgba(0, 0, 0, 0.4);

    /* Diff status colors */
    --diff-same: #50c878;
    --diff-same-bg: rgba(80, 200, 120, 0.15);
    --diff-only-left: #ffc107;
    --diff-only-left-bg: rgba(255, 193, 7, 0.15);
    --diff-only-right: #ff9800;
    --diff-only-right-bg: rgba(255, 152, 0, 0.15);
    --diff-modified: #2196f3;
    --diff-modified-bg: rgba(33, 150, 243, 0.15);
    --diff-error: #f44336;
    --diff-error-bg: rgba(244, 67, 54, 0.15);
    --diff-error-strong-bg: rgba(244, 67, 54, 0.25);

    --font-mono: "SF Mono", "Menlo", "Monaco", "Consolas", monospace;
    --font-sans: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", sans-serif;
  }

  @media (prefers-color-scheme: light) {
    :global(:root) {
      --surface-0: #ffffff;
      --surface-1: #fafafa;
      --surface-2: #f0f0f2;
      --border: #e6e6e6;
      --accent: #007aff;
      --accent-dim: rgba(0, 122, 255, 0.08);
      --danger: #ff3b30;
      --text-primary: #1d1d1f;
      --text-secondary: #8e8e93;
      --toolbar-action: #1d1d1f;
      --toolbar-action-hover: rgba(0, 0, 0, 0.04);
      --hover-bg: rgba(0, 0, 0, 0.02);
      --overlay-bg: rgba(0, 0, 0, 0.2);
      --shadow: rgba(0, 0, 0, 0.1);

      --diff-same: #28a745;
      --diff-same-bg: rgba(40, 167, 69, 0.1);
      --diff-only-left: #e67e00;
      --diff-only-left-bg: rgba(230, 126, 0, 0.1);
      --diff-only-right: #d45400;
      --diff-only-right-bg: rgba(212, 84, 0, 0.1);
      --diff-modified: #007aff;
      --diff-modified-bg: rgba(0, 122, 255, 0.08);
      --diff-error: #ff3b30;
      --diff-error-bg: rgba(255, 59, 48, 0.08);
      --diff-error-strong-bg: rgba(255, 59, 48, 0.12);
    }
  }

  :global(*) {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
    outline: none;
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
</style>
