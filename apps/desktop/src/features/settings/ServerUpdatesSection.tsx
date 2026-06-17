import { useEffect, useState } from 'react';
import {
  Card,
  CardHeader,
  CardTitle,
  CardDescription,
  CardContent,
} from '@mcpmux/ui';
import { Loader2, Package } from 'lucide-react';
import {
  getServerUpdateSettings,
  updateServerUpdateSettings,
  type ServerUpdateSettings,
  type UpdatePolicy,
} from '@/lib/api/settings';

const POLICY_OPTIONS: { value: UpdatePolicy; label: string; description: string }[] = [
  {
    value: 'notify',
    label: 'Notify',
    description: 'Surface available updates without changing packages automatically',
  },
  {
    value: 'auto',
    label: 'Auto',
    description: 'Always resolve the latest package on reconnect (npx/uvx servers only)',
  },
  {
    value: 'pinned',
    label: 'Pinned',
    description: 'Lock to a specific version (configure per server)',
  },
];

/**
 * Settings section for the app-wide default server update policy.
 */
export function ServerUpdatesSection() {
  const [settings, setSettings] = useState<ServerUpdateSettings>({
    defaultUpdatePolicy: 'notify',
  });
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    const load = async () => {
      try {
        const loaded = await getServerUpdateSettings();
        setSettings(loaded);
      } catch (err) {
        console.error('[Settings] Failed to load server update settings:', err);
      } finally {
        setLoading(false);
      }
    };
    load();
  }, []);

  /**
   * Persist a new default update policy for newly installed servers.
   */
  const handlePolicyChange = async (policy: UpdatePolicy) => {
    const previous = settings;
    const next = { defaultUpdatePolicy: policy };
    setSettings(next);
    setSaving(true);
    try {
      await updateServerUpdateSettings(next);
    } catch (err) {
      console.error('[Settings] Failed to save server update settings:', err);
      setSettings(previous);
    } finally {
      setSaving(false);
    }
  };

  return (
    <Card data-testid="settings-server-updates-section">
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Package className="h-5 w-5" />
          Server Updates
        </CardTitle>
        <CardDescription>
          Default update policy for newly installed npx and uvx servers. Override per server in
          Configure.
        </CardDescription>
      </CardHeader>
      <CardContent>
        {loading ? (
          <div className="flex items-center gap-2 text-sm text-[rgb(var(--muted))]">
            <Loader2 className="h-4 w-4 animate-spin" />
            Loading…
          </div>
        ) : (
          <div className="flex items-center justify-between gap-4">
            <div className="flex-1 min-w-0">
              <label className="text-sm font-medium" htmlFor="default-update-policy">
                Default update policy
              </label>
              <p className="text-xs text-[rgb(var(--muted))] mt-1">
                {
                  POLICY_OPTIONS.find((option) => option.value === settings.defaultUpdatePolicy)
                    ?.description
                }
              </p>
            </div>
            <select
              id="default-update-policy"
              value={settings.defaultUpdatePolicy}
              onChange={(e) => handlePolicyChange(e.target.value as UpdatePolicy)}
              disabled={saving}
              className="px-3 py-1.5 text-sm border border-[rgb(var(--border))] rounded-lg bg-[rgb(var(--surface))] text-[rgb(var(--foreground))]"
              data-testid="default-update-policy-select"
            >
              {POLICY_OPTIONS.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
