"use client";

import { createContext, useContext, type ReactNode } from "react";
import { useQuery } from "@tanstack/react-query";
import { checkAdminAccess } from "@/lib/api";

interface AdminContextValue {
  isAdmin: boolean;
  isLoading: boolean;
}

const AdminContext = createContext<AdminContextValue>({ isAdmin: false, isLoading: true });

export function AdminProvider({ children }: { children: ReactNode }) {
  const { data: isAdmin = false, isLoading } = useQuery({
    queryKey: ["admin-check"],
    queryFn: checkAdminAccess,
    retry: false,
  });

  return (
    <AdminContext.Provider value={{ isAdmin, isLoading }}>
      {children}
    </AdminContext.Provider>
  );
}

export function useAdmin(): AdminContextValue {
  return useContext(AdminContext);
}