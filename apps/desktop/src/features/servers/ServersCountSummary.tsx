import { HoverTooltip } from '@mcpmux/ui';
import {
  describeServerCountSummary,
  formatServerCountSummary,
  type ServerCountSummary,
} from './servers-page.helpers';

interface ServersCountSummaryProps {
  summary: ServerCountSummary;
}

/**
 * Inline installed-server counts beside the My Servers title, with hover breakdown.
 */
export function ServersCountSummary({ summary }: ServersCountSummaryProps) {
  if (summary.installed === 0) {
    return null;
  }

  return (
    <HoverTooltip
      title="Server Summary"
      lines={describeServerCountSummary(summary)}
      data-testid="servers-count-tooltip"
      className="flex-shrink min-w-0"
    >
      <p
        className="text-sm text-[rgb(var(--muted))] truncate cursor-default"
        data-testid="servers-count-summary"
      >
        {formatServerCountSummary(summary)}
      </p>
    </HoverTooltip>
  );
}
