import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { TerminalOutputPayload, TerminalExitPayload } from "../types";

const DOUBLE_ESC_MS = 400;

class TerminalStore {
  visible = $state(false);
  leftAlive = $state(false);
  rightAlive = $state(false);
  activeSide = $state<"left" | "right">("left");
  heightPercent = $state(20);

  private leftWriteCallback: ((data: string) => void) | null = null;
  private rightWriteCallback: ((data: string) => void) | null = null;
  private unlisteners: UnlistenFn[] = [];
  private lastEscapeTime = 0;

  async initListeners() {
    const [outputUn, exitUn] = await Promise.all([
      listen<TerminalOutputPayload>("terminal-output", (event) => {
        const { side, data } = event.payload;
        if (side === "left" && this.leftWriteCallback) {
          this.leftWriteCallback(data);
        } else if (side === "right" && this.rightWriteCallback) {
          this.rightWriteCallback(data);
        }
      }),
      listen<TerminalExitPayload>("terminal-exit", (event) => {
        const side = event.payload.side;
        if (side === "left") {
          this.leftAlive = false;
        } else {
          this.rightAlive = false;
        }
        // Auto-hide panel when the active side's shell exits
        if (this.visible && this.activeSide === side) {
          this.visible = false;
        }
      }),
    ]);
    this.unlisteners.push(outputUn, exitUn);
  }

  setWriteCallback(side: "left" | "right", cb: ((data: string) => void) | null) {
    if (side === "left") {
      this.leftWriteCallback = cb;
    } else {
      this.rightWriteCallback = cb;
    }
  }

  /** Returns true if the double-Escape was detected (panel should close). */
  handleEscape(): boolean {
    const now = Date.now();
    if (now - this.lastEscapeTime < DOUBLE_ESC_MS) {
      this.lastEscapeTime = 0;
      this.visible = false;
      return true;
    }
    this.lastEscapeTime = now;
    return false;
  }

  async toggle(side: "left" | "right", cwd: string) {
    if (this.visible && this.activeSide === side) {
      // Same side toggle — hide
      this.visible = false;
      return;
    }

    this.activeSide = side;
    this.visible = true;

    const alive = side === "left" ? this.leftAlive : this.rightAlive;
    if (!alive) {
      await this.spawn(side, cwd);
    }
  }

  async spawn(side: "left" | "right", cwd: string, rows = 24, cols = 80) {
    try {
      await invoke("spawn_terminal", { side, cwd, rows, cols });
      if (side === "left") {
        this.leftAlive = true;
      } else {
        this.rightAlive = true;
      }
    } catch (e) {
      console.error(`Failed to spawn ${side} terminal:`, e);
    }
  }

  async write(side: "left" | "right", data: string) {
    const alive = side === "left" ? this.leftAlive : this.rightAlive;
    if (!alive) return;
    try {
      await invoke("write_terminal", { side, data });
    } catch {
      // Silently ignore write failures
    }
  }

  async resize(side: "left" | "right", rows: number, cols: number) {
    const alive = side === "left" ? this.leftAlive : this.rightAlive;
    if (!alive) return;
    try {
      await invoke("resize_terminal", { side, rows, cols });
    } catch {
      // Silently ignore resize failures
    }
  }

  async kill(side: "left" | "right") {
    const alive = side === "left" ? this.leftAlive : this.rightAlive;
    if (!alive) return;
    try {
      await invoke("kill_terminal", { side });
    } catch {
      // Silently ignore kill failures
    }
    if (side === "left") {
      this.leftAlive = false;
    } else {
      this.rightAlive = false;
    }
  }

  async killAll() {
    await Promise.all([this.kill("left"), this.kill("right")]);
  }

  destroy() {
    for (const unlisten of this.unlisteners) {
      unlisten();
    }
    this.unlisteners = [];
    this.leftWriteCallback = null;
    this.rightWriteCallback = null;
  }
}

export const terminalStore = new TerminalStore();
