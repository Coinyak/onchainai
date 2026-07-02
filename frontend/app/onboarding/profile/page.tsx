"use client";

import { useState } from "react";
import Link from "next/link";
import { SiteShell } from "@/components/layout/SiteShell";
import { API_BASE } from "@/lib/api";

export default function OnboardingProfilePage() {
  const [nickname, setNickname] = useState("");
  const [bio, setBio] = useState("");
  const [message, setMessage] = useState<string | null>(null);

  async function saveProfile(e: React.FormEvent) {
    e.preventDefault();
    setMessage("Saving profile...");
    try {
      const res = await fetch(`${API_BASE}/auth/onboarding/complete`, {
        method: "POST",
        credentials: "include",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ nickname: nickname.trim(), bio: bio.trim() || null }),
      });
      setMessage(res.ok ? "Profile saved." : "Could not save profile.");
    } catch {
      setMessage("Could not save profile.");
    }
  }

  async function skip() {
    await fetch(`${API_BASE}/auth/onboarding/skip`, {
      method: "POST",
      credentials: "include",
    });
    window.location.href = "/";
  }

  return (
    <SiteShell>
      <div className="px-gutter md:px-8 py-12 max-w-[480px] mx-auto">
        <h1 className="text-h1 mb-2">Welcome to OnchainAI</h1>
        <p className="text-secondary text-body-md mb-8">
          Set up your profile. You can change this later.
        </p>
        <form className="space-y-4" onSubmit={saveProfile}>
          <label className="block">
            <span className="text-body-sm text-secondary">Nickname</span>
            <input
              required
              minLength={2}
              maxLength={20}
              className="mt-1 w-full min-h-touch px-4 rounded-md border border-border"
              value={nickname}
              onChange={(e) => setNickname(e.target.value)}
            />
          </label>
          <label className="block">
            <span className="text-body-sm text-secondary">Bio (optional)</span>
            <textarea
              maxLength={200}
              className="mt-1 w-full min-h-[100px] p-4 rounded-md border border-border"
              value={bio}
              onChange={(e) => setBio(e.target.value)}
            />
          </label>
          <div className="flex gap-3 justify-between">
            <button type="button" className="text-body-sm underline" onClick={skip}>
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
        {message && <p className="mt-4 text-body-sm text-secondary">{message}</p>}
        <p className="mt-6">
          <Link href="/" className="text-tertiary">Back to home</Link>
        </p>
      </div>
    </SiteShell>
  );
}