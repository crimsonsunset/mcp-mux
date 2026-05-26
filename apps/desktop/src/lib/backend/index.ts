/**
 * Unified backend facade — three channels: data (commands), events (Phase 2), shell (desktop-only).
 * @see docs/planning/unified-backend-facade.md
 */

export * from '../api';
export * from './data/transport';
export * from './data/fetch-api';
export * as shell from './shell';
