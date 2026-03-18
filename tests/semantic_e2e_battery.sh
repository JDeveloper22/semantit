#!/usr/bin/env bash
set -uo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CLI_BIN="$ROOT_DIR/target/debug/semantit-cli"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/semantit-e2e.XXXXXX")"

PASS_COUNT=0
FAIL_COUNT=0
STARTED_AT="$(date +%s)"

cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

print_section() {
  local title="$1"
  printf "\n==== %s ====\n" "$title"
}

ok() {
  local message="$1"
  PASS_COUNT=$((PASS_COUNT + 1))
  printf "  [PASS] %s\n" "$message"
}

ko() {
  local message="$1"
  FAIL_COUNT=$((FAIL_COUNT + 1))
  printf "  [FAIL] %s\n" "$message"
}

assert_jq() {
  local file="$1"
  local expression="$2"
  local message="$3"
  if jq -e "$expression" "$file" >/dev/null; then
    ok "$message"
  else
    ko "$message"
    printf "    expression: %s\n" "$expression"
    printf "    file: %s\n" "$file"
  fi
}

run_cli_json() {
  local output_file="$1"
  shift
  "$CLI_BIN" "$@" --json >"$output_file"
}

write_file() {
  local path="$1"
  cat >"$path"
}

print_section "Preparacion"
printf "Repositorio: %s\n" "$ROOT_DIR"
printf "Temporal: %s\n" "$TMP_DIR"

if ! command -v jq >/dev/null 2>&1; then
  echo "Error: jq es requerido para validar los JSON." >&2
  exit 1
fi

print_section "Build de semantit-cli"
if cargo build -p semantit-cli --manifest-path "$ROOT_DIR/Cargo.toml"; then
  ok "cargo build -p semantit-cli"
else
  ko "cargo build -p semantit-cli"
  echo "No se puede continuar sin el binario." >&2
  exit 1
fi

if [[ ! -x "$CLI_BIN" ]]; then
  ko "binario semantit-cli no encontrado en $CLI_BIN"
  exit 1
fi
ok "binario disponible en $CLI_BIN"

print_section "Caso 1: parse de archivo con varias entidades"
PARSE_FILE="$TMP_DIR/parse_sample.ts"
PARSE_JSON="$TMP_DIR/parse_sample.json"
write_file "$PARSE_FILE" <<'TS'
export interface User {
  id: string;
  email: string;
}

export type UserSummary = Pick<User, "id">;

export function makeUser(id: string, email: string): User {
  return { id, email };
}

export const formatUser = (user: User): string => `${user.id}:${user.email}`;
TS
run_cli_json "$PARSE_JSON" parse "$PARSE_FILE"
assert_jq "$PARSE_JSON" '.language == "typescript"' "lenguaje detectado como typescript"
assert_jq "$PARSE_JSON" '.entities | length >= 4' "se indexan al menos 4 entidades"
assert_jq "$PARSE_JSON" '.exports | map(.exported_name) | index("makeUser") != null' "export makeUser detectado"

print_section "Caso 2: diff de funcion movida sin ruido falso"
MOVE_OLD="$TMP_DIR/move_old.ts"
MOVE_NEW="$TMP_DIR/move_new.ts"
MOVE_JSON="$TMP_DIR/move_diff.json"
write_file "$MOVE_OLD" <<'TS'
export function foo(value: number): number {
  return value + 1;
}

export function bar(value: number): number {
  return value * 2;
}
TS
write_file "$MOVE_NEW" <<'TS'
export function bar(value: number): number {
  return value * 2;
}

export function foo(value: number): number {
  return value + 1;
}
TS
run_cli_json "$MOVE_JSON" diff "$MOVE_OLD" "$MOVE_NEW"
assert_jq "$MOVE_JSON" '.summary.moved == 2' "2 entidades detectadas como movidas"
assert_jq "$MOVE_JSON" '.summary.inserted == 0' "sin inserciones espurias"
assert_jq "$MOVE_JSON" '.summary.deleted == 0' "sin eliminaciones espurias"

print_section "Caso 3: merge limpio cuando ramas tocan entidades distintas"
DISJOINT_BASE="$TMP_DIR/disjoint_base.ts"
DISJOINT_OURS="$TMP_DIR/disjoint_ours.ts"
DISJOINT_THEIRS="$TMP_DIR/disjoint_theirs.ts"
DISJOINT_JSON="$TMP_DIR/disjoint_merge.json"
write_file "$DISJOINT_BASE" <<'TS'
export function foo(value: number): number {
  return value + 1;
}

export function bar(value: number): number {
  return value * 2;
}
TS
cp "$DISJOINT_BASE" "$DISJOINT_OURS"
cp "$DISJOINT_BASE" "$DISJOINT_THEIRS"
perl -0pi -e 's/return value \+ 1;/const adjusted = value + 10;\n  return adjusted + 1;/' "$DISJOINT_OURS"
perl -0pi -e 's/return value \* 2;/const doubled = value * 2;\n  return doubled + 5;/' "$DISJOINT_THEIRS"
run_cli_json "$DISJOINT_JSON" merge "$DISJOINT_BASE" "$DISJOINT_OURS" "$DISJOINT_THEIRS"
assert_jq "$DISJOINT_JSON" '.status == "clean"' "merge limpio"
assert_jq "$DISJOINT_JSON" '.conflicts | length == 0' "sin conflictos"
assert_jq "$DISJOINT_JSON" '.merged_index.entities | map(.path) | index("foo") != null' "foo presente en merge final"
assert_jq "$DISJOINT_JSON" '.merged_index.entities | map(.path) | index("bar") != null' "bar presente en merge final"

print_section "Caso 4: conflicto cuando ambas ramas cambian la misma funcion"
SAME_BASE="$TMP_DIR/same_base.ts"
SAME_OURS="$TMP_DIR/same_ours.ts"
SAME_THEIRS="$TMP_DIR/same_theirs.ts"
SAME_JSON="$TMP_DIR/same_merge.json"
write_file "$SAME_BASE" <<'TS'
export function foo(value: number): number {
  return value + 1;
}
TS
cp "$SAME_BASE" "$SAME_OURS"
cp "$SAME_BASE" "$SAME_THEIRS"
perl -0pi -e 's/return value \+ 1;/return value + 100;/' "$SAME_OURS"
perl -0pi -e 's/return value \+ 1;/return value - 1;/' "$SAME_THEIRS"
run_cli_json "$SAME_JSON" merge "$SAME_BASE" "$SAME_OURS" "$SAME_THEIRS"
assert_jq "$SAME_JSON" '.status == "conflicted"' "estado conflicted esperado"
assert_jq "$SAME_JSON" '.conflicts | length == 1' "un conflicto semantico"
assert_jq "$SAME_JSON" '.conflicts[0].cause == "concurrent_modification"' "causa concurrent_modification"

print_section "Caso 5: rename + cambio de implementacion"
RENAME_BASE="$TMP_DIR/rename_base.ts"
RENAME_OURS="$TMP_DIR/rename_ours.ts"
RENAME_THEIRS="$TMP_DIR/rename_theirs.ts"
RENAME_JSON="$TMP_DIR/rename_merge.json"
write_file "$RENAME_BASE" <<'TS'
export function foo(value: number): number {
  return value + 1;
}
TS
cp "$RENAME_BASE" "$RENAME_OURS"
cp "$RENAME_BASE" "$RENAME_THEIRS"
perl -0pi -e 's/function foo/function compute/g; s/export function compute/export function compute/g' "$RENAME_OURS"
perl -0pi -e 's/return value \+ 1;/const base = value + 1;\n  return base * 2;/' "$RENAME_THEIRS"
run_cli_json "$RENAME_JSON" merge "$RENAME_BASE" "$RENAME_OURS" "$RENAME_THEIRS"
assert_jq "$RENAME_JSON" '.status == "clean"' "merge limpio en rename+implementacion"
assert_jq "$RENAME_JSON" '.merged_index.entities | map(.path) | index("compute") != null' "entidad compute detectada"
assert_jq "$RENAME_JSON" '.merged_index.exports[0].exported_name == "compute"' "export actualizado a compute"

print_section "Caso 6: split ambiguo degradado a conflicto"
SPLIT_BASE="$TMP_DIR/split_base.ts"
SPLIT_OURS="$TMP_DIR/split_ours.ts"
SPLIT_THEIRS="$TMP_DIR/split_theirs.ts"
SPLIT_JSON="$TMP_DIR/split_merge.json"
write_file "$SPLIT_BASE" <<'TS'
export function big(value: number): number {
  const left = value + 1;
  const right = left * 2;
  return right - 3;
}
TS
write_file "$SPLIT_OURS" <<'TS'
function buildLeft(value: number): number {
  return value + 1;
}

function buildRight(value: number): number {
  return value * 2 - 3;
}

export function big(value: number): number {
  return buildRight(buildLeft(value));
}
TS
write_file "$SPLIT_THEIRS" <<'TS'
export function big(value: number): number {
  const left = value + 10;
  const right = left * 2;
  return right - 3;
}
TS
run_cli_json "$SPLIT_JSON" merge "$SPLIT_BASE" "$SPLIT_OURS" "$SPLIT_THEIRS"
assert_jq "$SPLIT_JSON" '.status == "conflicted"' "merge marcado como conflicted"
assert_jq "$SPLIT_JSON" '.conflicts | length >= 1' "al menos un conflicto en split ambiguo"

print_section "Caso 7: diff contextual de cambio pequeno en una funcion"
SMALL_OLD="$TMP_DIR/small_old.ts"
SMALL_NEW="$TMP_DIR/small_new.ts"
SMALL_JSON="$TMP_DIR/small_diff.json"
write_file "$SMALL_OLD" <<'TS'
export function foo(value: number): number {
  return value + 1;
}

export function bar(value: number): number {
  return value * 2;
}
TS
cp "$SMALL_OLD" "$SMALL_NEW"
perl -0pi -e 's/return value \+ 1;/const adjusted = value + 10;\n  return adjusted + 1;/' "$SMALL_NEW"
run_cli_json "$SMALL_JSON" diff "$SMALL_OLD" "$SMALL_NEW"
assert_jq "$SMALL_JSON" '.summary.implementation_changed == 1' "un cambio de implementacion detectado"
assert_jq "$SMALL_JSON" '.changes[] | select(.current.path == "foo") | .context.after | test("adjusted")' "contexto del cambio anclado a foo"

print_section "Resumen final"
ELAPSED=$(( $(date +%s) - STARTED_AT ))
printf "Total PASS: %d\n" "$PASS_COUNT"
printf "Total FAIL: %d\n" "$FAIL_COUNT"
printf "Duracion: %ss\n" "$ELAPSED"

if [[ "$FAIL_COUNT" -gt 0 ]]; then
  exit 1
fi
