import type { KeyboardEvent } from 'react';
import { FolderOpen, Globe, Monitor, Server, Wrench } from 'lucide-react';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@mcpmux/ui';
import { useNavigateTo, useViewSpace } from '@/stores';
import type { DashboardStats } from './dashboard.helpers';

interface DashboardStatCardsProps {
  stats: DashboardStats;
}

const STAT_CARD_CLASS =
  'cursor-pointer transition-all hover:shadow-lg hover:scale-[1.01] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary-500/50';

/**
 * Navigate when the user activates a stat card via click or keyboard.
 */
function activateStatCard(event: KeyboardEvent<HTMLDivElement>, navigate: () => void) {
  if (event.key === 'Enter' || event.key === ' ') {
    event.preventDefault();
    navigate();
  }
}

/**
 * Top-row stat cards with descriptions and deep links into sidebar destinations.
 */
export function DashboardStatCards({ stats }: DashboardStatCardsProps) {
  const navigateTo = useNavigateTo();
  const viewSpace = useViewSpace();

  return (
    <div
      className="grid grid-cols-1 gap-4 sm:grid-cols-2 xl:grid-cols-5"
      data-testid="dashboard-stats-grid"
    >
      <Card
        className={STAT_CARD_CLASS}
        data-testid="stat-servers"
        role="button"
        tabIndex={0}
        aria-label="View My Servers"
        onClick={() => navigateTo('servers')}
        onKeyDown={(event) => activateStatCard(event, () => navigateTo('servers'))}
      >
        <CardHeader className="mb-2">
          <CardTitle className="flex items-center gap-2 text-base">
            <Server className="h-5 w-5 text-primary-500" />
            My Servers
          </CardTitle>
          <CardDescription>Installed MCP servers</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="text-3xl font-bold" data-testid="stat-servers-value">
            {stats.connectedServers}/{stats.installedServers}
          </div>
          <div className="text-sm text-[rgb(var(--muted))]">connected / installed</div>
        </CardContent>
      </Card>

      <Card
        className={STAT_CARD_CLASS}
        data-testid="stat-featuresets"
        role="button"
        tabIndex={0}
        aria-label="View Feature Sets"
        onClick={() => navigateTo('featuresets')}
        onKeyDown={(event) => activateStatCard(event, () => navigateTo('featuresets'))}
      >
        <CardHeader className="mb-2">
          <CardTitle className="flex items-center gap-2 text-base">
            <Wrench className="h-5 w-5 text-primary-500" />
            Feature Sets
          </CardTitle>
          <CardDescription>Curated tool bundles</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="text-3xl font-bold" data-testid="stat-featuresets-value">
            {stats.featureSets}
          </div>
          <div className="text-sm text-[rgb(var(--muted))]">bundles</div>
        </CardContent>
      </Card>

      <Card
        className={STAT_CARD_CLASS}
        data-testid="stat-clients"
        role="button"
        tabIndex={0}
        aria-label="View Clients"
        onClick={() => navigateTo('clients')}
        onKeyDown={(event) => activateStatCard(event, () => navigateTo('clients'))}
      >
        <CardHeader className="mb-2">
          <CardTitle className="flex items-center gap-2 text-base">
            <Monitor className="h-5 w-5 text-primary-500" />
            Clients
          </CardTitle>
          <CardDescription>Connected AI clients</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="text-3xl font-bold" data-testid="stat-clients-value">
            {stats.clients}
          </div>
          <div className="text-sm text-[rgb(var(--muted))]">clients</div>
        </CardContent>
      </Card>

      <Card
        className={STAT_CARD_CLASS}
        data-testid="stat-active-space"
        role="button"
        tabIndex={0}
        aria-label="View Spaces"
        onClick={() => navigateTo('spaces')}
        onKeyDown={(event) => activateStatCard(event, () => navigateTo('spaces'))}
      >
        <CardHeader className="mb-2">
          <CardTitle className="flex items-center gap-2 text-base">
            <Globe className="h-5 w-5 text-primary-500" />
            Active Space
          </CardTitle>
          <CardDescription>Currently viewed space</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="truncate text-xl font-bold" data-testid="stat-active-space-value">
            {viewSpace?.icon} {viewSpace?.name ?? 'None'}
          </div>
          <div className="text-sm text-[rgb(var(--muted))]">
            {stats.spaces} space{stats.spaces !== 1 ? 's' : ''} total
          </div>
        </CardContent>
      </Card>

      <Card
        className={STAT_CARD_CLASS}
        data-testid="stat-workspaces"
        role="button"
        tabIndex={0}
        aria-label="View Workspaces"
        onClick={() => navigateTo('workspaces')}
        onKeyDown={(event) => activateStatCard(event, () => navigateTo('workspaces'))}
      >
        <CardHeader className="mb-2">
          <CardTitle className="flex items-center gap-2 text-base">
            <FolderOpen className="h-5 w-5 text-primary-500" />
            Workspaces
          </CardTitle>
          <CardDescription>Bound workspace roots</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="text-3xl font-bold" data-testid="stat-workspaces-value">
            {stats.workspaceBindings}
          </div>
          <div className="text-sm text-[rgb(var(--muted))]">bindings</div>
        </CardContent>
      </Card>
    </div>
  );
}
