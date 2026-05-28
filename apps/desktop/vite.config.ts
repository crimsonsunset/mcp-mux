import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { execSync } from 'node:child_process';
import path from 'path';

/** Git short SHA at build/dev-server start — compared against the backend in web-admin prod mode. */
function getGitSha(): string {
  try {
    return execSync('git rev-parse --short HEAD', { encoding: 'utf8' }).trim();
  } catch {
    return '';
  }
}

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  define: {
    'import.meta.env.VITE_ADMIN_WEB': JSON.stringify(process.env.VITE_ADMIN_WEB === 'true'),
    'import.meta.env.VITE_BUILD_GIT_SHA': JSON.stringify(getGitSha()),
  },
  plugins: [react()],

  // Path aliases
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
      '@mcpmux/ui': path.resolve(__dirname, '../../packages/ui/src'),
    },
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    // Web admin REST + SSE — requires AdminServer on :45819 (pnpm dev / dev:admin with admin enabled).
    proxy: {
      '/api': {
        target: `http://127.0.0.1:${process.env.MCPMUX_ADMIN_PORT ?? '45819'}`,
        changeOrigin: true,
      },
    },
    hmr: host
      ? {
          protocol: 'ws',
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ['**/src-tauri/**'],
    },
  },
}));
