"use client";

import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { listAdminCategories, createCategory } from "@/lib/api";

export default function AdminCategoriesPage() {
  const queryClient = useQueryClient();
  const [id, setId] = useState("");
  const [label, setLabel] = useState("");

  const categoriesQuery = useQuery({
    queryKey: ["admin-categories"],
    queryFn: listAdminCategories,
  });

  const createMut = useMutation({
    mutationFn: () => createCategory({ id: id.trim(), label: label.trim(), icon: "terminal" }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["admin-categories"] });
      setId("");
      setLabel("");
    },
  });

  return (
    <div className="px-gutter md:px-6 py-8 max-w-[900px] mx-auto">
      <h1 className="text-h2 mb-6">Category management</h1>
      <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3 mb-8">
        {categoriesQuery.data?.map(({ category, count }) => (
          <div key={category.id} className="border border-border rounded-md p-lg">
            <h3 className="text-h3">{category.label}</h3>
            <p className="text-body-sm text-secondary">{count} tools</p>
          </div>
        ))}
      </div>
      <form
        className="flex flex-wrap gap-3 items-end"
        onSubmit={(e) => {
          e.preventDefault();
          createMut.mutate();
        }}
      >
        <label>
          <span className="text-body-sm text-secondary">ID</span>
          <input className="block mt-1 min-h-touch px-3 rounded-md border border-border" value={id} onChange={(e) => setId(e.target.value)} />
        </label>
        <label>
          <span className="text-body-sm text-secondary">Label</span>
          <input className="block mt-1 min-h-touch px-3 rounded-md border border-border" value={label} onChange={(e) => setLabel(e.target.value)} />
        </label>
        <button type="submit" className="min-h-touch px-4 rounded-md bg-tertiary text-on-tertiary">
          Add category
        </button>
      </form>
    </div>
  );
}