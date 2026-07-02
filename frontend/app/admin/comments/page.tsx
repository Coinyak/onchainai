"use client";

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { listAdminComments, deleteAdminComment, deleteCommentAndBanUser } from "@/lib/api";
import { timeAgo } from "@/lib/format";

export default function AdminCommentsPage() {
  const queryClient = useQueryClient();
  const commentsQuery = useQuery({
    queryKey: ["admin-comments"],
    queryFn: listAdminComments,
  });

  const deleteMut = useMutation({
    mutationFn: deleteAdminComment,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["admin-comments"] }),
  });

  const banMut = useMutation({
    mutationFn: deleteCommentAndBanUser,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["admin-comments"] }),
  });

  return (
    <div className="px-gutter md:px-6 py-8 max-w-[900px] mx-auto">
      <h1 className="text-h2 mb-6">Comment management</h1>
      {commentsQuery.isLoading && <p className="text-secondary">Loading...</p>}
      <div className="divide-y divide-border border border-border rounded-md">
        {commentsQuery.data?.map((comment) => (
          <div key={comment.id} className="p-4">
            <p className="text-body-sm text-secondary">
              {comment.author_nickname} on {comment.tool_name} · {timeAgo(comment.created_at)}
            </p>
            <p className="text-body-md mt-2">{comment.content}</p>
            <div className="mt-3 flex gap-2">
              <button
                type="button"
                className="min-h-touch px-3 rounded-md border border-border"
                onClick={() => deleteMut.mutate(comment.id)}
              >
                Delete
              </button>
              <button
                type="button"
                className="min-h-touch px-3 rounded-md border border-error text-error"
                onClick={() => banMut.mutate(comment.id)}
              >
                Delete + Ban
              </button>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}