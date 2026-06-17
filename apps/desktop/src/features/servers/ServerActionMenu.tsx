import { MoreVertical, Settings, RefreshCw, RotateCcw, FileText, Code, Trash2, Copy, Download } from 'lucide-react';
import {
  DropdownMenu,
  DropdownMenuAction,
  DropdownMenuContent,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@mcpmux/ui';
import type { UpdatePolicy } from '@/lib/api/settings';

export interface ServerActionMenuProps {
  serverId: string;
  serverName: string;
  /** Whether the server has credential / config inputs. Servers with no inputs still show
   *  Configure so the display name can be edited. */
  hasInputs: boolean;
  isOAuth: boolean;
  isEnabled: boolean;
  isConnected: boolean;
  /** npx/uvx stdio transport — eligible for package update actions. */
  isPackageManaged?: boolean;
  /** Per-server update policy from installed state. */
  updatePolicy?: UpdatePolicy;
  /** Show "Add another account…" for registry/manual installs (not clones-of-clones). */
  canCloneAccount?: boolean;
  onConfigure: () => void;
  onRefresh: () => void;
  onReconnect: () => void;
  onUpdateNow?: () => void;
  onViewLogs: () => void;
  onViewDefinition: () => void;
  onCloneAccount?: () => void;
  onUninstall: () => void;
}

/**
 * Overflow menu for per-server actions (configure, logs, uninstall, etc.).
 */
export function ServerActionMenu({
  serverId,
  serverName: _serverName,
  hasInputs,
  isOAuth,
  isEnabled,
  isConnected: _isConnected,
  isPackageManaged = false,
  updatePolicy = 'notify',
  canCloneAccount = false,
  onConfigure,
  onRefresh,
  onReconnect,
  onUpdateNow,
  onViewLogs,
  onViewDefinition,
  onCloneAccount,
  onUninstall,
}: ServerActionMenuProps) {
  const showUpdateNow =
    isPackageManaged && updatePolicy === 'auto' && isEnabled && onUpdateNow != null;

  return (
    <DropdownMenu>
      <DropdownMenuTrigger>
        <button
          type="button"
          className="p-2 text-sm rounded-lg bg-[rgb(var(--surface-hover))] border border-[rgb(var(--border))] text-[rgb(var(--foreground))]/70 hover:bg-[rgb(var(--surface-elevated))] hover:text-[rgb(var(--foreground))] transition-colors"
          title="More actions"
          aria-label="More actions"
          data-testid={`action-menu-${serverId}`}
        >
          <MoreVertical className="h-4 w-4" />
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-48 py-1 p-1">
        <DropdownMenuAction
          icon={Settings}
          label={hasInputs ? 'Configure' : 'Settings'}
          onSelect={onConfigure}
        />
        {isEnabled && (
          <DropdownMenuAction icon={RefreshCw} label="Refresh" onSelect={onRefresh} />
        )}
        {showUpdateNow && (
          <DropdownMenuAction
            icon={Download}
            label="Update Now"
            onSelect={onUpdateNow}
            data-testid={`update-now-${serverId}`}
          />
        )}
        {isOAuth && isEnabled && (
          <DropdownMenuAction
            icon={RotateCcw}
            label="Reconnect"
            onSelect={onReconnect}
            variant="warning"
          />
        )}
        <DropdownMenuAction
          icon={FileText}
          label="View Logs"
          onSelect={onViewLogs}
          data-testid={`view-logs-${serverId}`}
        />
        <DropdownMenuAction
          icon={Code}
          label="View Definition"
          onSelect={onViewDefinition}
          data-testid={`view-definition-${serverId}`}
        />
        {canCloneAccount && onCloneAccount && (
          <DropdownMenuAction
            icon={Copy}
            label="Add another account…"
            onSelect={onCloneAccount}
            data-testid={`clone-account-${serverId}`}
          />
        )}
        <DropdownMenuSeparator />
        <DropdownMenuAction
          icon={Trash2}
          label="Uninstall"
          onSelect={onUninstall}
          variant="danger"
          data-testid={`uninstall-menu-${serverId}`}
        />
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
