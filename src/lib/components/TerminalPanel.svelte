<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { Terminal } from "@xterm/xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import "@xterm/xterm/css/xterm.css";
  import { terminalStore } from "../stores/terminal.svelte";

  let leftContainerEl: HTMLDivElement;
  let rightContainerEl: HTMLDivElement;
  let leftTerminal: Terminal | null = null;
  let rightTerminal: Terminal | null = null;
  let leftFitAddon: FitAddon | null = null;
  let rightFitAddon: FitAddon | null = null;
  let leftObserver: ResizeObserver | null = null;
  let rightObserver: ResizeObserver | null = null;

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

  function createTerminal(
    side: "left" | "right",
    container: HTMLDivElement,
  ): { terminal: Terminal; fitAddon: FitAddon; observer: ResizeObserver } {
    const colors = getThemeColors();

    const terminal = new Terminal({
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

    const fitAddon = new FitAddon();
    terminal.loadAddon(fitAddon);
    terminal.open(container);

    // Wire xterm input → PTY
    terminal.onData((data: string) => {
      terminalStore.write(side, data);
    });

    // Wire xterm resize → PTY
    terminal.onResize(({ cols, rows }) => {
      terminalStore.resize(side, rows, cols);
    });

    // Double-Escape closes panel; single Escape passes through to shell
    terminal.attachCustomKeyEventHandler((e: KeyboardEvent) => {
      if (e.key === "Escape" && e.type === "keydown") {
        const closed = terminalStore.handleEscape();
        return !closed; // false = prevent xterm from handling, true = let it through
      }
      // Backtick passes through to terminal (type the character)
      if (e.key === "`" && e.type === "keydown" && !e.ctrlKey && !e.metaKey && !e.altKey) {
        return true;
      }
      return true;
    });

    // Wire PTY output → xterm
    terminalStore.setWriteCallback(side, (data: string) => {
      terminal.write(data);
    });

    // Observe container resize
    const observer = new ResizeObserver(() => {
      if (terminalStore.visible && terminalStore.activeSide === side) {
        fitAddon.fit();
      }
    });
    observer.observe(container);

    return { terminal, fitAddon, observer };
  }

  onMount(() => {
    const left = createTerminal("left", leftContainerEl);
    leftTerminal = left.terminal;
    leftFitAddon = left.fitAddon;
    leftObserver = left.observer;

    const right = createTerminal("right", rightContainerEl);
    rightTerminal = right.terminal;
    rightFitAddon = right.fitAddon;
    rightObserver = right.observer;

    // Listen for theme changes
    const mql = window.matchMedia("(prefers-color-scheme: dark)");
    const updateTheme = () => {
      const c = getThemeColors();
      const theme = { background: c.background, foreground: c.foreground, cursor: c.cursor };
      if (leftTerminal) leftTerminal.options.theme = theme;
      if (rightTerminal) rightTerminal.options.theme = { ...theme };
    };
    mql.addEventListener("change", updateTheme);

    // Initial fit
    requestAnimationFrame(() => {
      leftFitAddon?.fit();
      rightFitAddon?.fit();
    });

    return () => {
      mql.removeEventListener("change", updateTheme);
    };
  });

  // When panel becomes visible or active side changes, focus + fit active terminal
  $effect(() => {
    const side = terminalStore.activeSide;
    const vis = terminalStore.visible;
    if (vis) {
      requestAnimationFrame(() => {
        if (side === "left" && leftTerminal && leftFitAddon) {
          leftFitAddon.fit();
          leftTerminal.focus();
        } else if (side === "right" && rightTerminal && rightFitAddon) {
          rightFitAddon.fit();
          rightTerminal.focus();
        }
      });
    }
  });

  onDestroy(() => {
    terminalStore.setWriteCallback("left", null);
    terminalStore.setWriteCallback("right", null);
    leftObserver?.disconnect();
    rightObserver?.disconnect();
    leftTerminal?.dispose();
    rightTerminal?.dispose();
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
      // Refit active terminal after drag
      if (terminalStore.activeSide === "left") {
        leftFitAddon?.fit();
      } else {
        rightFitAddon?.fit();
      }
    };

    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
  }

  function handleClose() {
    terminalStore.visible = false;
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
    <span class="side-label">{terminalStore.activeSide}</span>
    <button
      class="close-btn"
      onclick={handleClose}
      title="Close terminal (Esc Esc)"
      data-testid="terminal-close-btn"
    >&times;</button>
  </div>
  <div
    class="terminal-container"
    style:display={terminalStore.activeSide === "left" ? "block" : "none"}
    bind:this={leftContainerEl}
  ></div>
  <div
    class="terminal-container"
    style:display={terminalStore.activeSide === "right" ? "block" : "none"}
    bind:this={rightContainerEl}
  ></div>
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
    height: 20px;
    cursor: ns-resize;
    background: var(--surface-1);
    flex-shrink: 0;
    position: relative;
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

  .side-label {
    position: absolute;
    left: 10px;
    font-size: 10px;
    font-family: var(--font-mono);
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    pointer-events: none;
  }

  .close-btn {
    position: absolute;
    right: 6px;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    border: none;
    border-radius: 3px;
    background: transparent;
    color: var(--text-secondary);
    font-size: 14px;
    line-height: 1;
    cursor: pointer;
    padding: 0;
  }

  .close-btn:hover {
    background: var(--surface-2);
    color: var(--text-primary);
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
