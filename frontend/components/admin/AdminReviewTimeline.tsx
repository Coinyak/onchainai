import { timeAgo } from "@/lib/format";

export interface ReviewEntry {
  id: string;
  action: string;
  reason?: string | null;
  created_at: string;
  operator_nickname?: string | null;
}

interface AdminReviewTimelineProps {
  entries: ReviewEntry[];
}

export function AdminReviewTimeline({ entries }: AdminReviewTimelineProps) {
  if (!entries.length) {
    return <p className="text-body-sm text-secondary">No review history yet.</p>;
  }

  return (
    <ol className="admin-review-timeline">
      {entries.map((entry) => (
        <li key={entry.id} className="admin-review-timeline-item">
          <span className="admin-review-action">{entry.action}</span>
          <span className="admin-review-time">{timeAgo(entry.created_at)}</span>
          {entry.operator_nickname && (
            <span className="admin-review-operator">by {entry.operator_nickname}</span>
          )}
          {entry.reason && <p className="admin-review-reason">{entry.reason}</p>}
        </li>
      ))}
    </ol>
  );
}