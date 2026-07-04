import { cookies, headers } from "next/headers";
import { redirect } from "next/navigation";
import { checkAdminAccessServer } from "@/lib/server-api";
import AdminLayoutClient from "./AdminLayoutClient";

export const dynamic = "force-dynamic";

export default async function AdminLayout({ children }: { children: React.ReactNode }) {
  const cookieStore = await cookies();
  const cookieHeader = cookieStore
    .getAll()
    .map((cookie) => `${cookie.name}=${cookie.value}`)
    .join("; ");

  const isAdmin = await checkAdminAccessServer(cookieHeader);
  if (!isAdmin) {
    const headerStore = await headers();
    const returnPath = headerStore.get("x-pathname") ?? "/admin";
    redirect(
      `/login?return_to=${encodeURIComponent(returnPath)}&auth=admin_required`,
    );
  }

  return <AdminLayoutClient>{children}</AdminLayoutClient>;
}