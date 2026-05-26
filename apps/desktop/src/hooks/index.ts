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
} from './useDomainEvents';

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
} from './useDomainEvents';

export {
  useWorkspaceEvents,
  useWorkspaceEventListener,
} from './useWorkspaceEvents';

export type {
  WorkspaceEventChannel,
  WorkspaceBindingChangedPayload,
  WorkspaceNeedsBindingPayload,
} from './useWorkspaceEvents';

export {
  useOAuthClientEvents,
  useOAuthClientEventListener,
} from './useOAuthClientEvents';

export type { OAuthClientChangedPayload } from './useOAuthClientEvents';

export {
  useMetaToolEvents,
  useMetaToolEventListener,
} from './useMetaToolEvents';

// Server management
export { useServerManager } from './useServerManager';

// Space management
export { useSpaces } from './useSpaces';

