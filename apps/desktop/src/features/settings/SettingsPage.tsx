import { useState, useEffect } from 'react';
import {
  Card,
  CardHeader,
  CardTitle,
  CardDescription,
  CardContent,
  Button,
  Switch,
  useToast,
  ToastContainer,
} from '@mcpmux/ui';
import {
  Sun,
  Moon,
  Monitor,
  FileText,
  FolderOpen,
  Loader2,
  Power,
  Minimize2,
  XCircle,
  Trash2,
  BarChart3,
  Sparkles,
  Github,
  Bug,
  Lightbulb,
  Package,
  Heart,
  Network,
  RotateCcw,
  AlertCircle,
  Globe,
} from 'lucide-react';
import { useAppStore, useTheme, useAnalyticsEnabled } from '@/stores';
import { UpdateChecker } from './UpdateChecker';
import { getMetaToolsEnabled, setMetaToolsEnabled } from '@/lib/api/metaTools';
import {
  getAdminWebSettings,
  getGatewayPortSettings,
  getLogsPath,
  getStartupSettings,
  openLogsFolder,
  resetGatewayPort,
  setGatewayPort,
  setGatewayPublicUrl,
  updateAdminWebSettings,
  updateStartupSettings,
  type AdminWebSettings,
  type GatewayPortSettings,
  type StartupSettings,
} from '@/lib/api/settings';
import { getLogRetentionDays, setLogRetentionDays as saveLogRetentionDays } from '@/lib/api/logs';
import { MetaToolAuditLog, MetaToolGrantsPanel } from '@/features/metaTools';
import { useGatewayControl } from '@/features/gateway/useGatewayControl';
import { CONTRIBUTE, openExternal } from '@/lib/contribute';
import { isTauri } from '@/lib/api/transport';

export function SettingsPage() {
  const theme = useTheme();
  const setTheme = useAppStore((state) => state.setTheme);
  const analyticsEnabled = useAnalyticsEnabled();
  const setAnalyticsEnabled = useAppStore((state) => state.setAnalyticsEnabled);
  const [logsPath, setLogsPath] = useState<string>('');
  const [openingLogs, setOpeningLogs] = useState(false);
  const { toasts, success, error } = useToast();
  const gatewayControl = useGatewayControl();

  // Startup settings state
  const [startupSettings, setStartupSettings] = useState<StartupSettings>({
    autoLaunch: false,
    startMinimized: false,
    closeToTray: true,
  });
  const [loadingSettings, setLoadingSettings] = useState(true);
  const [savingSettings, setSavingSettings] = useState(false);

  // Log retention state
  const [logRetentionDays, setLogRetentionDays] = useState<number>(30);
  const [savingRetention, setSavingRetention] = useState(false);

  // Meta-tools master switch — gates the entire `mcpmux_*` namespace.
  const [metaToolsEnabled, setMetaToolsEnabledState] = useState<boolean>(true);
  const [loadingMetaTools, setLoadingMetaTools] = useState(true);

  // Gateway port — persisted user override, the default the app ships
  // with, and the port the currently-running gateway is bound to. When
  // saved ≠ active, the user has to restart the gateway to apply.
  const [portSettings, setPortSettings] = useState<GatewayPortSettings | null>(null);
  const [portDraft, setPortDraft] = useState<string>('');
  const [publicUrlDraft, setPublicUrlDraft] = useState<string>('');
  const [portError, setPortError] = useState<string | null>(null);
  const [publicUrlError, setPublicUrlError] = useState<string | null>(null);
  const [savingPort, setSavingPort] = useState(false);
  const [savingPublicUrl, setSavingPublicUrl] = useState(false);
  const [resettingPort, setResettingPort] = useState(false);

  const [adminWeb, setAdminWeb] = useState<AdminWebSettings | null>(null);
  const [adminPortDraft, setAdminPortDraft] = useState('45819');
  const [adminCfDomainDraft, setAdminCfDomainDraft] = useState('');
  const [loadingAdminWeb, setLoadingAdminWeb] = useState(true);
  const [savingAdminWeb, setSavingAdminWeb] = useState(false);

  const loadPortSettings = async () => {
    try {
      const s = await getGatewayPortSettings();
      setPortSettings(s);
      setPortDraft(String(s.configuredPort ?? s.defaultPort));
      setPublicUrlDraft(s.publicUrl ?? '');
      setPortError(null);
      setPublicUrlError(null);
    } catch (err) {
      console.error('Failed to load gateway port settings:', err);
    }
  };

  useEffect(() => {
    loadPortSettings();
  }, []);

  const loadAdminWebSettings = async () => {
    try {
      const s = await getAdminWebSettings();
      setAdminWeb(s);
      setAdminPortDraft(String(s.port));
      setAdminCfDomainDraft(s.cfTeamDomain);
    } catch (err) {
      console.error('Failed to load web admin settings:', err);
    } finally {
      setLoadingAdminWeb(false);
    }
  };

  useEffect(() => {
    if (!isTauri()) {
      setLoadingAdminWeb(false);
      return;
    }
    loadAdminWebSettings();
  }, []);

  const persistAdminWeb = async (next: AdminWebSettings) => {
    setSavingAdminWeb(true);
    try {
      await updateAdminWebSettings(next);
      setAdminWeb(next);
      setAdminPortDraft(String(next.port));
      setAdminCfDomainDraft(next.cfTeamDomain);
      success(
        'Web admin updated',
        next.enabled
          ? `Browse http://127.0.0.1:${next.port}. Run pnpm build:web:admin after UI changes, then hard-refresh.`
          : 'Web admin server stopped.'
      );
    } catch (err) {
      error('Failed to save web admin settings', String(err));
    } finally {
      setSavingAdminWeb(false);
    }
  };

  const handleSaveAdminPort = async () => {
    if (!adminWeb) return;
    const parsed = validatePort(adminPortDraft);
    if ('error' in parsed) {
      error('Invalid admin port', parsed.error);
      return;
    }
    await persistAdminWeb({ ...adminWeb, port: parsed.port });
  };

  const validatePort = (raw: string): { port: number } | { error: string } => {
    const trimmed = raw.trim();
    if (!trimmed) return { error: 'Enter a port number' };
    if (!/^\d+$/.test(trimmed)) return { error: 'Port must be a number' };
    const n = Number(trimmed);
    if (n < 1024 || n > 65535) {
      return { error: 'Port must be between 1024 and 65535' };
    }
    return { port: n };
  };

  const handleSavePort = async () => {
    const parsed = validatePort(portDraft);
    if ('error' in parsed) {
      setPortError(parsed.error);
      return;
    }
    setPortError(null);
    setSavingPort(true);
    try {
      await setGatewayPort(parsed.port);
      await loadPortSettings();
      success(
        'Gateway port saved',
        portSettings?.activePort && portSettings.activePort !== parsed.port
          ? `Restart the gateway for port ${parsed.port} to take effect.`
          : `Next gateway start will use port ${parsed.port}.`
      );
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      setPortError(msg);
      error('Failed to save port', msg);
    } finally {
      setSavingPort(false);
    }
  };

  const handleSavePublicUrl = async () => {
    setSavingPublicUrl(true);
    try {
      await setGatewayPublicUrl(publicUrlDraft);
      await loadPortSettings();
      setPublicUrlError(null);
      success(
        'Public gateway URL saved',
        'Remote clients via Cloudflare Tunnel will use this URL for OAuth discovery.'
      );
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      setPublicUrlError(msg);
      error('Failed to save public URL', msg);
    } finally {
      setSavingPublicUrl(false);
    }
  };

  const handleResetPort = async () => {
    setResettingPort(true);
    try {
      await resetGatewayPort();
      await loadPortSettings();
      success(
        'Reset to default',
        portSettings && portSettings.activePort !== portSettings.defaultPort
          ? `Restart the gateway for port ${portSettings.defaultPort} to take effect.`
          : `Next gateway start will use port ${portSettings?.defaultPort ?? ''}.`
      );
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      error('Failed to reset port', msg);
    } finally {
      setResettingPort(false);
    }
  };

  const handleRestartGateway = async () => {
    try {
      const outcome = await gatewayControl.restart();
      await loadPortSettings();
      if (outcome.status === 'cancelled') return;
      success(
        'Gateway restarted',
        outcome.fellBackToDynamic
          ? `Saved port was unavailable — now running on :${outcome.port} instead.`
          : 'The new port is now active.'
      );
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      error('Failed to restart gateway', msg);
    }
  };

  useEffect(() => {
    getMetaToolsEnabled()
      .then((v) => setMetaToolsEnabledState(v))
      .catch((e) => console.error('Failed to load meta_tools_enabled', e))
      .finally(() => setLoadingMetaTools(false));
  }, []);

  const handleToggleMetaTools = async (next: boolean) => {
    const previous = metaToolsEnabled;
    setMetaToolsEnabledState(next);
    try {
      await setMetaToolsEnabled(next);
      success(
        next ? 'Self-management tools enabled' : 'Self-management tools disabled',
        next
          ? 'Connected MCP clients will see the mcpmux_* toolset on next list_tools.'
          : 'mcpmux_* is hidden from connected MCP clients.'
      );
    } catch (e) {
      setMetaToolsEnabledState(previous);
      error('Failed to save setting', e instanceof Error ? e.message : String(e));
    }
  };

  // Load logs path on mount
  useEffect(() => {
    const loadLogsPath = async () => {
      try {
        const path = await getLogsPath();
        setLogsPath(path);
      } catch (error) {
        console.error('Failed to get logs path:', error);
      }
    };
    loadLogsPath();
  }, []);

  // Load log retention setting on mount
  useEffect(() => {
    const loadRetention = async () => {
      try {
        const days = await getLogRetentionDays();
        setLogRetentionDays(days);
      } catch (err) {
        console.error('Failed to load log retention setting:', err);
      }
    };
    loadRetention();
  }, []);

  // Load startup settings on mount
  useEffect(() => {
    const loadStartupSettings = async () => {
      try {
        const settings = await getStartupSettings();
        setStartupSettings(settings);
      } catch (error) {
        console.error('Failed to load startup settings:', error);
      } finally {
        setLoadingSettings(false);
      }
    };
    loadStartupSettings();
  }, []);

  // Save startup settings when they change
  const updateStartupSetting = async (
    key: keyof StartupSettings,
    value: boolean
  ) => {
    console.log(`[Settings] Updating ${key} to ${value}`);
    
    // Save old state for rollback
    const oldSettings = { ...startupSettings };
    const newSettings = { ...startupSettings, [key]: value };
    
    // Update UI immediately for better UX
    setStartupSettings(newSettings);
    setSavingSettings(true);
    
    try {
      console.log('[Settings] Invoking update_startup_settings:', newSettings);
      await updateStartupSettings(newSettings);
      console.log('[Settings] Successfully saved:', newSettings);
      
      // Show success toast
      success('Settings saved', 'Your preferences have been updated');
    } catch (err) {
      console.error('[Settings] Failed to save:', err);
      // Show error toast
      const errorMessage = err instanceof Error ? err.message : 'Unknown error';
      error('Failed to save settings', errorMessage);
      // Revert on error
      setStartupSettings(oldSettings);
    } finally {
      setSavingSettings(false);
    }
  };

  const handleRetentionChange = async (days: number) => {
    const oldDays = logRetentionDays;
    setLogRetentionDays(days);
    setSavingRetention(true);
    try {
      await saveLogRetentionDays(days);
      success('Settings saved', `Log retention set to ${days === 0 ? 'keep forever' : `${days} days`}`);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Unknown error';
      error('Failed to save setting', errorMessage);
      setLogRetentionDays(oldDays);
    } finally {
      setSavingRetention(false);
    }
  };

  const handleOpenLogs = async () => {
    setOpeningLogs(true);
    try {
      await openLogsFolder();
    } catch (error) {
      console.error('Failed to open logs folder:', error);
    } finally {
      setOpeningLogs(false);
    }
  };

  return (
    <>
      <ToastContainer toasts={toasts} onClose={(id) => toasts.find(t => t.id === id)?.onClose(id)} />
      {gatewayControl.ConfirmDialogElement}
      <div className="space-y-6">
        <div>
          <h1 className="text-2xl font-bold">Settings</h1>
          <p className="text-[rgb(var(--muted))]">Configure McpMux preferences.</p>
        </div>

      {/* Updates Section — desktop shell only */}
      {isTauri() ? <UpdateChecker /> : null}

      {/* Startup & System Tray Section - always show toggles so e2e and slow backends see the section */}
      <Card data-testid="settings-startup-section">
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Power className="h-5 w-5" />
            Startup & System Tray
          </CardTitle>
          <CardDescription>
            Control how McpMux starts and behaves with the system tray.
          </CardDescription>
        </CardHeader>
        <CardContent>
          {loadingSettings ? (
            <div className="flex items-center gap-2 text-sm text-[rgb(var(--muted))] mb-4">
              <Loader2 className="h-4 w-4 animate-spin" />
              Loading…
            </div>
          ) : null}
          <div className="space-y-6">
              <div className="flex items-center justify-between gap-4">
                <div className="flex items-start gap-3 flex-1 min-w-0">
                  <Power className="h-5 w-5 mt-0.5 text-[rgb(var(--muted))] flex-shrink-0" />
                  <div>
                    <label className="text-sm font-medium">Launch at Startup</label>
                    <p className="text-xs text-[rgb(var(--muted))] mt-1">
                      Start McpMux automatically when you log in to your system
                    </p>
                  </div>
                </div>
                <Switch
                  checked={startupSettings.autoLaunch}
                  onCheckedChange={(checked) => {
                    console.log('Auto-launch toggled:', checked);
                    updateStartupSetting('autoLaunch', checked);
                  }}
                  disabled={savingSettings}
                  data-testid="auto-launch-switch"
                />
              </div>

              <div className="flex items-center justify-between gap-4">
                <div className="flex items-start gap-3 flex-1 min-w-0">
                  <Minimize2 className="h-5 w-5 mt-0.5 text-[rgb(var(--muted))] flex-shrink-0" />
                  <div>
                    <label className="text-sm font-medium">Start Minimized</label>
                    <p className="text-xs text-[rgb(var(--muted))] mt-1">
                      Launch in background to system tray (requires auto-launch enabled)
                    </p>
                  </div>
                </div>
                <Switch
                  checked={startupSettings.startMinimized}
                  onCheckedChange={(checked) => {
                    console.log('Start minimized toggled:', checked);
                    updateStartupSetting('startMinimized', checked);
                  }}
                  disabled={savingSettings || !startupSettings.autoLaunch}
                  data-testid="start-minimized-switch"
                />
              </div>

              <div className="flex items-center justify-between gap-4">
                <div className="flex items-start gap-3 flex-1 min-w-0">
                  <XCircle className="h-5 w-5 mt-0.5 text-[rgb(var(--muted))] flex-shrink-0" />
                  <div>
                    <label className="text-sm font-medium">Close to Tray</label>
                    <p className="text-xs text-[rgb(var(--muted))] mt-1">
                      Keep running in system tray when window is closed (use "Quit" from tray to exit)
                    </p>
                  </div>
                </div>
                <Switch
                  checked={startupSettings.closeToTray}
                  onCheckedChange={(checked) => {
                    console.log('Close to tray toggled:', checked);
                    updateStartupSetting('closeToTray', checked);
                  }}
                  disabled={savingSettings}
                  data-testid="close-to-tray-switch"
                />
              </div>

              {savingSettings && (
                <div className="flex items-center gap-2 text-sm text-[rgb(var(--muted))]">
                  <Loader2 className="h-4 w-4 animate-spin" />
                  Saving settings...
                </div>
              )}
          </div>
        </CardContent>
      </Card>

      {/* Gateway Section — port override + reset to default */}
      <Card data-testid="settings-gateway-section">
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Network className="h-5 w-5" />
            Gateway
          </CardTitle>
          <CardDescription>
            The local port every AI client connects to. Changing it takes effect on the next
            gateway start — existing IDE configs pointing at the old port will need updating.
          </CardDescription>
        </CardHeader>
        <CardContent>
          {portSettings === null ? (
            <div className="flex items-center gap-2 text-sm text-[rgb(var(--muted))]">
              <Loader2 className="h-4 w-4 animate-spin" />
              Loading…
            </div>
          ) : (
            <div className="space-y-4">
              <div className="flex items-start gap-3">
                <Network className="h-5 w-5 mt-0.5 text-[rgb(var(--muted))] flex-shrink-0" />
                <div className="flex-1 min-w-0">
                  <label
                    htmlFor="gateway-port-input"
                    className="text-sm font-medium"
                  >
                    Gateway port
                  </label>
                  <p className="text-xs text-[rgb(var(--muted))] mt-1">
                    Default is <span className="font-mono">{portSettings.defaultPort}</span>.
                    Use a port between 1024 and 65535.
                    {portSettings.activePort !== null ? (
                      <>
                        {' '}Currently running on{' '}
                        <span
                          className="font-mono"
                          data-testid="gateway-active-port"
                        >
                          :{portSettings.activePort}
                        </span>
                        .
                      </>
                    ) : (
                      ' Gateway is stopped.'
                    )}
                  </p>
                  <div className="flex flex-wrap items-center gap-2 mt-3">
                    <input
                      id="gateway-port-input"
                      type="number"
                      inputMode="numeric"
                      min={1024}
                      max={65535}
                      value={portDraft}
                      onChange={(e) => {
                        setPortDraft(e.target.value);
                        if (portError) setPortError(null);
                      }}
                      disabled={savingPort || resettingPort}
                      className="w-28 px-3 py-1.5 text-sm font-mono border border-[rgb(var(--border))] rounded-lg bg-[rgb(var(--surface))] text-[rgb(var(--foreground))] focus:outline-none focus:ring-2 focus:ring-primary-500/40"
                      data-testid="gateway-port-input"
                    />
                    <Button
                      variant="primary"
                      size="sm"
                      onClick={handleSavePort}
                      disabled={
                        savingPort ||
                        resettingPort ||
                        portDraft.trim() === String(portSettings.configuredPort ?? portSettings.defaultPort)
                      }
                      data-testid="gateway-port-save-btn"
                    >
                      {savingPort ? (
                        <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                      ) : null}
                      Save
                    </Button>
                    <Button
                      variant="secondary"
                      size="sm"
                      onClick={handleResetPort}
                      disabled={
                        savingPort ||
                        resettingPort ||
                        portSettings.configuredPort === null
                      }
                      data-testid="gateway-port-reset-btn"
                      title={
                        portSettings.configuredPort === null
                          ? 'Already using the default port'
                          : `Reset to ${portSettings.defaultPort}`
                      }
                    >
                      {resettingPort ? (
                        <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                      ) : (
                        <RotateCcw className="h-4 w-4 mr-2" />
                      )}
                      Reset to default
                    </Button>
                  </div>
                  {portError ? (
                    <p
                      className="text-xs text-red-600 dark:text-red-400 mt-2"
                      data-testid="gateway-port-error"
                    >
                      {portError}
                    </p>
                  ) : null}
                </div>
              </div>

              <div className="flex items-start gap-3">
                <Globe className="h-5 w-5 mt-0.5 text-[rgb(var(--muted))] flex-shrink-0" />
                <div className="flex-1 min-w-0">
                  <label
                    htmlFor="gateway-public-url-input"
                    className="text-sm font-medium"
                  >
                    Public gateway URL
                  </label>
                  <p className="text-xs text-[rgb(var(--muted))] mt-1">
                    HTTPS hostname for remote MCP clients via Cloudflare Tunnel. OAuth discovery
                    uses this when requests arrive with matching{' '}
                    <span className="font-mono">X-Forwarded-Host</span>. Local clients keep using{' '}
                    <span className="font-mono">localhost</span>.
                  </p>
                  <div className="flex flex-wrap items-center gap-2 mt-3">
                    <input
                      id="gateway-public-url-input"
                      type="url"
                      placeholder="https://mcp.example.com"
                      value={publicUrlDraft}
                      onChange={(e) => {
                        setPublicUrlDraft(e.target.value);
                        if (publicUrlError) setPublicUrlError(null);
                      }}
                      disabled={savingPublicUrl}
                      className="min-w-[16rem] flex-1 px-3 py-1.5 text-sm font-mono border border-[rgb(var(--border))] rounded-lg bg-[rgb(var(--surface))] text-[rgb(var(--foreground))] focus:outline-none focus:ring-2 focus:ring-primary-500/40"
                      data-testid="gateway-public-url-input"
                    />
                    <Button
                      variant="primary"
                      size="sm"
                      onClick={handleSavePublicUrl}
                      disabled={
                        savingPublicUrl ||
                        publicUrlDraft.trim() === (portSettings.publicUrl ?? '')
                      }
                      data-testid="gateway-public-url-save-btn"
                    >
                      {savingPublicUrl ? (
                        <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                      ) : null}
                      Save
                    </Button>
                  </div>
                  {publicUrlError ? (
                    <p
                      className="text-xs text-red-600 dark:text-red-400 mt-2"
                      data-testid="gateway-public-url-error"
                    >
                      {publicUrlError}
                    </p>
                  ) : null}
                </div>
              </div>

              {portSettings.activePort !== null &&
              portSettings.configuredPort !== null &&
              portSettings.configuredPort !== portSettings.activePort ? (
                <div
                  className="flex items-start gap-2 p-3 rounded-lg border border-amber-300 dark:border-amber-700/60 bg-amber-50 dark:bg-amber-900/20 text-xs"
                  data-testid="gateway-port-restart-hint"
                >
                  <AlertCircle className="h-4 w-4 text-amber-600 dark:text-amber-400 mt-0.5 flex-shrink-0" />
                  <div className="flex-1">
                    <p className="font-semibold text-amber-800 dark:text-amber-200">
                      Restart required
                    </p>
                    <p className="text-amber-700 dark:text-amber-300 mt-0.5">
                      Saved port <span className="font-mono">:{portSettings.configuredPort}</span>{' '}
                      doesn't match the running port{' '}
                      <span className="font-mono">:{portSettings.activePort}</span>. Restart the
                      gateway to apply — your IDE configs will need to point at the new URL.
                    </p>
                  </div>
                  <Button
                    variant="secondary"
                    size="sm"
                    onClick={handleRestartGateway}
                    data-testid="gateway-restart-btn"
                  >
                    Restart gateway
                  </Button>
                </div>
              ) : null}

              {isTauri() ? (
              <div
                className="border-t border-[rgb(var(--border))] pt-4 mt-2"
                data-testid="settings-web-admin-section"
              >
                <div className="flex items-start gap-3">
                  <Globe className="h-5 w-5 mt-0.5 text-[rgb(var(--muted))] flex-shrink-0" />
                  <div className="flex-1 min-w-0 space-y-4">
                    <div>
                      <p className="text-sm font-medium">Web admin mode</p>
                      <p className="text-xs text-[rgb(var(--muted))] mt-1">
                        Serves the UI + REST API on loopback for remote access (e.g. Cloudflare
                        Tunnel). Use{' '}
                        <span className="font-mono">http://127.0.0.1:{adminPortDraft}</span> — not
                        the Vite dev URL on :1420 unless admin is enabled and proxied.
                      </p>
                    </div>

                    {loadingAdminWeb || adminWeb === null ? (
                      <div className="flex items-center gap-2 text-sm text-[rgb(var(--muted))]">
                        <Loader2 className="h-4 w-4 animate-spin" />
                        Loading…
                      </div>
                    ) : (
                      <>
                        <div className="flex items-center justify-between gap-4">
                          <div>
                            <p className="text-sm font-medium">Enable web admin</p>
                            <p className="text-xs text-[rgb(var(--muted))]">
                              Starts HTTP server on this Mac (default port 45819).
                            </p>
                          </div>
                          <Switch
                            checked={adminWeb.enabled}
                            disabled={savingAdminWeb}
                            onCheckedChange={(enabled) =>
                              persistAdminWeb({ ...adminWeb, enabled })
                            }
                            data-testid="settings-admin-enabled-switch"
                          />
                        </div>

                        <div>
                          <label
                            htmlFor="admin-port-input"
                            className="text-sm font-medium"
                          >
                            Admin port
                          </label>
                          <div className="flex flex-wrap items-center gap-2 mt-2">
                            <input
                              id="admin-port-input"
                              type="number"
                              inputMode="numeric"
                              min={1024}
                              max={65535}
                              value={adminPortDraft}
                              onChange={(e) => setAdminPortDraft(e.target.value)}
                              disabled={savingAdminWeb || !adminWeb.enabled}
                              className="w-28 px-3 py-1.5 text-sm font-mono border border-[rgb(var(--border))] rounded-lg bg-[rgb(var(--surface))]"
                              data-testid="settings-admin-port-input"
                            />
                            <Button
                              variant="primary"
                              size="sm"
                              onClick={handleSaveAdminPort}
                              disabled={
                                savingAdminWeb ||
                                !adminWeb.enabled ||
                                adminPortDraft.trim() === String(adminWeb.port)
                              }
                              data-testid="settings-admin-port-save-btn"
                            >
                              Save port
                            </Button>
                          </div>
                        </div>

                        <div className="flex items-center justify-between gap-4">
                          <div>
                            <p className="text-sm font-medium">Trust Cloudflare Access JWT</p>
                            <p className="text-xs text-[rgb(var(--muted))]">
                              Required for tunnel exposure; disable for local-only testing.
                            </p>
                          </div>
                          <Switch
                            checked={adminWeb.trustCfAccess}
                            disabled={savingAdminWeb || !adminWeb.enabled}
                            onCheckedChange={(trustCfAccess) =>
                              persistAdminWeb({ ...adminWeb, trustCfAccess })
                            }
                            data-testid="settings-admin-cf-access-switch"
                          />
                        </div>

                        {adminWeb.enabled ? (
                          <div>
                            <label
                              htmlFor="admin-cf-domain-input"
                              className="text-sm font-medium"
                            >
                              Cloudflare team domain
                            </label>
                            <p className="text-xs text-[rgb(var(--muted))] mt-1">
                              Set this before enabling Trust CF Access JWT. Just the team slug
                              (e.g. <code>your-team</code>), not the full URL.
                            </p>
                            <div className="flex flex-wrap items-center gap-2 mt-2">
                              <input
                                id="admin-cf-domain-input"
                                type="text"
                                placeholder="your-team.cloudflareaccess.com"
                                value={adminCfDomainDraft}
                                onChange={(e) => setAdminCfDomainDraft(e.target.value)}
                                disabled={savingAdminWeb || !adminWeb.enabled}
                                className="flex-1 min-w-[12rem] px-3 py-1.5 text-sm border border-[rgb(var(--border))] rounded-lg bg-[rgb(var(--surface))]"
                                data-testid="settings-admin-cf-domain-input"
                              />
                              <Button
                                variant="primary"
                                size="sm"
                                onClick={() =>
                                  persistAdminWeb({
                                    ...adminWeb,
                                    cfTeamDomain: adminCfDomainDraft.trim(),
                                  })
                                }
                                disabled={
                                  savingAdminWeb ||
                                  !adminWeb.enabled ||
                                  adminCfDomainDraft.trim() === adminWeb.cfTeamDomain
                                }
                              >
                                Save domain
                              </Button>
                            </div>
                          </div>
                        ) : null}
                      </>
                    )}
                  </div>
                </div>
              </div>
              ) : null}
            </div>
          )}
        </CardContent>
      </Card>

      {/* Appearance Section */}
      <Card>
        <CardHeader>
          <CardTitle>Appearance</CardTitle>
          <CardDescription>Customize the look and feel of McpMux.</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="space-y-4">
            <div>
              <label className="text-sm font-medium">Theme</label>
              <div className="flex gap-2 mt-2" data-testid="theme-buttons">
                <Button
                  variant={theme === 'light' ? 'primary' : 'secondary'}
                  size="sm"
                  onClick={() => setTheme('light')}
                  data-testid="theme-light-btn"
                >
                  <Sun className="h-4 w-4 mr-2" />
                  Light
                </Button>
                <Button
                  variant={theme === 'dark' ? 'primary' : 'secondary'}
                  size="sm"
                  onClick={() => setTheme('dark')}
                  data-testid="theme-dark-btn"
                >
                  <Moon className="h-4 w-4 mr-2" />
                  Dark
                </Button>
                <Button
                  variant={theme === 'system' ? 'primary' : 'secondary'}
                  size="sm"
                  onClick={() => setTheme('system')}
                  data-testid="theme-system-btn"
                >
                  <Monitor className="h-4 w-4 mr-2" />
                  System
                </Button>
              </div>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Self-management meta tools — `mcpmux_*` namespace */}
      <Card data-testid="settings-meta-tools-section">
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Sparkles className="h-5 w-5" />
            Self-management tools (mcpmux_*)
          </CardTitle>
          <CardDescription>
            When enabled, connected MCP clients see a fixed meta-tool surface (~14 tools)
            for search → schema → invoke and search → read/fetch workflows. FeatureSets
            control what is invokable/readable/fetchable; optional surfaced items can
            appear directly in tools/list, resources/list, or prompts/list. Writes always trigger
            a native approval dialog; reads are silent.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          <div className="flex items-center justify-between gap-4">
            <div className="flex items-start gap-3 flex-1 min-w-0">
              <Sparkles className="h-5 w-5 mt-0.5 text-[rgb(var(--muted))] flex-shrink-0" />
              <div>
                <label className="text-sm font-medium">Advertise self-management tools</label>
                <p className="text-xs text-[rgb(var(--muted))] mt-1">
                  Shows <code className="font-mono">mcpmux_search_tools</code>,&nbsp;
                  <code className="font-mono">mcpmux_invoke_tool</code>,&nbsp;
                  <code className="font-mono">mcpmux_search_resources</code>, and other meta tools
                  to every connected MCP client. Turn off to hide the whole namespace.
                </p>
              </div>
            </div>
            <Switch
              checked={metaToolsEnabled}
              onCheckedChange={handleToggleMetaTools}
              disabled={loadingMetaTools}
              data-testid="meta-tools-enabled-switch"
            />
          </div>
          <MetaToolGrantsPanel />
          <MetaToolAuditLog />
        </CardContent>
      </Card>

      {/* Analytics Section */}
      <Card data-testid="settings-analytics-section">
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <BarChart3 className="h-5 w-5" />
            Analytics
          </CardTitle>
          <CardDescription>
            Help improve McpMux by sharing anonymous usage data. No personal information is collected.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="flex items-center justify-between gap-4">
            <div className="flex items-start gap-3 flex-1 min-w-0">
              <BarChart3 className="h-5 w-5 mt-0.5 text-[rgb(var(--muted))] flex-shrink-0" />
              <div>
                <label className="text-sm font-medium">Share Usage Data</label>
                <p className="text-xs text-[rgb(var(--muted))] mt-1">
                  Sends anonymous data like app version, OS, and feature usage to help us prioritize improvements.
                  Location is approximated from IP by PostHog. No credentials or server configurations are shared.
                </p>
              </div>
            </div>
            <Switch
              checked={analyticsEnabled}
              onCheckedChange={setAnalyticsEnabled}
              data-testid="analytics-switch"
            />
          </div>
        </CardContent>
      </Card>

      {/* Contribute & feedback — the single global "help make mcpmux
          better" card. Mirrors the items in <ContributeMenu> so power
          users have quick access without digging into GitHub. */}
      <Card data-testid="settings-contribute-section">
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Heart className="h-5 w-5" />
            Contribute &amp; feedback
          </CardTitle>
          <CardDescription>
            mcpmux is open source. Request a server, report a bug, suggest a feature, or jump
            straight to the source.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
            <ContributeRow
              icon={Package}
              title="Request a new server"
              subtitle="Ask the community to add an MCP server to the registry"
              onClick={() => openExternal(CONTRIBUTE.requestServer())}
              testId="contribute-request-server"
            />
            <ContributeRow
              icon={Bug}
              title="Report a bug"
              subtitle="Something broken in the desktop app or gateway"
              onClick={() => openExternal(CONTRIBUTE.bug)}
              testId="contribute-report-bug"
            />
            <ContributeRow
              icon={Lightbulb}
              title="Suggest a feature"
              subtitle="An idea for mcpmux itself"
              onClick={() => openExternal(CONTRIBUTE.featureRequest)}
              testId="contribute-feature-request"
            />
            <ContributeRow
              icon={Github}
              title="Open on GitHub"
              subtitle="Browse source, issues, pull requests"
              onClick={() => openExternal(CONTRIBUTE.repo)}
              testId="contribute-open-github"
            />
          </div>
        </CardContent>
      </Card>

      {/* Logs Section */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <FileText className="h-5 w-5" />
            Logs
          </CardTitle>
          <CardDescription>View application logs for debugging and troubleshooting.</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="space-y-4">
            <div>
              <label className="text-sm font-medium">Log Files Location</label>
              <p className="text-sm text-[rgb(var(--muted))] mt-1 font-mono bg-surface-secondary rounded px-2 py-1" data-testid="logs-path">
                {logsPath || 'Loading...'}
              </p>
            </div>
            <div className="flex items-center gap-2">
              {isTauri() ? (
                <Button
                  variant="secondary"
                  size="sm"
                  onClick={handleOpenLogs}
                  disabled={openingLogs}
                  data-testid="open-logs-btn"
                >
                  {openingLogs ? (
                    <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                  ) : (
                    <FolderOpen className="h-4 w-4 mr-2" />
                  )}
                  Open Logs Folder
                </Button>
              ) : null}
            </div>
            <div className="border-t border-[rgb(var(--border))] pt-4">
              <div className="flex items-center justify-between gap-4">
                <div className="flex items-start gap-3 flex-1 min-w-0">
                  <Trash2 className="h-5 w-5 mt-0.5 text-[rgb(var(--muted))] flex-shrink-0" />
                  <div>
                    <label className="text-sm font-medium">Auto-Cleanup</label>
                    <p className="text-xs text-[rgb(var(--muted))] mt-1">
                      Automatically delete log files older than the selected period
                    </p>
                  </div>
                </div>
                <select
                  value={logRetentionDays}
                  onChange={(e) => handleRetentionChange(Number(e.target.value))}
                  disabled={savingRetention}
                  className="px-3 py-1.5 text-sm border border-[rgb(var(--border))] rounded-lg bg-[rgb(var(--surface))] text-[rgb(var(--foreground))]"
                  data-testid="log-retention-select"
                >
                  <option value={7}>7 days</option>
                  <option value={14}>14 days</option>
                  <option value={30}>30 days</option>
                  <option value={60}>60 days</option>
                  <option value={90}>90 days</option>
                  <option value={0}>Keep forever</option>
                </select>
              </div>
            </div>
            <p className="text-xs text-[rgb(var(--muted))]">
              Logs are rotated daily. Each file contains detailed debug information including thread IDs and source locations.
            </p>
          </div>
        </CardContent>
      </Card>
    </div>
    </>
  );
}

/**
 * Flat row used inside the Contribute card. Local to the Settings page — if
 * we ever need this elsewhere, promote it into @mcpmux/ui.
 */
function ContributeRow({
  icon: Icon,
  title,
  subtitle,
  onClick,
  testId,
}: {
  icon: React.ComponentType<{ className?: string }>;
  title: string;
  subtitle: string;
  onClick: () => void;
  testId?: string;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="text-left flex items-start gap-3 p-3 rounded-lg border border-[rgb(var(--border-subtle))] bg-[rgb(var(--surface))] hover:border-primary-400/60 hover:bg-primary-500/5 transition-colors"
      data-testid={testId}
    >
      <Icon className="h-4 w-4 mt-0.5 text-[rgb(var(--muted))] flex-shrink-0" />
      <div className="min-w-0">
        <p className="text-sm font-medium">{title}</p>
        <p className="text-[11px] text-[rgb(var(--muted))] leading-snug mt-0.5">{subtitle}</p>
      </div>
    </button>
  );
}
