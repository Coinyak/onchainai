"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { Menu } from "lucide-react";
import type { CategoryWithCount } from "@/lib/api";
import { SidebarBrand } from "@/components/layout/SidebarBrand";
import { CategoryList } from "@/components/layout/CategoryList";
import { FilterChip } from "@/components/layout/FilterChip";
import {
  type BrowserBase,
  toggleMulti,
  parseMulti,
  clearAxis,
  browserBasePath,
} from "@/lib/browser-query";

const ASSET_CLASSES = [
  { id: "crypto", label: "Crypto" },
  { id: "stablecoins", label: "Stablecoins" },
  { id: "derivatives", label: "Derivatives" },
  { id: "rwa", label: "RWA" },
];

const ACTORS = [
  { id: "human", label: "Human" },
  { id: "ai-agent", label: "AI Agent" },
];

const TYPES = [
  { id: "mcp", label: "MCP" },
  { id: "cli", label: "CLI" },
  { id: "sdk", label: "SDK" },
  { id: "api", label: "API" },
  { id: "x402", label: "x402" },
  { id: "skill", label: "Skill" },
];

const STATUSES = [
  { id: "community", label: "Community" },
  { id: "verified", label: "Verified" },
  { id: "official", label: "Official" },
];

const PRICING = [
  { id: "free", label: "Free" },
  { id: "x402", label: "x402" },
  { id: "paid", label: "Paid" },
  { id: "freemium", label: "Freemium" },
];

const INSTALL_RISK = [
  { id: "low", label: "Low" },
  { id: "medium", label: "Medium" },
  { id: "high", label: "High" },
];

interface SidebarSectionProps {
  id: string;
  title: string;
  open: boolean;
  collapsed: boolean;
  onToggle: () => void;
  children: React.ReactNode;
}

function SidebarSection({ title, open, collapsed, onToggle, children }: SidebarSectionProps) {
  return (
    <section className="sidebar-section">
      <button
        type="button"
        className="sidebar-title sidebar-toggle min-h-touch"
        aria-expanded={open}
        onClick={onToggle}
      >
        <span className="sidebar-title-text">{title}</span>
        <span className="sidebar-chevron" aria-hidden="true">
          {open ? "▾" : "▸"}
        </span>
      </button>
      <div className={collapsed || !open ? "sidebar-panel collapsed" : "sidebar-panel open"}>
        {children}
      </div>
    </section>
  );
}

interface SidebarProps {
  base: BrowserBase;
  categories: CategoryWithCount[];
  queryBase: string;
  activeFunction?: string;
  activeAssetClass?: string;
  activeActor?: string;
  activeType?: string;
  activeStatus?: string;
  activePricing?: string;
  activeInstallRisk?: string;
  defaultFunctionOpen?: boolean;
}

export function Sidebar({
  base,
  categories,
  queryBase,
  activeFunction,
  activeAssetClass,
  activeActor,
  activeType,
  activeStatus,
  activePricing,
  activeInstallRisk,
  defaultFunctionOpen = false,
}: SidebarProps) {
  const basePath = browserBasePath(base);
  const [collapsed, setCollapsed] = useState(true);
  const [loaded, setLoaded] = useState(false);
  const [sections, setSections] = useState<Record<string, boolean>>({
    function: defaultFunctionOpen,
    asset_class: false,
    actor: false,
    type: false,
    status: false,
    pricing: false,
    install_risk: false,
  });

  useEffect(() => {
    const storedCollapsed = localStorage.getItem("sidebar-collapsed");
    const storedSections = localStorage.getItem("sidebar-sections");
    const isNarrow = window.innerWidth < 1024;
    const id = window.setTimeout(() => {
      setCollapsed(storedCollapsed !== null ? storedCollapsed === "true" : isNarrow);
      if (storedSections) {
        try {
          setSections((s) => ({ ...s, ...JSON.parse(storedSections) }));
        } catch {
          /* ignore */
        }
      }
      setLoaded(true);
    }, 0);
    return () => window.clearTimeout(id);
  }, []);

  function persistCollapsed(next: boolean) {
    setCollapsed(next);
    localStorage.setItem("sidebar-collapsed", String(next));
  }

  function toggleSection(id: string) {
    setSections((prev) => {
      const next = { ...prev, [id]: !prev[id] };
      localStorage.setItem("sidebar-sections", JSON.stringify(next));
      return next;
    });
  }

  function collapseMobile() {
    if (window.innerWidth < 1024) persistCollapsed(true);
  }

  const clearHref = typeof base === "object" ? "/tools" : basePath;

  function renderOptions(
    key: string,
    options: { id: string; label: string }[],
    active?: string,
  ) {
    const activeList = parseMulti(active);
    return (
      <ul className="sidebar-list">
        {options.map((opt) => (
          <FilterChip
            key={opt.id}
            href={toggleMulti(basePath, queryBase, key, opt.id, activeList)}
            label={opt.label}
            active={activeList.includes(opt.id)}
            onNavigate={collapseMobile}
          />
        ))}
      </ul>
    );
  }

  const [isNarrow, setIsNarrow] = useState(false);
  useEffect(() => {
    const check = () => setIsNarrow(window.innerWidth < 1024);
    check();
    window.addEventListener("resize", check);
    return () => window.removeEventListener("resize", check);
  }, []);
  const showBackdrop = loaded && !collapsed && isNarrow;

  return (
    <div className="tools-sidebar-shell">
      {showBackdrop && (
        <button
          type="button"
          className="sidebar-mobile-backdrop"
          aria-label="Close filters"
          onClick={() => persistCollapsed(true)}
        />
      )}
      <aside
        className={collapsed ? "tools-sidebar tools-sidebar-collapsed" : "tools-sidebar"}
        data-sidebar-ready=""
        data-sidebar-storage-loaded={loaded ? "" : undefined}
        aria-busy={!loaded}
      >
        <SidebarBrand />
        <div className="sidebar-controls">
          <button
            type="button"
            className="sidebar-rail-toggle min-h-touch min-w-touch"
            aria-label="Toggle filters sidebar"
            aria-expanded={!collapsed}
            onClick={() => {
              const next = !collapsed;
              persistCollapsed(next);
              if (!next) setSections((s) => ({ ...s, function: true }));
            }}
          >
            <Menu className="sidebar-rail-toggle-icon" size={20} aria-hidden />
          </button>
          <Link
            href={clearAxis(clearHref, queryBase, "function")}
            className="sidebar-clear sidebar-title-text"
            onClick={collapseMobile}
          >
            Clear
          </Link>
        </div>

        <div className="sidebar-filters">
          <SidebarSection
            id="function"
            title="Function"
            open={sections.function}
            collapsed={collapsed}
            onToggle={() => toggleSection("function")}
          >
            <CategoryList
              base={base}
              categories={categories}
              queryBase={queryBase}
              activeFunction={activeFunction}
              onNavigate={collapseMobile}
            />
          </SidebarSection>

          <SidebarSection
            id="asset_class"
            title="Asset Class"
            open={sections.asset_class}
            collapsed={collapsed}
            onToggle={() => toggleSection("asset_class")}
          >
            {renderOptions("asset_class", ASSET_CLASSES, activeAssetClass)}
          </SidebarSection>

          <SidebarSection
            id="actor"
            title="Actor"
            open={sections.actor}
            collapsed={collapsed}
            onToggle={() => toggleSection("actor")}
          >
            {renderOptions("actor", ACTORS, activeActor)}
          </SidebarSection>

          <SidebarSection
            id="type"
            title="Type"
            open={sections.type}
            collapsed={collapsed}
            onToggle={() => toggleSection("type")}
          >
            {renderOptions("type", TYPES, activeType)}
          </SidebarSection>

          <SidebarSection
            id="status"
            title="Status"
            open={sections.status}
            collapsed={collapsed}
            onToggle={() => toggleSection("status")}
          >
            {renderOptions("status", STATUSES, activeStatus)}
          </SidebarSection>

          <SidebarSection
            id="pricing"
            title="Pricing"
            open={sections.pricing}
            collapsed={collapsed}
            onToggle={() => toggleSection("pricing")}
          >
            {renderOptions("pricing", PRICING, activePricing)}
          </SidebarSection>

          <SidebarSection
            id="install_risk"
            title="Install Risk"
            open={sections.install_risk}
            collapsed={collapsed}
            onToggle={() => toggleSection("install_risk")}
          >
            {renderOptions("install_risk", INSTALL_RISK, activeInstallRisk)}
          </SidebarSection>
        </div>
      </aside>
    </div>
  );
}