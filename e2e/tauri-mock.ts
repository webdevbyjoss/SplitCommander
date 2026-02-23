/**
 * Injects Tauri API mocks so the Svelte app works in a plain browser.
 *
 * The @tauri-apps/api internals look for window.__TAURI_INTERNALS__.invoke
 * and window.__TAURI_INTERNALS__.metadata.
 */
export async function injectTauriMocks(page: import("@playwright/test").Page) {
  await page.addInitScript(() => {
    const HOME = "/Users/testuser";

    type FakeEntry = {
      name: string;
      kind: string;
      size: number;
      modified: number | null;
    };

    const fakeFS: Record<string, FakeEntry[]> = {
      [HOME]: [
        { name: "Desktop", kind: "dir", size: 0, modified: 1700000000000 },
        { name: "Documents", kind: "dir", size: 0, modified: 1700000000000 },
        { name: "Downloads", kind: "dir", size: 0, modified: 1700000000000 },
        { name: "Projects", kind: "dir", size: 0, modified: 1700000000000 },
        { name: ".zshrc", kind: "file", size: 2048, modified: 1699000000000 },
        { name: "notes.txt", kind: "file", size: 512, modified: 1698000000000 },
      ],
      [`${HOME}/Documents`]: [
        { name: "archive", kind: "dir", size: 0, modified: 1699000000000 },
        { name: "cache", kind: "dir", size: 0, modified: 1699000000000 },
        { name: "drafts", kind: "dir", size: 0, modified: 1699000000000 },
        { name: "exports", kind: "dir", size: 0, modified: 1699000000000 },
        { name: "images", kind: "dir", size: 0, modified: 1699000000000 },
        { name: "logs", kind: "dir", size: 0, modified: 1699000000000 },
        { name: "photos", kind: "dir", size: 0, modified: 1699000000000 },
        { name: "scripts", kind: "dir", size: 0, modified: 1699000000000 },
        { name: "templates", kind: "dir", size: 0, modified: 1699000000000 },
        { name: "work", kind: "dir", size: 0, modified: 1699000000000 },
        { name: "backup.zip", kind: "file", size: 16384, modified: 1700000000000 },
        { name: "budget.xlsx", kind: "file", size: 2048, modified: 1700000000000 },
        { name: "config.json", kind: "file", size: 64, modified: 1700000000000 },
        { name: "data.csv", kind: "file", size: 8192, modified: 1700000000000 },
        { name: "notes.md", kind: "file", size: 512, modified: 1700000000000 },
        { name: "report.pdf", kind: "file", size: 1048576, modified: 1700000000000 },
        { name: "spec.docx", kind: "file", size: 4096, modified: 1700000000000 },
        { name: "todo.txt", kind: "file", size: 128, modified: 1700000000000 },
      ],
      [`${HOME}/Documents/archive`]: [
        { name: "old-backup.zip", kind: "file", size: 2048, modified: 1699000000000 },
      ],
      [`${HOME}/Documents/work`]: [
        { name: "proposal.pdf", kind: "file", size: 1024, modified: 1700000000000 },
      ],
      [`${HOME}/Documents/logs`]: [
        { name: "app.log", kind: "file", size: 1024, modified: 1700000000000 },
      ],
      [`${HOME}/Desktop`]: [
        { name: "todo.txt", kind: "file", size: 128, modified: 1700100000000 },
        { name: "screenshot.png", kind: "file", size: 524288, modified: 1700200000000 },
      ],
      [`${HOME}/Projects`]: [
        { name: "my-app", kind: "dir", size: 0, modified: 1700000000000 },
        { name: "README.md", kind: "file", size: 256, modified: 1700000000000 },
      ],
      [`${HOME}/Projects/my-app`]: [
        { name: "src", kind: "dir", size: 0, modified: 1700000000000 },
        { name: "index.html", kind: "file", size: 512, modified: 1700000000000 },
        { name: "package.json", kind: "file", size: 1024, modified: 1700000000000 },
      ],
      [`${HOME}/Downloads`]: [
        { name: "archive.zip", kind: "file", size: 10485760, modified: 1700000000000 },
        { name: "image.jpg", kind: "file", size: 2097152, modified: 1699500000000 },
      ],
      "/": [
        { name: "Applications", kind: "dir", size: 0, modified: 1700000000000 },
        { name: "System", kind: "dir", size: 0, modified: 1700000000000 },
        { name: "Users", kind: "dir", size: 0, modified: 1700000000000 },
      ],
      "/Users": [
        { name: "testuser", kind: "dir", size: 0, modified: 1700000000000 },
      ],
    };

    // Create mirror of home directory at /Users/mirror for compare tests
    // (same content, different path — needed because canCompare rejects same-path)
    const MIRROR = "/Users/mirror";
    const homeKeys = Object.keys(fakeFS).filter(k => k.startsWith(HOME));
    for (const key of homeKeys) {
      fakeFS[MIRROR + key.slice(HOME.length)] = [...fakeFS[key]];
    }
    fakeFS["/Users"].push({ name: "mirror", kind: "dir", size: 0, modified: 1700000000000 });

    let leftRoot: string | null = null;
    let rightRoot: string | null = null;
    let lastDiffs: any[] = [];
    let lastSummary: any = null;

    // Event callback registry: eventName -> array of handler callback IDs
    const eventCallbacks: Record<string, number[]> = {};
    let nextCallbackId = 1;
    let nextEventId = 1;

    function emitTauriEvent(eventName: string, payload: any) {
      const ids = eventCallbacks[eventName] || [];
      for (const id of ids) {
        const fn = (window as any)[`_${id}`];
        if (fn) {
          fn({ event: eventName, id, payload });
        }
      }
    }

    (window as any).__TAURI_INTERNALS__ = {
      metadata: {
        currentWindow: { label: "main" },
        currentWebview: { label: "main", windowLabel: "main" },
      },
      transformCallback: (callback: Function, once = false) => {
        const id = nextCallbackId++;
        (window as any)[`_${id}`] = (event: any) => {
          if (once) {
            delete (window as any)[`_${id}`];
          }
          callback(event);
        };
        return id;
      },
      invoke: async (cmd: string, args: any = {}) => {
        // Core app commands
        if (cmd === "init_browse") {
          const entries = fakeFS[HOME] || [];
          return {
            home: HOME,
            entries: [...entries].sort((a, b) => {
              if (a.kind === "dir" && b.kind !== "dir") return -1;
              if (a.kind !== "dir" && b.kind === "dir") return 1;
              return a.name.toLowerCase().localeCompare(b.name.toLowerCase());
            }),
          };
        }

        if (cmd === "list_directory") {
          const entries = fakeFS[args.path];
          if (!entries) throw new Error(`Not a directory: ${args.path}`);
          return [...entries].sort((a, b) => {
            if (a.kind === "dir" && b.kind !== "dir") return -1;
            if (a.kind !== "dir" && b.kind === "dir") return 1;
            return a.name.toLowerCase().localeCompare(b.name.toLowerCase());
          });
        }

        if (cmd === "set_root") {
          if (args.side === "left") leftRoot = args.path;
          if (args.side === "right") rightRoot = args.path;
          return null;
        }

        if (cmd === "start_compare") {
          const leftEntries = fakeFS[leftRoot!] || [];
          const rightEntries = fakeFS[rightRoot!] || [];

          setTimeout(() => {
            emitTauriEvent("scan-progress", {
              side: "left",
              entriesScanned: leftEntries.length,
              phase: "done",
            });
          }, 1);

          setTimeout(() => {
            emitTauriEvent("scan-progress", {
              side: "right",
              entriesScanned: rightEntries.length,
              phase: "done",
            });
          }, 2);

          setTimeout(() => {
            // Generate diffs
            const leftMap = new Map(leftEntries.map((e) => [e.name, e]));
            const rightMap = new Map(rightEntries.map((e) => [e.name, e]));
            const allNames = new Set([...leftMap.keys(), ...rightMap.keys()]);

            lastDiffs = [];
            let same = 0,
              onlyLeft = 0,
              onlyRight = 0,
              metaDiff = 0;

            for (const name of allNames) {
              const l = leftMap.get(name);
              const r = rightMap.get(name);
              const toMeta = (e: FakeEntry) => ({
                kind: e.kind,
                size: e.size,
                modified: e.modified,
                symlinkTarget: null,
              });

              if (l && !r) {
                onlyLeft++;
                lastDiffs.push({
                  relPath: name,
                  diffKind: "onlyLeft",
                  left: toMeta(l),
                  right: null,
                  errorMessage: null,
                });
              } else if (!l && r) {
                onlyRight++;
                lastDiffs.push({
                  relPath: name,
                  diffKind: "onlyRight",
                  left: null,
                  right: toMeta(r),
                  errorMessage: null,
                });
              } else if (l && r) {
                const isSame =
                  l.size === r.size && l.modified === r.modified;
                if (isSame) {
                  same++;
                  lastDiffs.push({
                    relPath: name,
                    diffKind: "same",
                    left: toMeta(l),
                    right: toMeta(r),
                    errorMessage: null,
                  });
                } else {
                  metaDiff++;
                  lastDiffs.push({
                    relPath: name,
                    diffKind: "metaDiff",
                    left: toMeta(l),
                    right: toMeta(r),
                    errorMessage: null,
                  });
                }
              }
            }

            lastSummary = {
              totalLeft: leftEntries.length,
              totalRight: rightEntries.length,
              onlyLeft,
              onlyRight,
              typeMismatch: 0,
              same,
              metaDiff,
              errors: 0,
            };

            emitTauriEvent("compare-done", { summary: lastSummary });
          }, 5);

          return null;
        }

        if (cmd === "compare_directory") {
          // Recursive helper: checks if two fakeFS dirs are identical
          function dirsAreSame(lPath: string, rPath: string): boolean {
            const lEntries = fakeFS[lPath] || [];
            const rEntries = fakeFS[rPath] || [];
            const lMap = new Map(lEntries.map((e) => [e.name.toLowerCase(), e]));
            const rMap = new Map(rEntries.map((e) => [e.name.toLowerCase(), e]));
            if (lMap.size !== rMap.size) return false;
            for (const [key, l] of lMap) {
              const r = rMap.get(key);
              if (!r) return false;
              if (l.kind !== r.kind) return false;
              if (l.kind === "dir") {
                if (!dirsAreSame(`${lPath}/${l.name}`, `${rPath}/${r.name}`)) return false;
              } else if (l.size !== r.size) {
                return false;
              }
            }
            return true;
          }

          // Count files recursively from left side
          function calcTotalSize(path: string): number {
            const entries = fakeFS[path] || [];
            let size = 0;
            for (const e of entries) {
              if (e.kind === "dir") {
                size += calcTotalSize(`${path}/${e.name}`);
              } else {
                size += e.size;
              }
            }
            return size;
          }

          const leftEntries = fakeFS[args.leftPath] || [];
          const rightEntries = fakeFS[args.rightPath] || [];

          const leftMap = new Map(leftEntries.map((e) => [e.name.toLowerCase(), e]));
          const rightMap = new Map(rightEntries.map((e) => [e.name.toLowerCase(), e]));
          const allKeys = new Set([...leftMap.keys(), ...rightMap.keys()]);

          const entries: any[] = [];
          // Track pending dirs for later resolution
          const pendingDirs: Array<{ name: string; leftPath: string; rightPath: string }> = [];
          let same = 0, onlyLeft = 0, onlyRight = 0, metaDiff = 0, typeMismatch = 0;

          for (const key of allKeys) {
            const l = leftMap.get(key);
            const r = rightMap.get(key);

            if (l && r) {
              if (l.kind !== r.kind) {
                typeMismatch++;
                entries.push({
                  name: l.name, kind: l.kind, status: "typeMismatch",
                  leftSize: l.size, rightSize: r.size,
                  leftModified: l.modified, rightModified: r.modified,
                  dirInfo: null,
                });
              } else if (l.kind === "dir") {
                // Phase 1: mark as pending (no recursion)
                pendingDirs.push({
                  name: l.name,
                  leftPath: `${args.leftPath}/${l.name}`,
                  rightPath: `${args.rightPath}/${r.name}`,
                });
                entries.push({
                  name: l.name, kind: "dir", status: "pending",
                  leftSize: null, rightSize: null,
                  leftModified: l.modified, rightModified: r.modified,
                  dirInfo: null,
                });
              } else if (l.size === r.size) {
                same++;
                entries.push({
                  name: l.name, kind: l.kind, status: "same",
                  leftSize: l.size, rightSize: r.size,
                  leftModified: l.modified, rightModified: r.modified,
                  dirInfo: null,
                });
              } else {
                metaDiff++;
                entries.push({
                  name: l.name, kind: l.kind, status: "modified",
                  leftSize: l.size, rightSize: r.size,
                  leftModified: l.modified, rightModified: r.modified,
                  dirInfo: null,
                });
              }
            } else if (l) {
              onlyLeft++;
              entries.push({
                name: l.name, kind: l.kind, status: "onlyLeft",
                leftSize: l.size, rightSize: null,
                leftModified: l.modified, rightModified: null,
                dirInfo: null,
              });
            } else if (r) {
              onlyRight++;
              entries.push({
                name: r.name, kind: r.kind, status: "onlyRight",
                leftSize: null, rightSize: r.size,
                leftModified: null, rightModified: r.modified,
                dirInfo: null,
              });
            }
          }

          // Store pending dirs info for resolve_dir_statuses
          (window as any).__pendingDirs = pendingDirs;
          (window as any).__dirsAreSame = dirsAreSame;
          (window as any).__calcTotalSize = calcTotalSize;

          // Sort: dirs first, then alphabetically
          entries.sort((a: any, b: any) => {
            const aDir = a.kind === "dir" ? 0 : 1;
            const bDir = b.kind === "dir" ? 0 : 1;
            if (aDir !== bDir) return aDir - bDir;
            return a.name.toLowerCase().localeCompare(b.name.toLowerCase());
          });

          return {
            entries,
            leftPath: args.leftPath,
            rightPath: args.rightPath,
            summary: {
              totalLeft: leftEntries.length,
              totalRight: rightEntries.length,
              onlyLeft, onlyRight, typeMismatch, same, metaDiff, errors: 0,
            },
          };
        }

        if (cmd === "resolve_dir_statuses") {
          const pendingDirs = (window as any).__pendingDirs || [];
          const dirsAreSameFn = (window as any).__dirsAreSame;
          const calcTotalSizeFn = (window as any).__calcTotalSize;
          (window as any).__dirResolveCancelled = false;

          pendingDirs.forEach((dir: any, i: number) => {
            setTimeout(() => {
              if ((window as any).__dirResolveCancelled) return;
              const isSame = dirsAreSameFn(dir.leftPath, dir.rightPath);
              const totalSize = calcTotalSizeFn(dir.leftPath);
              emitTauriEvent("dir-status-resolved", {
                name: dir.name,
                status: isSame ? "same" : "modified",
                leftPath: args.leftPath,
                rightPath: args.rightPath,
                totalSize,
              });
            }, (i + 1) * 10);
          });

          return null;
        }

        if (cmd === "cancel_dir_resolve") {
          (window as any).__dirResolveCancelled = true;
          return null;
        }

        if (cmd === "clear_dir_resolve_cache") {
          return null;
        }

        if (cmd === "cancel_compare") return null;
        if (cmd === "get_diffs") return lastDiffs;
        if (cmd === "get_summary") return lastSummary;
        if (cmd === "export_report") return null;
        if (cmd === "open_file") return null;
        if (cmd === "copy_entry") return null;
        if (cmd === "copy_entry_overwrite") return null;
        if (cmd === "move_entry") return null;
        if (cmd === "create_directory") {
          const parentEntries = fakeFS[args.parentPath];
          if (parentEntries) {
            parentEntries.push({
              name: args.name,
              kind: "dir",
              size: 0,
              modified: Date.now(),
            });
            fakeFS[args.parentPath + "/" + args.name] = [];
          }
          return null;
        }
        if (cmd === "delete_entry") return null;
        if (cmd === "load_app_state") return null;
        if (cmd === "save_app_state") return null;

        // Terminal commands (side-aware: "left" or "right")
        if (cmd === "spawn_terminal") {
          const sideKey = `__terminalAlive_${args.side}`;
          (window as any)[sideKey] = true;
          const side = args.side;
          setTimeout(() => {
            if ((window as any)[sideKey]) {
              emitTauriEvent("terminal-output", { side, data: "$ " });
            }
          }, 50);
          return null;
        }
        if (cmd === "write_terminal") {
          const sideKey = `__terminalAlive_${args.side}`;
          const side = args.side;
          if ((window as any)[sideKey]) {
            setTimeout(() => {
              emitTauriEvent("terminal-output", { side, data: args.data });
            }, 10);
          }
          return null;
        }
        if (cmd === "resize_terminal") return null;
        if (cmd === "kill_terminal") {
          const sideKey = `__terminalAlive_${args.side}`;
          (window as any)[sideKey] = false;
          return null;
        }

        // Event system: plugin:event|listen
        if (cmd === "plugin:event|listen") {
          const eventName = args.event;
          const handlerId = args.handler; // callback ID from transformCallback
          if (!eventCallbacks[eventName]) eventCallbacks[eventName] = [];
          eventCallbacks[eventName].push(handlerId);
          return nextEventId++; // Return eventId for unlisten
        }

        if (cmd === "plugin:event|unlisten") return null;

        // Window commands
        if (cmd === "plugin:window|show") return null;
        if (cmd === "plugin:window|close") return null;

        // Dialog plugin
        if (cmd === "plugin:dialog|open") return `${HOME}/Documents`;
        if (cmd === "plugin:dialog|save") return "/tmp/test-report.json";
        if (cmd === "plugin:dialog|ask") return true;
        if (cmd === "plugin:dialog|message") return null;

        console.warn(`[tauri-mock] Unhandled command: ${cmd}`, args);
        return null;
      },
    };
  });
}
