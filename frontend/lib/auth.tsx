"use client";

import {
  createContext,
  useContext,
  useEffect,
  type ReactNode,
} from "react";
import { useQuery } from "@tanstack/react-query";
import { getMe, type SessionUser } from "@/lib/api";

interface AuthContextValue {
  user: SessionUser | null;
  isLoading: boolean;
  isAuthenticated: boolean;
  isAdmin: boolean;
  refetch: () => Promise<unknown>;
}

const AuthContext = createContext<AuthContextValue | null>(null);

const SESSION_HINT = "onchainai_session=1";

function sessionHintPresent(): boolean {
  return typeof document !== "undefined" && document.cookie.includes(SESSION_HINT);
}

export function AuthProvider({ children }: { children: ReactNode }) {
  // Skip /api/v2/me for anonymous traffic (bots + guests). Session cookie
  // hint is set on login; only then do we burn an Edge rewrite on Vercel.
  const { data, isLoading, isFetching, refetch } = useQuery({
    queryKey: ["me"],
    queryFn: getMe,
    enabled: typeof document !== "undefined" && sessionHintPresent(),
    staleTime: 60_000,
    gcTime: 5 * 60_000,
    refetchOnMount: true,
    refetchOnWindowFocus: true,
    retry: false,
  });

  useEffect(() => {
    function onFocus() {
      if (sessionHintPresent()) {
        void refetch();
      }
    }

    window.addEventListener("focus", onFocus);
    return () => window.removeEventListener("focus", onFocus);
  }, [refetch]);

  const value: AuthContextValue = {
    user: data ?? null,
    // Guest (no session hint): not loading. Logged-in: wait for first /me.
    isLoading:
      typeof document !== "undefined" && sessionHintPresent()
        ? isLoading || (isFetching && data === undefined)
        : false,
    isAuthenticated: !!data,
    isAdmin: data?.is_admin ?? false,
    refetch,
  };

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth(): AuthContextValue {
  const ctx = useContext(AuthContext);
  if (!ctx) {
    throw new Error("useAuth must be used within AuthProvider");
  }
  return ctx;
}