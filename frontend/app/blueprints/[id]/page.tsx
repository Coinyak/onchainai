"use client";

import { use } from "react";
import { BlueprintEditor } from "@/components/blueprint/BlueprintEditor";

export default function BlueprintEditorPage({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = use(params);
  return <BlueprintEditor blueprintId={id} />;
}