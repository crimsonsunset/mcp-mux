import { useState } from 'react';
import { ChevronDown, SlidersHorizontal } from 'lucide-react';
import {
  Button,
  ChipButton,
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuTrigger,
  HoverTooltip,
} from '@mcpmux/ui';
import {
  STATUS_FILTERS,
  TRANSPORT_FILTERS,
  countActiveServerFilters,
  describeAppliedServerFilters,
  type StatusFilterKey,
  type TransportFilter,
} from './servers-page.helpers';

interface ServersFiltersPopoverProps {
  transportFilter: TransportFilter;
  onTransportFilterChange: (filter: TransportFilter) => void;
  activeStatusFilters: Set<StatusFilterKey>;
  onToggleStatusFilter: (statusKey: StatusFilterKey) => void;
  onClearStatusFilters: () => void;
  onClearAllFilters: () => void;
}

/**
 * Popover for transport (stdio/http) and Beeper-style multi-select status filters.
 */
export function ServersFiltersPopover({
  transportFilter,
  onTransportFilterChange,
  activeStatusFilters,
  onToggleStatusFilter,
  onClearStatusFilters,
  onClearAllFilters,
}: ServersFiltersPopoverProps) {
  const [open, setOpen] = useState(false);
  const activeCount = countActiveServerFilters(transportFilter, activeStatusFilters);
  const appliedFilterLines = describeAppliedServerFilters(transportFilter, activeStatusFilters);

  return (
    <HoverTooltip
      title="Applied filters"
      lines={appliedFilterLines}
      hidden={open}
      data-testid="servers-filters-tooltip"
      className="flex-shrink-0"
    >
      <DropdownMenu open={open} onOpenChange={setOpen}>
        <DropdownMenuTrigger data-testid="servers-filters-trigger">
          <Button
            variant="secondary"
            size="md"
            type="button"
            className={
              activeCount > 0
                ? 'bg-[rgb(var(--primary))]/10 text-[rgb(var(--primary))] border-[rgb(var(--primary))]/40'
                : undefined
            }
          >
            <SlidersHorizontal className="h-4 w-4" />
            Filters
            {activeCount > 0 && (
              <span
                className="min-w-[1.25rem] px-1.5 py-0.5 text-xs font-semibold rounded-full bg-[rgb(var(--primary))] text-[rgb(var(--primary-foreground))]"
                data-testid="servers-filters-count"
              >
                {activeCount}
              </span>
            )}
            <ChevronDown
              className={`h-4 w-4 text-[rgb(var(--muted))] transition-transform ${open ? 'rotate-180' : ''}`}
            />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end" className="w-72 p-4 space-y-4" data-testid="servers-filters-popover">
          <div className="space-y-2">
            <p className="text-xs font-medium text-[rgb(var(--muted))]">Transport</p>
            <div className="flex flex-wrap gap-2">
              {TRANSPORT_FILTERS.map((filter) => (
                <ChipButton
                  key={filter.id}
                  active={transportFilter === filter.id}
                  variant="fill"
                  onClick={() => onTransportFilterChange(filter.id)}
                  data-testid={`servers-transport-filter-${filter.id}`}
                >
                  {filter.label}
                </ChipButton>
              ))}
            </div>
          </div>

          <div className="space-y-2">
            <p className="text-xs font-medium text-[rgb(var(--muted))]">Status</p>
            <div className="flex flex-wrap gap-2">
              <ChipButton
                active={activeStatusFilters.size === 0}
                variant="fill"
                onClick={onClearStatusFilters}
                data-testid="servers-status-filter-all"
              >
                All
              </ChipButton>
              {STATUS_FILTERS.map((filter) => (
                <ChipButton
                  key={filter.id}
                  active={activeStatusFilters.has(filter.id)}
                  variant="outline"
                  onClick={() => onToggleStatusFilter(filter.id)}
                  data-testid={`servers-status-filter-${filter.id}`}
                >
                  {filter.label}
                </ChipButton>
              ))}
            </div>
            <p className="text-xs text-[rgb(var(--muted))]">
              Combine status filters (e.g. Connected + Error). All = no status filter.
            </p>
          </div>

          {activeCount > 0 && (
            <Button
              variant="ghost"
              size="sm"
              type="button"
              className="w-full"
              onClick={() => {
                onClearAllFilters();
                setOpen(false);
              }}
              data-testid="servers-filters-clear-all"
            >
              Clear all filters
            </Button>
          )}
        </DropdownMenuContent>
      </DropdownMenu>
    </HoverTooltip>
  );
}
