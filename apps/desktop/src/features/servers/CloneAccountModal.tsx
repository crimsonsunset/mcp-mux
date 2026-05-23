/**
 * CloneAccountModal — wizard for adding another account of an installed MCP server.
 */

import { useCallback, useEffect, useState } from 'react';
import { Copy, Loader2, X } from 'lucide-react';
import type { ServerViewModel } from '@/types/registry';
import {
  CLONE_SUFFIX_SUGGESTIONS,
  cloneServer,
  deriveCloneAlias,
  deriveCloneServerId,
  isCloneIdAvailable,
  suggestCloneSuffix,
  type ClonedInstalledServer,
} from '@/lib/api/serverClone';

export interface CloneAccountModalProps {
  open: boolean;
  spaceId: string;
  sourceServer: ServerViewModel;
  onClose: () => void;
  /** Called after a successful clone with the new install row. */
  onCloned: (cloned: ClonedInstalledServer) => void;
}

/**
 * Modal for creating a suffixed clone of an installed server in the same space.
 */
export function CloneAccountModal({
  open,
  spaceId,
  sourceServer,
  onClose,
  onCloned,
}: CloneAccountModalProps) {
  const [suffix, setSuffix] = useState('');
  const [isChecking, setIsChecking] = useState(false);
  const [isAvailable, setIsAvailable] = useState<boolean | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [submitError, setSubmitError] = useState<string | null>(null);
  const [isLoadingSuggestion, setIsLoadingSuggestion] = useState(false);

  const previewId = deriveCloneServerId(sourceServer.id, suffix);
  const previewAlias = deriveCloneAlias(suffix);
  const hasSuffix = suffix.trim().length > 0;
  const hasCollision = hasSuffix && isAvailable === false;

  /**
   * Load the first available suggested suffix when the modal opens.
   */
  useEffect(() => {
    if (!open) {
      return;
    }

    let cancelled = false;

    const loadSuggestion = async () => {
      setIsLoadingSuggestion(true);
      setSubmitError(null);
      try {
        const suggested = await suggestCloneSuffix(spaceId, sourceServer.id);
        if (!cancelled) {
          setSuffix(suggested);
        }
      } catch (e) {
        if (!cancelled) {
          setSuffix(CLONE_SUFFIX_SUGGESTIONS[0]);
          setSubmitError(String(e));
        }
      } finally {
        if (!cancelled) {
          setIsLoadingSuggestion(false);
        }
      }
    };

    loadSuggestion();

    return () => {
      cancelled = true;
    };
  }, [open, spaceId, sourceServer.id]);

  /**
   * Debounced collision check against the backend.
   */
  useEffect(() => {
    if (!open || !hasSuffix) {
      setIsAvailable(null);
      setIsChecking(false);
      return;
    }

    let cancelled = false;
    setIsChecking(true);

    const timer = setTimeout(async () => {
      try {
        const available = await isCloneIdAvailable(spaceId, sourceServer.id, suffix);
        if (!cancelled) {
          setIsAvailable(available);
        }
      } catch {
        if (!cancelled) {
          setIsAvailable(null);
        }
      } finally {
        if (!cancelled) {
          setIsChecking(false);
        }
      }
    }, 300);

    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [open, spaceId, sourceServer.id, suffix, hasSuffix]);

  /**
   * Submit the clone request.
   */
  const handleSubmit = useCallback(async () => {
    if (!hasSuffix || hasCollision || isChecking) {
      return;
    }

    setIsSubmitting(true);
    setSubmitError(null);

    try {
      const cloned = await cloneServer(spaceId, sourceServer.id, suffix);
      onCloned(cloned);
      onClose();
    } catch (e) {
      setSubmitError(String(e));
    } finally {
      setIsSubmitting(false);
    }
  }, [hasSuffix, hasCollision, isChecking, spaceId, sourceServer.id, suffix, onCloned, onClose]);

  if (!open) {
    return null;
  }

  const canSubmit = hasSuffix && !hasCollision && !isChecking && !isSubmitting && !isLoadingSuggestion;

  return (
    <div
      className="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50 p-4"
      data-testid="clone-account-modal-overlay"
    >
      <div
        className="dropdown-menu w-full max-w-md p-6 animate-in fade-in scale-in duration-150"
        data-testid="clone-account-modal"
      >
        <div className="flex items-start justify-between gap-3 mb-4">
          <div className="flex items-center gap-3">
            <div className="p-2 rounded-lg bg-[rgb(var(--primary))]/10">
              <Copy className="h-5 w-5 text-[rgb(var(--primary))]" />
            </div>
            <div>
              <h3 className="text-lg font-semibold text-[rgb(var(--foreground))]">
                Add another account
              </h3>
              <p className="text-sm text-[rgb(var(--muted))]">
                Clone {sourceServer.name} with a separate credential set
              </p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-1 rounded hover:bg-[rgb(var(--surface-hover))] text-[rgb(var(--muted))] transition-colors"
            aria-label="Close"
            data-testid="clone-account-close-btn"
          >
            <X className="h-5 w-5" />
          </button>
        </div>

        <div className="space-y-4">
          <div>
            <label
              htmlFor="clone-suffix"
              className="block text-sm font-medium text-[rgb(var(--foreground))] mb-1"
            >
              Account label
            </label>
            <p className="text-xs text-[rgb(var(--muted))] mb-2">
              Used in the server ID and tool prefix (e.g. work, personal)
            </p>
            <input
              id="clone-suffix"
              type="text"
              value={suffix}
              onChange={(e) => setSuffix(e.target.value)}
              placeholder="work"
              className={`input w-full ${hasCollision ? 'border-[rgb(var(--error))]' : ''}`}
              disabled={isLoadingSuggestion || isSubmitting}
              data-testid="clone-suffix-input"
            />
            {hasCollision && (
              <p className="text-xs text-[rgb(var(--error))] mt-1" data-testid="clone-collision-error">
                An account with this label already exists in this space
              </p>
            )}
          </div>

          <div>
            <p className="text-xs font-medium text-[rgb(var(--muted))] mb-2">Suggestions</p>
            <div className="flex flex-wrap gap-2">
              {CLONE_SUFFIX_SUGGESTIONS.map((suggestion) => (
                <button
                  key={suggestion}
                  type="button"
                  onClick={() => setSuffix(suggestion)}
                  className={`px-2.5 py-1 text-xs rounded-md border transition-colors ${
                    suffix === suggestion
                      ? 'border-[rgb(var(--primary))] bg-[rgb(var(--primary))]/10 text-[rgb(var(--primary))]'
                      : 'border-[rgb(var(--border))] text-[rgb(var(--muted))] hover:bg-[rgb(var(--surface-hover))]'
                  }`}
                  data-testid={`clone-suffix-suggestion-${suggestion}`}
                >
                  {suggestion}
                </button>
              ))}
            </div>
          </div>

          {hasSuffix && (
            <div className="rounded-lg border border-[rgb(var(--border-subtle))] bg-[rgb(var(--surface-dim))] p-3 space-y-2">
              <div className="flex items-center justify-between gap-2 text-sm">
                <span className="text-[rgb(var(--muted))]">Server ID</span>
                <code className="text-xs font-mono text-[rgb(var(--foreground))]">{previewId || '—'}</code>
              </div>
              <div className="flex items-center justify-between gap-2 text-sm">
                <span className="text-[rgb(var(--muted))]">Tool prefix</span>
                <code className="text-xs font-mono text-[rgb(var(--foreground))]">
                  {previewAlias ? `${previewAlias}_*` : '—'}
                </code>
              </div>
              {isChecking && (
                <div className="flex items-center gap-2 text-xs text-[rgb(var(--muted))]">
                  <Loader2 className="h-3 w-3 animate-spin" />
                  Checking availability…
                </div>
              )}
            </div>
          )}

          <p className="text-xs text-[rgb(var(--muted))]">
            The clone copies the server definition but not credentials. You will configure this
            account before enabling it.
          </p>

          {submitError && (
            <p className="text-sm text-[rgb(var(--error))]" data-testid="clone-submit-error">
              {submitError}
            </p>
          )}

          <div className="flex justify-end gap-2 pt-2">
            <button
              onClick={onClose}
              className="px-4 py-2 text-sm rounded-lg border border-[rgb(var(--border))] text-[rgb(var(--muted))] hover:bg-[rgb(var(--surface-hover))] transition-colors"
              disabled={isSubmitting}
              data-testid="clone-cancel-btn"
            >
              Cancel
            </button>
            <button
              onClick={handleSubmit}
              disabled={!canSubmit}
              className="px-4 py-2 text-sm rounded-lg bg-[rgb(var(--primary))] text-[rgb(var(--primary-foreground))] hover:bg-[rgb(var(--primary-hover))] disabled:opacity-50 transition-colors flex items-center gap-2"
              data-testid="clone-submit-btn"
            >
              {isSubmitting && <Loader2 className="h-4 w-4 animate-spin" />}
              {isSubmitting ? 'Creating…' : 'Create account'}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
