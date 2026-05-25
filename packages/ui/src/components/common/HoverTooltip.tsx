import { type ReactNode } from 'react';
import { cn } from '../../lib/cn';

export interface HoverTooltipProps {
  children: ReactNode;
  title: string;
  lines?: string[];
  side?: 'top' | 'bottom';
  className?: string;
  hidden?: boolean;
  'data-testid'?: string;
}

/**
 * Wraps a control and shows a tooltip panel on hover (hidden while `hidden` is true).
 */
export function HoverTooltip({
  children,
  title,
  lines = [],
  side = 'top',
  className,
  hidden = false,
  'data-testid': testId,
}: HoverTooltipProps) {
  return (
    <div className={cn('relative group', className)}>
      <div
        role="tooltip"
        className={cn(
          'pointer-events-none absolute z-[60] min-w-[10rem] max-w-xs dropdown-menu px-3 py-2 text-left',
          'transition-opacity duration-150',
          side === 'top' ? 'bottom-full mb-2' : 'top-full mt-2',
          'right-0',
          hidden ? 'opacity-0' : 'opacity-0 group-hover:opacity-100 group-focus-within:opacity-100'
        )}
        data-testid={testId}
      >
        <p className="text-xs font-medium text-[rgb(var(--foreground))] mb-1">{title}</p>
        {lines.map((line) => (
          <p key={line} className="text-xs text-[rgb(var(--muted))] leading-relaxed">
            {line}
          </p>
        ))}
      </div>
      {children}
    </div>
  );
}
