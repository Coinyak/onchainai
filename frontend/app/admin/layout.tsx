import { cookies } from "next/headers";
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
    redirect("/");
  }

  return <AdminLayoutClient>{children}</AdminLayoutClient>;
}