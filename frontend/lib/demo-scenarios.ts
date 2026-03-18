import { readFile } from "node:fs/promises";
import path from "node:path";

import type { DemoScenario } from "@/lib/types";

interface DemoScenarioSeed {
  id: string;
  title: string;
  subtitle: string;
  description: string;
  operation: DemoScenario["operation"];
  leftLabel: string;
  rightLabel: string;
  baseLabel?: string;
  leftFile: string;
  rightFile: string;
  baseFile?: string;
  focusPoints: string[];
}

const scenarioSeeds: DemoScenarioSeed[] = [
  {
    id: "move",
    title: "Mover sin ruido",
    subtitle: "La posición cambia, la identidad no.",
    description:
      "Una función se desplaza dentro del archivo, pero su firma y su cuerpo siguen intactos. Semantit debe reportar movimiento, no una falsa inserción/eliminación.",
    operation: "diff",
    leftLabel: "Antes",
    rightLabel: "Después",
    leftFile: "move_old.ts",
    rightFile: "move_new.ts",
    focusPoints: [
      "IDs estables cuando cambia el orden",
      "0 inserciones y 0 eliminaciones",
      "Movimiento semántico limpio",
    ],
  },
  {
    id: "disjoint",
    title: "Cambios disjuntos",
    subtitle: "Cada rama toca una entidad distinta.",
    description:
      "Una rama cambia `foo()` y la otra toca `bar()`. El merge debería resolverse automáticamente aunque el diff textual clásico tenga más ruido por desplazamientos.",
    operation: "merge",
    leftLabel: "Ours",
    rightLabel: "Theirs",
    baseLabel: "Base",
    leftFile: "disjoint_ours.ts",
    rightFile: "disjoint_theirs.ts",
    baseFile: "disjoint_base.ts",
    focusPoints: [
      "Merge limpio sin conflicto",
      "Cada cambio queda anclado a su entidad",
      "La salida muestra el índice fusionado",
    ],
  },
  {
    id: "same",
    title: "Conflicto real",
    subtitle: "Ambas ramas tocan la misma función.",
    description:
      "El conflicto aparece solo cuando dos ramas modifican la misma entidad. La vista inferior expone el conflicto con contexto semántico, no con ruido del archivo completo.",
    operation: "merge",
    leftLabel: "Ours",
    rightLabel: "Theirs",
    baseLabel: "Base",
    leftFile: "same_ours.ts",
    rightFile: "same_theirs.ts",
    baseFile: "same_base.ts",
    focusPoints: [
      "Conflicto centrado en `foo()`",
      "Contexto del cuerpo cambiado",
      "Sin falsos conflictos por desplazamiento",
    ],
  },
  {
    id: "rename",
    title: "Rename + implementación",
    subtitle: "Se mantiene la identidad semántica.",
    description:
      "Una rama renombra `foo()` a `compute()` y la otra cambia la implementación. Semantit intenta emparejar la misma entidad y resolver el merge de forma automática.",
    operation: "merge",
    leftLabel: "Ours",
    rightLabel: "Theirs",
    baseLabel: "Base",
    leftFile: "rename_ours.ts",
    rightFile: "rename_theirs.ts",
    baseFile: "rename_base.ts",
    focusPoints: [
      "Detección conservadora de rename",
      "Merge limpio con export actualizado",
      "Hash de cuerpo combinado con la nueva firma",
    ],
  },
  {
    id: "split",
    title: "Refactor ambiguo",
    subtitle: "Split y degradación elegante.",
    description:
      "Una función se divide parcialmente en más entidades. Cuando no hay emparejamiento fiable, Semantit baja a revisión manual en vez de inventar una resolución incorrecta.",
    operation: "merge",
    leftLabel: "Ours",
    rightLabel: "Theirs",
    baseLabel: "Base",
    leftFile: "split_ours.ts",
    rightFile: "split_theirs.ts",
    baseFile: "split_base.ts",
    focusPoints: [
      "Conflicto por split ambiguo",
      "La herramienta no fuerza una falsa certeza",
      "Buen caso para merge asistido",
    ],
  },
];

const repoRoot = path.resolve(process.cwd(), "..");
const fixtureRoot = path.join(repoRoot, "fixtures", "typescript");

async function readFixture(name: string): Promise<string> {
  return readFile(path.join(fixtureRoot, name), "utf8");
}

export async function loadDemoScenarios(): Promise<DemoScenario[]> {
  return Promise.all(
    scenarioSeeds.map(async (scenario) => ({
      id: scenario.id,
      title: scenario.title,
      subtitle: scenario.subtitle,
      description: scenario.description,
      operation: scenario.operation,
      leftLabel: scenario.leftLabel,
      rightLabel: scenario.rightLabel,
      baseLabel: scenario.baseLabel,
      left: await readFixture(scenario.leftFile),
      right: await readFixture(scenario.rightFile),
      base: scenario.baseFile ? await readFixture(scenario.baseFile) : undefined,
      focusPoints: scenario.focusPoints,
    })),
  );
}
