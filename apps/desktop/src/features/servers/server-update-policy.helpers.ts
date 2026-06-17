import type { UpdatePolicy } from '@/lib/api/settings';

/** Per-server update policy labels for Configure and Settings. */
export const UPDATE_POLICY_OPTIONS: {
  value: UpdatePolicy;
  label: string;
  description: string;
}[] = [
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
    description: 'Lock to a specific version on every spawn',
  },
];

/** Basic semver pattern (major.minor.patch with optional pre-release/build). */
const BASIC_SEMVER_PATTERN =
  /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/;

/**
 * Returns true when `version` matches a basic semver shape.
 */
export function isValidSemver(version: string): boolean {
  return BASIC_SEMVER_PATTERN.test(version.trim());
}

/**
 * Returns true when the stdio transport uses npx or uvx/uv (package-managed).
 */
export function isPackageManagedTransport(command: string | undefined): boolean {
  if (!command) {
    return false;
  }
  return command === 'npx' || command === 'uvx' || command === 'uv';
}

/**
 * Parse a semver-ish version string into numeric segments for comparison.
 */
function parseVersionParts(version: string): number[] {
  return version
    .trim()
    .replace(/^v/, '')
    .replace(/^=/, '')
    .split(/[^0-9]+/)
    .filter(Boolean)
    .map((part) => Number.parseInt(part, 10))
    .filter((part) => !Number.isNaN(part));
}

/**
 * Returns true when `latest` is strictly newer than `current`.
 */
export function isUpdateAvailable(
  latest: string | null | undefined,
  current: string | null | undefined
): boolean {
  if (!latest) {
    return false;
  }
  if (!current) {
    return true;
  }

  const latestParts = parseVersionParts(latest);
  const currentParts = parseVersionParts(current);
  const maxLen = Math.max(latestParts.length, currentParts.length);

  for (let index = 0; index < maxLen; index += 1) {
    const latestPart = latestParts[index] ?? 0;
    const currentPart = currentParts[index] ?? 0;
    if (latestPart > currentPart) {
      return true;
    }
    if (latestPart < currentPart) {
      return false;
    }
  }

  return latest !== current;
}

/**
 * Derive the effective current version for update badge display.
 */
export function resolveCurrentPackageVersion(input: {
  pinnedVersion?: string | null;
  transportCommand?: string;
  transportArgs?: string[];
}): string | null {
  if (input.pinnedVersion) {
    return input.pinnedVersion;
  }

  if (input.transportCommand === 'npx' && input.transportArgs) {
    const packageArg = findNpxPackageArg(input.transportArgs);
    if (!packageArg) {
      return null;
    }
    const atIndex = packageArg.lastIndexOf('@');
    if (packageArg.startsWith('@') && packageArg.indexOf('@', 1) > 0) {
      const scopedSplit = packageArg.indexOf('@', 1);
      return packageArg.slice(scopedSplit + 1) || null;
    }
    if (atIndex > 0) {
      return packageArg.slice(atIndex + 1) || null;
    }
  }

  if (
    (input.transportCommand === 'uvx' || input.transportCommand === 'uv') &&
    input.transportArgs
  ) {
    const packageArg = findUvxPackageArg(input.transportCommand, input.transportArgs);
    if (!packageArg) {
      return null;
    }
    const eqIndex = packageArg.indexOf('==');
    if (eqIndex >= 0) {
      return packageArg.slice(eqIndex + 2) || null;
    }
  }

  return null;
}

/**
 * Locate the npm package argument after `-y` / `--yes`.
 */
function findNpxPackageArg(args: string[]): string | undefined {
  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];
    if ((arg === '-y' || arg === '--yes') && index + 1 < args.length && !args[index + 1].startsWith('-')) {
      return args[index + 1];
    }
  }
  return args.find((arg) => !arg.startsWith('-') && arg !== '--');
}

/**
 * Locate the first positional package arg for uvx / uv run.
 */
function findUvxPackageArg(command: string, args: string[]): string | undefined {
  if (command === 'uvx') {
    return args.find((arg) => !arg.startsWith('-'));
  }
  if (command === 'uv' && args[0] === 'run') {
    for (let index = 1; index < args.length; index += 1) {
      const arg = args[index];
      if (arg.startsWith('-')) {
        if (arg === '-m' || arg === '--module') {
          index += 1;
        }
        continue;
      }
      return arg;
    }
  }
  return undefined;
}
