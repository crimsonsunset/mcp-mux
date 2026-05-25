/**
 * Shared server icon component that handles both URL-based and emoji icons.
 *
 * Server definitions may have an `icon` field that is either:
 * - An HTTP(S) URL to an image (e.g., GitHub avatar)
 * - An emoji string (e.g., "📦")
 * - null/undefined
 */

import { convertFileSrc } from '@tauri-apps/api/core';
import { useEffect, useMemo, useState } from 'react';
import { resolveWorkspaceIconPath } from '@/lib/api/workspaceAppearances';

interface ServerIconProps {
  icon: string | null | undefined;
  /** CSS classes for the img element when rendering a URL icon */
  className?: string;
  /** Fallback emoji when icon is missing or fails to load (default: '📦') */
  fallback?: string;
}

export function ServerIcon({ icon, className = 'w-9 h-9 object-contain', fallback = '📦' }: ServerIconProps) {
  const [failed, setFailed] = useState(false);
  const [resolvedSrc, setResolvedSrc] = useState<string | null>(null);
  const isLocalRef = icon?.startsWith('local:') ?? false;
  const isRemoteUrl = icon?.startsWith('http') ?? false;

  useEffect(() => {
    let cancelled = false;
    setFailed(false);
    if (!icon) {
      setResolvedSrc(null);
      return () => {
        cancelled = true;
      };
    }
    if (isRemoteUrl) {
      setResolvedSrc(icon);
      return () => {
        cancelled = true;
      };
    }
    if (!isLocalRef) {
      setResolvedSrc(null);
      return () => {
        cancelled = true;
      };
    }

    setResolvedSrc(null);
    void resolveWorkspaceIconPath(icon)
      .then((absolutePath) => {
        if (cancelled) return;
        setResolvedSrc(absolutePath ? convertFileSrc(absolutePath) : null);
      })
      .catch(() => {
        if (cancelled) return;
        setFailed(true);
      });
    return () => {
      cancelled = true;
    };
  }, [icon, isLocalRef, isRemoteUrl]);

  const shouldRenderImage = useMemo(
    () => isRemoteUrl || isLocalRef,
    [isLocalRef, isRemoteUrl]
  );

  if (!icon || failed) {
    return <span data-testid="server-icon-fallback">{fallback}</span>;
  }

  if (shouldRenderImage) {
    if (!resolvedSrc) {
      return <span data-testid="server-icon-fallback">{fallback}</span>;
    }
    return (
      <img
        src={resolvedSrc}
        alt=""
        className={className}
        data-testid="server-icon-img"
        onError={() => setFailed(true)}
      />
    );
  }

  return <span data-testid="server-icon-emoji">{icon}</span>;
}
