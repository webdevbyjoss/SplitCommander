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

export type CompareMode = "structure" | "smart";

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
