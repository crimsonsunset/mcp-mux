import { Button } from '@mcpmux/ui';
import { Download, Loader2 } from 'lucide-react';

import type { ServerPendingUpdate } from '@/features/servers/server-pending-updates.helpers';

interface ServerPendingUpdatesListProps {
  updates: ServerPendingUpdate[];
  updatingServerKey: string | null;
  updatingAll: boolean;
  onUpdateOne: (update: ServerPendingUpdate) => void;
  onUpdateAll: () => void;
}

/**
 * Row key for per-server update-in-progress tracking.
 */
export function pendingUpdateKey(update: ServerPendingUpdate): string {
  return `${update.spaceId}:${update.serverId}`;
}

/**
 * List of servers with available package updates and per-row / bulk actions.
 */
export function ServerPendingUpdatesList({
  updates,
  updatingServerKey,
  updatingAll,
  onUpdateOne,
  onUpdateAll,
}: ServerPendingUpdatesListProps) {
  if (updates.length === 0) {
    return null;
  }

  const enabledCount = updates.filter((update) => update.enabled).length;

  return (
    <div
      className="space-y-3 border-t border-[rgb(var(--border-subtle))] pt-4"
      data-testid="server-pending-updates-list"
    >
      <div className="flex items-center justify-between gap-3">
        <p className="text-sm font-medium">
          {updates.length} update{updates.length === 1 ? '' : 's'} available
        </p>
        <Button
          type="button"
          size="sm"
          onClick={onUpdateAll}
          disabled={updatingAll || enabledCount === 0}
          data-testid="update-all-servers-btn"
        >
          {updatingAll ? (
            <>
              <Loader2 className="h-4 w-4 animate-spin" />
              Updating…
            </>
          ) : (
            <>
              <Download className="h-4 w-4" />
              Update All
            </>
          )}
        </Button>
      </div>

      <ul className="space-y-2">
        {updates.map((update) => {
          const rowKey = pendingUpdateKey(update);
          const isUpdating = updatingAll || updatingServerKey === rowKey;

          return (
            <li
              key={rowKey}
              className="flex items-center justify-between gap-3 rounded-lg border border-[rgb(var(--border-subtle))] bg-[rgb(var(--surface-raised))] px-3 py-2"
              data-testid={`pending-update-${update.serverId}`}
            >
              <div className="min-w-0 flex-1">
                <p className="truncate text-sm font-medium">{update.name}</p>
                <p className="text-xs text-[rgb(var(--muted))]">
                  {update.currentVersion ? `v${update.currentVersion}` : 'unpinned'} → v
                  {update.latestVersion}
                </p>
              </div>
              <Button
                type="button"
                size="sm"
                variant="secondary"
                onClick={() => onUpdateOne(update)}
                disabled={!update.enabled || isUpdating}
                title={update.enabled ? undefined : 'Enable the server before updating'}
                data-testid={`update-server-${update.serverId}`}
              >
                {isUpdating ? (
                  <Loader2 className="h-4 w-4 animate-spin" />
                ) : (
                  'Update'
                )}
              </Button>
            </li>
          );
        })}
      </ul>
    </div>
  );
}
