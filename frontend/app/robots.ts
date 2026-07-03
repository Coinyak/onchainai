import type { MetadataRoute } from "next";
import { SITE_ORIGIN } from "@/lib/site";

export default function robots(): MetadataRoute.Robots {
  return {
    rules: {
      userAgent: "*",
      allow: "/",
      disallow: ["/admin/", "/dashboard/", "/toolkit/", "/blueprints/"],
    },
    sitemap: `${SITE_ORIGIN}/sitemap.xml`,
  };
}