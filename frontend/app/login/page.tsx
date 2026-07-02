import { SiteShell } from "@/components/layout/SiteShell";
import { LoginForm } from "@/components/auth/LoginForm";

export default function LoginPage() {
  return (
    <SiteShell>
      <div className="px-gutter md:px-8 py-12 max-w-[480px] mx-auto">
        <LoginForm />
      </div>
    </SiteShell>
  );
}