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
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold">Dashboard</h1>
        <p className="text-[rgb(var(--muted))]">
          Welcome to McpMux — your centralized MCP server manager.
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
