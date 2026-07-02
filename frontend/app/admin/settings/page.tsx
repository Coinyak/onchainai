"use client";

import { useState } from "react";
import { useQuery, useMutation } from "@tanstack/react-query";
import { getAdminSiteSettings, updateSiteSettings, type SiteSettings } from "@/lib/api";

function SettingsForm({ initial }: { initial: SiteSettings }) {
  const [siteName, setSiteName] = useState(initial.site_name);
  const [slogan, setSlogan] = useState(initial.slogan);
  const [description, setDescription] = useState(initial.description);
  const [mcpEndpoint, setMcpEndpoint] = useState(initial.mcp_endpoint);

  const saveMut = useMutation({
    mutationFn: () =>
      updateSiteSettings({
        site_name: siteName,
        slogan,
        description,
        mcp_endpoint: mcpEndpoint,
        search_keywords_raw: initial.search_keywords.join(", "),
        allow_free_registration: initial.allow_free_registration,
        require_tool_approval: initial.require_tool_approval,
        allow_x402_registration: initial.allow_x402_registration,
        default_referral_bps: initial.default_referral_bps,
        default_referral_payout_address: initial.default_referral_payout_address,
        x402_builder_code: initial.x402_builder_code,
        hero_title: initial.hero_title,
        hero_subtitle: initial.hero_subtitle,
        about_content: initial.about_content,
        footer_links: initial.footer_links,
      }),
  });

  return (
    <form
      className="space-y-4"
      onSubmit={(e) => {
        e.preventDefault();
        saveMut.mutate();
      }}
    >
      <label className="block">
        <span className="text-body-sm text-secondary">Site name</span>
        <input className="mt-1 w-full min-h-touch px-4 rounded-md border border-border" value={siteName} onChange={(e) => setSiteName(e.target.value)} />
      </label>
      <label className="block">
        <span className="text-body-sm text-secondary">Slogan</span>
        <input className="mt-1 w-full min-h-touch px-4 rounded-md border border-border" value={slogan} onChange={(e) => setSlogan(e.target.value)} />
      </label>
      <label className="block">
        <span className="text-body-sm text-secondary">Description</span>
        <textarea className="mt-1 w-full min-h-[100px] p-4 rounded-md border border-border" value={description} onChange={(e) => setDescription(e.target.value)} />
      </label>
      <label className="block">
        <span className="text-body-sm text-secondary">MCP endpoint</span>
        <input className="mt-1 w-full min-h-touch px-4 rounded-md border border-border font-mono text-code" value={mcpEndpoint} onChange={(e) => setMcpEndpoint(e.target.value)} />
      </label>
      <button type="submit" className="min-h-touch px-6 rounded-md bg-tertiary text-on-tertiary font-medium" disabled={saveMut.isPending}>
        Save settings
      </button>
      {saveMut.isSuccess && <p className="text-success text-body-sm">Saved.</p>}
    </form>
  );
}

export default function AdminSettingsPage() {
  const settingsQuery = useQuery({
    queryKey: ["admin-settings"],
    queryFn: getAdminSiteSettings,
  });

  return (
    <div className="px-gutter md:px-6 py-8 max-w-[720px] mx-auto">
      <h1 className="text-h2 mb-6">Site settings</h1>
      {settingsQuery.isLoading && <p className="text-secondary">Loading...</p>}
      {settingsQuery.data && <SettingsForm key={settingsQuery.data.updated_at} initial={settingsQuery.data} />}
    </div>
  );
}