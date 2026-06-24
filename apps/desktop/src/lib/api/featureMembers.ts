/**
 * Feature Members API
 *
 * For managing individual features (tools/prompts/resources) in feature sets
 */

/** @deprecated Prefer `@/lib/backend` — shim during facade migration. */
import { apiCall } from './transport';

export interface FeatureSetMember {
  id: string;
  feature_set_id: string;
  member_type: 'feature' | 'feature_set';
  member_id: string;
  mode: 'include' | 'exclude';
}

/** Add an individual feature to a feature set. */
export async function addFeatureToSet(
  featureSetId: string,
  featureId: string,
  mode: 'include' | 'exclude' = 'include'
): Promise<void> {
  return apiCall('add_feature_to_set', {
    featureSetId,
    featureId,
    mode,
  });
}

/** Remove an individual feature from a feature set. */
export async function removeFeatureFromSet(
  featureSetId: string,
  featureId: string
): Promise<void> {
  return apiCall('remove_feature_from_set', {
    featureSetId,
    featureId,
  });
}

/** Get all individual feature members of a feature set. */
export async function getFeatureSetMembers(
  featureSetId: string
): Promise<FeatureSetMember[]> {
  return apiCall('get_feature_set_members', {
    featureSetId,
  });
}
