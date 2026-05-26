/**
 * Backend events facade — Tauri IPC on desktop, admin SSE on web.
 * @see docs/planning/unified-backend-facade.md
 */

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
  ChannelCallback,
  AllEventsCallback,
  PayloadTypeMap,
} from './useDomainEvents';

export {
  useWorkspaceEvents,
  useWorkspaceEventListener,
} from './useWorkspaceEvents';

export type {
  WorkspaceEventChannel,
  WorkspaceBindingChangedPayload,
  WorkspaceNeedsBindingPayload,
  WorkspaceChannelCallback,
  WorkspaceEventsCallback,
  WorkspacePayloadTypeMap,
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

export {
  useBackendEventSubscription,
  type BackendEventSubscriptionOptions,
} from './use-backend-event-subscription';
