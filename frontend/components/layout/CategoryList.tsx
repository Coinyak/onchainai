import type { CategoryWithCount } from "@/lib/api";
import { FilterChip } from "@/components/layout/FilterChip";
import {
  type BrowserBase,
  toggleMulti,
  parseMulti,
  clearAxis,
  browserBasePath,
  categoryHref,
} from "@/lib/browser-query";

interface CategoryListProps {
  base: BrowserBase;
  categories: CategoryWithCount[];
  queryBase: string;
  activeFunction?: string;
  onNavigate?: () => void;
}

function functionLink(
  base: BrowserBase,
  queryBase: string,
  catId: string,
  active: string[],
): { href: string; active: boolean } {
  if (typeof base === "object") {
    const href = categoryHref(catId, queryBase);
    return { href, active: base.category === catId };
  }
  const basePath = browserBasePath(base);
  const href = toggleMulti(basePath, queryBase, "function", catId, active);
  return { href, active: active.includes(catId) };
}

export function CategoryList({
  base,
  categories,
  queryBase,
  activeFunction,
  onNavigate,
}: CategoryListProps) {
  const fnActive = parseMulti(activeFunction);
  const basePath = browserBasePath(base);
  const clearHref =
    typeof base === "object" ? "/tools" : clearAxis(basePath, queryBase, "function");

  return (
    <ul className="sidebar-list">
      <FilterChip
        href={clearHref}
        label="All"
        active={fnActive.length === 0 && typeof base !== "object"}
        onNavigate={onNavigate}
      />
      {categories.map(({ category, count }) => {
        const { href, active } = functionLink(base, queryBase, category.id, fnActive);
        return (
          <FilterChip
            key={category.id}
            href={href}
            label={category.label}
            active={active}
            count={count}
            onNavigate={onNavigate}
          />
        );
      })}
    </ul>
  );
}