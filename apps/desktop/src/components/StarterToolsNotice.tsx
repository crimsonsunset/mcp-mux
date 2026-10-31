/**
 * Tells the user what the Starter bundle grants to folders mapped to it and,
 * past the size warning, how to slim it down. Unmapped folders get nothing
 * (deny by default), so this is about what a deliberate mapping hands out.
 * Guidance only, never a limit.
 */
import { AlertTriangle, ArrowRight, Layers, Plus } from 'lucide-react';
import type { StarterToolSummary } from '@/lib/api/featureSets';
import { useNavigate } from '@/hooks/use-navigate.hook';
import { MuxPromptCode } from './MuxPrompt';

/**
 * Pluralize a count with its noun.
 * @param n - The count.
 * @param word - The singular noun.
 * @returns e.g. "1 tool" or "3 tools".
 */
function plural(n: number, word: string) {
  return `${n} ${word}${n === 1 ? '' : 's'}`;
}

interface OverThresholdProps {
  summary: StarterToolSummary;
  testId?: string;
}

/**
 * Amber warning once the Starter grants more tools than AI apps handle well.
 * Offers both ways out: ask `@mux`, or build a FeatureSet by hand.
 */
export function StarterOverThresholdWarning({
  summary,
  testId = 'starter-tools-warning',
}: OverThresholdProps) {
  const navigate = useNavigate();
  return (
    <div
      className="flex items-start gap-3 rounded-xl border border-amber-300 bg-amber-50 p-4 dark:border-amber-700/60 dark:bg-amber-900/20"
      data-testid={testId}
    >
      <AlertTriangle className="mt-0.5 h-5 w-5 flex-shrink-0 text-amber-600 dark:text-amber-400" />
      <div className="min-w-0 flex-1">
        <p className="text-sm font-semibold text-amber-900 dark:text-amber-100">
          Your Starter bundle has {plural(summary.tool_count, 'tool')}, which is a lot
        </p>
        <p className="mt-0.5 text-xs leading-relaxed text-amber-800 dark:text-amber-200">
          Everything still works, but past {summary.threshold} tools McpMux's tool search gets noisier
          and AI apps pick the wrong tool more often. Give each project just what it needs. Ask your
          AI app:
        </p>
        <div className="mt-2 flex flex-wrap items-center gap-x-3 gap-y-2">
          <MuxPromptCode testId={`${testId}-copy`} />
          <span className="text-xs text-amber-800 dark:text-amber-200">or</span>
          <button
            type="button"
            onClick={() => navigate('featuresets')}
            className="inline-flex items-center gap-1 text-xs font-semibold text-amber-900 hover:underline dark:text-amber-100"
            data-testid={`${testId}-create`}
          >
            <Plus className="h-3 w-3" />
            Create a FeatureSet
          </button>
        </div>
      </div>
    </div>
  );
}

interface DashboardCardProps {
  summary: StarterToolSummary;
}

/**
 * Dashboard card: "what does the Starter grant?" in one line, or the warning
 * when the Starter has grown past the threshold.
 */
export function StarterToolsCard({ summary }: DashboardCardProps) {
  const navigate = useNavigate();
  if (summary.over_threshold) {
    return <StarterOverThresholdWarning summary={summary} />;
  }
  if (summary.tool_count === 0) return null;

  const detail = summary.auto_include
    ? 'Every tool from every server, found through McpMux. New servers show up on their own.'
    : 'The tools you picked for your Starter bundle.';
  return (
    <button
      type="button"
      onClick={() => navigate('featuresets')}
      className="group flex w-full items-center gap-3 rounded-xl border border-[rgb(var(--border-subtle))] bg-[rgb(var(--card))] p-4 text-left shadow transition-all duration-200 hover:-translate-y-0.5 hover:border-[rgb(var(--border))] hover:shadow-md"
      data-testid="starter-tools-card"
    >
      <span className="flex h-9 w-9 flex-shrink-0 items-center justify-center rounded-lg bg-emerald-500/10 text-emerald-600 dark:text-emerald-400">
        <Layers className="h-5 w-5" />
      </span>
      <span className="min-w-0 flex-1">
        <span className="block text-sm font-semibold" data-testid="starter-tools-card-title">
          Starter bundle: {plural(summary.tool_count, 'tool')} from{' '}
          {plural(summary.server_count, 'server')}
        </span>
        <span className="block text-xs text-[rgb(var(--muted))]">{detail}</span>
      </span>
      <ArrowRight className="h-4 w-4 flex-shrink-0 text-[rgb(var(--muted))] transition-transform group-hover:translate-x-0.5" />
    </button>
  );
}
