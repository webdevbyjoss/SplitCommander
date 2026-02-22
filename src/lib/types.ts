export type EntryKind = "file" | "dir" | "symlink";

export interface EntryMeta {
  kind: EntryKind;
  size: number;
  modified: number | null;
  symlinkTarget: string | null;
}

export type DiffKind =
  | "onlyLeft"
  | "onlyRight"
  | "typeMismatch"
  | "same"
  | "metaDiff"
  | "error";

export interface DiffItem {
  relPath: string;
  diffKind: DiffKind;
  left: EntryMeta | null;
  right: EntryMeta | null;
  errorMessage: string | null;
}

export interface CompareSummary {
  totalLeft: number;
  totalRight: number;
  onlyLeft: number;
  onlyRight: number;
  typeMismatch: number;
  same: number;
  metaDiff: number;
  errors: number;
}

export interface ScanProgressPayload {
  side: "left" | "right";
  entriesScanned: number;
  phase: "scanning" | "done";
}

export interface CompareDonePayload {
  summary: CompareSummary;
}

export interface CompareErrorPayload {
  message: string;
}

export type ComparePhase =
  | "idle"
  | "scanning-left"
  | "scanning-right"
  | "comparing"
  | "done"
  | "error"
  | "cancelled";

export interface BrowseEntry {
  name: string;
  kind: EntryKind;
  size: number;
  modified: number | null;
}

export type AppMode = "browse" | "compare";

export type CompareStatus = "same" | "modified" | "onlyLeft" | "onlyRight" | "typeMismatch" | "pending";

export interface CompareEntry {
  name: string;
  kind: EntryKind;
  status: CompareStatus;
  leftSize: number | null;
  rightSize: number | null;
  leftModified: number | null;
  rightModified: number | null;
  dirInfo: { totalSize: number } | null;
}

export interface DirStatusResolvedPayload {
  name: string;
  status: CompareStatus;
  leftPath: string;
  rightPath: string;
  totalSize: number;
}

export interface CompareDirectoryResult {
  entries: CompareEntry[];
  leftPath: string;
  rightPath: string;
  summary: CompareSummary;
}

export interface TerminalOutputPayload {
  data: string;
}

export interface TerminalExitPayload {}
