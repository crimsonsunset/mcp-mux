/** @deprecated Prefer `@/lib/backend` — shim during facade migration. */
import { addToCursor as shellAddToCursor, addToVscode as shellAddToVscode } from '@/lib/backend/shell';

/** Add McpMux to VS Code via deep link. */
export async function addToVscode(gatewayUrl: string): Promise<void> {
  return shellAddToVscode(gatewayUrl);
}

/** Add McpMux to Cursor via deep link. */
export async function addToCursor(gatewayUrl: string): Promise<void> {
  return shellAddToCursor(gatewayUrl);
}
