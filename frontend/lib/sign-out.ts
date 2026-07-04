import type { QueryClient } from "@tanstack/react-query";
import { clientApiBase } from "@/lib/api";

/** Clear OnchainAI session cookies and hard-navigate away (avoids stale React Query `me`). */
export async function signOut(queryClient?: QueryClient): Promise<void> {
  const base = clientApiBase();
  try {
    await fetch(`${base}/auth/logout`, {
      method: "POST",
      credentials: "include",
      redirect: "manual",
    });
  } catch {
    // Still navigate — user may already be logged out.
  }

  queryClient?.removeQueries({ queryKey: ["me"] });
  queryClient?.removeQueries({ queryKey: ["admin-check"] });

  window.location.assign("/login?signed_out=1");
}