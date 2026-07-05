"use client";

import { useState } from "react";
import Image from "next/image";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { useAuth } from "@/lib/auth";
import { LoginModal } from "@/components/auth/LoginModal";
import { GitHubMarkIcon } from "@/components/icons/GitHubMarkIcon";
import { monogramFromName } from "@/lib/format";
import { signOut } from "@/lib/sign-out";

const GITHUB_REPO =
  process.env.NEXT_PUBLIC_GITHUB_REPO || "https://github.com/Coinyak/onchainai";

function ProfileMenu() {
  const { user } = useAuth();
  const [open, setOpen] = useState(false);
  const [signingOut, setSigningOut] = useState(false);
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
            {user.is_admin && (
              <Link
                href="/dashboard"
                role="menuitem"
                className="site-profile-dropdown-item"
                data-testid="profile-menu-dashboard"
              >
                Dashboard
              </Link>
            )}
            <Link
              href="/toolkit"
              role="menuitem"
              className="site-profile-dropdown-item"
              data-testid="profile-menu-toolkit"
            >
              My Toolkit
            </Link>
            <Link
              href="/connect#agent-sync"
              role="menuitem"
              className="site-profile-dropdown-item"
              data-testid="profile-menu-link-agent"
            >
              Link your agent
            </Link>
            <Link
              href="/blueprints"
              role="menuitem"
              className="site-profile-dropdown-item"
              data-testid="profile-menu-blueprints"
            >
              Blueprints
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
            <button
              type="button"
              role="menuitem"
              className="site-profile-dropdown-item site-profile-dropdown-item-signout w-full text-left"
              data-testid="profile-menu-sign-out"
              disabled={signingOut}
              onClick={() => {
                setSigningOut(true);
                signOut();
              }}
            >
              {signingOut ? "Signing out..." : "Sign out"}
            </button>
          </div>
        </>
      )}
    </div>
  );
}

export function TopNav() {
  const pathname = usePathname();
  const { isAuthenticated, isLoading } = useAuth();
  const [showLogin, setShowLogin] = useState(false);
  const hideSignInOnLoginPage = pathname === "/login";

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
              aria-label="OnchainAI on GitHub"
              title="OnchainAI on GitHub"
            >
              <GitHubMarkIcon />
            </a>
            {!isLoading && isAuthenticated ? (
              <div className="site-top-nav-auth" data-testid="auth-signed-in">
                <ProfileMenu />
              </div>
            ) : hideSignInOnLoginPage ? null : (
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