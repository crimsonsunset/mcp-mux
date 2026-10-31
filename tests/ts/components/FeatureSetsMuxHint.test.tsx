/**
 * FeatureSets page: the `@mux` hint banner (ported from upstream #243).
 *
 * The hint shows on a normal visit, and steps aside when the Starter is over
 * the tool threshold because that warning already carries the same prompt.
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { screen } from '@testing-library/react';

const { starterSummaryMock } = vi.hoisted(() => ({ starterSummaryMock: vi.fn() }));

vi.mock('@/hooks/useStarterToolSummary', () => ({
  useStarterToolSummary: starterSummaryMock,
}));

vi.mock('@/lib/api/featureSets', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/lib/api/featureSets')>()),
  listFeatureSetsBySpace: vi.fn().mockResolvedValue([]),
  getStarterToolSummary: vi.fn().mockResolvedValue(null),
}));

vi.mock('@/lib/backend/events', () => ({
  useWorkspaceEvents: () => ({
    subscribe: vi.fn(() => () => {}),
    subscribeMany: vi.fn(() => () => {}),
  }),
  useWorkspaceEventListener: vi.fn(),
}));

vi.mock('@/stores', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/stores')>();
  return {
    ...actual,
    useViewSpace: () => ({ id: 's1', name: 'Space One', icon: '🧪' }),
  };
});

import { renderWithI18n } from '../render-with-i18n.helpers';
import { FeatureSetsPage } from '@/features/featuresets/FeatureSetsPage';

const OK_SUMMARY = {
  feature_set_id: 'fs_default_s1',
  auto_include: true,
  tool_count: 10,
  server_count: 1,
  threshold: 80,
  over_threshold: false,
};

describe('FeatureSetsPage – @mux hint', () => {
  beforeEach(() => {
    starterSummaryMock.mockReset();
  });

  it('shows the hint with a copyable prompt on a normal visit', async () => {
    starterSummaryMock.mockReturnValue({ summary: OK_SUMMARY });
    renderWithI18n(<FeatureSetsPage />);

    expect(await screen.findByTestId('featuresets-mux-hint')).toBeTruthy();
    expect(screen.getByTestId('featuresets-mux-hint-copy')).toBeTruthy();
    expect(screen.queryByTestId('featuresets-starter-warning')).toBeNull();
  });

  it('shows the hint while the Starter summary is still loading', async () => {
    starterSummaryMock.mockReturnValue({ summary: null });
    renderWithI18n(<FeatureSetsPage />);

    expect(await screen.findByTestId('featuresets-mux-hint')).toBeTruthy();
  });

  it('shows only the over-threshold warning when the Starter is too large', async () => {
    starterSummaryMock.mockReturnValue({
      summary: { ...OK_SUMMARY, tool_count: 200, over_threshold: true },
    });
    renderWithI18n(<FeatureSetsPage />);

    expect(await screen.findByTestId('featuresets-starter-warning')).toBeTruthy();
    expect(screen.queryByTestId('featuresets-mux-hint')).toBeNull();
  });
});
