/**
 * Hooks - React hooks for the McpMux desktop application
 */

// Data synchronization
export { useDataSync } from './useDataSync';

// Domain events (event-driven architecture)
export {
  useDomainEvents,
  useSpaceEvents,
  useServerStatusEvents,
  useServerAuthProgress,
  useClientEvents,
  useGatewayEvents,
} from '@/lib/backend/events';

export type {
  DomainEventChannel,
  DomainEventPayload,
  SpaceChangedPayload,
  ServerChangedPayload,
  ServerStatusChangedPayload,
  ServerAuthProgressPayload,
  ServerFeaturesRefreshedPayload,
  FeatureSetChangedPayload,
  ClientChangedPayload,
  ClientGrantChangedPayload,
  GatewayChangedPayload,
  MCPNotificationPayload,
} from '@/lib/backend/events';

export {
  useWorkspaceEvents,
  useWorkspaceEventListener,
} from '@/lib/backend/events';

export type {
  WorkspaceEventChannel,
  WorkspaceBindingChangedPayload,
  WorkspaceNeedsBindingPayload,
} from '@/lib/backend/events';

export {
  useOAuthClientEvents,
  useOAuthClientEventListener,
} from '@/lib/backend/events';

export type { OAuthClientChangedPayload } from '@/lib/backend/events';

export {
  useMetaToolEvents,
  useMetaToolEventListener,
} from '@/lib/backend/events';

// Server management
export { useServerManager } from './useServerManager';

// Space management
export { useSpaces } from './useSpaces';
