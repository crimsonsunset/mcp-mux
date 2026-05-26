import { describe, it, expect } from 'vitest';

/**
 * Placeholder until Phase 4 — will move to `apps/desktop/src/lib/api/fetch-api.ts`.
 */
function routeFor(
  command: string,
  args?: Record<string, unknown>
): { method: string; path: string } {
  if (command === 'get_gateway_status') {
    const spaceId = args?.spaceId;
    const query = spaceId != null ? `?spaceId=${encodeURIComponent(String(spaceId))}` : '';
    return { method: 'GET', path: `/api/v1/gateway/status${query}` };
  }
  throw new Error(`Unknown command: ${command}`);
}

describe('admin transport mapping', () => {
  it('maps get_gateway_status to GET /api/v1/gateway/status', () => {
    const spaceId = 'test-space-id';
    expect(routeFor('get_gateway_status', { spaceId })).toEqual({
      method: 'GET',
      path: '/api/v1/gateway/status?spaceId=test-space-id',
    });
  });
});
