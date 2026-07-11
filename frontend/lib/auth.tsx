"use client";

import {
  createContext,
  useContext,
  useEffect,
  useState,
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
  // Cookie is only readable after mount (SSR has no document). Start as
  // "unknown" so we do not flash guest chrome for logged-in users once the
  // session hint is detected — then load /me only when the hint is present.
  const [sessionHint, setSessionHint] = useState<boolean | null>(null);

  useEffect(() => {
    setSessionHint(sessionHintPresent());
  }, []);

  // Skip /api/v2/me for anonymous traffic (bots + guests). Session cookie
  // hint is set on login; only then do we burn an Edge rewrite on Vercel.
  const { data, isLoading, isFetching, refetch } = useQuery({
    queryKey: ["me"],
    queryFn: getMe,
    enabled: sessionHint === true,
    staleTime: 60_000,
    gcTime: 5 * 60_000,
    refetchOnMount: true,
    refetchOnWindowFocus: true,
    retry: false,
  });

  useEffect(() => {
    function onFocus() {
      if (sessionHintPresent()) {
        setSessionHint(true);
        void refetch();
      }
    }

    window.addEventListener("focus", onFocus);
    return () => window.removeEventListener("focus", onFocus);
  }, [refetch]);

  const value: AuthContextValue = {
    user: data ?? null,
    // null = not mounted yet; true = wait for /me; false = guest.
    isLoading:
      sessionHint === null
        ? true
        : sessionHint
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