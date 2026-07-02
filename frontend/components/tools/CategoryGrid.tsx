import Link from "next/link";
import type { CategoryWithCount } from "@/lib/api";

interface CategoryGridProps {
  categories: CategoryWithCount[];
}

export function CategoryGrid({ categories }: CategoryGridProps) {
  if (!categories.length) return null;
  return (
    <section className="mb-6">
      <h2 className="text-h2 mb-4">Browse by function</h2>
      <div className="category-grid">
        {categories.map(({ category, count }) => (
          <Link
            key={category.id}
            href={`/categories/${category.id}`}
            className="category-grid-card no-underline"
          >
            <span className="category-grid-label">{category.label}</span>
            <span className="category-grid-count">{count} tools</span>
          </Link>
        ))}
      </div>
    </section>
  );
}