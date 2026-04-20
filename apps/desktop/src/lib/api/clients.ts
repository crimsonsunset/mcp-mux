import { invoke } from '@tauri-apps/api/core';

/**
 * A Client represents an AI assistant (Cursor, VS Code, Claude, etc.)
 */
export interface Client {
  id: string;
  name: string;
  client_type: string;
  connection_mode: 'locked' | 'follow_active' | 'ask_on_change';
  locked_space_id: string | null;
  grants: Record<string, string[]>; // space_id -> feature_set_ids (legacy, to be removed)
  /**
   * Resolver v2: Space this access key belongs to (chosen at approval).
   * `null` on legacy pre-migration clients — resolver falls through to the
   * default Space.
   */
  pinned_space_id: string | null;
  /**
   * Resolver v2: explicit FS pin. `null` means "follow workspace binding /
   * space active FS".
   */
  pinned_feature_set_id: string | null;
  last_seen: string | null;
}

/**
 * Input for creating a client.
 */
export interface CreateClientInput {
  name: string;
  client_type: string;
  connection_mode: string;
  locked_space_id?: string;
}

/**
 * Input for updating client grants.
 */
export interface UpdateGrantsInput {
  space_id: string;
  feature_set_ids: string[];
}

/**
 * List all clients.
 */
export async function listClients(): Promise<Client[]> {
  return invoke('list_clients');
}

/**
 * Get a client by ID.
 */
export async function getClient(id: string): Promise<Client | null> {
  return invoke('get_client', { id });
}

/**
 * Create a new client.
 */
export async function createClient(input: CreateClientInput): Promise<Client> {
  return invoke('create_client', { input });
}

/**
 * Delete a client.
 */
export async function deleteClient(id: string): Promise<void> {
  return invoke('delete_client', { id });
}

/**
 * Update client grants for a specific space (replaces existing).
 */
export async function updateClientGrants(
  clientId: string,
  input: UpdateGrantsInput
): Promise<Client> {
  return invoke('update_client_grants', { clientId, input });
}

/**
 * Get grants for a client in a specific space.
 */
export async function getClientGrants(
  clientId: string,
  spaceId: string
): Promise<string[]> {
  return invoke('get_client_grants', { clientId, spaceId });
}

/**
 * Get all grants for a client across all spaces.
 */
export async function getAllClientGrants(
  clientId: string
): Promise<Record<string, string[]>> {
  return invoke('get_all_client_grants', { clientId });
}

/**
 * Grant a specific feature set to a client.
 */
export async function grantFeatureSetToClient(
  clientId: string,
  spaceId: string,
  featureSetId: string
): Promise<void> {
  return invoke('grant_feature_set_to_client', { clientId, spaceId, featureSetId });
}

/**
 * Revoke a specific feature set from a client.
 */
export async function revokeFeatureSetFromClient(
  clientId: string,
  spaceId: string,
  featureSetId: string
): Promise<void> {
  return invoke('revoke_feature_set_from_client', { clientId, spaceId, featureSetId });
}

/**
 * Update client connection mode.
 */
export async function updateClientMode(
  clientId: string,
  mode: string,
  lockedSpaceId?: string
): Promise<Client> {
  return invoke('update_client_mode', { clientId, mode, lockedSpaceId });
}

/**
 * Initialize preset clients (Cursor, VS Code, Claude).
 */
export async function initPresetClients(): Promise<void> {
  return invoke('init_preset_clients');
}

/**
 * Pin a client to a Space + optional FeatureSet (resolver v2).
 *
 * Precedence used by the gateway's FeatureSetResolver:
 *   1. pinned_feature_set_id (this pin)        → source = Pin
 *   2. workspace binding matches a reported root → source = WorkspaceBinding
 *   3. space.active_feature_set_id             → source = SpaceActive
 *
 * Pass `pinnedFeatureSetId = undefined` to let the resolver fall through to
 * workspace/space default.
 */
export async function updateClientPin(
  clientId: string,
  pinnedSpaceId: string,
  pinnedFeatureSetId?: string | null
): Promise<void> {
  return invoke('update_client_pin', {
    clientId,
    pinnedSpaceId,
    pinnedFeatureSetId: pinnedFeatureSetId ?? null,
  });
}
