"use client";

import { useEffect, useState } from "react";
import { useAuth } from "@/lib/auth";
import {
  getAdminSiteSettings,
  updateSiteSettings,
  type FooterLink,
  type SiteSettings,
  type UpdateSiteSettingsPayload,
} from "@/lib/api";

function toPayload(settings: SiteSettings, content: {
  hero_title: string;
  hero_subtitle: string;
  about_content: string;
  footer_links: FooterLink[];
}): UpdateSiteSettingsPayload {
  return {
    site_name: settings.site_name,
    slogan: settings.slogan,
    description: settings.description,
    mcp_endpoint: settings.mcp_endpoint,
    search_keywords_raw: settings.search_keywords.join(", "),
    allow_free_registration: settings.allow_free_registration,
    require_tool_approval: settings.require_tool_approval,
    allow_x402_registration: settings.allow_x402_registration,
    default_referral_bps: settings.default_referral_bps ?? null,
    default_referral_payout_address: settings.default_referral_payout_address ?? null,
    x402_builder_code: settings.x402_builder_code ?? null,
    hero_title: content.hero_title.trim() || null,
    hero_subtitle: content.hero_subtitle.trim() || null,
    about_content: content.about_content.trim() || null,
    footer_links: content.footer_links,
  };
}

function parseFooterLinks(raw: string): FooterLink[] {
  const parsed = JSON.parse(raw) as unknown;
  if (!Array.isArray(parsed)) {
    throw new Error("Footer links must be a JSON array");
  }
  return parsed.map((item) => {
    if (
      typeof item !== "object" ||
      item === null ||
      typeof (item as FooterLink).label !== "string" ||
      typeof (item as FooterLink).url !== "string"
    ) {
      throw new Error("Each footer link needs label and url strings");
    }
    return {
      label: (item as FooterLink).label,
      url: (item as FooterLink).url,
    };
  });
}

export default function AdminContentPage() {
  const { isLoading, isAdmin } = useAuth();
  const [settings, setSettings] = useState<SiteSettings | null>(null);
  const [heroTitle, setHeroTitle] = useState("");
  const [heroSubtitle, setHeroSubtitle] = useState("");
  const [aboutContent, setAboutContent] = useState("");
  const [footerLinksRaw, setFooterLinksRaw] = useState("[]");
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (!isAdmin) return;
    void getAdminSiteSettings()
      .then((data) => {
        setSettings(data);
        setHeroTitle(data.hero_title ?? "");
        setHeroSubtitle(data.hero_subtitle ?? "");
        setAboutContent(data.about_content ?? "");
        setFooterLinksRaw(JSON.stringify(data.footer_links ?? [], null, 2));
      })
      .catch((e: Error) => setError(e.message));
  }, [isAdmin]);

  if (isLoading) {
    return <p className="p-margin text-body-md text-secondary">Loading...</p>;
  }

  if (!isAdmin) {
    return (
      <p className="p-margin text-body-md text-error">Admin access required.</p>
    );
  }

  const onSave = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!settings) return;
    setSaving(true);
    setError(null);
    setSaved(false);
    try {
      const footer_links = parseFooterLinks(footerLinksRaw);
      const updated = await updateSiteSettings(
        toPayload(settings, {
          hero_title: heroTitle,
          hero_subtitle: heroSubtitle,
          about_content: aboutContent,
          footer_links,
        }),
      );
      setSettings(updated);
      setSaved(true);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to save content");
    } finally {
      setSaving(false);
    }
  };

  return (
    <main className="mx-auto max-w-3xl p-margin">
      <h1 className="text-h2 text-primary">Content Management</h1>
      <p className="mt-sm text-body-md text-secondary">
        Edit hero copy, about page content, and footer links.
      </p>

      {!settings ? (
        <p className="mt-md text-body-md text-secondary">Loading settings...</p>
      ) : (
        <form className="mt-lg space-y-md" onSubmit={onSave}>
          <label className="block">
            <span className="text-body-md font-medium">Hero title</span>
            <input
              className="mt-xs w-full rounded-md border border-border px-sm py-sm text-body-md"
              value={heroTitle}
              onChange={(e) => setHeroTitle(e.target.value)}
            />
          </label>

          <label className="block">
            <span className="text-body-md font-medium">Hero subtitle</span>
            <input
              className="mt-xs w-full rounded-md border border-border px-sm py-sm text-body-md"
              value={heroSubtitle}
              onChange={(e) => setHeroSubtitle(e.target.value)}
            />
          </label>

          <label className="block">
            <span className="text-body-md font-medium">About content</span>
            <textarea
              className="mt-xs min-h-40 w-full rounded-md border border-border px-sm py-sm text-body-md"
              value={aboutContent}
              onChange={(e) => setAboutContent(e.target.value)}
            />
          </label>

          <label className="block">
            <span className="text-body-md font-medium">Footer links (JSON)</span>
            <p className="mt-xs text-body-sm text-secondary">
              Array of objects with label and url, e.g. [{"{"}&quot;label&quot;:&quot;Docs&quot;,&quot;url&quot;:&quot;https://...&quot;{"}"}]
            </p>
            <textarea
              className="mt-xs min-h-32 w-full rounded-md border border-border px-sm py-sm font-mono text-body-sm"
              value={footerLinksRaw}
              onChange={(e) => setFooterLinksRaw(e.target.value)}
            />
          </label>

          {error ? <p className="text-body-md text-error">{error}</p> : null}
          {saved ? <p className="text-body-md text-success">Content saved.</p> : null}

          <button
            type="submit"
            disabled={saving}
            className="rounded-md bg-primary px-md py-sm text-body-md text-neutral-bg disabled:opacity-50"
          >
            {saving ? "Saving..." : "Save content"}
          </button>
        </form>
      )}
    </main>
  );
}