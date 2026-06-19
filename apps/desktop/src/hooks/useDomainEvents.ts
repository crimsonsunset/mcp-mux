/** @deprecated Prefer `@/lib/backend/events` */
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
  ServerUpdateAvailablePayload,
  ServerStatusChangedPayload,
  ServerAuthProgressPayload,
  ServerFeaturesRefreshedPayload,
  FeatureSetChangedPayload,
  ClientChangedPayload,
  ClientGrantChangedPayload,
  GatewayChangedPayload,
  MCPNotificationPayload,
} from '@/lib/backend/events';

export { default } from '@/lib/backend/events/useDomainEvents';
