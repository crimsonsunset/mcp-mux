import { useCallback, useEffect, useState } from 'react';
import { listClients } from '@/lib/api/clients';
import { listFeatureSets, listFeatureSetsBySpace } from '@/lib/api/featureSets';
import { getGatewayStatus } from '@/lib/api/gateway';
import { listInstalledServers } from '@/lib/api/registry';
import { getServerStatuses } from '@/lib/api/serverManager';
import { listSpaces } from '@/lib/api/spaces';
import { listWorkspaceBindings } from '@/lib/api/workspaceBindings';
import { useGatewayEvents, useServerStatusEvents } from '@/hooks/useDomainEvents';
import { useSpaces, useViewSpace } from '@/stores';
import {
  buildAttentionServers,
  type AttentionServer,
  type DashboardStats,
} from './dashboard.helpers';

const EMPTY_STATS: DashboardStats = {
  installedServers: 0,
  connectedServers: 0,
  featureSets: 0,
  clients: 0,
  workspaceBindings: 0,
  spaces: 0,
};

/**
 * Loads dashboard stats and server-health rows, refreshing on Space or gateway changes.
 */
export function useDashboardData() {
  const viewSpace = useViewSpace();
  const spaces = useSpaces();
  const [stats, setStats] = useState<DashboardStats>(EMPTY_STATS);
  const [attentionServers, setAttentionServers] = useState<AttentionServer[]>([]);
  const [isLoading, setIsLoading] = useState(true);

  const reload = useCallback(async () => {
    const spaceId = viewSpace?.id;

    try {
      const [clients, featureSets, gateway, installedServers, workspaceBindings] =
        await Promise.all([
          listClients(),
          spaceId ? listFeatureSetsBySpace(spaceId) : listFeatureSets(),
          getGatewayStatus(spaceId),
          listInstalledServers(spaceId),
          listWorkspaceBindings(),
        ]);

      let nextAttention: AttentionServer[] = [];
      if (spaceId) {
        const statuses = await getServerStatuses(spaceId);
        nextAttention = buildAttentionServers(installedServers, statuses);
      }

      setStats({
        installedServers: installedServers.length,
        connectedServers: gateway.connected_backends,
        featureSets: featureSets.length,
        clients: clients.length,
        workspaceBindings: workspaceBindings.length,
        spaces: spaces.length,
      });
      setAttentionServers(nextAttention);
    } catch (error) {
      console.error('Failed to load dashboard data:', error);
    } finally {
      setIsLoading(false);
    }
  }, [spaces.length, viewSpace?.id]);

  useEffect(() => {
    setIsLoading(true);
    reload();
  }, [reload]);

  useEffect(() => {
    listSpaces()
      .then((nextSpaces) => {
        setStats((prev) => ({ ...prev, spaces: nextSpaces.length }));
      })
      .catch(() => {
        /* keep store-backed count */
      });
  }, []);

  useGatewayEvents((payload) => {
    if (payload.action === 'started') {
      reload();
      return;
    }

    if (payload.action === 'stopped') {
      setStats((prev) => ({ ...prev, connectedServers: 0 }));
    }
  });

  useServerStatusEvents((payload) => {
    if (payload.status === 'connected' || payload.status === 'disconnected') {
      reload();
    }
  });

  return { stats, attentionServers, isLoading, reload };
}
