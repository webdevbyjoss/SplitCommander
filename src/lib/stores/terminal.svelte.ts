import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { TerminalOutputPayload, TerminalExitPayload } from "../types";

class TerminalStore {
  visible = $state(false);
  alive = $state(false);
  heightPercent = $state(20);

  private writeCallback: ((data: string) => void) | null = null;
  private unlisteners: UnlistenFn[] = [];

  async initListeners() {
    const [outputUn, exitUn] = await Promise.all([
      listen<TerminalOutputPayload>("terminal-output", (event) => {
        if (this.writeCallback) {
          this.writeCallback(event.payload.data);
        }
      }),
      listen<TerminalExitPayload>("terminal-exit", () => {
        this.alive = false;
      }),
    ]);
    this.unlisteners.push(outputUn, exitUn);
  }

  setWriteCallback(cb: ((data: string) => void) | null) {
    this.writeCallback = cb;
  }

  async toggle(cwd: string) {
    this.visible = !this.visible;
    if (this.visible && !this.alive) {
      await this.spawn(cwd);
    }
  }

  async spawn(cwd: string, rows = 24, cols = 80) {
    try {
      await invoke("spawn_terminal", { cwd, rows, cols });
      this.alive = true;
    } catch (e) {
      console.error("Failed to spawn terminal:", e);
      this.alive = false;
    }
  }

  async write(data: string) {
    if (!this.alive) return;
    try {
      await invoke("write_terminal", { data });
    } catch {
      // Silently ignore write failures
    }
  }

  async resize(rows: number, cols: number) {
    if (!this.alive) return;
    try {
      await invoke("resize_terminal", { rows, cols });
    } catch {
      // Silently ignore resize failures
    }
  }

  async kill() {
    if (!this.alive) return;
    try {
      await invoke("kill_terminal");
    } catch {
      // Silently ignore kill failures
    }
    this.alive = false;
  }

  destroy() {
    for (const unlisten of this.unlisteners) {
      unlisten();
    }
    this.unlisteners = [];
    this.writeCallback = null;
  }
}

export const terminalStore = new TerminalStore();
