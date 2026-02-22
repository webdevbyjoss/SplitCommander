import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";
import type {
  CompareMode,
  ComparePhase,
  CompareSummary,
  DiffItem,
  ScanProgressPayload,
  CompareDonePayload,
  CompareErrorPayload,
} from "../types";

class CompareStore {
  leftRoot = $state<string | null>(null);
  rightRoot = $state<string | null>(null);
  mode = $state<CompareMode>("smart");
  phase = $state<ComparePhase>("idle");
  diffs = $state<DiffItem[]>([]);
  summary = $state<CompareSummary | null>(null);
  scanProgress = $state({ left: 0, right: 0 });
  error = $state<string | null>(null);
  selectedItem = $state<DiffItem | null>(null);
  activePane = $state<"left" | "right">("left");
  showDetails = $state(false);
  filterText = $state("");

  private unlisteners: UnlistenFn[] = [];

  get canCompare(): boolean {
    return (
      this.leftRoot !== null &&
      this.rightRoot !== null &&
      this.phase !== "scanning-left" &&
      this.phase !== "scanning-right" &&
      this.phase !== "comparing"
    );
  }

  get isRunning(): boolean {
    return (
      this.phase === "scanning-left" ||
      this.phase === "scanning-right" ||
      this.phase === "comparing"
    );
  }

  get filteredDiffs(): DiffItem[] {
    if (!this.filterText) return this.diffs;
    const lower = this.filterText.toLowerCase();
    return this.diffs.filter((d) => d.relPath.toLowerCase().includes(lower));
  }

  async init() {
    this.unlisteners.push(
      await listen<ScanProgressPayload>("scan-progress", (event) => {
        const { side, entriesScanned, phase } = event.payload;
        this.scanProgress = {
          ...this.scanProgress,
          [side]: entriesScanned,
        };
        if (phase === "done") {
          this.phase = side === "left" ? "scanning-right" : "comparing";
        } else {
          this.phase = side === "left" ? "scanning-left" : "scanning-right";
        }
      }),
    );

    this.unlisteners.push(
      await listen<CompareDonePayload>("compare-done", async (event) => {
        this.summary = event.payload.summary;
        try {
          this.diffs = await invoke<DiffItem[]>("get_diffs");
        } catch (e) {
          this.diffs = [];
        }
        this.phase = "done";
      }),
    );

    this.unlisteners.push(
      await listen<CompareErrorPayload>("compare-error", (event) => {
        this.error = event.payload.message;
        this.phase = "error";
      }),
    );
  }

  destroy() {
    for (const unlisten of this.unlisteners) {
      unlisten();
    }
    this.unlisteners = [];
  }

  async selectRoot(side: "left" | "right") {
    const selected = await open({ directory: true, multiple: false });
    if (selected) {
      await invoke("set_root", { side, path: selected });
      if (side === "left") {
        this.leftRoot = selected;
      } else {
        this.rightRoot = selected;
      }
    }
  }

  async startCompare() {
    this.phase = "scanning-left";
    this.diffs = [];
    this.summary = null;
    this.scanProgress = { left: 0, right: 0 };
    this.error = null;
    this.selectedItem = null;
    await invoke("start_compare", { mode: this.mode });
  }

  async cancelCompare() {
    await invoke("cancel_compare");
    this.phase = "cancelled";
  }

  setMode(mode: CompareMode) {
    this.mode = mode;
  }

  switchPane() {
    this.activePane = this.activePane === "left" ? "right" : "left";
  }

  toggleDetails() {
    this.showDetails = !this.showDetails;
  }

  selectItem(item: DiffItem | null) {
    this.selectedItem = item;
    if (item) {
      this.showDetails = true;
    }
  }

  async exportReport() {
    const filePath = await save({
      filters: [{ name: "JSON", extensions: ["json"] }],
      defaultPath: "splitcommander-report.json",
    });
    if (filePath) {
      await invoke("export_report", { path: filePath });
    }
  }
}

export const compareStore = new CompareStore();
