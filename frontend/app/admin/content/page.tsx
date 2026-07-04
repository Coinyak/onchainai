"use client";

import { useState } from "react";
import { useQuery, useMutation } from "@tanstack/react-query";
import {
  getAdminSiteSettings,
  updateSiteSettings,
  type FooterLink,
  type SiteSettings,
  type UpdateSiteSettingsPayload,
} from "@/lib/api";

function toPayload(
  settings: SiteSettings,
  content: {
    hero_title: string;
    hero_subtitle: string;
    about_content: string;
    footer_links: FooterLink[];
  },
): UpdateSiteSettingsPayload {
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
    mcp_premium_enabled: settings.mcp_premium_enabled,
    mcp_premium_pay_to_address: settings.mcp_premium_pay_to_address ?? null,
    mcp_premium_price: settings.mcp_premium_price ?? null,
    mcp_premium_network: settings.mcp_premium_network,
    mcp_premium_asset: settings.mcp_premium_asset ?? null,
    mcp_premium_display_price: settings.mcp_premium_display_price ?? null,
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

function ContentForm({ initial }: { initial: SiteSettings }) {
  const [heroTitle, setHeroTitle] = useState(initial.hero_title ?? "");
  const [heroSubtitle, setHeroSubtitle] = useState(initial.hero_subtitle ?? "");
  const [aboutContent, setAboutContent] = useState(initial.about_content ?? "");
  const [footerLinksRaw, setFooterLinksRaw] = useState(
    JSON.stringify(initial.footer_links ?? [], null, 2),
  );
  const [parseError, setParseError] = useState<string | null>(null);

  const saveMut = useMutation({
    mutationFn: async () => {
      const footer_links = parseFooterLinks(footerLinksRaw);
      return updateSiteSettings(
        toPayload(initial, {
          hero_title: heroTitle,
          hero_subtitle: heroSubtitle,
          about_content: aboutContent,
          footer_links,
        }),
      );
    },
    onError: (error) => {
      setParseError(error instanceof Error ? error.message : "Failed to save content");
    },
    onSuccess: (updated) => {
      setParseError(null);
      setFooterLinksRaw(JSON.stringify(updated.footer_links ?? [], null, 2));
    },
  });

  return (
    <form
      className="space-y-4"
      onSubmit={(e) => {
        e.preventDefault();
        setParseError(null);
        saveMut.mutate();
      }}
    >
      <label className="block">
        <span className="text-body-sm text-secondary">Hero title</span>
        <input
          className="mt-2 w-full min-h-touch px-4 rounded-md border border-border"
          value={heroTitle}
          onChange={(e) => setHeroTitle(e.target.value)}
        />
      </label>

      <label className="block">
        <span className="text-body-sm text-secondary">Hero subtitle</span>
        <input
          className="mt-2 w-full min-h-touch px-4 rounded-md border border-border"
          value={heroSubtitle}
          onChange={(e) => setHeroSubtitle(e.target.value)}
        />
      </label>

      <label className="block">
        <span className="text-body-sm text-secondary">About content</span>
        <textarea
          className="mt-2 w-full min-h-[160px] p-4 rounded-md border border-border"
          value={aboutContent}
          onChange={(e) => setAboutContent(e.target.value)}
        />
      </label>

      <label className="block">
        <span className="text-body-sm text-secondary">Footer links (JSON)</span>
        <p className="mt-2 text-body-sm text-secondary">
          Array of objects with label and url, e.g. [{"{"}&quot;label&quot;:&quot;Docs&quot;,&quot;url&quot;:&quot;https://...&quot;{"}"}]
        </p>
        <textarea
          className="mt-2 w-full min-h-[128px] p-4 rounded-md border border-border font-mono text-code"
          value={footerLinksRaw}
          onChange={(e) => setFooterLinksRaw(e.target.value)}
        />
      </label>

      {parseError ? <p className="text-body-sm text-error">{parseError}</p> : null}
      {saveMut.isSuccess && !saveMut.isPending && (
        <p className="text-success text-body-sm">Content saved.</p>
      )}

      <button
        type="submit"
        className="min-h-touch px-6 rounded-md bg-tertiary text-on-tertiary font-medium disabled:opacity-50"
        disabled={saveMut.isPending}
      >
        {saveMut.isPending ? "Saving..." : "Save content"}
      </button>
    </form>
  );
}

export default function AdminContentPage() {
  const settingsQuery = useQuery({
    queryKey: ["admin-settings"],
    queryFn: getAdminSiteSettings,
  });

  return (
    <div className="px-gutter md:px-6 py-8 max-w-[720px] mx-auto">
      <h1 className="text-h2 mb-6">Content management</h1>
      <p className="text-secondary text-body-md mb-6">
        Edit hero copy, about page content, and footer links.
      </p>

      {settingsQuery.isLoading && <p className="text-secondary">Loading settings...</p>}
      {settingsQuery.isError && (
        <p className="text-error text-body-md">
          {settingsQuery.error instanceof Error
            ? settingsQuery.error.message
            : "Failed to load settings"}
        </p>
      )}
      {settingsQuery.data && (
        <ContentForm key={settingsQuery.data.updated_at} initial={settingsQuery.data} />
      )}
    </div>
  );
}