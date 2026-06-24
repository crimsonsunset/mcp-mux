import { Info } from 'lucide-react';
import {
  Card,
  CardHeader,
  CardTitle,
  CardDescription,
  CardContent,
} from '@mcpmux/ui';
import { BuildStampPanelContent } from './BuildStampPanel';
import { useBuildStamp } from './use-build-stamp.hook';

/**
 * Web-admin Settings card showing app version and build stamp metadata.
 */
export function AboutSection() {
  const stamp = useBuildStamp();

  return (
    <Card data-testid="about-section">
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Info className="h-5 w-5" />
          About
        </CardTitle>
        <CardDescription>App version and build information</CardDescription>
      </CardHeader>
      <CardContent>
        <div>
          <label className="text-sm font-medium">Current Version</label>
          <p className="text-sm text-[rgb(var(--muted))] mt-1" data-testid="current-version">
            {stamp.loading ? 'Loading…' : `v${stamp.version || 'unknown'}`}
          </p>
          <BuildStampPanelContent context="web-admin" stamp={stamp} />
        </div>
      </CardContent>
    </Card>
  );
}
