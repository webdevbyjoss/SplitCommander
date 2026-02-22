<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { Terminal } from "@xterm/xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import "@xterm/xterm/css/xterm.css";
  import { terminalStore } from "../stores/terminal.svelte";

  let containerEl: HTMLDivElement;
  let terminal: Terminal | null = null;
  let fitAddon: FitAddon | null = null;
  let resizeObserver: ResizeObserver | null = null;

  // Drag state
  let dragging = $state(false);
  let dragStartY = 0;
  let dragStartHeight = 0;

  function getThemeColors(): { background: string; foreground: string; cursor: string } {
    const style = getComputedStyle(document.documentElement);
    return {
      background: style.getPropertyValue("--surface-0").trim() || "#1a1a2e",
      foreground: style.getPropertyValue("--text-primary").trim() || "#e0e0e0",
      cursor: style.getPropertyValue("--accent").trim() || "#4fc3f7",
    };
  }

  onMount(() => {
    const colors = getThemeColors();

    terminal = new Terminal({
      fontFamily: '"SF Mono", "Menlo", "Monaco", "Consolas", monospace',
      fontSize: 13,
      theme: {
        background: colors.background,
        foreground: colors.foreground,
        cursor: colors.cursor,
      },
      cursorBlink: true,
      allowProposedApi: true,
    });

    fitAddon = new FitAddon();
    terminal.loadAddon(fitAddon);
    terminal.open(containerEl);

    // Wire xterm input → PTY
    terminal.onData((data: string) => {
      terminalStore.write(data);
    });

    // Wire xterm resize → PTY
    terminal.onResize(({ cols, rows }) => {
      terminalStore.resize(rows, cols);
    });

    // Escape hides terminal panel
    terminal.attachCustomKeyEventHandler((e: KeyboardEvent) => {
      if (e.key === "Escape" && e.type === "keydown") {
        terminalStore.visible = false;
        return false;
      }
      // Prevent backtick from toggling when typing in terminal
      if (e.key === "`" && e.type === "keydown" && !e.ctrlKey && !e.metaKey && !e.altKey) {
        return true; // let xterm handle it (type the backtick)
      }
      return true;
    });

    // Wire PTY output → xterm
    terminalStore.setWriteCallback((data: string) => {
      terminal?.write(data);
    });

    // Observe container resize
    resizeObserver = new ResizeObserver(() => {
      if (terminalStore.visible && fitAddon) {
        fitAddon.fit();
      }
    });
    resizeObserver.observe(containerEl);

    // Listen for theme changes
    const mql = window.matchMedia("(prefers-color-scheme: dark)");
    const updateTheme = () => {
      if (!terminal) return;
      const c = getThemeColors();
      terminal.options.theme = {
        background: c.background,
        foreground: c.foreground,
        cursor: c.cursor,
      };
    };
    mql.addEventListener("change", updateTheme);

    // Initial fit
    requestAnimationFrame(() => {
      fitAddon?.fit();
    });

    return () => {
      mql.removeEventListener("change", updateTheme);
    };
  });

  // When panel becomes visible, focus terminal and refit
  $effect(() => {
    if (terminalStore.visible && terminal && fitAddon) {
      requestAnimationFrame(() => {
        fitAddon?.fit();
        terminal?.focus();
      });
    }
  });

  onDestroy(() => {
    terminalStore.setWriteCallback(null);
    resizeObserver?.disconnect();
    terminal?.dispose();
  });

  function handleDragStart(e: MouseEvent) {
    dragging = true;
    dragStartY = e.clientY;
    dragStartHeight = terminalStore.heightPercent;
    e.preventDefault();

    const onMove = (ev: MouseEvent) => {
      const deltaY = dragStartY - ev.clientY;
      const deltaPct = (deltaY / window.innerHeight) * 100;
      terminalStore.heightPercent = Math.min(70, Math.max(10, dragStartHeight + deltaPct));
    };

    const onUp = () => {
      dragging = false;
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
      // Refit after resize
      fitAddon?.fit();
    };

    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
  }
</script>

<div
  class="terminal-panel"
  style:display={terminalStore.visible ? "flex" : "none"}
  style:height="{terminalStore.heightPercent}vh"
  data-testid="terminal-panel"
>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="drag-handle"
    onmousedown={handleDragStart}
    data-testid="terminal-drag-handle"
  >
    <div class="drag-grip"></div>
  </div>
  <div class="terminal-container" bind:this={containerEl}></div>
</div>

<style>
  .terminal-panel {
    flex-direction: column;
    flex-shrink: 0;
    border-top: 1px solid var(--border);
    background: var(--surface-0);
    overflow: hidden;
  }

  .drag-handle {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 6px;
    cursor: ns-resize;
    background: var(--surface-1);
    flex-shrink: 0;
  }

  .drag-handle:hover {
    background: var(--surface-2);
  }

  .drag-grip {
    width: 40px;
    height: 2px;
    border-radius: 1px;
    background: var(--text-secondary);
    opacity: 0.5;
  }

  .drag-handle:hover .drag-grip {
    opacity: 0.8;
  }

  .terminal-container {
    flex: 1;
    min-height: 0;
    padding: 4px;
  }

  /* Ensure xterm fills container */
  .terminal-container :global(.xterm) {
    height: 100%;
  }

  .terminal-container :global(.xterm-viewport) {
    overflow-y: auto !important;
  }
</style>
