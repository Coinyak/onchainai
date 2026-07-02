"use client";

import { useEffect } from "react";
import { useRouter } from "next/navigation";
import { AdminProvider, useAdmin } from "@/components/admin/AdminContext";
import { AdminShell } from "@/components/admin/AdminShell";

function AdminGuard({ children }: { children: React.ReactNode }) {
  const { isAdmin, isLoading } = useAdmin();
  const router = useRouter();

  useEffect(() => {
    if (!isLoading && !isAdmin) {
      router.replace("/");
    }
  }, [isAdmin, isLoading, router]);

  if (isLoading) {
    return <div className="p-8 text-secondary">Checking admin access...</div>;
  }
  if (!isAdmin) return null;

  return <AdminShell>{children}</AdminShell>;
}

export default function AdminLayout({ children }: { children: React.ReactNode }) {
  return (
    <AdminProvider>
      <AdminGuard>{children}</AdminGuard>
    </AdminProvider>
  );
}