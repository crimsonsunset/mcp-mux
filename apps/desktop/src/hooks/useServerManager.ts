/**
 * useServerManager - React hook for event-driven server management
 *
 * Provides:
 * - Automatic subscription to server events
 * - Local state management for server statuses
 * - Helper functions for common operations
 */

import { useCallback, useEffect, useRef, useState } from "react";
import {
  ServerStatusResponse,
  FeaturesUpdatedEvent,
  getServerStatuses,
  enableServer,
  disableServer,
  startAuth,
  cancelAuth,
  retryConnection,
  getConnectButtonLabel,
  getServerAction,
  ConnectionStatus,
} from "../lib/api/serverManager";
import { listServerFeaturesByServer } from "../lib/api/serverFeatures";
import {
  useDomainEvents,
  type ServerAuthProgressPayload,
  type ServerFeaturesRefreshedPayload,
  type ServerStatusChangedPayload,
} from "./useDomainEvents";

interface UseServerManagerOptions {
  spaceId: string;
  /** Called when features are updated */
  onFeaturesChange?: (event: FeaturesUpdatedEvent) => void;
}

interface UseServerManagerResult {
  /** Map of server_id -> status info */
  statuses: Record<string, ServerStatusResponse>;
  /** Loading state for initial fetch */
  loading: boolean;
  /** Error message if initial fetch failed */
  error: string | null;
  /** Auth progress for servers in Authenticating state (server_id -> remaining seconds) */
  authProgress: Record<string, number>;
  /** Enable and connect a server */
  enable: (serverId: string) => Promise<void>;
  /** Disable a server */
  disable: (serverId: string) => Promise<void>;
  /** Start OAuth flow */
  connect: (serverId: string) => Promise<void>;
  /** Cancel OAuth flow */
  cancel: (serverId: string) => Promise<void>;
  /** Retry connection */
  retry: (serverId: string) => Promise<void>;
  /** Get button label for a server */
  getButtonLabel: (serverId: string) => string;
  /** Get action type for a server */
  getAction: (
    serverId: string
  ) =>
    | "enable"
    | "disable"
    | "connect"
    | "cancel"
    | "retry"
    | "connected"
    | "connecting";
  /** Refresh statuses from backend */
  refresh: () => Promise<void>;
}

/**
 * Normalize backend status strings to the UI ConnectionStatus union.
 */
function normalizeConnectionStatus(status: string): ConnectionStatus {
  if (status === "auth_required") {
    return "oauth_required";
  }
  return status as ConnectionStatus;
}

/**
 * Map a REST/Tauri status payload into ServerStatusResponse.
 */
function toServerStatusResponse(
  serverId: string,
  payload: Pick<
    ServerStatusResponse,
    "status" | "flow_id" | "has_connected_before" | "message"
  > & { status: string }
): ServerStatusResponse {
  return {
    server_id: serverId,
    status: normalizeConnectionStatus(payload.status),
    flow_id: payload.flow_id,
    has_connected_before: payload.has_connected_before,
    message: payload.message ?? null,
  };
}

export function useServerManager({
  spaceId,
  onFeaturesChange,
}: UseServerManagerOptions): UseServerManagerResult {
  const [statuses, setStatuses] = useState<
    Record<string, ServerStatusResponse>
  >({});
  const [authProgress, setAuthProgress] = useState<Record<string, number>>({});
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const prevSpaceId = useRef<string | null>(null);

  // Stable ref for onFeaturesChange to avoid re-subscribing on every render
  const onFeaturesChangeRef = useRef(onFeaturesChange);
  onFeaturesChangeRef.current = onFeaturesChange;
  const { subscribe } = useDomainEvents();

  // Fetch initial statuses
  const refresh = useCallback(async () => {
    if (!spaceId) return;

    try {
      setLoading(true);
      setError(null);
      const result = await getServerStatuses(spaceId);
      const normalized = Object.fromEntries(
        Object.entries(result).map(([serverId, status]) => [
          serverId,
          toServerStatusResponse(serverId, status),
        ])
      );
      console.log(
        `[useServerManager] Loaded ${Object.keys(normalized).length} server statuses for space ${spaceId}`
      );
      setStatuses(normalized);
    } catch (e) {
      const message = e instanceof Error ? e.message : String(e);
      console.error(`[useServerManager] Failed to load statuses for space ${spaceId}:`, message);
      setError(message);
    } finally {
      setLoading(false);
    }
  }, [spaceId]);

  useEffect(() => {
    if (!spaceId || prevSpaceId.current !== spaceId) {
      setStatuses({});
      setAuthProgress({});
      setError(null);
      setLoading(true); // Set loading true while fetching
      prevSpaceId.current = spaceId || null;
      
      // Immediately hydrate from backend on space switch
      if (spaceId) {
        refresh();
      }
    }
  }, [spaceId, refresh]);

  // Initial fetch (only on mount, not on space change since handled above)
  useEffect(() => {
    if (!prevSpaceId.current) {
      refresh();
    }
  }, [refresh]);

  // Subscribe to domain events (Tauri IPC on desktop, SSE on web admin)
  useEffect(() => {
    if (!spaceId) return;

    const unsubs = [
      subscribe("server-status-changed", (event: ServerStatusChangedPayload) => {
        if (event.space_id !== spaceId) return;

        setStatuses((prev) => {
          const existing = prev[event.server_id];
          const flowId = event.flow_id;
          if (existing && existing.flow_id > flowId) {
            return prev;
          }

          return {
            ...prev,
            [event.server_id]: toServerStatusResponse(event.server_id, {
              status: event.status,
              flow_id: flowId,
              has_connected_before: event.has_connected_before,
              message: event.message ?? null,
            }),
          };
        });

        if (event.status !== "authenticating") {
          setAuthProgress((prev) => {
            const next = { ...prev };
            delete next[event.server_id];
            return next;
          });
        }
      }),
      subscribe("server-auth-progress", (event: ServerAuthProgressPayload) => {
        if (event.space_id !== spaceId) return;

        setAuthProgress((prev) => ({
          ...prev,
          [event.server_id]: event.remaining_seconds,
        }));
      }),
      subscribe("server-features-refreshed", (event: ServerFeaturesRefreshedPayload) => {
        if (event.space_id !== spaceId) return;

        void (async () => {
          const allFeatures = await listServerFeaturesByServer(
            event.space_id,
            event.server_id
          );
          const features = {
            tools: allFeatures.filter((f) => f.feature_type === "tool"),
            prompts: allFeatures.filter((f) => f.feature_type === "prompt"),
            resources: allFeatures.filter((f) => f.feature_type === "resource"),
          };
          onFeaturesChangeRef.current?.({
            type: "features_updated",
            space_id: event.space_id,
            server_id: event.server_id,
            features,
            added: event.added,
            removed: event.removed,
          });
        })();
      }),
    ];

    refresh();

    return () => {
      unsubs.forEach((unsub) => unsub());
    };
  }, [spaceId, subscribe, refresh]);

  // Actions
  const enable = useCallback(
    (serverId: string) => enableServer(spaceId, serverId),
    [spaceId]
  );

  const disable = useCallback(
    (serverId: string) => disableServer(spaceId, serverId),
    [spaceId]
  );

  const connect = useCallback(
    (serverId: string) => startAuth(spaceId, serverId),
    [spaceId]
  );

  const cancel = useCallback(
    (serverId: string) => cancelAuth(spaceId, serverId),
    [spaceId]
  );

  const retry = useCallback(
    (serverId: string) => retryConnection(spaceId, serverId),
    [spaceId]
  );

  // Helpers
  const getButtonLabel = useCallback(
    (serverId: string) => {
      const status = statuses[serverId];
      if (!status) return "Enable";
      return getConnectButtonLabel(status.status, status.has_connected_before);
    },
    [statuses]
  );

  const getAction = useCallback(
    (serverId: string) => {
      const status = statuses[serverId];
      if (!status) return "enable";
      return getServerAction(status.status);
    },
    [statuses]
  );

  return {
    statuses,
    loading,
    error,
    authProgress,
    enable,
    disable,
    connect,
    cancel,
    retry,
    getButtonLabel,
    getAction,
    refresh,
  };
}
