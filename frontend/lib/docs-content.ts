export type DocBlock =
  | { type: "paragraph"; text: string }
  | { type: "list"; items: string[] }
  | { type: "steps"; items: string[] }
  | { type: "code"; language: string; code: string }
  | { type: "callout"; tone: "info" | "warn" | "success"; title: string; text: string }
  | { type: "checklist"; items: { label: string; done: boolean }[] };

export interface DocSection {
  id: string;
  title: string;
  blocks: DocBlock[];
}

export interface DocPage {
  slug: string;
  title: string;
  summary: string;
  category: "getting-started" | "concepts" | "reference" | "contributing";
  audience: "all" | "maintainers" | "contributors";
  updatedAt: string;
  readingMinutes: number;
  sections: DocSection[];
}

export interface DocCategory {
  id: DocPage["category"];
  title: string;
  description: string;
}

export const docCategories: DocCategory[] = [
  {
    id: "getting-started",
    title: "Getting Started",
    description: "Instalacion, flujo rapido y primer uso de Semantit.",
  },
  {
    id: "concepts",
    title: "Conceptos",
    description: "Modelo mental de indice semantico, diff y merge.",
  },
  {
    id: "reference",
    title: "Referencia",
    description: "CLI, JSON, formatos de salida y decisiones tecnicas.",
  },
  {
    id: "contributing",
    title: "Contribucion",
    description: "Buenas practicas para colaborar y abrir PRs utiles.",
  },
];

const docs: DocPage[] = [
  {
    slug: "vision",
    title: "Vision de Semantit",
    summary:
      "Que problema resuelve Semantit y por que la unidad semantica importa mas que la linea.",
    category: "getting-started",
    audience: "all",
    updatedAt: "2026-03-16",
    readingMinutes: 6,
    sections: [
      {
        id: "problem",
        title: "Problema que atacamos",
        blocks: [
          {
            type: "paragraph",
            text: "Los merges por lineas fallan cuando el codigo se mueve o se reordena. Semantit cambia la unidad de comparacion: en vez de comparar texto plano, compara entidades de codigo con identidad estable (funciones, metodos, clases, interfaces y tipos).",
          },
          {
            type: "list",
            items: [
              "Mover una funcion dentro del archivo no deberia verse como cambio funcional.",
              "Cambios disjuntos en entidades distintas deberian mergearse sin conflicto.",
              "El conflicto real aparece cuando dos ramas modifican la misma entidad.",
            ],
          },
        ],
      },
      {
        id: "principles",
        title: "Principios de producto",
        blocks: [
          {
            type: "checklist",
            items: [
              { label: "El parser produce indice semantico por archivo", done: true },
              { label: "El diff reporta cambios por entidad, no por archivo plano", done: true },
              { label: "El merge de 3 vias solo marca conflicto cuando es necesario", done: true },
              { label: "Los casos ambiguos degradan a revision manual", done: true },
            ],
          },
          {
            type: "callout",
            tone: "info",
            title: "Importante",
            text: "Semantit no reemplaza a Git. Se integra como capa semantica para mejorar diff y merge.",
          },
        ],
      },
      {
        id: "quick-tour",
        title: "Tour rapido",
        blocks: [
          {
            type: "steps",
            items: [
              "Ejecuta `semantit parse` para ver entidades y hashes.",
              "Ejecuta `semantit diff old.ts new.ts` para clasificar cambios semanticos.",
              "Ejecuta `semantit merge base.ts ours.ts theirs.ts` para resolver o detectar conflictos por entidad.",
              "Usa el playground web para inspeccionar resultados y JSON.",
            ],
          },
          {
            type: "code",
            language: "bash",
            code: "cargo run -p semantit-cli -- parse fixtures/typescript/parse_sample.ts --json\ncargo run -p semantit-cli -- diff fixtures/typescript/move_old.ts fixtures/typescript/move_new.ts --json\ncargo run -p semantit-cli -- merge fixtures/typescript/disjoint_base.ts fixtures/typescript/disjoint_ours.ts fixtures/typescript/disjoint_theirs.ts --json",
          },
        ],
      },
    ],
  },
  {
    slug: "semantic-index",
    title: "Indice semantico",
    summary:
      "Estructura del JSON que Semantit genera por archivo y como leer cada campo.",
    category: "concepts",
    audience: "all",
    updatedAt: "2026-03-16",
    readingMinutes: 8,
    sections: [
      {
        id: "entity-model",
        title: "Modelo de entidad",
        blocks: [
          {
            type: "paragraph",
            text: "Cada entidad representa una definicion de codigo con metadatos suficientes para hacer matching, detectar renames y separar cambios de firma vs implementacion.",
          },
          {
            type: "list",
            items: [
              "`id`: identificador estable para matching principal.",
              "`kind`, `name`, `path`, `parent`: ubicacion logica y jerarquia.",
              "`signature_hash`, `body_hash`, `full_hash`, `shape_hash`: huellas para clasificar cambios.",
              "`span` y `semantic_span`: posiciones para contexto y trazabilidad.",
              "`dependencies` y `children`: relaciones sintactico-semanticas.",
            ],
          },
        ],
      },
      {
        id: "stability",
        title: "Estrategia de estabilidad",
        blocks: [
          {
            type: "paragraph",
            text: "La identidad no debe romperse por formato ni por reordenamiento. Por eso el ID usa firma normalizada y ancla semantica de padre, no coordenadas de linea.",
          },
          {
            type: "code",
            language: "text",
            code: "id = hash(kind + semantic_parent_anchor + declared_name + normalized_signature_shape)\nbody_hash = hash(cuerpo_canonico_sin_trivia)\nsignature_hash = hash(firma_canonica)\nshape_hash = hash(forma_sin_nombre)",
          },
          {
            type: "callout",
            tone: "success",
            title: "Resultado practico",
            text: "Un movimiento de funcion suele quedar como `moved` sin `inserted/deleted`, lo que reduce ruido en code review.",
          },
        ],
      },
      {
        id: "json-example",
        title: "Ejemplo minimo de salida",
        blocks: [
          {
            type: "code",
            language: "json",
            code: "{\n  \"language\": \"typescript\",\n  \"path\": \"example.ts\",\n  \"entities\": [\n    {\n      \"id\": \"...\",\n      \"kind\": \"function\",\n      \"name\": \"foo\",\n      \"path\": \"foo\",\n      \"signature\": \"function foo(value:number)number\",\n      \"signature_hash\": \"...\",\n      \"body_hash\": \"...\"\n    }\n  ],\n  \"exports\": [\n    {\n      \"exported_name\": \"foo\",\n      \"local_name\": \"foo\",\n      \"is_default\": false,\n      \"type_only\": false\n    }\n  ]\n}",
          },
        ],
      },
    ],
  },
  {
    slug: "semantic-diff",
    title: "Motor de diff semantico",
    summary:
      "Como Semantit compara dos indices y clasifica cambios sin depender del diff textual.",
    category: "concepts",
    audience: "all",
    updatedAt: "2026-03-16",
    readingMinutes: 7,
    sections: [
      {
        id: "matching",
        title: "Pipeline de matching",
        blocks: [
          {
            type: "steps",
            items: [
              "Match exacto por `id`.",
              "Match de respaldo por mismo nombre cuando hay cambios de firma.",
              "Heuristica conservadora de rename con `shape_hash` + similitud.",
              "Clasificacion de restantes como inserted/deleted o ambiguos split/merged.",
            ],
          },
        ],
      },
      {
        id: "change-kinds",
        title: "Tipos de cambio",
        blocks: [
          {
            type: "list",
            items: [
              "`unchanged`",
              "`moved`",
              "`signature_changed`",
              "`implementation_changed`",
              "`renamed`",
              "`inserted`",
              "`deleted`",
              "`split` y `merged`",
            ],
          },
          {
            type: "paragraph",
            text: "Un mismo cambio puede tener varios tags. Ejemplo: `renamed + signature_changed + implementation_changed`.",
          },
        ],
      },
      {
        id: "contextual-diff",
        title: "Diff contextual por entidad",
        blocks: [
          {
            type: "paragraph",
            text: "Cuando cambia implementacion, la salida incluye diff unificado sobre el cuerpo de la entidad. Esto facilita revision local del cambio sin perder contexto por reordenamientos globales.",
          },
          {
            type: "code",
            language: "bash",
            code: "cargo run -p semantit-cli -- diff old.ts new.ts --json | jq '.changes[] | {path: .current.path, kinds: .kinds, context: .context.unified_diff}'",
          },
        ],
      },
    ],
  },
  {
    slug: "semantic-merge",
    title: "Merge de tres vias",
    summary:
      "Reglas de resolucion automatica, causas de conflicto y degradacion segura en casos ambiguos.",
    category: "concepts",
    audience: "all",
    updatedAt: "2026-03-16",
    readingMinutes: 9,
    sections: [
      {
        id: "merge-flow",
        title: "Flujo base/ours/theirs",
        blocks: [
          {
            type: "steps",
            items: [
              "Parse de las tres versiones.",
              "Diff base->ours y base->theirs.",
              "Resolucion por entidad usando reglas semanticas.",
              "Construccion de merged_index o lista de conflictos.",
            ],
          },
        ],
      },
      {
        id: "auto-resolve",
        title: "Casos auto-resolubles",
        blocks: [
          {
            type: "list",
            items: [
              "Cambios de posicion (moved) sin cambios funcionales.",
              "Cambio en una sola rama.",
              "Cambios en entidades distintas.",
              "Rename en una rama + cambio de implementacion en la otra si el match es inequivoco.",
            ],
          },
          {
            type: "callout",
            tone: "success",
            title: "Regla de seguridad",
            text: "Si no hay confianza alta en el matching, Semantit prefiere conflicto explicito antes que merge incorrecto.",
          },
        ],
      },
      {
        id: "conflicts",
        title: "Causas de conflicto",
        blocks: [
          {
            type: "list",
            items: [
              "`concurrent_modification`",
              "`rename_vs_modification`",
              "`delete_vs_modification`",
              "`divergent_rename`",
              "`ambiguous_split`",
              "`ambiguous_merge`",
            ],
          },
          {
            type: "code",
            language: "bash",
            code: "cargo run -p semantit-cli -- merge base.ts ours.ts theirs.ts --json | jq '.status, .conflicts[]?.cause'",
          },
        ],
      },
    ],
  },
  {
    slug: "cli-reference",
    title: "Referencia CLI",
    summary:
      "Comandos disponibles, formatos de salida y patrones recomendados para automatizacion.",
    category: "reference",
    audience: "all",
    updatedAt: "2026-03-16",
    readingMinutes: 7,
    sections: [
      {
        id: "commands",
        title: "Comandos",
        blocks: [
          {
            type: "code",
            language: "bash",
            code: "semantit parse <file> [--json]\nsemantit diff <old> <new> [--json]\nsemantit merge <base> <ours> <theirs> [--json]",
          },
          {
            type: "paragraph",
            text: "Sin `--json` la CLI imprime salida humana para debug rapido. Con `--json` produce estructura estable para tooling y tests snapshot.",
          },
        ],
      },
      {
        id: "automation",
        title: "Automatizacion en scripts",
        blocks: [
          {
            type: "list",
            items: [
              "Usa `jq -e` para convertir validaciones en exit code.",
              "Normaliza rutas temporales cuando compares snapshots.",
              "Separa fixtures por escenario (move/disjoint/rename/split).",
            ],
          },
          {
            type: "code",
            language: "bash",
            code: "semantit diff a.ts b.ts --json > diff.json\njq -e '.summary.moved == 2 and .summary.inserted == 0 and .summary.deleted == 0' diff.json",
          },
        ],
      },
      {
        id: "json-contract",
        title: "Contrato JSON",
        blocks: [
          {
            type: "paragraph",
            text: "El contrato se define por los tipos publicos de `semantit-core` (`FileSemanticIndex`, `SemanticDiff`, `MergeResult`). Si cambias estos tipos, actualiza docs, ejemplos y tests snapshot en el mismo PR.",
          },
          {
            type: "checklist",
            items: [
              { label: "No romper campos existentes sin migracion", done: true },
              { label: "Versionar cambios de parser cuando cambie el shape", done: true },
              { label: "Agregar fixture y test por cada cambio de contrato", done: true },
            ],
          },
        ],
      },
    ],
  },
  {
    slug: "architecture",
    title: "Arquitectura del proyecto",
    summary:
      "Mapa de crates, responsabilidades y extension a nuevos lenguajes.",
    category: "reference",
    audience: "maintainers",
    updatedAt: "2026-03-16",
    readingMinutes: 8,
    sections: [
      {
        id: "workspace-map",
        title: "Mapa de workspace",
        blocks: [
          {
            type: "code",
            language: "text",
            code: "crates/\n  semantit-core/   # modelo, diff, merge, registry, hashes\n  semantit-ts/     # parser TypeScript con SWC\n  semantit-cli/    # comandos parse/diff/merge\nfrontend/          # playground y docs web en Next.js\nfixtures/          # casos TS para pruebas\nexamples/outputs/  # muestras JSON",
          },
          {
            type: "paragraph",
            text: "La direccion de dependencias es intencional: `core` no conoce implementaciones concretas de parser; cada lenguaje implementa el trait comun y se registra en CLI.",
          },
        ],
      },
      {
        id: "language-extension",
        title: "Agregar un nuevo lenguaje",
        blocks: [
          {
            type: "steps",
            items: [
              "Crear crate `semantit-<lang>`.",
              "Implementar `LanguageParser` y convertir AST a `SemanticEntity`.",
              "Agregar registro en CLI y extension de `supports_path`.",
              "Crear fixtures, tests y snapshots especificos.",
            ],
          },
          {
            type: "callout",
            tone: "warn",
            title: "Regla de compatibilidad",
            text: "Evita diferencias semanticas por formato del parser. El cuerpo y la firma deben canonicarse antes de hashear.",
          },
        ],
      },
      {
        id: "frontend-purpose",
        title: "Rol del frontend",
        blocks: [
          {
            type: "paragraph",
            text: "El frontend no reimplementa el motor semantico. Solo orquesta entradas y ejecuta la CLI real por API local para que el comportamiento de demo sea identico al binario.",
          },
        ],
      },
    ],
  },
  {
    slug: "release-playbook",
    title: "Playbook de release y publicacion",
    summary:
      "Checklist practico para abrir Semantit al publico y escalar colaboracion con contributors.",
    category: "contributing",
    audience: "maintainers",
    updatedAt: "2026-03-16",
    readingMinutes: 9,
    sections: [
      {
        id: "repo-hardening",
        title: "Hardening del repositorio",
        blocks: [
          {
            type: "checklist",
            items: [
              { label: "README con propuesta de valor y quickstart", done: true },
              { label: "Licencia clara (MIT/Apache-2.0)", done: true },
              { label: "Guia de contribucion y quality gates", done: true },
              { label: "Plantillas de issue/PR", done: false },
              { label: "CODE_OF_CONDUCT y SECURITY.md", done: false },
            ],
          },
          {
            type: "callout",
            tone: "warn",
            title: "Antes de anunciar",
            text: "No publiques el proyecto hasta que el onboarding de una persona nueva pueda completarse sin ayuda directa.",
          },
        ],
      },
      {
        id: "ci-and-versioning",
        title: "CI y versionado",
        blocks: [
          {
            type: "steps",
            items: [
              "Configurar CI para `cargo fmt --check`, `cargo clippy`, `cargo test` y `bash tests/semantic_e2e_battery.sh`.",
              "Agregar build de frontend (`npm --prefix frontend run build`) como gate.",
              "Definir convencion de releases (ejemplo: semver + changelog por version).",
              "Publicar artefactos reproducibles del binario en cada release.",
            ],
          },
          {
            type: "code",
            language: "bash",
            code: "# flujo minimo de release local\ncargo fmt --check\ncargo clippy --all-targets --all-features -- -D warnings\ncargo test\nbash tests/semantic_e2e_battery.sh\nnpm --prefix frontend run build",
          },
        ],
      },
      {
        id: "community-workflow",
        title: "Flujo de comunidad",
        blocks: [
          {
            type: "list",
            items: [
              "Etiquetas de issues: good first issue, parser, diff, merge, docs, frontend.",
              "Roadmap publico con milestones trimestrales.",
              "Etiquetar PRs de ruptura de contrato JSON.",
              "Responder issues nuevas en <= 72h para mantener traccion.",
            ],
          },
          {
            type: "paragraph",
            text: "Una comunidad saludable no depende solo de codigo; depende de expectativas claras, tiempos de respuesta y decisiones tecnicas documentadas.",
          },
        ],
      },
      {
        id: "launch-checklist",
        title: "Checklist de lanzamiento publico",
        blocks: [
          {
            type: "checklist",
            items: [
              { label: "Demo web funcional en / y docs completas en /docs", done: true },
              { label: "Bateria E2E ejecutable en una sola orden", done: true },
              { label: "Ejemplos de casos move/disjoint/rename/split", done: true },
              { label: "Issue tracker preparado para recibir contribuciones", done: false },
              { label: "Primer paquete de good first issues creado", done: false },
            ],
          },
        ],
      },
    ],
  },
  {
    slug: "contributing",
    title: "Guia de contribucion",
    summary:
      "Como preparar entorno, abrir cambios de calidad y colaborar sin friccion.",
    category: "contributing",
    audience: "contributors",
    updatedAt: "2026-03-16",
    readingMinutes: 10,
    sections: [
      {
        id: "setup",
        title: "Setup local recomendado",
        blocks: [
          {
            type: "code",
            language: "bash",
            code: "git clone <repo>\ncd semantit\ncargo build -p semantit-cli\ncargo test\nnpm --prefix frontend install\nnpm --prefix frontend run build",
          },
          {
            type: "checklist",
            items: [
              { label: "Rust toolchain actualizado", done: true },
              { label: "Node.js LTS y npm disponibles", done: true },
              { label: "jq instalado para pruebas de script", done: true },
            ],
          },
        ],
      },
      {
        id: "quality-gates",
        title: "Quality gates antes de abrir PR",
        blocks: [
          {
            type: "steps",
            items: [
              "Ejecuta `cargo fmt --check`.",
              "Ejecuta `cargo clippy --all-targets --all-features -- -D warnings`.",
              "Ejecuta `cargo test`.",
              "Ejecuta `bash tests/semantic_e2e_battery.sh`.",
              "Si tocaste frontend: `npm --prefix frontend run build`.",
            ],
          },
          {
            type: "callout",
            tone: "info",
            title: "PR pequeño gana",
            text: "Es mas facil revisar cambios acotados con fixtures y casos claros que una mega PR con multiples objetivos.",
          },
        ],
      },
      {
        id: "pr-template",
        title: "Estructura recomendada del PR",
        blocks: [
          {
            type: "list",
            items: [
              "Contexto: que problema resuelve y por que ahora.",
              "Cambios: parser/modelo/diff/merge/frontend/tests.",
              "Riesgos: donde podria romper matching o estabilidad de IDs.",
              "Validacion: comandos ejecutados y resultado.",
              "Siguiente paso opcional si aplica.",
            ],
          },
        ],
      },
      {
        id: "issues-roadmap",
        title: "Como proponer features",
        blocks: [
          {
            type: "paragraph",
            text: "Para nuevas capacidades (nuevo lenguaje, heuristicas de rename, merge asistido), abre issue con ejemplo minimo reproducible y expected behavior. Incluye caso base/ours/theirs cuando aplique.",
          },
        ],
      },
    ],
  },
  {
    slug: "faq",
    title: "FAQ y troubleshooting",
    summary:
      "Preguntas frecuentes, limites del MVP y resolucion de problemas comunes.",
    category: "contributing",
    audience: "all",
    updatedAt: "2026-03-16",
    readingMinutes: 6,
    sections: [
      {
        id: "faq-general",
        title: "Preguntas frecuentes",
        blocks: [
          {
            type: "list",
            items: [
              "¿Semantit reemplaza Git? No. Complementa diff/merge con semantica.",
              "¿Puede regenerar automaticamente el archivo fusionado? En este MVP no; entrega merge semantico y conflictos explicados.",
              "¿Soporta lenguajes ademas de TypeScript? Aun no en runtime, pero la arquitectura ya es extensible.",
            ],
          },
        ],
      },
      {
        id: "errors",
        title: "Errores comunes",
        blocks: [
          {
            type: "list",
            items: [
              "Error de parser TS: revisa sintaxis invalida o extensiones no soportadas por la configuracion actual.",
              "No aparece `semantit-cli`: ejecuta `cargo build -p semantit-cli`.",
              "Script E2E falla por `jq`: instala `jq` y reintenta.",
            ],
          },
        ],
      },
      {
        id: "diagnostics",
        title: "Diagnostico rapido",
        blocks: [
          {
            type: "code",
            language: "bash",
            code: "cargo run -p semantit-cli -- parse your_file.ts --json | jq '.entities | length'\ncargo run -p semantit-cli -- diff old.ts new.ts --json | jq '.summary'\ncargo run -p semantit-cli -- merge base.ts ours.ts theirs.ts --json | jq '.status, .conflicts'",
          },
        ],
      },
    ],
  },
];

export function getAllDocs(): DocPage[] {
  return docs;
}

export function getDocBySlug(slug: string): DocPage | undefined {
  return docs.find((doc) => doc.slug === slug);
}

export function getDocsByCategory(category: DocPage["category"]): DocPage[] {
  return docs.filter((doc) => doc.category === category);
}

export function getDocSlugs(): string[] {
  return docs.map((doc) => doc.slug);
}

export function getNeighborDocs(slug: string): {
  previous?: DocPage;
  next?: DocPage;
} {
  const index = docs.findIndex((doc) => doc.slug === slug);
  if (index === -1) {
    return {};
  }
  return {
    previous: index > 0 ? docs[index - 1] : undefined,
    next: index < docs.length - 1 ? docs[index + 1] : undefined,
  };
}
