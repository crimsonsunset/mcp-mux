/**
 * Shared Cursor `preToolUse` hook installer.
 *
 * Used by the register-client result screen and the Connections side panel.
 * Writes `~/.cursor/hooks.json` on the machine running the gateway.
 */

import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Loader2 } from 'lucide-react';
import { Button } from '@mcpmux/ui';
import {
  getCursorHookStatus,
  installCursorHook,
  uninstallCursorHook,
  type CursorHookResult,
} from '@/lib/api/cursorHooks';

/**
 * Build a hook-result payload for a thrown installer error.
 */
function errorResult(error: string): CursorHookResult {
  return {
    action: 'error',
    installed: false,
    hooks_path: '',
    script_path: '',
    backed_up: null,
    error,
    jsonc_refused: false,
    manual_entry: '',
  };
}

interface CursorHookInstallSectionProps {
  /** When true, wrap in the bordered box used by the register-client modal. */
  boxed?: boolean;
}

/**
 * Status, install/uninstall, and JSONC manual-copy controls for the managed hook.
 */
export function CursorHookInstallSection({ boxed = false }: CursorHookInstallSectionProps) {
  const { t } = useTranslation('clients');
  const [hookStatus, setHookStatus] = useState<CursorHookResult | null>(null);
  const [hookBusy, setHookBusy] = useState(false);
  const [hookManualCopied, setHookManualCopied] = useState(false);

  useEffect(() => {
    void (async () => {
      try {
        setHookStatus(await getCursorHookStatus());
      } catch (e) {
        setHookStatus(errorResult(e instanceof Error ? e.message : String(e)));
      }
    })();
  }, []);

  /**
   * Install or update the managed Cursor `preToolUse` hook.
   */
  const handleInstallHook = async () => {
    setHookBusy(true);
    try {
      setHookStatus(await installCursorHook());
    } catch (e) {
      setHookStatus(errorResult(e instanceof Error ? e.message : String(e)));
    } finally {
      setHookBusy(false);
    }
  };

  /**
   * Remove the managed hook entry and script.
   */
  const handleUninstallHook = async () => {
    setHookBusy(true);
    try {
      setHookStatus(await uninstallCursorHook());
    } catch (e) {
      setHookStatus(errorResult(e instanceof Error ? e.message : String(e)));
    } finally {
      setHookBusy(false);
    }
  };

  /**
   * Copy the manual hooks.json fallback snippet.
   */
  const handleCopyHookManual = async () => {
    const text = hookStatus?.manual_entry;
    if (!text) return;
    try {
      await navigator.clipboard.writeText(text);
      setHookManualCopied(true);
      setTimeout(() => setHookManualCopied(false), 2000);
    } catch {
      // Snippet is selectable as a fallback.
    }
  };

  const body = (
    <>
      <p className="text-xs text-[rgb(var(--muted))]">{t('cursorBridge.hookHint')}</p>
      {hookStatus?.installed ? (
        <p
          className="text-sm text-emerald-700 dark:text-emerald-300"
          data-testid="cursor-hook-installed"
        >
          {t('cursorBridge.hookInstalled')}
        </p>
      ) : null}
      {hookStatus?.backed_up ? (
        <p className="text-xs text-[rgb(var(--muted))]">
          {t('cursorBridge.hookBackup', { path: hookStatus.backed_up })}
        </p>
      ) : null}
      {hookStatus?.error ? (
        <p className="text-sm text-red-600 dark:text-red-400" data-testid="cursor-hook-error">
          {hookStatus.jsonc_refused ? t('cursorBridge.hookJsoncRefused') : hookStatus.error}
        </p>
      ) : null}
      <div className="flex flex-wrap gap-2">
        <Button
          variant="secondary"
          size="sm"
          onClick={() => void handleInstallHook()}
          disabled={hookBusy}
          data-testid="cursor-hook-install"
        >
          {hookBusy ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : null}
          {hookBusy ? t('cursorBridge.hookInstalling') : t('cursorBridge.hookInstall')}
        </Button>
        {hookStatus?.installed ? (
          <Button
            variant="secondary"
            size="sm"
            onClick={() => void handleUninstallHook()}
            disabled={hookBusy}
            data-testid="cursor-hook-uninstall"
          >
            {t('cursorBridge.hookUninstall')}
          </Button>
        ) : null}
        {hookStatus?.manual_entry ? (
          <Button
            variant="secondary"
            size="sm"
            onClick={() => void handleCopyHookManual()}
            data-testid="cursor-hook-copy-manual"
          >
            {hookManualCopied ? t('cursorBridge.copied') : t('cursorBridge.hookCopyManual')}
          </Button>
        ) : null}
      </div>
      {hookStatus?.jsonc_refused && hookStatus.manual_entry ? (
        <div>
          <p className="mb-1 text-xs font-medium uppercase tracking-wide text-[rgb(var(--muted))]">
            {t('cursorBridge.hookManualLabel')}
          </p>
          <pre
            data-testid="cursor-hook-manual"
            className="max-h-40 overflow-auto rounded-lg border border-[rgb(var(--border))] bg-[rgb(var(--surface))] p-3 font-mono text-xs"
          >
            {hookStatus.manual_entry}
          </pre>
        </div>
      ) : null}
    </>
  );

  if (boxed) {
    return (
      <div className="space-y-2 rounded-xl border border-[rgb(var(--border))] p-3.5">{body}</div>
    );
  }
  return <div className="space-y-2">{body}</div>;
}
