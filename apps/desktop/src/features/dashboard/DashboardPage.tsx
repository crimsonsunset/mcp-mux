import { ConnectionCard } from '@/components/ConnectionCard';
import { DashboardQuickLinks } from './DashboardQuickLinks';
import { DashboardRecentActivity } from './DashboardRecentActivity';
import { DashboardServerHealth } from './DashboardServerHealth';
import { DashboardStatCards } from './DashboardStatCards';
import { useDashboardData } from './useDashboardData';

/**
 * Home dashboard — gateway connection, stat cards, server health, and quick navigation.
 */
export function DashboardPage() {
  const { stats, attentionServers, isLoading } = useDashboardData();

  return (
    <div className="space-y-6" data-testid="dashboard-page">
      <div>
        <h1 className="text-2xl font-bold" data-testid="dashboard-title">
          Dashboard
        </h1>
        <p className="text-[rgb(var(--muted))]" data-testid="dashboard-welcome">
          Welcome to McpMux. Here's an overview of your setup.
        </p>
      </div>

      <ConnectionCard />

      <DashboardStatCards stats={stats} />

      <div className="grid grid-cols-1 gap-4 lg:grid-cols-3">
        <div className="space-y-4 lg:col-span-2">
          <DashboardServerHealth attentionServers={attentionServers} isLoading={isLoading} />
          <DashboardRecentActivity />
        </div>
        <DashboardQuickLinks />
      </div>
    </div>
  );
}
