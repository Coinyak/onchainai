import { clientApiBase } from "@/lib/api";

/** Clear session via GET navigation so Set-Cookie clears apply reliably. */
export function signOut(): void {
  const base = clientApiBase();
  window.location.assign(`${base}/auth/logout`);
}