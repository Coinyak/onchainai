interface ClaimStep {
  label: string;
  done: boolean;
}

interface ClaimStatusTimelineProps {
  claimState: string;
}

export function defaultClaimSteps(claimState: string): ClaimStep[] {
  const states = ["unclaimed", "claim_pending", "claimed", "revoked"];
  const idx = states.indexOf(claimState);
  return [
    { label: "Unclaimed", done: idx >= 0 },
    { label: "Pending review", done: idx >= 1 },
    { label: "Claimed", done: idx >= 2 },
    { label: "Revoked", done: claimState === "revoked" },
  ];
}

export function ClaimStatusTimeline({ claimState }: ClaimStatusTimelineProps) {
  const steps = defaultClaimSteps(claimState);
  return (
    <ol className="claim-timeline">
      {steps.map((step) => (
        <li key={step.label} className={step.done ? "claim-step done" : "claim-step"}>
          {step.label}
        </li>
      ))}
    </ol>
  );
}