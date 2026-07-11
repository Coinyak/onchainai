import {
  dehydrate,
  HydrationBoundary,
  QueryClient,
} from "@tanstack/react-query";
import { HomePageClient } from "@/app/HomePageClient";
import { HideOnHydrate } from "@/components/tools/HideOnHydrate";
import { ServerHomeToolList } from "@/components/tools/ServerHomeToolList";
import {
  defaultHomeBrowserQueryKey,
  emptyToolFilters,
} from "@/lib/browser-query";
import {
  getCategoriesServer,
  getFeaturedCardsServer,
  getSiteSettingsServer,
  loadBrowserDataServer,
} from "@/lib/server-api";

/** ISR: bots and repeat visitors hit cached HTML instead of cold CSR + API fan-out. */
export const revalidate = 120;

export default async function HomePage() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        staleTime: 120 * 1000,
      },
    },
  });

  const filters = emptyToolFilters();
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

  // Seed successful payloads (including empty featured) so client skips remount fetch.
  if (browserData) {
    queryClient.setQueryData(browserKey, browserData);
  }
  queryClient.setQueryData(["featured"], featured);
  if (settings) {
    queryClient.setQueryData(["site-settings"], settings);
  }
  if (categories.length > 0) {
    queryClient.setQueryData(["catalog-categories"], categories);
  }

  return (
    <HydrationBoundary state={dehydrate(queryClient)}>
      {/* Crawler-visible tool list; removed on hydrate so interactive list is not doubled. */}
      {browserData ? (
        <HideOnHydrate>
          <ServerHomeToolList data={browserData} />
        </HideOnHydrate>
      ) : null}
      <HomePageClient />
    </HydrationBoundary>
  );
}
