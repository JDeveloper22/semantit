"use client";

import dynamic from "next/dynamic";
import Link from "next/link";
import { BookOpenText, Play, Sparkles, TerminalSquare } from "lucide-react";
import { useEffect, useRef, useState, useTransition } from "react";

import type {
  ApiResponse,
  DemoScenario,
  FileSemanticIndex,
  MergeResult,
  PlaygroundOperation,
  SemanticDiff,
} from "@/lib/types";

const MonacoEditor = dynamic(() => import("@monaco-editor/react"), {
  ssr: false,
  loading: () => (
    <div className="flex h-[420px] items-center justify-center text-sm text-white/70">
      Cargando editor...
    </div>
  ),
});

type ParseTarget = "left" | "right" | "base";
type PlaygroundResponse = ApiResponse<FileSemanticIndex | SemanticDiff | MergeResult>;
type ConsoleKind = "system" | "command" | "output" | "error";

interface ConsoleEntry {
  id: number;
  kind: ConsoleKind;
  text: string;
}

const editorOptions = {
  automaticLayout: true,
  fontFamily: "var(--font-mono)",
  fontLigatures: true,
  fontSize: 14,
  lineHeight: 22,
  minimap: { enabled: false },
  padding: { top: 18, bottom: 18 },
  scrollBeyondLastLine: false,
  smoothScrolling: true,
  wordWrap: "on" as const,
};

const commandHints = [
  "help",
  "scenarios",
  "parse left",
  "parse right",
  "diff",
  "merge",
  "json",
  "status",
];

const helpText = [
  "Comandos:",
  "  help",
  "  scenarios",
  "  scenario <id>",
  "  parse [left|right|base]",
  "  diff",
  "  merge",
  "  status",
  "  json",
  "  reset",
  "  clear",
].join("\n");

export function SemanticWorkbench({ scenarios }: { scenarios: DemoScenario[] }) {
  const initialScenario = scenarios[0];
  const [selectedScenarioId, setSelectedScenarioId] = useState(
    initialScenario?.id ?? "",
  );
  const [leftSource, setLeftSource] = useState(initialScenario?.left ?? "");
  const [rightSource, setRightSource] = useState(initialScenario?.right ?? "");
  const [baseSource, setBaseSource] = useState(initialScenario?.base ?? "");
  const [response, setResponse] = useState<PlaygroundResponse | null>(null);
  const [toolError, setToolError] = useState<string | null>(null);
  const [isRunning, setIsRunning] = useState(false);

  const [consoleEntries, setConsoleEntries] = useState<ConsoleEntry[]>([
    { id: 1, kind: "system", text: "Semantit console lista. Escribe `help`." },
  ]);
  const nextConsoleId = useRef(2);
  const [consoleInput, setConsoleInput] = useState("");
  const [commandHistory, setCommandHistory] = useState<string[]>([]);
  const [historyIndex, setHistoryIndex] = useState<number | null>(null);
  const consoleViewportRef = useRef<HTMLDivElement | null>(null);

  const hasBooted = useRef(false);
  const [isScenarioPending, startScenarioTransition] = useTransition();

  const currentScenario =
    scenarios.find((scenario) => scenario.id === selectedScenarioId) ?? initialScenario;

  useEffect(() => {
    const viewport = consoleViewportRef.current;
    if (viewport) {
      viewport.scrollTop = viewport.scrollHeight;
    }
  }, [consoleEntries, isRunning]);

  useEffect(() => {
    if (!currentScenario || hasBooted.current) {
      return;
    }
    hasBooted.current = true;
    void runScenario(currentScenario, true);
  }, [currentScenario]);

  function appendConsole(kind: ConsoleKind, text: string) {
    setConsoleEntries((previous) => [
      ...previous,
      {
        id: nextConsoleId.current++,
        kind,
        text,
      },
    ]);
  }

  function resetToScenario(scenario: DemoScenario) {
    startScenarioTransition(() => {
      setSelectedScenarioId(scenario.id);
      setLeftSource(scenario.left);
      setRightSource(scenario.right);
      setBaseSource(scenario.base ?? "");
      setToolError(null);
    });
  }

  async function executeOperation(
    operation: PlaygroundOperation,
    override?: {
      left?: string;
      right?: string;
      base?: string;
      target?: ParseTarget;
    },
  ) {
    if (isRunning) {
      appendConsole("system", "Hay una ejecucion en curso. Espera un momento.");
      return null;
    }

    setIsRunning(true);
    setToolError(null);

    try {
      const payload = {
        operation,
        left: override?.left ?? leftSource,
        right: override?.right ?? rightSource,
        base: override?.base ?? baseSource,
        target: override?.target ?? "left",
      };

      const fetchResponse = await fetch("/api/semantit", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(payload),
      });
      const json = await fetchResponse.json();

      if (!fetchResponse.ok) {
        const message =
          typeof json === "object" &&
          json !== null &&
          "error" in json &&
          typeof json.error === "string"
            ? json.error
            : "Semantit devolvio un error inesperado.";
        throw new Error(message);
      }

      const result = json as PlaygroundResponse;
      setResponse(result);
      appendConsole("output", result.command.join(" "));
      appendConsole("output", summarizeResult(result));
      if (result.stderr) {
        appendConsole("error", result.stderr);
      }
      return result;
    } catch (error) {
      const message =
        error instanceof Error ? error.message : "No se pudo ejecutar Semantit.";
      setToolError(message);
      appendConsole("error", message);
      return null;
    } finally {
      setIsRunning(false);
    }
  }

  async function runScenario(scenario: DemoScenario, announce: boolean) {
    resetToScenario(scenario);
    if (announce) {
      appendConsole(
        "system",
        `Escenario cargado: ${scenario.id} (${scenario.operation}).`,
      );
    }
    return executeOperation(scenario.operation, {
      left: scenario.left,
      right: scenario.right,
      base: scenario.base,
      target: "left",
    });
  }

  async function executeConsoleCommand(raw: string) {
    const command = raw.trim();
    if (!command) {
      return;
    }

    appendConsole("command", `semantit> ${command}`);
    const [name, ...args] = command.split(/\s+/);
    const normalized = name.toLowerCase();

    if (normalized === "help") {
      appendConsole("system", helpText);
      return;
    }

    if (normalized === "clear") {
      setConsoleEntries([
        { id: nextConsoleId.current++, kind: "system", text: "Consola limpiada." },
      ]);
      return;
    }

    if (normalized === "scenarios") {
      appendConsole(
        "system",
        scenarios
          .map((scenario) => `- ${scenario.id}: ${scenario.title}`)
          .join("\n"),
      );
      return;
    }

    if (normalized === "scenario") {
      const scenarioId = args[0];
      if (!scenarioId) {
        appendConsole("error", "Uso: scenario <id>");
        return;
      }
      const nextScenario = scenarios.find((scenario) => scenario.id === scenarioId);
      if (!nextScenario) {
        appendConsole("error", `Escenario no encontrado: ${scenarioId}`);
        return;
      }
      await runScenario(nextScenario, true);
      return;
    }

    if (normalized === "status") {
      appendConsole(
        "system",
        `Escenario=${currentScenario.id} running=${isRunning ? "yes" : "no"} last=${response?.operation ?? "none"}`,
      );
      return;
    }

    if (normalized === "json") {
      if (!response) {
        appendConsole("system", "Aun no hay resultado para mostrar.");
        return;
      }
      appendConsole("output", JSON.stringify(response.result, null, 2));
      return;
    }

    if (normalized === "reset") {
      resetToScenario(currentScenario);
      appendConsole("system", "Editores restaurados al escenario actual.");
      return;
    }

    if (normalized === "parse") {
      const target = (args[0] ?? "left").toLowerCase() as ParseTarget;
      if (target !== "left" && target !== "right" && target !== "base") {
        appendConsole("error", "Uso: parse [left|right|base]");
        return;
      }
      if (target === "base" && !baseSource.trim()) {
        appendConsole("error", "No hay contenido base para parsear.");
        return;
      }
      await executeOperation("parse", { target });
      return;
    }

    if (normalized === "diff") {
      await executeOperation("diff");
      return;
    }

    if (normalized === "merge") {
      if (!baseSource.trim()) {
        appendConsole("error", "Este escenario no tiene base para merge.");
        return;
      }
      await executeOperation("merge");
      return;
    }

    appendConsole("error", `Comando no reconocido: ${name}`);
  }

  function submitConsoleCommand() {
    const command = consoleInput.trim();
    if (!command) {
      return;
    }
    setCommandHistory((previous) => [...previous, command]);
    setHistoryIndex(null);
    setConsoleInput("");
    void executeConsoleCommand(command);
  }

  function onConsoleKeyDown(event: React.KeyboardEvent<HTMLInputElement>) {
    if (event.key === "Enter") {
      event.preventDefault();
      submitConsoleCommand();
      return;
    }

    if (event.key === "ArrowUp") {
      event.preventDefault();
      if (commandHistory.length === 0) {
        return;
      }
      setHistoryIndex((previous) => {
        const next =
          previous === null
            ? commandHistory.length - 1
            : Math.max(previous - 1, 0);
        setConsoleInput(commandHistory[next] ?? "");
        return next;
      });
      return;
    }

    if (event.key === "ArrowDown") {
      event.preventDefault();
      if (commandHistory.length === 0) {
        return;
      }
      setHistoryIndex((previous) => {
        if (previous === null) {
          return null;
        }
        const next = previous + 1;
        if (next >= commandHistory.length) {
          setConsoleInput("");
          return null;
        }
        setConsoleInput(commandHistory[next] ?? "");
        return next;
      });
    }
  }

  return (
    <main className="px-4 py-6 md:px-8 lg:px-10">
      <div className="mx-auto flex max-w-[1500px] flex-col gap-5">
        <section className="shell-card rounded-[26px] px-5 py-5 sm:px-6">
          <div className="flex flex-col gap-4 lg:flex-row lg:items-end lg:justify-between">
            <div className="max-w-4xl">
              <div className="mb-3 inline-flex items-center gap-2 rounded-full border border-[var(--line)] bg-white/70 px-3 py-1 text-xs font-semibold uppercase tracking-[0.18em] text-[var(--ink-soft)]">
                <Sparkles className="h-4 w-4 text-[var(--accent)]" />
                Semantit Playground
              </div>
              <h1 className="text-3xl font-semibold tracking-[-0.04em] text-[var(--ink)] sm:text-4xl">
                Editores + consola semantica
              </h1>
              <p className="mt-2 text-sm leading-7 text-[var(--ink-soft)] sm:text-base">
                Carga ejemplos en el editor y ejecuta la herramienta desde la
                consola integrada.
              </p>
            </div>
            <div className="flex flex-wrap items-center gap-2">
              <select
                aria-label="Seleccionar escenario demo"
                value={selectedScenarioId}
                onChange={(event) => {
                  const nextScenario = scenarios.find(
                    (scenario) => scenario.id === event.target.value,
                  );
                  if (nextScenario) {
                    void runScenario(nextScenario, true);
                  }
                }}
                className="min-w-[260px] rounded-xl border border-[var(--line)] bg-white/80 px-3 py-2 text-sm font-medium text-[var(--ink)] outline-none transition focus:border-[var(--accent)] focus:ring-4 focus:ring-[rgba(221,107,66,0.16)]"
              >
                {scenarios.map((scenario) => (
                  <option key={scenario.id} value={scenario.id}>
                    {scenario.id} · {scenario.title}
                  </option>
                ))}
              </select>
              <button
                type="button"
                onClick={() => void executeConsoleCommand(currentScenario.operation)}
                disabled={isRunning}
                className="inline-flex items-center gap-2 rounded-xl bg-[var(--ink)] px-4 py-2 text-sm font-semibold text-white transition hover:bg-[#22325b] focus:outline-none focus:ring-4 focus:ring-[rgba(20,33,61,0.2)] disabled:cursor-not-allowed disabled:opacity-60"
              >
                <Play className="h-4 w-4" />
                Ejecutar {currentScenario.operation}
              </button>
              <Link
                href="/docs"
                className="inline-flex items-center gap-2 rounded-xl border border-[var(--line)] bg-white/80 px-4 py-2 text-sm font-medium text-[var(--ink)] transition hover:border-[var(--accent)] hover:text-[var(--accent)]"
              >
                <BookOpenText className="h-4 w-4" />
                Docs
              </Link>
            </div>
          </div>
          <div className="mt-4 flex flex-wrap gap-2">
            {commandHints.map((hint) => (
              <button
                key={hint}
                type="button"
                onClick={() => setConsoleInput(hint)}
                className="rounded-full border border-[var(--line)] bg-white/70 px-3 py-1 text-xs font-medium text-[var(--ink-soft)] transition hover:border-[var(--accent)] hover:text-[var(--accent)]"
              >
                {hint}
              </button>
            ))}
          </div>
        </section>

        <section className="grid gap-4 lg:grid-cols-2">
          <EditorPanel
            title={currentScenario.leftLabel}
            value={leftSource}
            onChange={(value) => setLeftSource(value ?? "")}
          />
          <EditorPanel
            title={currentScenario.rightLabel}
            value={rightSource}
            onChange={(value) => setRightSource(value ?? "")}
          />
        </section>

        {baseSource.trim() ? (
          <section className="shell-card rounded-[24px] p-4">
            <div className="mb-2 text-xs font-semibold uppercase tracking-[0.18em] text-[var(--ink-soft)]">
              Base ({currentScenario.baseLabel ?? "base"})
            </div>
            <div className="editor-frame overflow-hidden rounded-[16px]">
              <MonacoEditor
                height="220px"
                defaultLanguage="typescript"
                language="typescript"
                theme="vs-dark"
                value={baseSource}
                onChange={(value) => setBaseSource(value ?? "")}
                options={editorOptions}
              />
            </div>
          </section>
        ) : null}

        <section className="grid gap-4 xl:grid-cols-[1.35fr_0.65fr]">
          <section className="shell-card rounded-[24px] p-4">
            <div className="mb-3 flex items-center justify-between gap-3">
              <div className="inline-flex items-center gap-2 text-sm font-semibold text-[var(--ink)]">
                <TerminalSquare className="h-4 w-4 text-[var(--accent)]" />
                Consola
              </div>
              <div className="text-xs uppercase tracking-[0.16em] text-[var(--ink-soft)]">
                {isRunning || isScenarioPending ? "running" : "idle"}
              </div>
            </div>
            <div
              ref={consoleViewportRef}
              className="semantic-scrollbar h-[330px] overflow-auto rounded-[16px] border border-[rgba(16,24,34,0.24)] bg-[#0d141d] p-3"
              aria-live="polite"
            >
              {consoleEntries.map((entry) => (
                <pre
                  key={entry.id}
                  className={`mb-2 whitespace-pre-wrap break-words text-[12.5px] leading-6 ${
                    entry.kind === "command"
                      ? "text-[#7bb5ff]"
                      : entry.kind === "error"
                        ? "text-[#ff9f7f]"
                        : entry.kind === "system"
                          ? "text-[#c1cce2]"
                          : "text-[#e2e8f7]"
                  }`}
                >
                  {entry.text}
                </pre>
              ))}
            </div>
            <div className="mt-3 flex items-center gap-2">
              <span className="rounded-md bg-[rgba(20,33,61,0.08)] px-2 py-1 text-xs font-semibold text-[var(--ink-soft)]">
                semantit&gt;
              </span>
              <input
                value={consoleInput}
                onChange={(event) => setConsoleInput(event.target.value)}
                onKeyDown={onConsoleKeyDown}
                placeholder="help, scenario rename, parse left, diff, merge..."
                className="min-w-0 flex-1 rounded-xl border border-[var(--line)] bg-white/80 px-3 py-2 text-sm text-[var(--ink)] outline-none transition focus:border-[var(--accent)] focus:ring-4 focus:ring-[rgba(221,107,66,0.16)]"
              />
              <button
                type="button"
                onClick={submitConsoleCommand}
                disabled={isRunning}
                className="rounded-xl bg-[var(--ink)] px-4 py-2 text-sm font-semibold text-white transition hover:bg-[#22325b] disabled:cursor-not-allowed disabled:opacity-60"
              >
                Run
              </button>
            </div>
          </section>

          <section className="shell-card rounded-[24px] p-4">
            <div className="mb-3 text-sm font-semibold text-[var(--ink)]">
              Resultado
            </div>
            {toolError ? (
              <div className="rounded-xl border border-[rgba(180,83,9,0.2)] bg-[rgba(255,250,243,0.92)] p-3 text-sm text-[var(--warning)]">
                {toolError}
              </div>
            ) : null}
            {response ? (
              <div className="space-y-3">
                <div className="rounded-xl border border-[var(--line)] bg-white/75 p-3">
                  <div className="text-xs uppercase tracking-[0.16em] text-[var(--ink-soft)]">
                    Operacion
                  </div>
                  <div className="mt-1 text-sm font-semibold text-[var(--ink)]">
                    {response.operation} · {response.durationMs} ms
                  </div>
                </div>
                <div className="rounded-xl border border-[var(--line)] bg-white/75 p-3">
                  <div className="text-xs uppercase tracking-[0.16em] text-[var(--ink-soft)]">
                    Resumen
                  </div>
                  <p className="mt-1 text-sm leading-7 text-[var(--ink)]">
                    {summarizeResult(response)}
                  </p>
                </div>
                <pre className="semantic-scrollbar max-h-[260px] overflow-auto rounded-xl border border-[rgba(16,24,34,0.24)] bg-[#101822] p-3 font-mono text-[12px] leading-6 text-white/90">
                  {JSON.stringify(response.result, null, 2)}
                </pre>
              </div>
            ) : (
              <div className="rounded-xl border border-dashed border-[var(--line)] bg-white/65 p-4 text-sm text-[var(--ink-soft)]">
                Ejecuta un comando para ver salida semantica.
              </div>
            )}
          </section>
        </section>
      </div>
    </main>
  );
}

function summarizeResult(response: PlaygroundResponse): string {
  if (response.operation === "parse" && isParseResult(response.result)) {
    return `entities=${response.result.entities.length} exports=${response.result.exports.length}`;
  }
  if (response.operation === "diff" && isDiffResult(response.result)) {
    const summary = response.result.summary;
    return `moved=${summary.moved} impl=${summary.implementation_changed} renamed=${summary.renamed} inserted=${summary.inserted} deleted=${summary.deleted}`;
  }
  if (response.operation === "merge" && isMergeResult(response.result)) {
    return `status=${response.result.status} resolved=${response.result.resolved_changes.length} conflicts=${response.result.conflicts.length}`;
  }
  return "Resultado recibido.";
}

function isParseResult(
  value: FileSemanticIndex | SemanticDiff | MergeResult,
): value is FileSemanticIndex {
  return "entities" in value && "exports" in value;
}

function isDiffResult(
  value: FileSemanticIndex | SemanticDiff | MergeResult,
): value is SemanticDiff {
  return "summary" in value && "changes" in value;
}

function isMergeResult(
  value: FileSemanticIndex | SemanticDiff | MergeResult,
): value is MergeResult {
  return "status" in value && "conflicts" in value;
}

function EditorPanel({
  title,
  value,
  onChange,
}: {
  title: string;
  value: string;
  onChange: (value: string | undefined) => void;
}) {
  return (
    <section className="shell-card rounded-[24px] p-4">
      <div className="mb-2 text-xs font-semibold uppercase tracking-[0.18em] text-[var(--ink-soft)]">
        {title}
      </div>
      <div className="editor-frame overflow-hidden rounded-[16px]">
        <MonacoEditor
          height="420px"
          defaultLanguage="typescript"
          language="typescript"
          theme="vs-dark"
          value={value}
          onChange={onChange}
          options={editorOptions}
        />
      </div>
    </section>
  );
}

