"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { BookOpenText, Compass, Rocket, Wrench } from "lucide-react";
import type { ReactNode } from "react";

import {
  docCategories,
  getDocsByCategory,
  type DocCategory,
} from "@/lib/docs-content";

const categoryIcons: Record<DocCategory["id"], ReactNode> = {
  "getting-started": <Rocket className="h-4 w-4" />,
  concepts: <Compass className="h-4 w-4" />,
  reference: <Wrench className="h-4 w-4" />,
  contributing: <BookOpenText className="h-4 w-4" />,
};

export function DocsSidebar() {
  const pathname = usePathname();

  return (
    <aside className="docs-sidebar rounded-[28px] p-5">
      <div className="mb-4 text-xs font-semibold uppercase tracking-[0.2em] text-[var(--ink-soft)]">
        Documentacion Semantit
      </div>
      <Link
        href="/docs"
        className={`mb-5 flex items-center justify-between rounded-2xl border px-3 py-3 text-sm font-medium transition ${
          pathname === "/docs"
            ? "border-[var(--accent)] bg-[var(--accent-soft)] text-[var(--accent)]"
            : "border-[var(--line)] bg-white/75 text-[var(--ink)] hover:border-[var(--accent)] hover:text-[var(--accent)]"
        }`}
      >
        Inicio de docs
        <span className="text-xs uppercase tracking-[0.16em]">Home</span>
      </Link>
      <div className="space-y-5">
        {docCategories.map((category) => {
          const docs = getDocsByCategory(category.id);
          return (
            <section key={category.id}>
              <div className="mb-2 flex items-center gap-2 text-xs font-semibold uppercase tracking-[0.16em] text-[var(--ink-soft)]">
                <span className="text-[var(--accent)]">{categoryIcons[category.id]}</span>
                {category.title}
              </div>
              <div className="space-y-2">
                {docs.map((doc) => {
                  const href = `/docs/${doc.slug}`;
                  const active = pathname === href;
                  return (
                    <Link
                      key={doc.slug}
                      href={href}
                      className={`block rounded-xl px-3 py-2 text-sm leading-6 transition ${
                        active
                          ? "bg-[rgba(20,33,61,0.12)] font-semibold text-[var(--ink)]"
                          : "text-[var(--ink-soft)] hover:bg-white/65 hover:text-[var(--ink)]"
                      }`}
                    >
                      {doc.title}
                    </Link>
                  );
                })}
              </div>
            </section>
          );
        })}
      </div>
    </aside>
  );
}
