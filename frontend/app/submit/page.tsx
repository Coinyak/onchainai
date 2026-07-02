"use client";

import { useState } from "react";
import { useQuery, useMutation } from "@tanstack/react-query";
import { SiteShell } from "@/components/layout/SiteShell";
import { LoginForm } from "@/components/auth/LoginForm";
import { useAuth } from "@/lib/auth";
import { submitTool, listMySubmissions } from "@/lib/api";

export default function SubmitPage() {
  const { isAuthenticated } = useAuth();
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [toolType, setToolType] = useState("mcp");
  const functionId = "dev-tool";
  const [repoUrl, setRepoUrl] = useState("");
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const submissionsQuery = useQuery({
    queryKey: ["my-submissions"],
    queryFn: listMySubmissions,
    enabled: isAuthenticated,
  });

  const submitMut = useMutation({
    mutationFn: () =>
      submitTool({
        name,
        description,
        tool_type: toolType,
        function: functionId,
        repo_url: repoUrl || null,
      }),
    onSuccess: (row) => {
      setMessage(`Submission received (status: ${row.status}).`);
      submissionsQuery.refetch();
    },
    onError: (e: Error) => setError(e.message),
  });

  if (!isAuthenticated) {
    return (
      <SiteShell>
        <div className="px-gutter py-12 max-w-[480px] mx-auto">
          <LoginForm />
        </div>
      </SiteShell>
    );
  }

  return (
    <SiteShell>
      <div className="px-gutter md:px-8 py-8 max-w-[720px] mx-auto">
        <h1 className="text-h1 mb-2">Submit a tool</h1>
        <p className="text-secondary text-body-md mb-8">
          Suggest a crypto tool for operator review before it appears publicly.
        </p>
        <form
          className="space-y-4"
          onSubmit={(e) => {
            e.preventDefault();
            setError(null);
            submitMut.mutate();
          }}
        >
          <label className="block">
            <span className="text-body-sm text-secondary">Name</span>
            <input
              required
              className="mt-1 w-full min-h-touch px-4 rounded-md border border-border"
              value={name}
              onChange={(e) => setName(e.target.value)}
            />
          </label>
          <label className="block">
            <span className="text-body-sm text-secondary">Description</span>
            <textarea
              required
              className="mt-1 w-full min-h-[120px] p-4 rounded-md border border-border"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
            />
          </label>
          <label className="block">
            <span className="text-body-sm text-secondary">Type</span>
            <select
              className="mt-1 w-full min-h-touch px-4 rounded-md border border-border"
              value={toolType}
              onChange={(e) => setToolType(e.target.value)}
            >
              {["mcp", "cli", "sdk", "api", "x402", "skill"].map((t) => (
                <option key={t} value={t}>{t.toUpperCase()}</option>
              ))}
            </select>
          </label>
          <label className="block">
            <span className="text-body-sm text-secondary">Repository URL</span>
            <input
              className="mt-1 w-full min-h-touch px-4 rounded-md border border-border"
              value={repoUrl}
              onChange={(e) => setRepoUrl(e.target.value)}
            />
          </label>
          <button
            type="submit"
            className="min-h-touch px-6 rounded-md bg-tertiary text-on-tertiary font-medium hover:bg-[#D96400]"
            disabled={submitMut.isPending}
          >
            Submit for review
          </button>
        </form>
        {message && <p className="mt-4 text-success text-body-sm">{message}</p>}
        {error && <p className="mt-4 text-error text-body-sm">{error}</p>}

        {submissionsQuery.data && submissionsQuery.data.length > 0 && (
          <section className="mt-12">
            <h2 className="text-h2 mb-4">Your submissions</h2>
            <ul className="divide-y divide-border">
              {submissionsQuery.data.map((s) => (
                <li key={s.id} className="py-3 text-body-md">
                  {s.name} — <span className="text-secondary">{s.status}</span>
                </li>
              ))}
            </ul>
          </section>
        )}
      </div>
    </SiteShell>
  );
}