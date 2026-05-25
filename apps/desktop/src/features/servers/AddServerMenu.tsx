import { ChevronDown, Compass, FileJson, Plus } from 'lucide-react';
import {
  Button,
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@mcpmux/ui';

interface AddServerMenuProps {
  /** Opens the Discover page to browse the community server registry. */
  onDiscover: () => void;
  /** Opens the Space JSON editor to add a custom server definition. */
  onCustom: () => void;
}

/**
 * Dropdown for the two ways to add MCP servers: registry discover vs custom JSON.
 */
export function AddServerMenu({ onDiscover, onCustom }: AddServerMenuProps) {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger data-testid="add-server-menu-trigger">
        <Button variant="primary" size="md" type="button">
          <Plus className="h-4 w-4" />
          Add Server
          <ChevronDown className="h-4 w-4" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-80 p-1.5" data-testid="add-server-menu">
        <DropdownMenuItem
          icon={Compass}
          label="Discover from registry"
          description="Browse the community catalog — install with guided setup, OAuth, and credential fields."
          onSelect={onDiscover}
          data-testid="add-server-option-discover"
        />
        <DropdownMenuItem
          icon={FileJson}
          label="Add custom server"
          description="Edit your Space JSON config for local CLIs, private servers, or anything not in the registry."
          onSelect={onCustom}
          data-testid="add-server-option-custom"
        />
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
