"use client";

import { useEffect, useRef, useState } from "react";
import Link from "next/link";
import {
  BadgeCheck,
  Coins,
  Layers,
  Plug,
  Shield,
  Tag,
  Users,
  type LucideIcon,
} from "lucide-react";
import type { CategoryWithCount } from "@/lib/api";
import { CategoryList } from "@/components/layout/CategoryList";
import { FilterChip } from "@/components/layout/FilterChip";
import {
  type BrowserBase,
  toggleMulti,
  parseMulti,
  clearAxis,
  browserBasePath,
  sidebarDefaultCollapsedForViewport,
  shouldCollapseMobileSidebarOnRouteChange,
} from "@/lib/browser-query";

const SIDEBAR_COLLAPSED_KEY = "onchain-ai-sidebar-collapsed";
const SIDEBAR_SECTIONS_KEY = "onchain-ai-sidebar-sections";

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

const RAIL_ICONS: { id: string; label: string; Icon: LucideIcon }[] = [
  { id: "function", label: "Function", Icon: Layers },
  { id: "asset_class", label: "Asset Class", Icon: Coins },
  { id: "actor", label: "Actor", Icon: Users },
  { id: "type", label: "Type", Icon: Plug },
  { id: "status", label: "Status", Icon: BadgeCheck },
  { id: "pricing", label: "Pricing", Icon: Tag },
  { id: "install_risk", label: "Install Risk", Icon: Shield },
];

interface SidebarSectionProps {
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

function readCollapsed(defaultValue: boolean): boolean {
  const raw = localStorage.getItem(SIDEBAR_COLLAPSED_KEY);
  if (raw === "1" || raw === "true") return true;
  if (raw === "0" || raw === "false") return false;
  return defaultValue;
}

function writeCollapsed(value: boolean) {
  localStorage.setItem(SIDEBAR_COLLAPSED_KEY, value ? "1" : "0");
}

interface SidebarProps {
  base: BrowserBase;
  categories: CategoryWithCount[];
  queryBase: string;
  filterRevision?: string;
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
  filterRevision = "",
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
  const revisionRef = useRef("");
  const backdropRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    const isNarrow = sidebarDefaultCollapsedForViewport();
    const storedSections = localStorage.getItem(SIDEBAR_SECTIONS_KEY);
    const id = window.setTimeout(() => {
      setCollapsed(readCollapsed(isNarrow));
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

  useEffect(() => {
    if (!loaded) return;
    const mobile = sidebarDefaultCollapsedForViewport();
    if (mobile && !collapsed) {
      document.body.classList.add("sidebar-scroll-locked");
    } else {
      document.body.classList.remove("sidebar-scroll-locked");
    }
    return () => document.body.classList.remove("sidebar-scroll-locked");
  }, [collapsed, loaded]);

  useEffect(() => {
    if (!loaded || !filterRevision) return;
    const prev = revisionRef.current;
    if (!prev) {
      revisionRef.current = filterRevision;
      return;
    }
    if (
      shouldCollapseMobileSidebarOnRouteChange(
        prev,
        filterRevision,
        sidebarDefaultCollapsedForViewport(),
      )
    ) {
      persistCollapsed(true);
    }
    revisionRef.current = filterRevision;
  }, [filterRevision, loaded]);

  useEffect(() => {
    if (!loaded || collapsed || !sidebarDefaultCollapsedForViewport()) return;
    backdropRef.current?.focus();
  }, [collapsed, loaded]);

  function persistCollapsed(next: boolean) {
    setCollapsed(next);
    writeCollapsed(next);
  }

  function toggleSection(id: string) {
    setSections((prev) => {
      const next = { ...prev, [id]: !prev[id] };
      localStorage.setItem(SIDEBAR_SECTIONS_KEY, JSON.stringify(next));
      return next;
    });
  }

  function railSectionActive(sectionId: string): boolean {
    const has = (value?: string) => parseMulti(value).length > 0;
    switch (sectionId) {
      case "function":
        return has(activeFunction);
      case "asset_class":
        return has(activeAssetClass);
      case "actor":
        return has(activeActor);
      case "type":
        return has(activeType);
      case "status":
        return has(activeStatus);
      case "pricing":
        return has(activePricing);
      case "install_risk":
        return has(activeInstallRisk);
      default:
        return false;
    }
  }

  function openRailSection(sectionId: string) {
    persistCollapsed(false);
    setSections((prev) => {
      const next = { ...prev, [sectionId]: true };
      if (loaded) {
        localStorage.setItem(SIDEBAR_SECTIONS_KEY, JSON.stringify(next));
      }
      return next;
    });
  }

  function collapseMobile() {
    if (sidebarDefaultCollapsedForViewport()) persistCollapsed(true);
  }

  function handleEscape() {
    if (!collapsed) persistCollapsed(true);
  }

  const clearHref = typeof base === "object" ? "/tools" : basePath;
  const showBackdrop = loaded && !collapsed && sidebarDefaultCollapsedForViewport();

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

  return (
    <div className="tools-sidebar-shell">
      {showBackdrop && (
        <button
          ref={backdropRef}
          type="button"
          className="sidebar-mobile-backdrop"
          aria-label="Close filters"
          tabIndex={-1}
          onClick={() => persistCollapsed(true)}
          onKeyDown={(ev) => {
            if (ev.key === "Escape") {
              ev.stopPropagation();
              handleEscape();
            }
          }}
        />
      )}
      <aside
        className={collapsed ? "tools-sidebar tools-sidebar-collapsed" : "tools-sidebar"}
        data-sidebar-ready=""
        data-filter-revision={filterRevision}
        data-sidebar-storage-loaded={loaded ? "" : undefined}
        data-testid="tools-sidebar"
        aria-busy={!loaded}
        onKeyDown={(ev) => {
          if (ev.key === "Escape" && !collapsed) {
            ev.stopPropagation();
            handleEscape();
          }
        }}
      >
        <div className="sidebar-controls">
          <button
            type="button"
            className="sidebar-rail-toggle min-h-touch min-w-touch"
            aria-label="Toggle filters sidebar"
            aria-expanded={!collapsed}
            onClick={() => {
              const wasCollapsed = collapsed;
              const next = !collapsed;
              persistCollapsed(next);
              if (wasCollapsed) {
                setSections((s) => ({ ...s, function: true }));
              }
            }}
          >
            <svg
              className="sidebar-rail-toggle-icon"
              width="20"
              height="20"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              aria-hidden="true"
            >
              <line x1="4" y1="7" x2="20" y2="7" />
              <line x1="4" y1="12" x2="20" y2="12" />
              <line x1="4" y1="17" x2="20" y2="17" />
            </svg>
          </button>
          <Link
            href={clearAxis(clearHref, queryBase, "function")}
            scroll={false}
            className="sidebar-clear sidebar-title-text"
            onClick={collapseMobile}
          >
            Clear
          </Link>
        </div>

        <div className="sidebar-rail-icons">
          {RAIL_ICONS.map(({ id, label, Icon }, index) => (
            <button
              key={id}
              type="button"
              className={
                railSectionActive(id)
                  ? "sidebar-rail-icon sidebar-rail-icon-active"
                  : "sidebar-rail-icon"
              }
              title={label}
              aria-label={label}
              data-rail-section={id}
              data-rail-divider={index > 0 ? "true" : undefined}
              onClick={() => openRailSection(id)}
            >
              <Icon size={20} strokeWidth={2} aria-hidden="true" />
            </button>
          ))}
        </div>

        <div className="sidebar-body">
          <SidebarSection
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
            title="Asset Class"
            open={sections.asset_class}
            collapsed={collapsed}
            onToggle={() => toggleSection("asset_class")}
          >
            {renderOptions("asset_class", ASSET_CLASSES, activeAssetClass)}
          </SidebarSection>

          <SidebarSection
            title="Actor"
            open={sections.actor}
            collapsed={collapsed}
            onToggle={() => toggleSection("actor")}
          >
            {renderOptions("actor", ACTORS, activeActor)}
          </SidebarSection>

          <SidebarSection
            title="Type"
            open={sections.type}
            collapsed={collapsed}
            onToggle={() => toggleSection("type")}
          >
            {renderOptions("type", TYPES, activeType)}
          </SidebarSection>

          <SidebarSection
            title="Status"
            open={sections.status}
            collapsed={collapsed}
            onToggle={() => toggleSection("status")}
          >
            {renderOptions("status", STATUSES, activeStatus)}
          </SidebarSection>

          <SidebarSection
            title="Pricing"
            open={sections.pricing}
            collapsed={collapsed}
            onToggle={() => toggleSection("pricing")}
          >
            {renderOptions("pricing", PRICING, activePricing)}
          </SidebarSection>

          <SidebarSection
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