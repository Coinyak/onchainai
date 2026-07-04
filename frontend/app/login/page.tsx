"use client";

import { Suspense, useEffect } from "react";
import { useSearchParams } from "next/navigation";
import { SiteShell } from "@/components/layout/SiteShell";
import { LoginForm } from "@/components/auth/LoginForm";
import { authErrorMessage } from "@/lib/auth-errors";
import { persistReturnTo, safeReturnTo } from "@/lib/return-to";

function LoginPageContent() {
  const searchParams = useSearchParams();
  const authCode = searchParams.get("auth");
  const authError = authErrorMessage(authCode);
  const signedOut = searchParams.get("signed_out") === "1";
  const returnTo = safeReturnTo(searchParams.get("return_to"));

  useEffect(() => {
    persistReturnTo(returnTo, { fromLoginPage: Boolean(returnTo) });
  }, [returnTo]);

  return (
    <SiteShell>
      <div className="px-gutter md:px-8 py-12 max-w-[480px] mx-auto">
        <LoginForm authError={authError} signedOut={signedOut} />
      </div>
    </SiteShell>
  );
}

export default function LoginPage() {
  return (
    <Suspense
      fallback={
        <SiteShell>
          <div className="px-gutter md:px-8 py-12 max-w-[480px] mx-auto">
            <p className="text-secondary text-body-md">Loading sign-in...</p>
          </div>
        </SiteShell>
      }
    >
      <LoginPageContent />
    </Suspense>
  );
}