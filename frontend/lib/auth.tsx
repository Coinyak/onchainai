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
  const { data, isLoading, refetch } = useQuery({
    queryKey: ["me"],
    queryFn: getMe,
    staleTime: 0,
    gcTime: 0,
    refetchOnMount: "always",
    refetchOnWindowFocus: true,
    retry: false,
  });

  useEffect(() => {
    if (sessionHintPresent()) {
      void refetch();
    }

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
    isLoading,
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