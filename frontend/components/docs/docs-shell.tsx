import Link from "next/link";
import { ArrowLeftRight, BookMarked, ExternalLink } from "lucide-react";

import { DocsSidebar } from "@/components/docs/docs-sidebar";

export function DocsShell({ children }: { children: React.ReactNode }) {
  const repoUrl = process.env.NEXT_PUBLIC_SEMANTIT_REPO_URL ?? "https://github.com";

  return (
    <div className="px-4 py-6 md:px-8 lg:px-10">
      <div className="mx-auto grid max-w-[1600px] gap-5 xl:grid-cols-[330px_minmax(0,1fr)]">
        <div className="space-y-5">
          <section className="docs-top shell-card rounded-[28px] p-5">
            <div className="mb-4 inline-flex items-center gap-2 rounded-full border border-[var(--line)] bg-white/75 px-3 py-1 text-xs font-semibold uppercase tracking-[0.18em] text-[var(--ink-soft)]">
              <BookMarked className="h-4 w-4 text-[var(--accent)]" />
              Docs
            </div>
            <h1 className="text-2xl font-semibold tracking-[-0.03em] text-[var(--ink)]">
              Semantit
            </h1>
            <p className="mt-2 text-sm leading-6 text-[var(--ink-soft)]">
              Documentacion completa para entender el modelo semantico, usar la
              CLI y contribuir con cambios de calidad.
            </p>
            <div className="mt-4 grid gap-2">
              <Link
                href="/"
                className="inline-flex items-center justify-between rounded-xl border border-[var(--line)] bg-white/75 px-3 py-2 text-sm font-medium text-[var(--ink)] transition hover:border-[var(--accent)] hover:text-[var(--accent)]"
              >
                Ir al playground
                <ArrowLeftRight className="h-4 w-4" />
              </Link>
              <a
                href={repoUrl}
                target="_blank"
                rel="noreferrer"
                className="inline-flex items-center justify-between rounded-xl border border-[var(--line)] bg-white/75 px-3 py-2 text-sm font-medium text-[var(--ink)] transition hover:border-[var(--accent)] hover:text-[var(--accent)]"
              >
                Repo de contribucion
                <ExternalLink className="h-4 w-4" />
              </a>
            </div>
          </section>
          <DocsSidebar />
        </div>
        <section className="docs-content shell-card rounded-[30px] p-6 sm:p-8">
          {children}
        </section>
      </div>
    </div>
  );
}
