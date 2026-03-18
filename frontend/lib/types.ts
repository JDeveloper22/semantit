export type PlaygroundOperation = "parse" | "diff" | "merge";

export interface DemoScenario {
  id: string;
  title: string;
  subtitle: string;
  description: string;
  operation: Exclude<PlaygroundOperation, "parse">;
  leftLabel: string;
  rightLabel: string;
  baseLabel?: string;
  left: string;
  right: string;
  base?: string;
  focusPoints: string[];
}

export interface SpanLocation {
  byte: number;
  line: number;
  column: number;
}

export interface SpanRange {
  start: SpanLocation;
  end: SpanLocation;
}

export interface ExportBinding {
  exported_name: string;
  local_name: string;
  entity_id?: string | null;
  is_default: boolean;
  type_only: boolean;
  source?: string | null;
}

export interface SemanticEntity {
  id: string;
  kind: string;
  name: string;
  signature: string;
  path: string;
  parent?: string | null;
  body_hash: string;
  full_hash: string;
  signature_hash: string;
  shape_hash: string;
  span: SpanRange;
  semantic_span: SpanRange;
  dependencies: string[];
  children: string[];
  metadata: Record<string, unknown>;
  snippet?: string | null;
  body?: string | null;
}

export interface FileSemanticIndex {
  language: string;
  path: string;
  entities: SemanticEntity[];
  root_entities: string[];
  exports: ExportBinding[];
  parser_version: string;
}

export type SemanticChangeKind =
  | "unchanged"
  | "moved"
  | "signature_changed"
  | "implementation_changed"
  | "renamed"
  | "inserted"
  | "deleted"
  | "split"
  | "merged";

export interface EntitySnapshot {
  id: string;
  kind: string;
  name: string;
  path: string;
  signature: string;
  body_hash: string;
  full_hash: string;
  signature_hash: string;
  shape_hash: string;
  snippet?: string | null;
  body?: string | null;
}

export interface ContextDiff {
  before?: string | null;
  after?: string | null;
  unified_diff: string;
}

export interface SemanticChange {
  anchor_id: string;
  kinds: SemanticChangeKind[];
  previous?: EntitySnapshot | null;
  current?: EntitySnapshot | null;
  related_entities: EntitySnapshot[];
  context?: ContextDiff | null;
  explanation: string;
}

export interface DiffSummary {
  unchanged: number;
  moved: number;
  signature_changed: number;
  implementation_changed: number;
  renamed: number;
  inserted: number;
  deleted: number;
  split: number;
  merged: number;
}

export interface SemanticDiff {
  old_path: string;
  new_path: string;
  matching_strategy: string;
  summary: DiffSummary;
  changes: SemanticChange[];
}

export interface SemanticConflict {
  anchor_id: string;
  cause: string;
  base?: EntitySnapshot | null;
  ours?: EntitySnapshot | null;
  theirs?: EntitySnapshot | null;
  context?: ContextDiff | null;
  message: string;
}

export interface MergeResult {
  status: "clean" | "conflicted";
  resolved_changes: SemanticChange[];
  conflicts: SemanticConflict[];
  merged_index?: FileSemanticIndex | null;
  notes: string[];
}

export interface ApiResponse<T> {
  operation: PlaygroundOperation;
  command: string[];
  durationMs: number;
  stderr: string;
  result: T;
}
