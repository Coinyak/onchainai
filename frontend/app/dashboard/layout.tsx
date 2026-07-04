import { cookies, headers } from "next/headers";
import { redirect } from "next/navigation";
import { checkAdminAccessServer, getSessionUserServer } from "@/lib/server-api";

export const dynamic = "force-dynamic";

/** Catalog dashboard — operators only; regular users use /tools. */
export default async function DashboardLayout({ children }: { children: React.ReactNode }) {
  const cookieStore = await cookies();
  const cookieHeader = cookieStore
    .getAll()
    .map((cookie) => `${cookie.name}=${cookie.value}`)
    .join("; ");

  const isAdmin = await checkAdminAccessServer(cookieHeader);
  if (isAdmin) {
    return children;
  }

  const session = await getSessionUserServer(cookieHeader);
  if (session) {
    redirect("/tools");
  }

  const headerStore = await headers();
  const returnPath = headerStore.get("x-pathname") ?? "/dashboard";
  redirect(`/login?return_to=${encodeURIComponent(returnPath)}&auth=admin_required`);
}