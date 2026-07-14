import type { MetadataRoute } from "next";
import {
  getCategoriesServer,
  listAllToolSlugsServer,
} from "@/lib/server-api";
import { SITE_ORIGIN } from "@/lib/site";

// Cache the generated sitemap for an hour. Without this the route renders per
// request — every crawler hit becomes a serverless invocation that scans all
// tool slugs + categories from Railway. Crawlers do not need sub-hour freshness.
export const revalidate = 3600;

const STATIC_ROUTES: Array<{
  path: string;
  changeFrequency: MetadataRoute.Sitemap[number]["changeFrequency"];
  priority: number;
}> = [
  { path: "", changeFrequency: "daily", priority: 1 },
  { path: "/tools", changeFrequency: "daily", priority: 0.9 },
  { path: "/compare", changeFrequency: "weekly", priority: 0.7 },
  { path: "/about", changeFrequency: "monthly", priority: 0.5 },
  { path: "/submit", changeFrequency: "monthly", priority: 0.6 },
  { path: "/login", changeFrequency: "yearly", priority: 0.3 },
  { path: "/connect", changeFrequency: "monthly", priority: 0.7 },
];

export default async function sitemap(): Promise<MetadataRoute.Sitemap> {
  const now = new Date();

  const staticEntries: MetadataRoute.Sitemap = STATIC_ROUTES.map((route) => ({
    url: `${SITE_ORIGIN}${route.path}`,
    lastModified: now,
    changeFrequency: route.changeFrequency,
    priority: route.priority,
  }));

  let toolEntries: MetadataRoute.Sitemap = [];
  let categoryEntries: MetadataRoute.Sitemap = [];

  try {
    const [tools, categories] = await Promise.all([
      listAllToolSlugsServer(),
      getCategoriesServer(),
    ]);

    toolEntries = tools.map((tool) => ({
      url: `${SITE_ORIGIN}/tools/${tool.slug}`,
      lastModified: new Date(tool.updated_at),
      changeFrequency: "weekly" as const,
      priority: 0.8,
    }));

    categoryEntries = categories.map(({ category }) => ({
      url: `${SITE_ORIGIN}/categories/${category.id}`,
      lastModified: now,
      changeFrequency: "weekly" as const,
      priority: 0.7,
    }));
  } catch {
    // Sitemap should still publish static routes if the API is temporarily unavailable.
  }

  return [...staticEntries, ...categoryEntries, ...toolEntries];
}