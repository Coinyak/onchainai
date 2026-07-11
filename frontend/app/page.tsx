import {
  dehydrate,
  HydrationBoundary,
  QueryClient,
} from "@tanstack/react-query";
import { HomePageClient } from "@/app/HomePageClient";
import { buildToolFilters } from "@/lib/browser-query";
import {
  getCategoriesServer,
  getFeaturedCardsServer,
  getSiteSettingsServer,
  loadBrowserDataServer,
} from "@/lib/server-api";
/** ISR: bots and repeat visitors hit cached HTML instead of cold CSR + API fan-out. */
export const revalidate = 120;

/** Default home list query key — must match ToolsBrowser for empty URL state. */
function defaultHomeBrowserQueryKey() {
  const filters = buildToolFilters({ sort: "hot", page: 1 });
  return ["browser-data", "home", "hot", filters, undefined, 1] as const;
}

export default async function HomePage() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        staleTime: 120 * 1000,
      },
    },
  });

  const filters = buildToolFilters({ sort: "hot", page: 1 });
  const browserKey = defaultHomeBrowserQueryKey();

  const [browserData, featured, settings, categories] = await Promise.all([
    loadBrowserDataServer({
      sort: "hot",
      filters,
      search_q: null,
      selected: null,
      page: 1,
    }),
    getFeaturedCardsServer(),
    getSiteSettingsServer(),
    getCategoriesServer(),
  ]);

  // Seed only successful payloads so client does not remount-fetch defaults.
  if (browserData) {
    queryClient.setQueryData(browserKey, browserData);
  }
  if (featured.length > 0) {
    queryClient.setQueryData(["featured"], featured);
  }
  if (settings) {
    queryClient.setQueryData(["site-settings"], settings);
  }
  if (categories.length > 0) {
    queryClient.setQueryData(["catalog-categories"], categories);
  }

  return (
    <HydrationBoundary state={dehydrate(queryClient)}>
      <HomePageClient />
    </HydrationBoundary>
  );
}
