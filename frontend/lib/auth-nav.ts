import type { QueryClient } from "@tanstack/react-query";
import { getMe } from "@/lib/api";

/** Drop cached session and fetch `/api/v2/me` with the latest cookies. */
export async function refreshSessionAfterAuth(queryClient: QueryClient) {
  queryClient.removeQueries({ queryKey: ["me"] });
  try {
    return await queryClient.fetchQuery({ queryKey: ["me"], queryFn: getMe });
  } catch {
    return null;
  }
}

/**
 * Hard navigation that always reloads auth state.
 * Same-path redirects use `reload()`; cross-path adds a cache-busting query param.
 */
export function hardNavigateAfterAuth(path: string) {
  const target = new URL(path, window.location.origin);
  const current = `${window.location.pathname}${window.location.search}`;
  const next = `${target.pathname}${target.search}`;
  if (next === current) {
    window.location.reload();
    return;
  }
  target.searchParams.set("_auth", Date.now().toString(36));
  window.location.replace(`${target.pathname}${target.search}${target.hash}`);
}