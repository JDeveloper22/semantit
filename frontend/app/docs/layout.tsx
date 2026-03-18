import type { Metadata } from "next";

import { DocsShell } from "@/components/docs/docs-shell";

export const metadata: Metadata = {
  title: "Semantit Docs",
  description:
    "Documentacion de Semantit: conceptos, arquitectura, CLI, merge semantico y guia de contribucion.",
};

export default function DocsLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return <DocsShell>{children}</DocsShell>;
}

