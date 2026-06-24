import type { InstallationSource } from '@/types/registry';

/**
 * Get the appropriate uninstall action label based on source.
 */
export function getUninstallLabel(source: InstallationSource | undefined): string {
  if (!source) {
    return 'Uninstall';
  }

  switch (source.type) {
    case 'user_config':
      return 'Remove from Config';
    case 'manual_entry':
      return 'Remove';
    case 'registry':
    default:
      return 'Uninstall';
  }
}

/**
 * Get confirmation message for uninstalling based on source.
 */
export function getUninstallConfirmMessage(
  serverName: string,
  source: InstallationSource | undefined
): string {
  if (source?.type === 'user_config') {
    return `This will remove "${serverName}" from your config file. You can re-add it by editing the config file.`;
  }
  return `Are you sure you want to uninstall "${serverName}"? You can reinstall it from the registry.`;
}
