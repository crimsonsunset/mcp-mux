import { useEffect } from 'react';
import { useAppStore } from '@/stores/appStore';
import {
  isTauri,
  listSpaces,
  refreshOAuthTokensOnStartup,
  waitForAdminReady,
} from '@/lib/backend';

let startupSyncPromise: Promise<void> | null = null;

/**
 * Syncs data from Rust backend to Zustand store.
 * Run once at app startup.
 */
export function useDataSync() {
  const setSpaces = useAppStore((state) => state.setSpaces);
  const setLoading = useAppStore((state) => state.setLoading);

  useEffect(() => {
    /**
     * Load spaces and kick off a non-blocking OAuth refresh.
     */
    async function syncData(): Promise<void> {
      const transport = isTauri() ? 'tauri' : 'admin-http';
      console.log(`[useDataSync] Starting data sync (transport=${transport})...`);
      setLoading('spaces', true);
      try {
        if (!isTauri()) {
          console.log('[useDataSync] Waiting for admin API...');
          await waitForAdminReady();
        }

        console.log('[useDataSync] Calling listSpaces...');
        const spaces = await listSpaces();
        console.log('[useDataSync] listSpaces returned:', spaces.length, 'spaces');
        setSpaces(spaces);

        try {
          const refreshResult = await refreshOAuthTokensOnStartup();
          console.log('[useDataSync] OAuth token refresh result:', refreshResult);
        } catch (error) {
          console.warn('[useDataSync] OAuth token refresh failed (non-fatal):', error);
        }
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        console.error('[useDataSync] Failed to sync:', message, error);
        if (!isTauri()) {
          console.error(
            '[useDataSync] Web admin hint: ensure the admin server is running on :45819 (or wait for Tauri hot-reload to finish).'
          );
        }
      } finally {
        setLoading('spaces', false);
        console.log('[useDataSync] Data sync complete');
      }
    }

    if (!startupSyncPromise) {
      startupSyncPromise = syncData();
    }
    void startupSyncPromise;
  }, [setSpaces, setLoading]);
}
