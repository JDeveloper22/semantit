import type { Metadata } from "next";
import { notFound } from "next/navigation";

import { DocArticle } from "@/components/docs/docs-renderer";
import { getDocBySlug, getDocSlugs } from "@/lib/docs-content";

interface DocPageProps {
  params: Promise<{
    slug: string;
  }>;
}

export async function generateMetadata({
  params,
}: DocPageProps): Promise<Metadata> {
  const { slug } = await params;
  const doc = getDocBySlug(slug);
  if (!doc) {
    return {
      title: "Documento no encontrado | Semantit Docs",
    };
  }

  return {
    title: `${doc.title} | Semantit Docs`,
    description: doc.summary,
  };
}

export function generateStaticParams() {
  return getDocSlugs().map((slug) => ({ slug }));
}

export default async function DocsDetailPage({ params }: DocPageProps) {
  const { slug } = await params;
  const doc = getDocBySlug(slug);
  if (!doc) {
    notFound();
  }

  return <DocArticle doc={doc} />;
}

