import Link from "next/link";
import { SiteShell } from "@/components/layout/SiteShell";

export default function OnboardingProfilePage() {
  const completeAction = "/onboarding/complete";
  const skipAction = "/onboarding/skip";

  return (
    <SiteShell>
      <div className="px-gutter md:px-8 py-12 max-w-[480px] mx-auto">
        <h1 className="text-h1 mb-2">Welcome to OnchainAI</h1>
        <p className="text-secondary text-body-md mb-8">
          Set up your profile. You can change this later.
        </p>
        <form className="space-y-4" method="post" action={completeAction}>
          <input type="hidden" name="next" value="/" />
          <label className="block">
            <span className="text-body-sm text-secondary">Nickname</span>
            <input
              required
              minLength={2}
              maxLength={20}
              name="nickname"
              className="mt-1 w-full min-h-touch px-4 rounded-md border border-border"
            />
          </label>
          <label className="block">
            <span className="text-body-sm text-secondary">Bio (optional)</span>
            <textarea
              maxLength={200}
              name="bio"
              className="mt-1 w-full min-h-[100px] p-4 rounded-md border border-border"
            />
          </label>
          <div className="flex gap-3 justify-between">
            <button
              type="submit"
              formAction={skipAction}
              formNoValidate
              className="text-body-sm underline bg-transparent border-0 p-0 cursor-pointer"
            >
              Skip for now
            </button>
            <button
              type="submit"
              className="min-h-touch px-6 rounded-md bg-tertiary text-on-tertiary font-medium"
            >
              Save &amp; Continue
            </button>
          </div>
        </form>
        <p className="mt-6">
          <Link href="/" className="text-tertiary">
            Back to home
          </Link>
        </p>
      </div>
    </SiteShell>
  );
}