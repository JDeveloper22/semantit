import Link from "next/link";
import { CheckCircle2, CircleAlert, CircleCheckBig, Info } from "lucide-react";

import {
  docCategories,
  getAllDocs,
  getDocsByCategory,
  getNeighborDocs,
  type DocBlock,
  type DocPage,
} from "@/lib/docs-content";

function categoryTitle(id: DocPage["category"]) {
  return docCategories.find((category) => category.id === id)?.title ?? id;
}

function blockToneClass(tone: "info" | "warn" | "success") {
  if (tone === "warn") {
    return "border-[rgba(180,83,9,0.24)] bg-[rgba(255,247,237,0.9)] text-[var(--warning)]";
  }
  if (tone === "success") {
    return "border-[rgba(29,120,116,0.24)] bg-[rgba(239,250,249,0.92)] text-[var(--teal)]";
  }
  return "border-[rgba(20,33,61,0.16)] bg-[rgba(255,255,255,0.82)] text-[var(--ink)]";
}

function blockIcon(tone: "info" | "warn" | "success") {
  if (tone === "warn") {
    return <CircleAlert className="h-4 w-4" />;
  }
  if (tone === "success") {
    return <CheckCircle2 className="h-4 w-4" />;
  }
  return <Info className="h-4 w-4" />;
}

function renderBlock(block: DocBlock, index: number) {
  if (block.type === "paragraph") {
    return (
      <p key={index} className="text-[15px] leading-8 text-[var(--ink-soft)]">
        {block.text}
      </p>
    );
  }

  if (block.type === "list") {
    return (
      <ul key={index} className="space-y-2 text-[15px] leading-7 text-[var(--ink-soft)]">
        {block.items.map((item) => (
          <li key={item} className="flex items-start gap-2">
            <span className="mt-2 h-1.5 w-1.5 rounded-full bg-[var(--accent)]" />
            <span>{item}</span>
          </li>
        ))}
      </ul>
    );
  }

  if (block.type === "steps") {
    return (
      <ol key={index} className="space-y-2 text-[15px] leading-7 text-[var(--ink-soft)]">
        {block.items.map((item, stepIndex) => (
          <li key={item} className="flex items-start gap-3">
            <span className="inline-flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-[var(--accent-soft)] text-xs font-semibold text-[var(--accent)]">
              {stepIndex + 1}
            </span>
            <span>{item}</span>
          </li>
        ))}
      </ol>
    );
  }

  if (block.type === "code") {
    return (
      <pre
        key={index}
        className="semantic-scrollbar docs-code overflow-auto rounded-2xl border border-[rgba(16,24,34,0.3)] bg-[#101822] p-4 text-[13px] leading-6 text-white/90"
      >
        <code>{block.code}</code>
      </pre>
    );
  }

  if (block.type === "callout") {
    return (
      <div
        key={index}
        className={`rounded-2xl border p-4 ${blockToneClass(block.tone)}`}
      >
        <div className="mb-1 inline-flex items-center gap-2 text-sm font-semibold">
          {blockIcon(block.tone)}
          {block.title}
        </div>
        <p className="text-sm leading-7 text-[var(--ink-soft)]">{block.text}</p>
      </div>
    );
  }

  return (
    <ul key={index} className="space-y-2 text-[15px] leading-7 text-[var(--ink-soft)]">
      {block.items.map((item) => (
        <li key={item.label} className="flex items-start gap-3">
          <span
            className={`mt-1 ${
              item.done ? "text-[var(--teal)]" : "text-[var(--ink-soft)]"
            }`}
          >
            {item.done ? (
              <CircleCheckBig className="h-4 w-4" />
            ) : (
              <CircleAlert className="h-4 w-4" />
            )}
          </span>
          <span>{item.label}</span>
        </li>
      ))}
    </ul>
  );
}

export function DocsHome() {
  const docs = getAllDocs();

  return (
    <div className="space-y-8">
      <header className="rounded-[26px] border border-[var(--line)] bg-[rgba(255,255,255,0.76)] p-6">
        <div className="mb-3 inline-flex rounded-full bg-[var(--accent-soft)] px-3 py-1 text-xs font-semibold uppercase tracking-[0.16em] text-[var(--accent)]">
          Open source readiness
        </div>
        <h2 className="text-3xl font-semibold tracking-[-0.04em] text-[var(--ink)] sm:text-4xl">
          Documentacion para entender y contribuir.
        </h2>
        <p className="mt-3 max-w-4xl text-[15px] leading-8 text-[var(--ink-soft)]">
          Esta seccion esta pensada para que una persona nueva pueda entender
          Semantit desde conceptos hasta flujo de PR sin depender de contexto
          oral. Empieza por la vision, luego pasa por indice/diff/merge, y
          termina en contribucion y troubleshooting.
        </p>
      </header>

      <section className="grid gap-4 sm:grid-cols-2">
        {docCategories.map((category) => {
          const byCategory = getDocsByCategory(category.id);
          return (
            <article
              key={category.id}
              className="rounded-[24px] border border-[var(--line)] bg-[rgba(255,255,255,0.72)] p-5"
            >
              <h3 className="text-lg font-semibold text-[var(--ink)]">
                {category.title}
              </h3>
              <p className="mt-1 text-sm leading-7 text-[var(--ink-soft)]">
                {category.description}
              </p>
              <div className="mt-4 space-y-2">
                {byCategory.map((doc) => (
                  <Link
                    key={doc.slug}
                    href={`/docs/${doc.slug}`}
                    className="block rounded-xl border border-[var(--line)] bg-white/70 px-3 py-2 text-sm font-medium text-[var(--ink)] transition hover:border-[var(--accent)] hover:text-[var(--accent)]"
                  >
                    {doc.title}
                  </Link>
                ))}
              </div>
            </article>
          );
        })}
      </section>

      <section className="rounded-[24px] border border-[var(--line)] bg-[rgba(255,255,255,0.72)] p-5">
        <h3 className="text-xl font-semibold tracking-[-0.02em] text-[var(--ink)]">
          Mapa completo
        </h3>
        <div className="mt-4 grid gap-3">
          {docs.map((doc) => (
            <Link
              key={doc.slug}
              href={`/docs/${doc.slug}`}
              className="rounded-xl border border-[var(--line)] bg-white/70 px-4 py-3 transition hover:border-[var(--accent)]"
            >
              <div className="flex flex-wrap items-center justify-between gap-2">
                <span className="font-semibold text-[var(--ink)]">{doc.title}</span>
                <span className="text-xs uppercase tracking-[0.16em] text-[var(--ink-soft)]">
                  {categoryTitle(doc.category)} · {doc.readingMinutes} min
                </span>
              </div>
              <p className="mt-1 text-sm leading-7 text-[var(--ink-soft)]">
                {doc.summary}
              </p>
            </Link>
          ))}
        </div>
      </section>
    </div>
  );
}

export function DocArticle({ doc }: { doc: DocPage }) {
  const neighbors = getNeighborDocs(doc.slug);

  return (
    <article className="space-y-10">
      <header className="rounded-[26px] border border-[var(--line)] bg-[rgba(255,255,255,0.76)] p-6">
        <div className="mb-3 flex flex-wrap items-center gap-2 text-xs font-semibold uppercase tracking-[0.16em] text-[var(--ink-soft)]">
          <span className="rounded-full bg-[rgba(20,33,61,0.08)] px-3 py-1 text-[var(--ink)]">
            {categoryTitle(doc.category)}
          </span>
          <span>{doc.readingMinutes} min</span>
          <span>actualizado {doc.updatedAt}</span>
        </div>
        <h2 className="text-3xl font-semibold tracking-[-0.04em] text-[var(--ink)] sm:text-4xl">
          {doc.title}
        </h2>
        <p className="mt-3 max-w-4xl text-[15px] leading-8 text-[var(--ink-soft)]">
          {doc.summary}
        </p>
      </header>

      {doc.sections.map((section) => (
        <section key={section.id} id={section.id} className="docs-anchor scroll-mt-20">
          <h3 className="mb-4 text-2xl font-semibold tracking-[-0.03em] text-[var(--ink)]">
            {section.title}
          </h3>
          <div className="space-y-4">{section.blocks.map(renderBlock)}</div>
        </section>
      ))}

      <footer className="grid gap-3 border-t border-[var(--line)] pt-6 sm:grid-cols-2">
        {neighbors.previous ? (
          <Link
            href={`/docs/${neighbors.previous.slug}`}
            className="rounded-2xl border border-[var(--line)] bg-white/70 px-4 py-3 text-sm transition hover:border-[var(--accent)]"
          >
            <div className="text-xs uppercase tracking-[0.14em] text-[var(--ink-soft)]">
              Anterior
            </div>
            <div className="mt-1 font-semibold text-[var(--ink)]">
              {neighbors.previous.title}
            </div>
          </Link>
        ) : (
          <div />
        )}
        {neighbors.next ? (
          <Link
            href={`/docs/${neighbors.next.slug}`}
            className="rounded-2xl border border-[var(--line)] bg-white/70 px-4 py-3 text-right text-sm transition hover:border-[var(--accent)]"
          >
            <div className="text-xs uppercase tracking-[0.14em] text-[var(--ink-soft)]">
              Siguiente
            </div>
            <div className="mt-1 font-semibold text-[var(--ink)]">
              {neighbors.next.title}
            </div>
          </Link>
        ) : (
          <div />
        )}
      </footer>
    </article>
  );
}

