/**
 * Navigation model — the single source of truth for the sidebar.
 *
 * The app's IA follows the superapp plan (mcpmux.space/superapp/03-experience-design.md):
 * a "use" zone on top (today just Dashboard; Chat and Agents land here later), a
 * "Library" zone for capabilities (My Servers, Built-in, Search; Models lands here
 * later), and a "Control" zone for routing & access (Clients, Projects, Bundles,
 * Spaces). Settings is pinned to the sidebar footer.
 *
 * To add a future surface, append an entry to the right zone — the sidebar
 * renders from this data and nothing else.
 *
 * NOTE: `key` values are NavItem store keys and `testId`s are the e2e selector
 * contract (ADR-003) — both are stable identifiers. Only `labelKey`/`hintKey`/
 * `icon` are presentation and safe to change.
 */
import type { ComponentType } from 'react';
import type { LucideIcon } from 'lucide-react';
import {
  LayoutDashboard,
  Sparkles,
  Search,
  Monitor,
  FolderOpen,
  ShoppingBasket,
  Globe,
  Settings,
} from 'lucide-react';
import { McpNavIcon } from '@/components/McpNavIcon';
import type { NavItem } from '@/stores/types';
import type nav from '@/locales/en/nav.json';

/** Top-level nav label keys (excludes nested `zones` / `hints` objects). */
type NavLabelKey = Exclude<keyof typeof nav, 'zones' | 'hints'>;

/** Sidebar hint keys under `nav.hints`. */
type NavHintKey = `hints.${keyof typeof nav.hints}`;

/** Zone section title keys under `nav.zones`. */
type NavZoneTitleKey = `zones.${keyof typeof nav.zones}`;

export type NavIcon = LucideIcon | ComponentType<{ className?: string }>;

export interface NavEntry {
  key: NavItem;
  /** i18n key under the `nav` namespace (e.g. `dashboard` → nav:dashboard). */
  labelKey: NavLabelKey;
  icon: NavIcon;
  testId: string;
  /** i18n key under nav:hints.* */
  hintKey: NavHintKey;
  /** Optional native tooltip when the visible label is an alias (e.g. Bundles → FeatureSets). */
  labelTitleKey?: NavHintKey;
}

export interface NavZone {
  /** Zone label i18n key under nav:zones.*; omitted for the top-level "use" zone. */
  titleKey?: NavZoneTitleKey;
  entries: NavEntry[];
}

export const NAV_ZONES: NavZone[] = [
  {
    entries: [
      {
        key: 'dashboard',
        labelKey: 'dashboard',
        icon: LayoutDashboard,
        testId: 'nav-dashboard',
        hintKey: 'hints.dashboard',
      },
    ],
  },
  {
    titleKey: 'zones.library',
    entries: [
      {
        key: 'servers',
        labelKey: 'myServers',
        icon: McpNavIcon,
        testId: 'nav-my-servers',
        hintKey: 'hints.myServers',
      },
      {
        key: 'builtin-servers',
        labelKey: 'builtin',
        icon: Sparkles,
        testId: 'nav-builtin-servers',
        hintKey: 'hints.builtin',
      },
      {
        key: 'registry',
        labelKey: 'search',
        icon: Search,
        testId: 'nav-discover',
        hintKey: 'hints.search',
      },
    ],
  },
  {
    titleKey: 'zones.control',
    entries: [
      {
        key: 'clients',
        labelKey: 'clients',
        icon: Monitor,
        testId: 'nav-clients',
        hintKey: 'hints.clients',
      },
      {
        key: 'workspaces',
        labelKey: 'projects',
        icon: FolderOpen,
        testId: 'nav-workspaces',
        hintKey: 'hints.projects',
      },
      {
        key: 'featuresets',
        labelKey: 'bundles',
        icon: ShoppingBasket,
        testId: 'nav-featuresets',
        hintKey: 'hints.bundles',
        labelTitleKey: 'hints.bundlesTooltip',
      },
      {
        key: 'spaces',
        labelKey: 'spaces',
        icon: Globe,
        testId: 'nav-spaces',
        hintKey: 'hints.spaces',
      },
    ],
  },
];

/** Pinned to the sidebar footer, below the scrolling zones. */
export const NAV_SETTINGS: NavEntry = {
  key: 'settings',
  labelKey: 'settings',
  icon: Settings,
  testId: 'nav-settings',
  hintKey: 'hints.settings',
};
