"use client";

import { useState } from "react";
import { useQuery, useMutation } from "@tanstack/react-query";
import { getAdminSiteSettings, updateSiteSettings, type SiteSettings } from "@/lib/api";

function SettingsForm({ initial }: { initial: SiteSettings }) {
  const [siteName, setSiteName] = useState(initial.site_name);
  const [slogan, setSlogan] = useState(initial.slogan);
  const [description, setDescription] = useState(initial.description);
  const [mcpEndpoint, setMcpEndpoint] = useState(initial.mcp_endpoint);
  const [mcpPremiumEnabled, setMcpPremiumEnabled] = useState(initial.mcp_premium_enabled);
  const [mcpPremiumPayTo, setMcpPremiumPayTo] = useState(initial.mcp_premium_pay_to_address ?? "");
  const [mcpPremiumPrice, setMcpPremiumPrice] = useState(initial.mcp_premium_price ?? "");
  const [mcpPremiumNetwork, setMcpPremiumNetwork] = useState(initial.mcp_premium_network);
  const [mcpPremiumDisplayPrice, setMcpPremiumDisplayPrice] = useState(
    initial.mcp_premium_display_price ?? "",
  );
  const [x402BuilderCode, setX402BuilderCode] = useState(initial.x402_builder_code ?? "");
  const [allowX402Registration, setAllowX402Registration] = useState(
    initial.allow_x402_registration,
  );
  const [defaultReferralBps, setDefaultReferralBps] = useState(
    initial.default_referral_bps != null ? String(initial.default_referral_bps) : "",
  );
  const [defaultReferralPayout, setDefaultReferralPayout] = useState(
    initial.default_referral_payout_address ?? "",
  );

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
        allow_x402_registration: allowX402Registration,
        default_referral_bps: (() => {
          const trimmed = defaultReferralBps.trim();
          if (!trimmed) return null;
          const parsed = Number.parseInt(trimmed, 10);
          return Number.isFinite(parsed) ? parsed : null;
        })(),
        default_referral_payout_address: defaultReferralPayout.trim() || null,
        x402_builder_code: x402BuilderCode.trim() || null,
        mcp_premium_enabled: mcpPremiumEnabled,
        mcp_premium_pay_to_address: mcpPremiumPayTo.trim() || null,
        mcp_premium_price: mcpPremiumPrice.trim() || null,
        mcp_premium_network: mcpPremiumNetwork.trim() || "eip155:8453",
        mcp_premium_asset: initial.mcp_premium_asset ?? null,
        mcp_premium_display_price: mcpPremiumDisplayPrice.trim() || null,
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

      <fieldset className="space-y-3 rounded-md border border-border p-4" data-testid="x402-site-settings">
        <legend className="text-body-sm font-medium px-1">x402 site defaults</legend>
        <p className="text-body-sm text-secondary">
          Site-level referral defaults apply when per-tool fields are empty. Open listing stays off
          until you enable registration below.
        </p>
        <label className="flex items-center gap-2 min-h-touch">
          <input
            type="checkbox"
            checked={allowX402Registration}
            onChange={(e) => setAllowX402Registration(e.target.checked)}
            data-testid="allow-x402-registration"
          />
          <span className="text-body-sm">Allow x402 open listing submissions</span>
        </label>
        <label className="block">
          <span className="text-body-sm text-secondary">Default referral bps (0–10000)</span>
          <input
            className="mt-1 w-full min-h-touch px-4 rounded-md border border-border"
            type="number"
            min={0}
            max={10000}
            value={defaultReferralBps}
            onChange={(e) => setDefaultReferralBps(e.target.value)}
            placeholder="250"
            data-testid="default-referral-bps"
          />
        </label>
        <label className="block">
          <span className="text-body-sm text-secondary">Default referral payout address</span>
          <input
            className="mt-1 w-full min-h-touch px-4 rounded-md border border-border font-mono text-code"
            value={defaultReferralPayout}
            onChange={(e) => setDefaultReferralPayout(e.target.value)}
            placeholder="0x..."
            data-testid="default-referral-payout"
          />
        </label>
      </fieldset>

      <label className="block">
        <span className="text-body-sm text-secondary">x402 Builder Code (Base)</span>
        <input
          className="mt-1 w-full min-h-touch px-4 rounded-md border border-border font-mono text-code"
          value={x402BuilderCode}
          onChange={(e) => setX402BuilderCode(e.target.value)}
          placeholder="bc_..."
          data-testid="x402-builder-code"
        />
        <p className="mt-1 text-body-sm text-secondary">
          From{" "}
          <a href="https://dashboard.base.org" className="underline" target="_blank" rel="noopener noreferrer">
            dashboard.base.org
          </a>
          . Used for onchain attribution on OnchainAI x402 settlements.
        </p>
      </label>

      <fieldset className="space-y-3 rounded-md border border-border p-4" data-testid="mcp-premium-settings">
        <legend className="text-body-sm font-medium px-1">MCP premium (Axis B x402)</legend>
        <p className="text-body-sm text-secondary">
          Charge for export_toolkit via HTTP 402 on POST /mcp. compare_tools is Free Forever (OD-FTG) and is never charged. Discovery tools stay free.
        </p>
        <label className="flex items-center gap-2 min-h-touch">
          <input
            type="checkbox"
            checked={mcpPremiumEnabled}
            onChange={(e) => setMcpPremiumEnabled(e.target.checked)}
            data-testid="mcp-premium-enabled"
          />
          <span className="text-body-sm">Enable MCP premium x402</span>
        </label>
        <label className="block">
          <span className="text-body-sm text-secondary">Pay-to address (EVM)</span>
          <input
            className="mt-1 w-full min-h-touch px-4 rounded-md border border-border font-mono text-code"
            value={mcpPremiumPayTo}
            onChange={(e) => setMcpPremiumPayTo(e.target.value)}
            placeholder="0x..."
            data-testid="mcp-premium-pay-to"
          />
        </label>
        <label className="block">
          <span className="text-body-sm text-secondary">Price (x402 accepts)</span>
          <input
            className="mt-1 w-full min-h-touch px-4 rounded-md border border-border"
            value={mcpPremiumPrice}
            onChange={(e) => setMcpPremiumPrice(e.target.value)}
            placeholder="$0.01"
            data-testid="mcp-premium-price"
          />
        </label>
        <label className="block">
          <span className="text-body-sm text-secondary">Network</span>
          <input
            className="mt-1 w-full min-h-touch px-4 rounded-md border border-border font-mono text-code"
            value={mcpPremiumNetwork}
            onChange={(e) => setMcpPremiumNetwork(e.target.value)}
            placeholder="eip155:8453"
            data-testid="mcp-premium-network"
          />
        </label>
        <label className="block">
          <span className="text-body-sm text-secondary">Display price (agent notice)</span>
          <input
            className="mt-1 w-full min-h-touch px-4 rounded-md border border-border"
            value={mcpPremiumDisplayPrice}
            onChange={(e) => setMcpPremiumDisplayPrice(e.target.value)}
            placeholder="$0.01/call"
            data-testid="mcp-premium-display-price"
          />
        </label>
      </fieldset>

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
