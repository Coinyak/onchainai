interface SiteShellProps {
  children: React.ReactNode;
}

export function SiteShell({ children }: SiteShellProps) {
  return (
    <div className="site-content-shell">
      <main className="site-main site-main-full">{children}</main>
    </div>
  );
}