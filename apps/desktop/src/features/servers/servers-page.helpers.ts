import type { ServerFeature } from '@/lib/api/serverFeatures';
import type { ServerViewModel } from '../../types/registry';

/** Runtime action used to derive status filter buckets. */
export type ServerActionKey =
  | 'enable'
  | 'configure'
  | 'connecting'
  | 'authenticating'
  | 'auth_required'
  | 'running'
  | 'error'
  | 'connected_auto';

/** Transport filter for installed servers. */
export type TransportFilter = 'all' | 'stdio' | 'http';

/** Status bucket for Beeper-style multi-select filters. */
export type StatusFilterKey = 'connected' | 'disabled' | 'error' | 'needs_setup';

export const TRANSPORT_FILTER_IDS: TransportFilter[] = ['all', 'stdio', 'http'];

export const STATUS_FILTER_IDS: StatusFilterKey[] = [
  'connected',
  'disabled',
  'error',
  'needs_setup',
];

/**
 * Resolve the display label for a transport filter chip.
 */
export function getTransportFilterLabel(id: TransportFilter): string {
  switch (id) {
    case 'all':
      return 'All';
    case 'stdio':
      return 'Stdio';
    case 'http':
      return 'HTTP';
    default: {
      const _exhaustive: never = id;
      return _exhaustive;
    }
  }
}

/**
 * Resolve the display label for a status filter chip.
 */
export function getStatusFilterLabel(id: StatusFilterKey): string {
  switch (id) {
    case 'connected':
      return 'Connected';
    case 'disabled':
      return 'Disabled';
    case 'error':
      return 'Error';
    case 'needs_setup':
      return 'Needs Setup';
    default: {
      const _exhaustive: never = id;
      return _exhaustive;
    }
  }
}

/** Group discovered features by installed server id. */
export function groupFeaturesByServerId(features: ServerFeature[]): Record<string, ServerFeature[]> {
  return features.reduce<Record<string, ServerFeature[]>>((acc, feature) => {
    const bucket = acc[feature.server_id] ?? [];
    bucket.push(feature);
    acc[feature.server_id] = bucket;
    return acc;
  }, {});
}

/**
 * Map a server action to the status filter bucket it belongs in.
 */
export function statusKeyFromAction(action: ServerActionKey): StatusFilterKey {
  switch (action) {
    case 'running':
    case 'connected_auto':
      return 'connected';
    case 'enable':
      return 'disabled';
    case 'error':
      return 'error';
    default:
      return 'needs_setup';
  }
}

/** Whether a server matches the selected transport filter. */
export function matchesTransport(server: ServerViewModel, transportFilter: TransportFilter): boolean {
  if (transportFilter === 'all') {
    return true;
  }

  return server.transport.type === transportFilter;
}

/**
 * Whether a server matches active status toggles.
 * Empty set means show all (Beeper-style: no status filter applied).
 */
export function matchesStatus(
  action: ServerActionKey,
  activeStatusFilters: ReadonlySet<StatusFilterKey>
): boolean {
  if (activeStatusFilters.size === 0) {
    return true;
  }

  return activeStatusFilters.has(statusKeyFromAction(action));
}

/** Whether a feature name or description matches the search query. */
function featureMatchesQuery(feature: ServerFeature, query: string): boolean {
  return (
    feature.feature_name.toLowerCase().includes(query) ||
    (feature.display_name?.toLowerCase().includes(query) ?? false) ||
    (feature.description?.toLowerCase().includes(query) ?? false)
  );
}

/**
 * Whether an installed server matches transport, status, and search filters.
 */
export function serverMatchesFilters(
  server: ServerViewModel,
  searchQuery: string,
  features: ServerFeature[],
  transportFilter: TransportFilter,
  activeStatusFilters: ReadonlySet<StatusFilterKey>,
  serverAction: ServerActionKey
): boolean {
  if (!matchesTransport(server, transportFilter)) {
    return false;
  }

  if (!matchesStatus(serverAction, activeStatusFilters)) {
    return false;
  }

  const query = searchQuery.trim().toLowerCase();
  if (!query) {
    return true;
  }

  const metadataMatch =
    server.name.toLowerCase().includes(query) ||
    server.id.toLowerCase().includes(query) ||
    (server.description?.toLowerCase().includes(query) ?? false);

  if (metadataMatch) {
    return true;
  }

  return features.some((feature) => featureMatchesQuery(feature, query));
}

/**
 * Count non-default transport and status filters for the Filters button badge.
 */
export function countActiveServerFilters(
  transportFilter: TransportFilter,
  activeStatusFilters: ReadonlySet<StatusFilterKey>
): number {
  let count = activeStatusFilters.size;
  if (transportFilter !== 'all') {
    count += 1;
  }
  return count;
}

/** Per-status counts for the My Servers header summary. */
export type ServerCountSummary = {
  installed: number;
  connected: number;
  disabled: number;
  error: number;
  needsSetup: number;
};

/**
 * Aggregate installed-server counts by status bucket (same buckets as status filters).
 */
export function computeServerCountSummary(
  servers: ServerViewModel[],
  getAction: (server: ServerViewModel) => ServerActionKey
): ServerCountSummary {
  const summary: ServerCountSummary = {
    installed: servers.length,
    connected: 0,
    disabled: 0,
    error: 0,
    needsSetup: 0,
  };

  for (const server of servers) {
    switch (statusKeyFromAction(getAction(server))) {
      case 'connected':
        summary.connected += 1;
        break;
      case 'disabled':
        summary.disabled += 1;
        break;
      case 'error':
        summary.error += 1;
        break;
      case 'needs_setup':
        summary.needsSetup += 1;
        break;
    }
  }

  return summary;
}

/**
 * Compact inline summary next to the My Servers title.
 */
export function formatServerCountSummary(summary: ServerCountSummary): string {
  return `${summary.connected} connected, ${summary.installed - summary.connected} other`;
}

/**
 * Tooltip lines for the server count hover panel.
 */
export function describeServerCountSummary(summary: ServerCountSummary): string[] {
  const lines = [
    `${summary.installed} installed`,
    `${summary.connected} connected`,
    `${summary.disabled} disabled`,
    `${summary.error} with errors`,
  ];

  if (summary.needsSetup > 0) {
    lines.push(`${summary.needsSetup} need setup`);
  }

  return lines;
}

/**
 * Human-readable lines describing the currently applied server list filters.
 */
export function describeAppliedServerFilters(
  transportFilter: TransportFilter,
  activeStatusFilters: ReadonlySet<StatusFilterKey>
): string[] {
  const transportLabel = getTransportFilterLabel(transportFilter);

  const statusLabel =
    activeStatusFilters.size === 0
      ? 'All'
      : STATUS_FILTER_IDS.filter((filterId) => activeStatusFilters.has(filterId))
          .map((filterId) => getStatusFilterLabel(filterId))
          .join(', ');

  if (countActiveServerFilters(transportFilter, activeStatusFilters) === 0) {
    return ['No filters applied', 'Showing all servers'];
  }

  return [`Transport: ${transportLabel}`, `Status: ${statusLabel}`];
}
