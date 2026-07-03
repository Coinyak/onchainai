"use client";

import { useState } from "react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { useAuth } from "@/lib/auth";
import { LoginModal } from "@/components/auth/LoginModal";
import { monogramFromName } from "@/lib/format";

const GITHUB_REPO =
  process.env.NEXT_PUBLIC_GITHUB_REPO || "https://github.com/hoyeon4315-cpu/onchainai";

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
            <form action={`${process.env.NEXT_PUBLIC_API_URL || ""}/auth/logout`} method="post" className="site-profile-dropdown-signout">
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
  const pathname = usePathname();
  const { isAuthenticated, isLoading } = useAuth();
  const [showLogin, setShowLogin] = useState(false);
  const hideSignInOnLoginPage = pathname === "/login";

  return (
    <>
      <LoginModal open={showLogin} onClose={() => setShowLogin(false)} />
      <header className="site-top-nav">
        <div className="site-top-nav-inner site-top-nav-inner-actions">
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
              <svg
                viewBox="0 0 16 16"
                width={20}
                height={20}
                fill="currentColor"
                aria-hidden="true"
              >
                <path d="M8 0c4.42 0 8 3.58 8 8a8.013 8.013 0 0 1-5.45 7.59c-.4.08-.55-.17-.55-.38 0-.27.01-1.13.01-2.2 0-.75-.25-1.23-.54-1.48 1.78-.2 3.65-.88 3.65-3.95 0-.88-.31-1.59-.82-2.15.08-.2.36-1.02-.08-2.12 0 0-.67-.22-2.2.82-.64-.18-1.32-.27-2-.27-.68 0-1.36.09-2 .27-1.53-1.04-2.2-.82-2.2-.82-.44 1.1-.16 1.92-.08 2.12-.51.56-.82 1.27-.82 2.15 0 3.06 1.86 3.75 3.64 3.95-.23.2-.44.55-.51 1.07-.46.21-1.61.55-2.33-.66-.15-.24-.6-.83-1.23-.82-.67.01-.27.38.01.53.34.19.73.9.82 1.13.16.45.68 1.31 2.69.94 0 .67.01 1.3.01 1.49 0 .21-.15.45-.55.38A7.995 7.995 0 0 1 0 8c0-4.42 3.58-8 8-8Z" />
              </svg>
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