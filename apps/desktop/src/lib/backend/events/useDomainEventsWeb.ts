/**
 * SSE-based domain event listener for web admin mode.
 *
 * Mirrors the `useDomainEvents` API (`subscribe`, `subscribeAll`, `subscribeMany`)
 * using `GET /api/v1/events`.
 */

import { useCallback, useEffect, useRef, useState } from 'react';

import { isTauri } from '../data/transport';

import type {
  AllEventsCallback,
  ChannelCallback,
  DomainEventChannel,
  DomainEventPayload,
  PayloadTypeMap,
} from './useDomainEvents';

/** All domain channels streamed over SSE. */
const ALL_CHANNELS: DomainEventChannel[] = [
  'space-changed',
  'server-changed',
  'server-status-changed',
  'server-auth-progress',
  'server-features-refreshed',
  'feature-set-changed',
  'client-changed',
  'client-grant-changed',
  'gateway-changed',
  'mcp-notification',
];

/**
 * Subscribe to admin SSE domain events in web mode.
 */
export function useDomainEventsWeb() {
  const handlersRef = useRef<Map<DomainEventChannel, Set<(payload: DomainEventPayload) => void>>>(
    new Map()
  );
  const allHandlersRef = useRef<Set<AllEventsCallback>>(new Set());
  const sourceRef = useRef<EventSource | null>(null);
  const [lastEvent, setLastEvent] = useState<{
    channel: DomainEventChannel;
    payload: DomainEventPayload;
  } | null>(null);

  useEffect(() => {
    if (isTauri()) {
      return;
    }
    const source = new EventSource('/api/v1/events');
    sourceRef.current = source;

    const dispatch = (channel: DomainEventChannel, payload: DomainEventPayload) => {
      setLastEvent({ channel, payload });
      handlersRef.current.get(channel)?.forEach((handler) => handler(payload));
      allHandlersRef.current.forEach((handler) => handler(channel, payload));
    };

    for (const channel of ALL_CHANNELS) {
      source.addEventListener(channel, (event: MessageEvent<string>) => {
        try {
          const payload = JSON.parse(event.data) as DomainEventPayload;
          dispatch(channel, payload);
        } catch {
          // ignore malformed frames
        }
      });
    }

    return () => {
      source.close();
      sourceRef.current = null;
    };
  }, []);

  /**
   * Subscribe to a specific SSE event channel.
   */
  const subscribe = useCallback(
    <T extends DomainEventChannel>(channel: T, callback: ChannelCallback<T>): (() => void) => {
      if (!handlersRef.current.has(channel)) {
        handlersRef.current.set(channel, new Set());
      }
      const wrapped = callback as (payload: DomainEventPayload) => void;
      handlersRef.current.get(channel)!.add(wrapped);
      return () => {
        handlersRef.current.get(channel)?.delete(wrapped);
      };
    },
    []
  );

  /**
   * Subscribe to all domain SSE channels.
   */
  const subscribeAll = useCallback((callback: AllEventsCallback): (() => void) => {
    allHandlersRef.current.add(callback);
    return () => {
      allHandlersRef.current.delete(callback);
    };
  }, []);

  /**
   * Subscribe to multiple domain SSE channels with one callback.
   */
  const subscribeMany = useCallback(
    (channels: DomainEventChannel[], callback: AllEventsCallback): (() => void) => {
      const unsubs = channels.map((channel) =>
        subscribe(channel, (payload) => {
          callback(channel, payload as PayloadTypeMap[typeof channel]);
        })
      );
      return () => {
        unsubs.forEach((unsub) => unsub());
      };
    },
    [subscribe]
  );

  return {
    subscribe,
    subscribeAll,
    subscribeMany,
    lastEvent,
    channels: ALL_CHANNELS,
  };
}
