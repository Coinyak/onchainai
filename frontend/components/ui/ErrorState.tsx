"use client";

interface ErrorStateProps {
  message: string;
  onRetry?: () => void;
}

export function ErrorState({ message, onRetry }: ErrorStateProps) {
  return (
    <div className="empty-state-panel">
      <p className="empty-state-message">Failed to load data.</p>
      <p className="empty-state-hint">{message}</p>
      {onRetry && (
        <div className="empty-state-actions">
          <button type="button" className="empty-state-clear-btn" onClick={onRetry}>
            Retry
          </button>
        </div>
      )}
    </div>
  );
}