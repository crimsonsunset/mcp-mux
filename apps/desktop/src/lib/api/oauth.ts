import { invoke } from '@tauri-apps/api/core';

import { apiCall } from './transport';

/** Inbound client registration type (per MCP spec 2025-11-25). */
export type RegistrationType = 'cimd' | 'dcr' | 'preregistered';

/**
 * Inbound OAuth client (Cursor, Claude Desktop, etc.) connecting to McpMux.
 */
export interface OAuthClient {
  client_id: string;
  registration_type: RegistrationType;
  client_name: string;
  client_alias: string | null;
  redirect_uris: string[];
  scope: string | null;
  approved: boolean;
  logo_uri?: string | null;
  client_uri?: string | null;
  software_id?: string | null;
  software_version?: string | null;
  metadata_url?: string | null;
  metadata_cached_at?: string | null;
  metadata_cache_ttl?: number | null;
  last_seen: string | null;
  created_at: string;
  reports_roots: boolean;
  roots_capability_known: boolean;
}

/** Editable OAuth client fields. */
export interface UpdateClientRequest {
  client_alias?: string;
}

/** Full consent request details returned by the backend. */
export interface ConsentRequestDetails {
  requestId: string;
  clientId: string;
  clientName: string;
  redirectUri: string;
  scope: string;
  state: string | null;
  expiresAt: number;
  consentToken: string;
}

/** Consent validation or approval error from the backend. */
export interface ConsentError {
  code: 'NOT_FOUND' | 'EXPIRED' | 'ALREADY_PROCESSED' | 'GATEWAY_UNAVAILABLE';
  message: string;
}

/** Payload sent when approving or denying OAuth consent. */
export interface ConsentApprovalRequest {
  request_id: string;
  approved: boolean;
  consent_token: string;
  client_alias: string | null;
}

/** Response from consent approval. */
export interface ConsentApprovalResponse {
  success: boolean;
  redirect_url: string;
  error: string | null;
}

/**
 * Validate a pending OAuth consent request and load authoritative details.
 */
export async function getPendingConsent(requestId: string): Promise<ConsentRequestDetails> {
  return invoke('get_pending_consent', { requestId });
}

/**
 * Approve or deny a pending OAuth consent request.
 */
export async function approveOAuthConsent(
  request: ConsentApprovalRequest
): Promise<ConsentApprovalResponse> {
  return invoke('approve_oauth_consent', { request });
}

/**
 * Flush a cold-start deep link buffered on the Rust side after the consent
 * listener is subscribed.
 */
export async function flushPendingDeepLink(): Promise<void> {
  return invoke('flush_pending_deep_link');
}

/**
 * List all registered OAuth clients.
 */
export async function listOAuthClients(): Promise<OAuthClient[]> {
  return apiCall('get_oauth_clients');
}

/**
 * Update an OAuth client's settings.
 */
export async function updateOAuthClient(
  clientId: string,
  settings: UpdateClientRequest
): Promise<OAuthClient> {
  return invoke('update_oauth_client', { clientId, settings });
}

/**
 * Delete an OAuth client registration.
 */
export async function deleteOAuthClient(clientId: string): Promise<void> {
  return invoke('delete_oauth_client', { clientId });
}

/**
 * Read FeatureSet ids granted to a rootless OAuth client in a space.
 */
export async function getOAuthClientGrants(
  clientId: string,
  spaceId: string
): Promise<string[]> {
  return apiCall('get_oauth_client_grants', { clientId, spaceId });
}

/**
 * Grant a FeatureSet to an OAuth client in a space.
 */
export async function grantOAuthClientFeatureSet(
  clientId: string,
  spaceId: string,
  featureSetId: string
): Promise<void> {
  return invoke('grant_oauth_client_feature_set', {
    clientId,
    spaceId,
    featureSetId,
  });
}

/**
 * Revoke a FeatureSet from an OAuth client in a space.
 */
export async function revokeOAuthClientFeatureSet(
  clientId: string,
  spaceId: string,
  featureSetId: string
): Promise<void> {
  return invoke('revoke_oauth_client_feature_set', {
    clientId,
    spaceId,
    featureSetId,
  });
}
