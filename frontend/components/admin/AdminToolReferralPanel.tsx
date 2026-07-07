"use client";

import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { updateToolReferral, type Tool, type UpdateToolReferralPayload } from "@/lib/api";

interface AdminToolReferralPanelProps {
  tool: Tool;
  onSaved?: (tool: Tool) => void;
}

function toolToPayload(tool: Tool): UpdateToolReferralPayload {
  return {
    slug: tool.slug,
    referral_enabled: tool.referral_enabled ?? false,
    referral_bps: tool.referral_bps ?? null,
    referral_payout_address: tool.referral_payout_address ?? null,
    referral_model: tool.referral_model ?? "attribution",
    x402_pay_to_address: tool.x402_pay_to_address ?? null,
    x402_builder_code: tool.x402_builder_code ?? null,
    payment_verified: tool.payment_verified,
    x402_endpoint_verified: tool.x402_endpoint_verified,
    price_verified: tool.price_verified,
    x402_endpoint: tool.x402_endpoint ?? null,
  };
}

export function AdminToolReferralPanel({ tool, onSaved }: AdminToolReferralPanelProps) {
  const [form, setForm] = useState(() => toolToPayload(tool));
  const [error, setError] = useState<string | null>(null);

  const saveMut = useMutation({
    mutationFn: () => updateToolReferral(form),
    onSuccess: (updated) => {
      setError(null);
      onSaved?.(updated);
    },
    onError: (e: Error) => setError(e.message),
  });

  const verificationIncomplete =
    form.referral_enabled &&
    !(form.payment_verified && form.x402_endpoint_verified && form.price_verified);

  return (
    <fieldset
      className="space-y-3 rounded-md border border-border p-4 mt-4"
      data-testid="tool-referral-panel"
    >
      <legend className="text-body-sm font-medium px-1">x402 referral (per tool)</legend>
      <p className="text-body-sm text-secondary">
        Attribution metadata only — OnchainAI does not process payments or hold funds.
      </p>

      <label className="flex items-center gap-2 min-h-touch">
        <input
          type="checkbox"
          checked={form.referral_enabled}
          onChange={(e) => setForm((prev) => ({ ...prev, referral_enabled: e.target.checked }))}
          data-testid="tool-referral-enabled"
        />
        <span className="text-body-sm">Referral enabled</span>
      </label>

      {verificationIncomplete && (
        <p className="text-body-sm text-secondary" role="status">
          Verification flags are incomplete. Public copy will show “not operator verified yet.”
        </p>
      )}

      <div className="grid gap-3 sm:grid-cols-2">
        <label className="block">
          <span className="text-body-sm text-secondary">Referral bps (0–10000)</span>
          <input
            className="mt-1 w-full min-h-touch px-4 rounded-md border border-border"
            type="number"
            min={0}
            max={10000}
            value={form.referral_bps ?? ""}
            onChange={(e) =>
              setForm((prev) => ({
                ...prev,
                referral_bps: e.target.value === "" ? null : Number(e.target.value),
              }))
            }
            data-testid="tool-referral-bps"
          />
        </label>
        <label className="block">
          <span className="text-body-sm text-secondary">Referral model</span>
          <select
            className="mt-1 w-full min-h-touch px-4 rounded-md border border-border"
            value={form.referral_model ?? "attribution"}
            onChange={(e) => setForm((prev) => ({ ...prev, referral_model: e.target.value }))}
            data-testid="tool-referral-model"
          >
            <option value="attribution">attribution</option>
            <option value="split">split</option>
          </select>
        </label>
      </div>

      <label className="block">
        <span className="text-body-sm text-secondary">Referral payout address</span>
        <input
          className="mt-1 w-full min-h-touch px-4 rounded-md border border-border font-mono text-code"
          value={form.referral_payout_address ?? ""}
          onChange={(e) =>
            setForm((prev) => ({
              ...prev,
              referral_payout_address: e.target.value.trim() || null,
            }))
          }
          placeholder="0x... (falls back to site default when empty)"
          data-testid="tool-referral-payout"
        />
      </label>

      <label className="block">
        <span className="text-body-sm text-secondary">x402 pay-to address</span>
        <input
          className="mt-1 w-full min-h-touch px-4 rounded-md border border-border font-mono text-code"
          value={form.x402_pay_to_address ?? ""}
          onChange={(e) =>
            setForm((prev) => ({
              ...prev,
              x402_pay_to_address: e.target.value.trim() || null,
            }))
          }
          placeholder="0x..."
          data-testid="tool-x402-pay-to"
        />
      </label>

      <label className="block">
        <span className="text-body-sm text-secondary">x402 builder code</span>
        <input
          className="mt-1 w-full min-h-touch px-4 rounded-md border border-border font-mono text-code"
          value={form.x402_builder_code ?? ""}
          onChange={(e) =>
            setForm((prev) => ({
              ...prev,
              x402_builder_code: e.target.value.trim() || null,
            }))
          }
          placeholder="bc_..."
          data-testid="tool-x402-builder-code"
        />
      </label>

      <label className="block">
        <span className="text-body-sm text-secondary">x402 probe endpoint (https)</span>
        <input
          className="mt-1 w-full min-h-touch px-4 rounded-md border border-border font-mono text-code"
          value={form.x402_endpoint ?? ""}
          onChange={(e) =>
            setForm((prev) => ({
              ...prev,
              x402_endpoint: e.target.value.trim() || null,
            }))
          }
          placeholder="https://..."
          data-testid="tool-x402-endpoint"
        />
      </label>

      <div className="flex flex-wrap gap-4">
        {(
          [
            ["payment_verified", "Payment verified"],
            ["x402_endpoint_verified", "x402 endpoint verified"],
            ["price_verified", "Price verified"],
          ] as const
        ).map(([key, label]) => (
          <label key={key} className="flex items-center gap-2 min-h-touch">
            <input
              type="checkbox"
              checked={form[key]}
              onChange={(e) => setForm((prev) => ({ ...prev, [key]: e.target.checked }))}
              data-testid={`tool-${key.replace(/_/g, "-")}`}
            />
            <span className="text-body-sm">{label}</span>
          </label>
        ))}
      </div>

      <button
        type="button"
        className="min-h-touch px-6 rounded-md bg-tertiary text-on-tertiary font-medium"
        disabled={saveMut.isPending}
        onClick={() => saveMut.mutate()}
        data-testid="tool-referral-save"
      >
        Save referral settings
      </button>
      {saveMut.isSuccess && <p className="text-success text-body-sm">Saved.</p>}
      {error && (
        <p className="text-body-sm text-error" role="alert">
          {error}
        </p>
      )}
    </fieldset>
  );
}