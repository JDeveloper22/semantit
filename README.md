# Semantit

Semantit es un MVP en Rust para diff y merge semántico de código fuente. En esta primera iteración trabaja con TypeScript y modela cada archivo como un índice JSON de entidades semánticas.

## Workspace

- `crates/semantit-core`: modelo, hashes, diff, merge y registry de parsers.
- `crates/semantit-ts`: parser TypeScript sobre `swc`.
- `crates/semantit-cli`: CLI con `parse`, `diff` y `merge`.
- `frontend`: playground Next.js + Tailwind con dos editores y ejecución real de la CLI.
- `fixtures/typescript`: casos de prueba del MVP.
- `examples/outputs`: ejemplos JSON generados por la CLI.

## Comandos

```bash
cargo run -p semantit-cli -- parse fixtures/typescript/parse_sample.ts --json
cargo run -p semantit-cli -- diff fixtures/typescript/move_old.ts fixtures/typescript/move_new.ts --json
cargo run -p semantit-cli -- merge fixtures/typescript/rename_base.ts fixtures/typescript/rename_ours.ts fixtures/typescript/rename_theirs.ts --json
```

## Playground frontend

```bash
npm run frontend:install
npm run frontend:dev
```

Luego abre [http://localhost:3000](http://localhost:3000).

Opcional para enlace de contribucion en docs:

```bash
export NEXT_PUBLIC_SEMANTIT_REPO_URL="https://github.com/<org>/<repo>"
```

El frontend:

- carga presets reales desde `fixtures/typescript`
- deja editar izquierda, derecha y base directamente en Monaco Editor
- incluye consola integrada para ejecutar `help`, `scenarios`, `scenario <id>`, `parse left|right|base`, `diff`, `merge`, `json`, `status`
- ejecuta `semantit-cli` real mediante `/api/semantit`
- visualiza parse, diff y merge semántico con JSON de depuración

Documentacion web completa:

- [http://localhost:3000/docs](http://localhost:3000/docs)
- secciones de vision, indice semantico, diff, merge, arquitectura, CLI, FAQ y contribucion

Validación del frontend:

```bash
npm run frontend:build
```

## Validación

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
npm run frontend:build
```

## Bateria E2E por script

Para ejecutar un flujo completo que crea archivos temporales, aplica cambios,
ejecuta `parse`, `diff` y `merge` y valida todo por consola:

```bash
bash tests/semantic_e2e_battery.sh
```

El script imprime resultados `PASS/FAIL` por caso y corta con exit code `1`
si detecta alguna regresión.
