"use client";

import { useState, useSyncExternalStore } from "react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { SiteShell } from "@/components/layout/SiteShell";
import { LoginModal } from "@/components/auth/LoginModal";
import { useAuth } from "@/lib/auth";
import { createBlueprint, listBlueprints } from "@/lib/api";
import {
  BLUEPRINT_DRAFT_ID,
  getLocalBlueprintDraftSnapshot,
  subscribeLocalBlueprintDraft,
} from "@/lib/blueprint-storage";
import { timeAgo } from "@/lib/format";

export default function BlueprintsPage() {
  const router = useRouter();
  const queryClient = useQueryClient();
  const { isAuthenticated } = useAuth();
  const [showLogin, setShowLogin] = useState(false);

  const listQuery = useQuery({
    queryKey: ["blueprints"],
    queryFn: listBlueprints,
    enabled: isAuthenticated,
  });

  const createMutation = useMutation({
    mutationFn: () => createBlueprint({ title: "Untitled blueprint", nodes: [] }),
    onSuccess: (blueprint) => {
      queryClient.invalidateQueries({ queryKey: ["blueprints"] });
      router.push(`/blueprints/${blueprint.id}`);
    },
    onError: (err: Error) => {
      alert(err.message);
    },
  });

  const localDraft = useSyncExternalStore(
    subscribeLocalBlueprintDraft,
    getLocalBlueprintDraftSnapshot,
    () => null,
  );

  const handleNewBlueprint = () => {
    if (!isAuthenticated) {
      router.push(`/blueprints/${BLUEPRINT_DRAFT_ID}`);
      return;
    }
    createMutation.mutate();
  };

  return (
    <SiteShell>
      <LoginModal open={showLogin} onClose={() => setShowLogin(false)} />
      <div className="px-gutter md:px-8 py-8 max-w-[1100px] mx-auto" data-testid="blueprint-list">
        <div className="blueprint-list-header">
          <div>
            <h1 className="text-h1 mb-2">Blueprints</h1>
            <p className="text-secondary text-body-md">
              Plan your agent stack on a grid canvas.
            </p>
          </div>
          <button
            type="button"
            className="toolkit-primary-link"
            onClick={handleNewBlueprint}
            disabled={createMutation.isPending}
          >
            New blueprint
          </button>
        </div>

        {!isAuthenticated && (
          <div className="blueprint-list-guest mt-6">
            <p className="text-secondary text-body-md mb-4">
              Sign in to save blueprints to your account. You can try one local draft without signing in.
            </p>
            <div className="flex flex-wrap gap-3">
              <button
                type="button"
                className="toolkit-secondary-link"
                onClick={() => setShowLogin(true)}
              >
                Sign in
              </button>
              <Link href={`/blueprints/${BLUEPRINT_DRAFT_ID}`} className="toolkit-browse-link">
                Open local draft
              </Link>
            </div>
          </div>
        )}

        {isAuthenticated && listQuery.isLoading && (
          <p className="text-secondary mt-8">Loading blueprints...</p>
        )}

        {isAuthenticated && listQuery.data && (
          <div className="blueprint-card-grid mt-8">
            {localDraft && (
              <Link
                href={`/blueprints/${BLUEPRINT_DRAFT_ID}`}
                className="blueprint-card no-underline text-inherit"
              >
                <h2 className="blueprint-card-title">{localDraft.title}</h2>
                <p className="blueprint-card-meta">
                  {localDraft.nodes.length} nodes · Local draft · Updated {timeAgo(localDraft.updatedAt)}
                </p>
              </Link>
            )}
            {listQuery.data.map((item) => (
              <Link
                key={item.id}
                href={`/blueprints/${item.id}`}
                className="blueprint-card no-underline text-inherit"
              >
                <h2 className="blueprint-card-title">{item.title}</h2>
                <p className="blueprint-card-meta">
                  {item.node_count} nodes · Updated {timeAgo(item.updated_at)}
                </p>
              </Link>
            ))}
            {listQuery.data.length === 0 && !localDraft && (
              <p className="text-secondary col-span-full">
                No blueprints yet. Create one to start planning your stack.
              </p>
            )}
          </div>
        )}
      </div>
    </SiteShell>
  );
}