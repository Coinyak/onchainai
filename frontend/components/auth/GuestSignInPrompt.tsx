import Link from "next/link";

interface GuestSignInPromptProps {
  title: string;
  description: string;
  testId?: string;
}

/** Single CTA for gated pages — avoids duplicating LoginForm alongside TopNav. */
export function GuestSignInPrompt({
  title,
  description,
  testId = "guest-sign-in",
}: GuestSignInPromptProps) {
  return (
    <div className="px-gutter py-12 max-w-[480px] mx-auto text-center">
      <h1 className="text-h2 mb-3">{title}</h1>
      <p className="text-secondary text-body-md mb-6">{description}</p>
      <Link
        href="/login"
        data-testid={testId}
        className="inline-flex items-center justify-center min-h-touch px-6 rounded-md bg-tertiary text-on-tertiary font-medium no-underline hover:bg-[#D96400]"
      >
        Sign in
      </Link>
    </div>
  );
}