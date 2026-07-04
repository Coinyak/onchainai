"use client";

import { use, Suspense } from "react";
import Link from "next/link";
import { ToolsBrowser } from "@/components/tools/ToolsBrowser";
import { ToolListSkeleton } from "@/components/ui/Skeleton";

function formatCategoryId(id: string): string {
  return id
    .split("-")
    .map((part) => (part ? part[0].toUpperCase() + part.slice(1).toLowerCase() : part))
    .join(" ");
}

function CategoryHeader({ id }: { id: string }) {
  const name = formatCategoryId(id);

  return (
    <header className="category-page-header pt-4 md:pt-6" data-testid="category-page-header">
      <nav
        aria-label="Breadcrumb"
        className="text-body-sm text-secondary mb-2"
        data-testid="category-breadcrumb"
      >
        <Link href="/" className="hover:underline underline-offset-2">
          Home
        </Link>
        <span aria-hidden="true"> → </span>
        <Link href="/tools" className="hover:underline underline-offset-2">
          Tools
        </Link>
        <span aria-hidden="true"> → </span>
        <span aria-current="page">{name}</span>
      </nav>
      <h1 className="text-h1 font-bold">{name}</h1>
    </header>
  );
}

function CategoryContent({ id }: { id: string }) {
  return (
    <ToolsBrowser base={{ category: id }} showToolbarSearch={false}>
      <CategoryHeader id={id} />
    </ToolsBrowser>
  );
}

// CategoryContent uses useSearchParams via ToolsBrowser — wrapped in Suspense below.

export default function CategoryPage({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = use(params);
  return (
    <Suspense fallback={<ToolListSkeleton count={6} />}>
      <CategoryContent id={id} />
    </Suspense>
  );
}