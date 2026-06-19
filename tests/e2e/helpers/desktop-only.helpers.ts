/**
 * Playwright `playwright.config.ts` runs the web-admin SPA (Vite + admin HTTP), not the
 * Tauri desktop shell. Use these markers for specs that only apply to WDIO / `pnpm dev`.
 */
export const DESKTOP_TAURI_ONLY =
  'Desktop Tauri shell only — not rendered in web-admin (VITE_ADMIN_WEB)';
