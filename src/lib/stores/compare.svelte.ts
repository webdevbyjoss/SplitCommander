import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open, save } from "@tauri-apps/plugin-dialog";
import type {
  AppMode,
  BrowseEntry,
  CompareDirectoryResult,
  CompareEntry,
  ComparePhase,
  CompareSummary,
  DiffItem,
  ScanProgressPayload,
  CompareDonePayload,
  CompareErrorPayload,
  DirStatusResolvedPayload,
} from "../types";

class CompareStore {
  // App mode: browse (navigate directories) or compare (diff results)
  appMode = $state<AppMode>("browse");

  // Browse state
  leftPath = $state<string>("");
  rightPath = $state<string>("");
  leftEntries = $state<BrowseEntry[]>([]);
  rightEntries = $state<BrowseEntry[]>([]);

  // Compare state
  leftRoot = $state<string | null>(null);
  rightRoot = $state<string | null>(null);
  phase = $state<ComparePhase>("idle");
  diffs = $state<DiffItem[]>([]);
  summary = $state<CompareSummary | null>(null);
  scanProgress = $state({ left: 0, right: 0 });
  error = $state<string | null>(null);
  private errorTimer: ReturnType<typeof setTimeout> | null = null;
  selectedItem = $state<DiffItem | null>(null);
  activePane = $state<"left" | "right">("left");
  showDetails = $state(false);
  filterText = $state("");
  loading = $state(false);
  leftShowHidden = $state(false);
  rightShowHidden = $state(false);

  // Directory-level compare state
  compareRelPath = $state<string>("");
  compareEntries = $state<CompareEntry[]>([]);
  compareSelectedIndex = $state<number>(-1);
  showIdentical = $state<boolean>(true);
  compareSummary = $state<CompareSummary | null>(null);

  // Initial pane state for restoring after app restart
  leftInitState = $state<{ selectedIndex: number; scrollTop: number } | null>(null);
  rightInitState = $state<{ selectedIndex: number; scrollTop: number } | null>(null);

  // Reported pane state (from BrowsePane) for persistence
  private leftReported = { selectedIndex: -1, scrollTop: 0 };
  private rightReported = { selectedIndex: -1, scrollTop: 0 };
  private saveTimer: ReturnType<typeof setTimeout> | null = null;

  private unlisteners: UnlistenFn[] = [];

  get canCompare(): boolean {
    return (
      this.leftPath !== "" &&
      this.rightPath !== "" &&
      this.phase !== "scanning-left" &&
      this.phase !== "scanning-right" &&
      this.phase !== "comparing"
    );
  }

  setError(msg: string | null) {
    if (this.errorTimer) {
      clearTimeout(this.errorTimer);
      this.errorTimer = null;
    }
    this.error = msg;
    if (msg) {
      this.errorTimer = setTimeout(() => {
        this.error = null;
        this.errorTimer = null;
      }, 20_000);
    }
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
    // Fire all IPC calls in parallel: event listeners + init_browse + saved state
    const [scanUn, doneUn, errorUn, dirResolvedUn, initResult, savedState] = await Promise.all([
      listen<ScanProgressPayload>("scan-progress", (event) => {
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
      listen<CompareDonePayload>("compare-done", async (event) => {
        this.summary = event.payload.summary;
        try {
          this.diffs = await invoke<DiffItem[]>("get_diffs");
        } catch {
          this.diffs = [];
        }
        this.phase = "done";
        this.appMode = "compare";
      }),
      listen<CompareErrorPayload>("compare-error", (event) => {
        this.setError(event.payload.message);
        this.phase = "error";
      }),
      listen<DirStatusResolvedPayload>("dir-status-resolved", (event) => {
        const p = event.payload;
        // Staleness check: payload paths must match current view
        const leftFull = this.compareRelPath
          ? this.leftRoot + "/" + this.compareRelPath
          : this.leftRoot;
        const rightFull = this.compareRelPath
          ? this.rightRoot + "/" + this.compareRelPath
          : this.rightRoot;
        if (p.leftPath !== leftFull || p.rightPath !== rightFull) return;

        // Patch matching entry in-place
        this.compareEntries = this.compareEntries.map((e) => {
          if (e.name === p.name && e.status === "pending") {
            return {
              ...e,
              status: p.status,
              dirInfo: { fileCount: p.fileCount },
            };
          }
          return e;
        });

        // Update summary counts
        if (this.compareSummary) {
          const s = { ...this.compareSummary };
          if (p.status === "same") s.same++;
          else if (p.status === "modified") s.metaDiff++;
          this.compareSummary = s;
        }
      }),
      invoke<{ home: string; entries: BrowseEntry[] }>("init_browse").catch(() => null),
      invoke<{
        leftPath: string; rightPath: string;
        leftSelectedIndex: number; leftScrollTop: number;
        rightSelectedIndex: number; rightScrollTop: number;
        leftShowHidden: boolean; rightShowHidden: boolean;
      } | null>("load_app_state").catch(() => null),
    ]);

    this.unlisteners.push(scanUn, doneUn, errorUn, dirResolvedUn);

    if (savedState) {
      // Restore saved state — load directory listings for saved paths
      this.leftShowHidden = savedState.leftShowHidden;
      this.rightShowHidden = savedState.rightShowHidden;
      this.leftInitState = { selectedIndex: savedState.leftSelectedIndex, scrollTop: savedState.leftScrollTop };
      this.rightInitState = { selectedIndex: savedState.rightSelectedIndex, scrollTop: savedState.rightScrollTop };

      // Load both saved directories in parallel
      const [leftOk, rightOk] = await Promise.all([
        this.loadDirectory("left", savedState.leftPath),
        this.loadDirectory("right", savedState.rightPath),
      ]);
      // Fall back to home for any that failed
      if (!leftOk && initResult) {
        this.leftPath = initResult.home;
        this.leftEntries = initResult.entries;
        this.leftInitState = null;
      }
      if (!rightOk && initResult) {
        this.rightPath = initResult.home;
        this.rightEntries = [...initResult.entries];
        this.rightInitState = null;
      }
    } else if (initResult) {
      this.leftPath = initResult.home;
      this.rightPath = initResult.home;
      this.leftEntries = initResult.entries;
      this.rightEntries = [...initResult.entries];
    }

    // Window starts hidden (tauri.conf.json visible:false) — show once content is ready
    try {
      await getCurrentWindow().show();
    } catch {
      // In browser/test environments, this will fail silently
    }
  }

  destroy() {
    for (const unlisten of this.unlisteners) {
      unlisten();
    }
    this.unlisteners = [];
  }

  async loadDirectory(side: "left" | "right", pathOverride?: string): Promise<boolean> {
    const path = pathOverride ?? (side === "left" ? this.leftPath : this.rightPath);
    if (!path) return false;

    this.loading = true;
    try {
      const entries = await invoke<BrowseEntry[]>("list_directory", { path });
      // Commit path + entries together — no breadcrumb flicker on failure
      if (side === "left") {
        this.leftPath = path;
        this.leftEntries = entries;
      } else {
        this.rightPath = path;
        this.rightEntries = entries;
      }
      this.setError(null);
      return true;
    } catch (e) {
      this.setError(`Cannot open ${path}: ${e}`);
      return false;
    } finally {
      this.loading = false;
    }
  }

  async navigateTo(side: "left" | "right", dirName: string) {
    const currentPath = side === "left" ? this.leftPath : this.rightPath;
    const newPath = currentPath + "/" + dirName;
    await this.loadDirectory(side, newPath);
  }

  async navigateUp(side: "left" | "right") {
    const currentPath = side === "left" ? this.leftPath : this.rightPath;
    const parts = currentPath.split("/");
    if (parts.length <= 1) return; // At root

    parts.pop();
    const newPath = parts.join("/") || "/";
    await this.loadDirectory(side, newPath);
  }

  async selectRoot(side: "left" | "right") {
    const selected = await open({ directory: true, multiple: false });
    if (selected) {
      if (side === "left") {
        this.leftPath = selected;
      } else {
        this.rightPath = selected;
      }
      await this.loadDirectory(side);
    }
  }

  async startCompare() {
    // Set roots from current browse paths
    this.leftRoot = this.leftPath;
    this.rightRoot = this.rightPath;
    this.compareRelPath = "";
    this.compareSelectedIndex = -1;
    this.setError(null);
    this.appMode = "compare";
    await this.loadCompareDirectory();
  }

  async loadCompareDirectory() {
    const leftFull = this.compareRelPath
      ? this.leftRoot + "/" + this.compareRelPath
      : this.leftRoot!;
    const rightFull = this.compareRelPath
      ? this.rightRoot + "/" + this.compareRelPath
      : this.rightRoot!;

    this.loading = true;
    try {
      const result = await invoke<CompareDirectoryResult>("compare_directory", {
        leftPath: leftFull,
        rightPath: rightFull,
      });
      this.compareEntries = result.entries;
      this.compareSummary = result.summary;

      // Phase 2: resolve pending directories in background
      const hasPending = result.entries.some((e) => e.status === "pending");
      if (hasPending) {
        invoke("resolve_dir_statuses", {
          leftPath: leftFull,
          rightPath: rightFull,
        }).catch(() => {});
      }
    } catch (e) {
      this.setError(`Compare failed: ${e}`);
    } finally {
      this.loading = false;
    }
  }

  async navigateCompareDir(dirName: string) {
    invoke("cancel_dir_resolve").catch(() => {});
    this.compareRelPath = this.compareRelPath
      ? this.compareRelPath + "/" + dirName
      : dirName;
    this.compareSelectedIndex = -1;
    await this.loadCompareDirectory();
  }

  async navigateCompareUp() {
    if (!this.compareRelPath) return;
    invoke("cancel_dir_resolve").catch(() => {});
    const parts = this.compareRelPath.split("/");
    parts.pop();
    this.compareRelPath = parts.join("/");
    this.compareSelectedIndex = -1;
    await this.loadCompareDirectory();
  }

  toggleIdentical() {
    this.showIdentical = !this.showIdentical;
  }

  get filteredCompareEntries(): CompareEntry[] {
    if (this.showIdentical) return this.compareEntries;
    return this.compareEntries.filter((e) => e.status !== "same");
  }

  get isAtCompareRoot(): boolean {
    return this.compareRelPath === "";
  }

  async cancelCompare() {
    await invoke("cancel_compare");
    invoke("cancel_dir_resolve").catch(() => {});
    this.phase = "cancelled";
  }

  backToBrowse() {
    invoke("cancel_dir_resolve").catch(() => {});
    this.appMode = "browse";
    this.phase = "idle";
    this.diffs = [];
    this.summary = null;
    this.selectedItem = null;
    this.compareEntries = [];
    this.compareSummary = null;
    this.compareRelPath = "";
    this.compareSelectedIndex = -1;
  }

  switchPane() {
    this.activePane = this.activePane === "left" ? "right" : "left";
  }

  toggleDetails() {
    this.showDetails = !this.showDetails;
  }

  toggleHidden(side?: "left" | "right") {
    const s = side ?? this.activePane;
    if (s === "left") {
      this.leftShowHidden = !this.leftShowHidden;
    } else {
      this.rightShowHidden = !this.rightShowHidden;
    }
    this.debouncedSave();
  }

  async refresh() {
    await this.loadDirectory(this.activePane);
  }

  async openFile(side: "left" | "right", fileName: string) {
    const dir = side === "left" ? this.leftPath : this.rightPath;
    const fullPath = dir + "/" + fileName;
    try {
      await invoke("open_file", { path: fullPath });
    } catch (e) {
      this.setError(`${e}`);
    }
  }

  reportPaneState(side: "left" | "right", selectedIndex: number, scrollTop: number) {
    if (side === "left") {
      this.leftReported = { selectedIndex, scrollTop };
    } else {
      this.rightReported = { selectedIndex, scrollTop };
    }
    this.debouncedSave();
  }

  private debouncedSave() {
    if (this.saveTimer) clearTimeout(this.saveTimer);
    this.saveTimer = setTimeout(() => this.saveState(), 300);
  }

  private async saveState() {
    try {
      await invoke("save_app_state", {
        state: {
          leftPath: this.leftPath,
          rightPath: this.rightPath,
          leftSelectedIndex: this.leftReported.selectedIndex,
          leftScrollTop: this.leftReported.scrollTop,
          rightSelectedIndex: this.rightReported.selectedIndex,
          rightScrollTop: this.rightReported.scrollTop,
          leftShowHidden: this.leftShowHidden,
          rightShowHidden: this.rightShowHidden,
        },
      });
    } catch {
      // Silently ignore save failures
    }
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
