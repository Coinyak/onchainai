"use client";

import { useState } from "react";
import Link from "next/link";
import Image from "next/image";
import { useAuth } from "@/lib/auth";
import { LoginModal } from "@/components/auth/LoginModal";
import { monogramFromName } from "@/lib/format";

const GITHUB_REPO = "https://github.com/hoyeon4315-cpu/onchainai";

function ProfileMenu() {
  const { user } = useAuth();
  const [open, setOpen] = useState(false);
  if (!user) return null;

  const nickname = user.nickname || "User";
  const monogram = monogramFromName(nickname);

  return (
    <div className="site-profile-menu" data-testid="profile-menu">
      <button
        type="button"
        className="site-profile-btn"
        data-testid="profile-menu-btn"
        aria-label={`Account menu for ${nickname}`}
        aria-haspopup="menu"
        aria-expanded={open}
        onClick={() => setOpen((v) => !v)}
      >
        {user.avatar_url ? (
          <img
            className="site-profile-avatar"
            src={user.avatar_url}
            alt=""
            width={32}
            height={32}
          />
        ) : (
          <span className="site-profile-monogram" aria-hidden="true">
            {monogram}
          </span>
        )}
      </button>
      {open && (
        <>
          <div
            className="site-profile-backdrop"
            aria-hidden="true"
            onClick={() => setOpen(false)}
          />
          <div
            className="site-profile-dropdown"
            role="menu"
            data-testid="profile-menu-dropdown"
            onClick={(e) => e.stopPropagation()}
          >
            <Link
              href="/dashboard"
              role="menuitem"
              className="site-profile-dropdown-item"
              data-testid="profile-menu-dashboard"
            >
              Dashboard
            </Link>
            <Link
              href="/toolkit"
              role="menuitem"
              className="site-profile-dropdown-item"
              data-testid="profile-menu-toolkit"
            >
              My Toolkit
            </Link>
            {user.is_admin && (
              <Link
                href="/admin"
                role="menuitem"
                className="site-profile-dropdown-item site-profile-dropdown-item-admin"
                data-testid="profile-menu-admin"
              >
                Admin
              </Link>
            )}
            <form action={`${process.env.NEXT_PUBLIC_API_URL || "http://localhost:3000"}/auth/logout`} method="post" className="site-profile-dropdown-signout">
              <button
                type="submit"
                role="menuitem"
                className="site-profile-dropdown-item site-profile-dropdown-item-signout"
                data-testid="profile-menu-sign-out"
              >
                Sign out
              </button>
            </form>
          </div>
        </>
      )}
    </div>
  );
}

export function TopNav() {
  const { isAuthenticated, isLoading } = useAuth();
  const [showLogin, setShowLogin] = useState(false);

  return (
    <>
      <LoginModal open={showLogin} onClose={() => setShowLogin(false)} />
      <header className="site-top-nav">
        <div className="site-top-nav-inner">
          <Link href="/" className="site-top-nav-logo" aria-label="OnchainAI home">
            <Image
              className="site-top-nav-mark"
              src="/brand/onchainai-logo.png"
              alt=""
              width={34}
              height={34}
            />
            <span>OnchainAI</span>
          </Link>
          <nav className="site-top-nav-actions" aria-label="Site actions">
            <Link href="/submit" className="site-top-nav-btn site-top-nav-btn-primary">
              Submit
            </Link>
            <a
              href={GITHUB_REPO}
              target="_blank"
              rel="noopener noreferrer"
              className="site-top-nav-repo"
            >
              GitHub
            </a>
            {!isLoading && isAuthenticated ? (
              <div className="site-top-nav-auth" data-testid="auth-signed-in">
                <ProfileMenu />
              </div>
            ) : (
              <div className="site-top-nav-auth" data-testid="auth-sign-in">
                <button
                  type="button"
                  className="site-top-nav-btn site-top-nav-btn-outline"
                  data-testid="top-nav-sign-in"
                  onClick={() => setShowLogin(true)}
                >
                  Sign in
                </button>
              </div>
            )}
          </nav>
        </div>
      </header>
    </>
  );
}