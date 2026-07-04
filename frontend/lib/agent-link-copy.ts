/** Friendly error copy for Agent Sync — maps API messages for beginners. */

export function mapAgentLinkError(raw: string): string {
  const lower = raw.toLowerCase();
  if (lower.includes("invalid or expired code")) {
    return "That code didn't work. Check for typos, or get a new code from your coding app and try again.";
  }
  if (lower.includes("code expired")) {
    return "This code expired. In your coding app, start connecting again to get a new code.";
  }
  if (lower.includes("already used")) {
    return "This code was already used. If your app still isn't connected, start linking again to get a new code.";
  }
  if (lower.includes("too many") || lower.includes("rate limit")) {
    return "Too many tries. Wait a minute, then try again.";
  }
  if (lower.includes("at most") && lower.includes("active agent tokens")) {
    return "You have the maximum number of connected apps. Disconnect one below, then try again.";
  }
  return "Something went wrong. Try again, or get a new code from your coding app.";
}

export function emptyCodeValidationMessage(): string {
  return "Enter the code from your coding app.";
}