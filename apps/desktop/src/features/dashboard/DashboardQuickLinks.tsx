import type { ReactNode } from 'react';
import {
  FolderOpen,
  Globe,
  Monitor,
  Search,
  Server,
  Settings,
  ShoppingBasket,
} from 'lucide-react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@mcpmux/ui';
import { useNavigateTo } from '@/stores';
import type { NavItem } from '@/stores/types';

type QuickLinkConfig = {
  nav: NavItem;
  label: string;
  description: string;
  icon: ReactNode;
  testId: string;
};

const QUICK_LINK_CONFIG: QuickLinkConfig[] = [
  {
    nav: 'servers',
    label: 'My Servers',
    description: 'Manage your installed MCP servers',
    icon: <Server className="h-4 w-4" />,
    testId: 'quick-link-servers',
  },
  {
    nav: 'registry',
    label: 'Discover',
    description: 'Browse the MCP server registry',
    icon: <Search className="h-4 w-4" />,
    testId: 'quick-link-discover',
  },
  {
    nav: 'spaces',
    label: 'Spaces',
    description: 'Manage your connected AI client spaces',
    icon: <Globe className="h-4 w-4" />,
    testId: 'quick-link-spaces',
  },
  {
    nav: 'featuresets',
    label: 'Bundles',
    description: 'Curated tool sets for specific workflows',
    icon: <ShoppingBasket className="h-4 w-4" />,
    testId: 'quick-link-featuresets',
  },
  {
    nav: 'workspaces',
    label: 'Projects',
    description: 'Bind workspace roots to Spaces',
    icon: <FolderOpen className="h-4 w-4" />,
    testId: 'quick-link-workspaces',
  },
  {
    nav: 'clients',
    label: 'Clients',
    description: 'Manage connected AI clients',
    icon: <Monitor className="h-4 w-4" />,
    testId: 'quick-link-clients',
  },
  {
    nav: 'settings',
    label: 'Settings',
    description: 'Configure McpMux preferences',
    icon: <Settings className="h-4 w-4" />,
    testId: 'quick-link-settings',
  },
];

/**
 * Compact navigation grid covering every sidebar destination except Dashboard.
 */
export function DashboardQuickLinks() {
  const navigateTo = useNavigateTo();

  return (
    <Card data-testid="dashboard-quick-links">
      <CardHeader>
        <CardTitle className="text-base">Quick Links</CardTitle>
        <CardDescription>Jump to any section</CardDescription>
      </CardHeader>
      <CardContent>
        <div className="grid grid-cols-1 gap-2">
          {QUICK_LINK_CONFIG.map((link) => (
            <button
              key={link.nav}
              type="button"
              onClick={() => navigateTo(link.nav)}
              data-testid={link.testId}
              className="flex items-start gap-3 rounded-lg border border-[rgb(var(--border-subtle))] px-3 py-2.5 text-left transition-colors hover:border-[rgb(var(--primary))/30] hover:bg-[rgb(var(--surface-hover))]"
            >
              <span className="mt-0.5 text-[rgb(var(--primary))]">{link.icon}</span>
              <span className="min-w-0">
                <span className="block text-sm font-medium">{link.label}</span>
                <span className="block truncate text-xs text-[rgb(var(--muted))]">
                  {link.description}
                </span>
              </span>
            </button>
          ))}
        </div>
      </CardContent>
    </Card>
  );
}
