"use client";

import { use, Suspense } from "react";
import { ToolsBrowser } from "@/components/tools/ToolsBrowser";
import { ToolListSkeleton } from "@/components/ui/Skeleton";

function CategoryContent({ id }: { id: string }) {
  return <ToolsBrowser base={{ category: id }} showToolbarSearch={false} />;
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