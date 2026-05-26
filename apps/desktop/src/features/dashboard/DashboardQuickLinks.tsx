import type { ReactNode } from 'react';
import {
  FolderOpen,
  Globe,
  Monitor,
  Server,
  Settings,
  Wrench,
} from 'lucide-react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@mcpmux/ui';
import { useNavigateTo } from '@/stores';
import type { NavItem } from '@/stores/types';

type QuickLink = {
  nav: NavItem;
  label: string;
  description: string;
  icon: ReactNode;
  testId: string;
};

const QUICK_LINKS: QuickLink[] = [
  {
    nav: 'servers',
    label: 'My Servers',
    description: 'Manage installed backends',
    icon: <Server className="h-4 w-4" />,
    testId: 'quick-link-servers',
  },
  {
    nav: 'registry',
    label: 'Discover',
    description: 'Browse the server registry',
    icon: <Server className="h-4 w-4" />,
    testId: 'quick-link-discover',
  },
  {
    nav: 'spaces',
    label: 'Spaces',
    description: 'Organize servers and permissions',
    icon: <Globe className="h-4 w-4" />,
    testId: 'quick-link-spaces',
  },
  {
    nav: 'featuresets',
    label: 'FeatureSets',
    description: 'Edit permission bundles',
    icon: <Wrench className="h-4 w-4" />,
    testId: 'quick-link-featuresets',
  },
  {
    nav: 'workspaces',
    label: 'Workspaces',
    description: 'Bind folders to FeatureSets',
    icon: <FolderOpen className="h-4 w-4" />,
    testId: 'quick-link-workspaces',
  },
  {
    nav: 'clients',
    label: 'Clients',
    description: 'Manage connected AI apps',
    icon: <Monitor className="h-4 w-4" />,
    testId: 'quick-link-clients',
  },
  {
    nav: 'settings',
    label: 'Settings',
    description: 'Gateway, updates, and preferences',
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
        <CardTitle className="text-base">Quick links</CardTitle>
        <CardDescription>Jump to any section from the sidebar</CardDescription>
      </CardHeader>
      <CardContent>
        <div className="grid grid-cols-1 gap-2">
          {QUICK_LINKS.map((link) => (
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
