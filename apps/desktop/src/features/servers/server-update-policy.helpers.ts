/**
 * Returns true when the stdio transport uses npx or uvx/uv (package-managed).
 */
export function isPackageManagedTransport(command: string | undefined): boolean {
  if (!command) {
    return false;
  }
  return command === 'npx' || command === 'uvx' || command === 'uv';
}
