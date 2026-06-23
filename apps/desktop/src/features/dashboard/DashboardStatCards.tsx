import type { KeyboardEvent } from 'react';
import {
  Globe,
  Monitor,
  FolderOpen,
  Server,
  Wrench,
} from 'lucide-react';
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
function activateStatCard(
  event: KeyboardEvent<HTMLDivElement>,
  navigate: () => void
) {
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
        aria-label="Go to My Servers"
        onClick={() => navigateTo('servers')}
        onKeyDown={(event) => activateStatCard(event, () => navigateTo('servers'))}
      >
        <CardHeader className="mb-2">
          <CardTitle className="flex items-center gap-2 text-base">
            <Server className="h-5 w-5 text-primary-500" />
            Servers
          </CardTitle>
          <CardDescription>MCP backends installed in this Space</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="text-3xl font-bold" data-testid="stat-servers-value">
            {stats.connectedServers}/{stats.installedServers}
          </div>
          <div className="text-sm text-[rgb(var(--muted))]">Connected / Installed</div>
        </CardContent>
      </Card>

      <Card
        className={STAT_CARD_CLASS}
        data-testid="stat-featuresets"
        role="button"
        tabIndex={0}
        aria-label="Go to Bundles"
        onClick={() => navigateTo('featuresets')}
        onKeyDown={(event) => activateStatCard(event, () => navigateTo('featuresets'))}
      >
        <CardHeader className="mb-2">
          <CardTitle className="flex items-center gap-2 text-base">
            <Wrench className="h-5 w-5 text-primary-500" />
            Bundles
          </CardTitle>
          <CardDescription>Permission bundles scoped to this Space</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="text-3xl font-bold" data-testid="stat-featuresets-value">
            {stats.featureSets}
          </div>
          <div className="text-sm text-[rgb(var(--muted))]">Active bundles</div>
        </CardContent>
      </Card>

      <Card
        className={STAT_CARD_CLASS}
        data-testid="stat-clients"
        role="button"
        tabIndex={0}
        aria-label="Go to Clients"
        onClick={() => navigateTo('clients')}
        onKeyDown={(event) => activateStatCard(event, () => navigateTo('clients'))}
      >
        <CardHeader className="mb-2">
          <CardTitle className="flex items-center gap-2 text-base">
            <Monitor className="h-5 w-5 text-primary-500" />
            Clients
          </CardTitle>
          <CardDescription>AI apps authorized to use your gateway</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="text-3xl font-bold" data-testid="stat-clients-value">
            {stats.clients}
          </div>
          <div className="text-sm text-[rgb(var(--muted))]">Registered clients</div>
        </CardContent>
      </Card>

      <Card
        className={STAT_CARD_CLASS}
        data-testid="stat-active-space"
        role="button"
        tabIndex={0}
        aria-label="Go to Spaces"
        onClick={() => navigateTo('spaces')}
        onKeyDown={(event) => activateStatCard(event, () => navigateTo('spaces'))}
      >
        <CardHeader className="mb-2">
          <CardTitle className="flex items-center gap-2 text-base">
            <Globe className="h-5 w-5 text-primary-500" />
            Space
          </CardTitle>
          <CardDescription>Isolation boundary for servers and permissions</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="truncate text-xl font-bold" data-testid="stat-active-space-value">
            {viewSpace?.icon} {viewSpace?.name || 'None'}
          </div>
          <div className="text-sm text-[rgb(var(--muted))]">
            {stats.spaces} Space{stats.spaces === 1 ? '' : 's'} total
          </div>
        </CardContent>
      </Card>

      <Card
        className={STAT_CARD_CLASS}
        data-testid="stat-workspaces"
        role="button"
        tabIndex={0}
        aria-label="Go to Projects"
        onClick={() => navigateTo('workspaces')}
        onKeyDown={(event) => activateStatCard(event, () => navigateTo('workspaces'))}
      >
        <CardHeader className="mb-2">
          <CardTitle className="flex items-center gap-2 text-base">
            <FolderOpen className="h-5 w-5 text-primary-500" />
            Projects
          </CardTitle>
          <CardDescription>Folder paths mapped to Bundles</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="text-3xl font-bold" data-testid="stat-workspaces-value">
            {stats.workspaceBindings}
          </div>
          <div className="text-sm text-[rgb(var(--muted))]">Path bindings</div>
        </CardContent>
      </Card>
    </div>
  );
}
