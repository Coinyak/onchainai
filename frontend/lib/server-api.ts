import type {
  CategoryRow,
  CategoryWithCount,
  PublicDashboardSnapshot,
  SessionUser,
  Tool,
  ToolFilters,
  ToolListRequest,
} from "@/lib/api";
import { resolveApiOrigin } from "@/lib/api-origin";
import { SEO_REVALIDATE_SECONDS } from "@/lib/site";

const SERVER_FETCH_TIMEOUT_MS = 8_000;

function serverApiBase(): string {
  return resolveApiOrigin();
}

export class ServerApiError extends Error {
  status: number;

  constructor(message: string, status: number) {
    super(message);
    this.name = "ServerApiError";
    this.status = status;
  }
}

export async function serverApiFetch<T>(
  path: string,
  options?: RequestInit & { revalidate?: number; timeoutMs?: number },
): Promise<T> {
  const {
    revalidate = SEO_REVALIDATE_SECONDS,
    timeoutMs = SERVER_FETCH_TIMEOUT_MS,
    ...fetchOptions
  } = options ?? {};
  const base = serverApiBase();
  const url = `${base}${path}`;

  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), timeoutMs);

  try {
    const res = await fetch(url, {
      ...fetchOptions,
      signal: controller.signal,
      next: { revalidate },
      headers: {
        Accept: "application/json",
        ...(fetchOptions.body && !(fetchOptions.body instanceof FormData)
          ? { "Content-Type": "application/json" }
          : {}),
        ...(fetchOptions.headers as Record<string, string> | undefined),
      },
    });

    if (!res.ok) {
      const error = await res.json().catch(() => ({ error: { message: "Request failed" } }));
      throw new ServerApiError(error.error?.message || "Request failed", res.status);
    }

    if (res.status === 204) return undefined as T;
    return res.json() as Promise<T>;
  } catch (error) {
    if (error instanceof ServerApiError) throw error;
    if (error instanceof Error && error.name === "AbortError") {
      throw new ServerApiError("Request timed out", 408);
    }
    throw error;
  } finally {
    clearTimeout(timeout);
  }
}

function normalizeCategory(row: CategoryRow): CategoryWithCount {
  return { category: row[0], count: row[1] };
}

export async function getToolBySlugServer(slug: string): Promise<Tool> {
  return serverApiFetch<Tool>(`/api/v2/tools/${encodeURIComponent(slug)}`);
}

export async function getToolCommentCountServer(slug: string): Promise<number> {
  return serverApiFetch<number>(
    `/api/v2/tools/${encodeURIComponent(slug)}/comment-count`,
  );
}

export async function getCategoriesServer(): Promise<CategoryWithCount[]> {
  try {
    const rows = await serverApiFetch<CategoryRow[]>("/api/v2/categories");
    return rows.map(normalizeCategory);
  } catch {
    return [];
  }
}

export async function listToolsServer(
  req: ToolListRequest,
  revalidate = SEO_REVALIDATE_SECONDS,
): Promise<Tool[]> {
  try {
    return await serverApiFetch<Tool[]>("/api/v2/tools/list", {
      method: "POST",
      body: JSON.stringify(req),
      revalidate,
    });
  } catch {
    return [];
  }
}

export async function listAllToolSlugsServer(): Promise<
  Array<{ slug: string; updated_at: string }>
> {
  try {
    const tools = await listToolsServer({
      sort: "hot",
      offset: 0,
      limit: 500,
      filters: {} satisfies ToolFilters,
    });

    return tools.map((tool) => ({
      slug: tool.slug,
      updated_at: tool.updated_at,
    }));
  } catch {
    return [];
  }
}

export async function getPublicDashboardServer(
  limit = 6,
): Promise<PublicDashboardSnapshot | null> {
  try {
    return await serverApiFetch<PublicDashboardSnapshot>(
      `/api/v2/dashboard?limit=${limit}`,
      { revalidate: 300 },
    );
  } catch {
    return null;
  }
}

export async function getSessionUserServer(cookieHeader: string): Promise<SessionUser | null> {
  if (!cookieHeader) return null;

  try {
    return await serverApiFetch<SessionUser | null>("/api/v2/me", {
      headers: { Cookie: cookieHeader },
      revalidate: 0,
    });
  } catch {
    return null;
  }
}

export async function checkAdminAccessServer(cookieHeader: string): Promise<boolean> {
  if (!cookieHeader) return false;

  try {
    await serverApiFetch<{ ok: boolean }>("/api/v2/admin/check", {
      headers: { Cookie: cookieHeader },
      revalidate: 0,
    });
    return true;
  } catch {
    return false;
  }
}