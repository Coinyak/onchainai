/** Maps `/login?auth=` codes from the API to user-facing English messages. */
export function authErrorMessage(code: string | null | undefined): string | null {
  if (!code?.trim()) return null;
  const messages: Record<string, string> = {
    github_denied: "GitHub sign-in was cancelled.",
    github_missing_code: "GitHub did not return an authorization code. Try again.",
    github_missing_state: "GitHub sign-in expired. Try again.",
    github_state_mismatch: "GitHub sign-in could not be verified. Try again.",
    github_token_exchange: "Could not complete GitHub sign-in. Try again later.",
    github_redirect_mismatch:
      "GitHub OAuth callback URL is not registered for this site. Add https://www.onchain-ai.xyz/auth/callback in your GitHub OAuth app settings, then try again.",
    github_user_fetch: "Could not load your GitHub profile. Try again later.",
    github_profile_exists: "This GitHub account is already linked to another profile.",
    github_profile_setup: "Could not create your profile. Try again or contact support.",
    github_profile: "Could not save your profile. Try again.",
    google_denied: "Google sign-in was cancelled.",
    google_not_configured: "Google sign-in isn't available yet. Use GitHub, email, or wallet.",
    google_missing_code: "Google did not return an authorization code. Try again.",
    google_missing_state: "Google sign-in expired. Try again.",
    google_state_mismatch: "Google sign-in could not be verified. Try again.",
    google_token_exchange: "Could not complete Google sign-in. Try again later.",
    google_user_fetch: "Could not load your Google profile. Try again later.",
    google_profile_exists: "This Google account is already linked to another profile.",
    google_profile_setup: "Could not create your profile. Try again or contact support.",
    google_profile: "Could not save your profile. Try again.",
    wallet_failed: "Wallet sign-in failed. Try again.",
    wallet_profile_setup: "Could not create your profile. Try again or contact support.",
    admin_required: "Admin access required. Sign in with an admin account to continue.",
  };
  return messages[code] ?? "Sign-in failed. Try again.";
}