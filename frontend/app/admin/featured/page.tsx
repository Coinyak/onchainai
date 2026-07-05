"use client";

import { Suspense, useEffect, useRef, useState } from "react";
import { useSearchParams, useRouter } from "next/navigation";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import Link from "next/link";
import {
  createFeaturedCard,
  deleteFeaturedCard,
  listFeaturedCardsAdmin,
  searchToolsForPicker,
  updateFeaturedCard,
  uploadFeaturedImage,
  type AdminFeaturedCard,
  type FeaturedCardInput,
  type ToolPickerItem,
} from "@/lib/api";
import { isRenderableFeaturedImageUrl } from "@/lib/featured";

interface FeaturedCardFormProps {
  mode: "create" | "edit";
  initial?: AdminFeaturedCard;
  prefillToolSlug?: string | null;
  onSaved: () => void;
  onCancel: () => void;
}

function FeaturedCardForm({
  mode,
  initial,
  prefillToolSlug,
  onSaved,
  onCancel,
}: FeaturedCardFormProps) {
  const [toolQuery, setToolQuery] = useState(initial?.tool_name ?? "");
  const [selectedTool, setSelectedTool] = useState<ToolPickerItem | null>(
    initial
      ? { id: initial.tool_id, name: initial.tool_name, slug: initial.tool_slug }
      : null,
  );
  const [pickerResults, setPickerResults] = useState<ToolPickerItem[]>([]);
  const [pickerLoading, setPickerLoading] = useState(false);
  const [imageUrl, setImageUrl] = useState(initial?.image_url ?? "");
  const [headline, setHeadline] = useState(initial?.headline ?? "");
  const [subtitle, setSubtitle] = useState(initial?.subtitle ?? "");
  const [sortOrder, setSortOrder] = useState(String(initial?.sort_order ?? 0));
  const [isActive, setIsActive] = useState(initial?.is_active ?? true);
  const [error, setError] = useState<string | null>(null);
  const [uploading, setUploading] = useState(false);
  const prefillDone = useRef(false);

  useEffect(() => {
    if (!prefillToolSlug || prefillDone.current || initial) return;
    prefillDone.current = true;
    let cancelled = false;
    (async () => {
      setPickerLoading(true);
      try {
        const results = await searchToolsForPicker(prefillToolSlug, 10);
        if (cancelled) return;
        const exact = results.find((item) => item.slug === prefillToolSlug);
        if (exact) {
          setSelectedTool(exact);
          setToolQuery(exact.name);
          setPickerResults(results);
        } else {
          setToolQuery(prefillToolSlug);
          setPickerResults([]);
          setError(
            `No approved tool matched "${prefillToolSlug}". Search for the tool manually.`,
          );
        }
      } catch (e) {
        if (!cancelled) setError(e instanceof Error ? e.message : "Tool lookup failed");
      } finally {
        if (!cancelled) setPickerLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [prefillToolSlug, initial]);

  useEffect(() => {
    const query = toolQuery.trim();
    if (!query || (selectedTool && query === selectedTool.name)) {
      return;
    }
    let cancelled = false;
    const timer = setTimeout(async () => {
      setPickerLoading(true);
      try {
        const results = await searchToolsForPicker(query, 20);
        if (!cancelled) setPickerResults(results);
      } catch {
        if (!cancelled) setPickerResults([]);
      } finally {
        if (!cancelled) setPickerLoading(false);
      }
    }, 250);
    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [toolQuery, selectedTool]);

  const imageUrlInvalid =
    imageUrl.trim().length > 0 && !isRenderableFeaturedImageUrl(imageUrl);

  const saveMut = useMutation({
    mutationFn: async () => {
      if (!selectedTool) throw new Error("Select an approved tool");
      if (!isRenderableFeaturedImageUrl(imageUrl)) {
        throw new Error("Image URL must start with http:// or https://");
      }
      const payload: FeaturedCardInput = {
        tool_id: selectedTool.id,
        image_url: imageUrl.trim(),
        headline: headline.trim() || null,
        subtitle: subtitle.trim() || null,
        sort_order: Number.parseInt(sortOrder, 10) || 0,
        is_active: isActive,
      };
      if (mode === "edit" && initial) {
        return updateFeaturedCard(initial.id, payload);
      }
      return createFeaturedCard(payload);
    },
    onSuccess: () => {
      setError(null);
      onSaved();
    },
    onError: (e: Error) => setError(e.message),
  });

  const deleteMut = useMutation({
    mutationFn: () => {
      if (!initial) throw new Error("No card to delete");
      return deleteFeaturedCard(initial.id);
    },
    onSuccess: () => {
      setError(null);
      onSaved();
    },
    onError: (e: Error) => setError(e.message),
  });

  const handleUpload = async (file: File | null) => {
    if (!file) return;
    setUploading(true);
    setError(null);
    try {
      const result = await uploadFeaturedImage(file);
      setImageUrl(result.url);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Upload failed");
    } finally {
      setUploading(false);
    }
  };

  return (
    <form
      className="border border-border rounded-md p-lg space-y-4"
      onSubmit={(e) => {
        e.preventDefault();
        saveMut.mutate();
      }}
    >
      <h2 className="text-h3">{mode === "create" ? "New featured card" : "Edit featured card"}</h2>

      <label className="block">
        <span className="text-body-sm text-secondary">Tool</span>
        <input
          className="mt-1 w-full min-h-touch px-4 rounded-md border border-border"
          value={toolQuery}
          onChange={(e) => {
            setToolQuery(e.target.value);
            setSelectedTool(null);
            setPickerResults([]);
          }}
          placeholder="Search approved tools..."
        />
        {pickerLoading && <p className="mt-1 text-body-sm text-secondary">Searching...</p>}
        {pickerResults.length > 0 && !selectedTool && (
          <ul className="mt-2 border border-border rounded-md divide-y divide-border max-h-48 overflow-y-auto">
            {pickerResults.map((item) => (
              <li key={item.id}>
                <button
                  type="button"
                  className="w-full text-left px-4 py-3 hover:bg-neutral-hover min-h-touch"
                  onClick={() => {
                    setSelectedTool(item);
                    setToolQuery(item.name);
                    setPickerResults([]);
                  }}
                >
                  <span className="font-medium">{item.name}</span>
                  <span className="text-body-sm text-secondary ml-2">{item.slug}</span>
                </button>
              </li>
            ))}
          </ul>
        )}
        {selectedTool && (
          <p className="mt-1 text-body-sm text-secondary">
            Selected: {selectedTool.name} ({selectedTool.slug})
          </p>
        )}
      </label>

      <label className="block">
        <span className="text-body-sm text-secondary">Image URL</span>
        <input
          className="mt-1 w-full min-h-touch px-4 rounded-md border border-border font-mono text-code"
          type="url"
          value={imageUrl}
          onChange={(e) => setImageUrl(e.target.value)}
          placeholder="https://..."
          required
          aria-invalid={imageUrlInvalid}
        />
        {imageUrlInvalid && (
          <p className="mt-1 text-body-sm text-error" role="alert">
            Use an http:// or https:// URL so the card appears on the home carousel.
          </p>
        )}
      </label>

      <label className="block">
        <span className="text-body-sm text-secondary">Upload image</span>
        <input
          className="mt-1 block w-full text-body-sm"
          type="file"
          accept="image/jpeg,image/png,image/webp,image/svg+xml"
          disabled={uploading}
          onChange={(e) => void handleUpload(e.target.files?.[0] ?? null)}
        />
        {uploading && <p className="mt-1 text-body-sm text-secondary">Uploading...</p>}
      </label>

      {imageUrl.trim() && (
        <img
          src={imageUrl}
          alt=""
          className="w-full max-w-sm aspect-video object-contain rounded-md border border-border bg-neutral-bg"
        />
      )}

      <label className="block">
        <span className="text-body-sm text-secondary">Headline (optional)</span>
        <input
          className="mt-1 w-full min-h-touch px-4 rounded-md border border-border"
          value={headline}
          onChange={(e) => setHeadline(e.target.value)}
          maxLength={120}
          placeholder="Falls back to tool name when empty"
        />
      </label>

      <label className="block">
        <span className="text-body-sm text-secondary">Subtitle (optional)</span>
        <textarea
          className="mt-1 w-full min-h-[72px] p-4 rounded-md border border-border"
          value={subtitle}
          onChange={(e) => setSubtitle(e.target.value)}
          maxLength={200}
        />
      </label>

      <label className="block">
        <span className="text-body-sm text-secondary">Sort order</span>
        <input
          className="mt-1 w-32 min-h-touch px-4 rounded-md border border-border"
          type="number"
          value={sortOrder}
          onChange={(e) => setSortOrder(e.target.value)}
        />
      </label>

      <label className="flex items-center gap-2 min-h-touch">
        <input
          type="checkbox"
          checked={isActive}
          onChange={(e) => setIsActive(e.target.checked)}
        />
        <span className="text-body-sm">Active on home carousel</span>
      </label>

      {error && <p className="text-body-sm text-error">{error}</p>}

      <div className="flex flex-wrap gap-3">
        <button
          type="submit"
          className="min-h-touch px-6 rounded-md bg-tertiary text-on-tertiary font-medium hover:bg-[#D96400] disabled:opacity-60"
          disabled={saveMut.isPending || uploading || imageUrlInvalid}
        >
          {saveMut.isPending ? "Saving..." : mode === "create" ? "Create card" : "Save changes"}
        </button>
        <button
          type="button"
          className="min-h-touch px-4 rounded-md border border-border-strong bg-neutral-bg hover:bg-neutral-hover"
          onClick={onCancel}
        >
          Cancel
        </button>
        {mode === "edit" && (
          <button
            type="button"
            className="min-h-touch px-4 rounded-md border border-border-strong text-error hover:bg-neutral-hover disabled:opacity-60"
            disabled={deleteMut.isPending}
            onClick={() => {
              if (!window.confirm("Delete this featured card?")) return;
              deleteMut.mutate();
            }}
          >
            {deleteMut.isPending ? "Deleting..." : "Delete"}
          </button>
        )}
      </div>
    </form>
  );
}

function AdminFeaturedContent() {
  const searchParams = useSearchParams();
  const router = useRouter();
  const queryClient = useQueryClient();
  const editId = searchParams.get("edit");
  const isNew = searchParams.get("new") === "1";
  const prefillToolSlug = searchParams.get("tool");
  const editCardRef = useRef<HTMLDivElement>(null);

  const featuredQuery = useQuery({
    queryKey: ["admin-featured"],
    queryFn: listFeaturedCardsAdmin,
  });

  const editingCard = editId
    ? featuredQuery.data?.find((card) => card.id === editId)
    : undefined;

  useEffect(() => {
    if (!editId || !editingCard || featuredQuery.isLoading) return;
    editCardRef.current?.scrollIntoView({ behavior: "smooth", block: "start" });
  }, [editId, editingCard, featuredQuery.isLoading]);

  const clearParams = () => router.push("/admin/featured");

  const handleSaved = () => {
    void queryClient.invalidateQueries({ queryKey: ["admin-featured"] });
    clearParams();
  };

  return (
    <div className="px-gutter md:px-6 py-8 max-w-[900px] mx-auto">
      <div className="flex flex-wrap items-center justify-between gap-4 mb-6">
        <div>
          <h1 className="text-h2">Featured carousel</h1>
          <p className="text-secondary text-body-md mt-2">
            Manage hero carousel cards shown on the home page when active.
          </p>
        </div>
        {!isNew && !editId && (
          <Link
            href="/admin/featured?new=1"
            className="inline-flex items-center min-h-touch px-6 rounded-md bg-tertiary text-on-tertiary font-medium no-underline hover:bg-[#D96400]"
          >
            Add card
          </Link>
        )}
      </div>

      {isNew && (
        <div className="mb-8">
          <FeaturedCardForm
            mode="create"
            prefillToolSlug={prefillToolSlug}
            onSaved={handleSaved}
            onCancel={clearParams}
          />
        </div>
      )}

      {editId && !featuredQuery.isLoading && !editingCard && (
        <p className="text-body-sm text-secondary mb-6">
          Card not found. Showing the current list.
        </p>
      )}

      <div className="space-y-4">
        {featuredQuery.data?.map((card) => (
          <article
            key={card.id}
            ref={editId === card.id ? editCardRef : undefined}
            className="border border-border rounded-md p-lg"
          >
            {editId === card.id ? (
              <FeaturedCardForm
                mode="edit"
                initial={card}
                onSaved={handleSaved}
                onCancel={clearParams}
              />
            ) : (
              <div className="flex gap-4">
                <img src={card.image_url} alt="" className="w-24 h-16 object-cover rounded-sm" />
                <div className="flex-1 min-w-0">
                  <div className="flex flex-wrap items-center gap-2">
                    <h3 className="text-h3">{card.headline || card.tool_name}</h3>
                    <span
                      className={
                        card.is_active
                          ? "text-body-sm text-tertiary"
                          : "text-body-sm text-secondary"
                      }
                    >
                      {card.is_active ? "Active" : "Inactive"}
                    </span>
                  </div>
                  <p className="text-body-sm text-secondary">{card.subtitle}</p>
                  <p className="text-body-sm text-secondary">
                    Order {card.sort_order} · {card.tool_slug}
                  </p>
                  <div className="flex flex-wrap gap-3 mt-2">
                    <Link
                      href={`/admin/featured?edit=${card.id}`}
                      className="inline-flex items-center min-h-touch px-3 text-tertiary text-body-sm no-underline hover:underline"
                    >
                      Edit
                    </Link>
                    <Link
                      href={`/tools/${card.tool_slug}`}
                      className="inline-flex items-center min-h-touch px-3 text-tertiary text-body-sm no-underline hover:underline"
                    >
                      View tool
                    </Link>
                  </div>
                </div>
              </div>
            )}
          </article>
        ))}
        {featuredQuery.data?.length === 0 && !isNew && (
          <p className="text-secondary">No featured cards yet.</p>
        )}
      </div>
    </div>
  );
}

export default function AdminFeaturedPage() {
  return (
    <Suspense fallback={<p className="p-8 text-secondary">Loading...</p>}>
      <AdminFeaturedContent />
    </Suspense>
  );
}