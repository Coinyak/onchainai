"use client";

import { Suspense } from "react";
import { ToolsBrowser } from "@/components/tools/ToolsBrowser";
import { ToolListSkeleton } from "@/components/ui/Skeleton";

function ToolsListContent() {
  return <ToolsBrowser base="tools" showToolbarSearch />;
}

export default function ToolsListPage() {
  return (
    <Suspense fallback={<ToolListSkeleton count={6} />}>
      <ToolsListContent />
    </Suspense>
  );
}