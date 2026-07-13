"use client";

import { useEffect } from "react";
import { usePathname, useRouter } from "next/navigation";
import { useAuth } from "@/lib/auth";
import { consumeReturnTo, peekReturnTo, persistReturnTo } from "@/lib/return-to";

/** After OAuth sign-in, honor return_to stored from /login. */
export function ReturnToRedirect() {
  const { isAuthenticated, isLoading, isAdmin } = useAuth();
  const router = useRouter();
  const pathname = usePathname();

  useEffect(() => {
    if (isLoading || !isAuthenticated) return;
    const target = peekReturnTo();
    if (!target) return;
    if (target === pathname) {
      consumeReturnTo();
      return;
    }
    if (target.startsWith("/admin") && !isAdmin) {
      persistReturnTo(null);
      return;
    }
    consumeReturnTo();
    router.replace(target);
  }, [isAuthenticated, isAdmin, isLoading, pathname, router]);

  return null;
}