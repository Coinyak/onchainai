"use client";

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import {
  listAdminUsers,
  setUserBanned,
  setUserAdmin,
  deleteUser,
} from "@/lib/api";

export default function AdminUsersPage() {
  const queryClient = useQueryClient();
  const usersQuery = useQuery({
    queryKey: ["admin-users"],
    queryFn: listAdminUsers,
  });

  const refresh = () => queryClient.invalidateQueries({ queryKey: ["admin-users"] });

  const banMut = useMutation({
    mutationFn: ({ id, banned }: { id: string; banned: boolean }) => setUserBanned(id, banned),
    onSuccess: refresh,
  });

  const adminMut = useMutation({
    mutationFn: ({ id, isAdmin }: { id: string; isAdmin: boolean }) => setUserAdmin(id, isAdmin),
    onSuccess: refresh,
  });

  const deleteMut = useMutation({
    mutationFn: deleteUser,
    onSuccess: refresh,
  });

  return (
    <div className="px-gutter md:px-6 py-8 max-w-[900px] mx-auto">
      <h1 className="text-h2 mb-6">User management</h1>
      <div className="divide-y divide-border border border-border rounded-md">
        {usersQuery.data?.map((user) => (
          <div key={user.id} className="p-4 flex flex-wrap items-center justify-between gap-3">
            <div>
              <span className="font-medium">{user.nickname || "—"}</span>
              {user.is_admin && <span className="ml-2 badge badge-verified">Admin</span>}
              {user.is_banned && <span className="ml-2 badge badge-neutral">Banned</span>}
              <p className="text-body-sm text-secondary">
                {user.comment_count} comments · {user.bookmark_count} bookmarks
              </p>
            </div>
            <div className="flex flex-wrap gap-2">
              <button
                type="button"
                className="min-h-touch px-3 rounded-md border border-border"
                onClick={() => banMut.mutate({ id: user.id, banned: !user.is_banned })}
              >
                {user.is_banned ? "Unban" : "Ban"}
              </button>
              <button
                type="button"
                className="min-h-touch px-3 rounded-md border border-border"
                onClick={() => adminMut.mutate({ id: user.id, isAdmin: !user.is_admin })}
              >
                {user.is_admin ? "Remove admin" : "Make admin"}
              </button>
              <button
                type="button"
                className="min-h-touch px-3 rounded-md border border-error text-error"
                onClick={() => deleteMut.mutate(user.id)}
              >
                Delete
              </button>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}