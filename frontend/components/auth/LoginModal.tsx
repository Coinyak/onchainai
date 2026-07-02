"use client";

import { useEffect } from "react";
import { X } from "lucide-react";
import { LoginForm } from "@/components/auth/LoginForm";

interface LoginModalProps {
  open: boolean;
  onClose: () => void;
}

export function LoginModal({ open, onClose }: LoginModalProps) {
  useEffect(() => {
    if (open) {
      document.body.style.overflow = "hidden";
      return () => {
        document.body.style.overflow = "";
      };
    }
  }, [open]);

  if (!open) return null;

  return (
    <div className="login-modal-overlay" role="dialog" aria-modal="true" aria-labelledby="login-modal-title">
      <button type="button" className="login-modal-backdrop" aria-label="Close" onClick={onClose} />
      <div className="login-modal-panel">
        <button type="button" className="login-modal-close" onClick={onClose} aria-label="Close">
          <X size={20} />
        </button>
        <LoginForm compact headingId="login-modal-title" onCancel={onClose} />
      </div>
    </div>
  );
}