import { defineConfig, type Plugin } from 'vite';
import react from '@vitejs/plugin-react';
import fs from 'node:fs';
import path from 'path';

import {
  buildStampJson,
  formatBuildStampLine,
  getBuildStamp,
} from '../../scripts/build-stamp.mjs';

const stamp = getBuildStamp();
const isAdminWeb = process.env.VITE_ADMIN_WEB === 'true';

/**
 * Log build metadata on dev-server start and production builds (web admin only).
 */
function mcpmuxBuildBannerPlugin(): Plugin {
  return {
    name: 'mcpmux-build-banner',
    configureServer() {
      if (!isAdminWeb) {
        return;
      }
      console.log(formatBuildStampLine('[WebAdmin] Dev server', stamp));
    },
    writeBundle(options) {
      if (!isAdminWeb) {
        return;
      }
      const outDir = options.dir ?? path.resolve(__dirname, 'dist');
      fs.writeFileSync(
        path.join(outDir, 'build-stamp.json'),
        `${JSON.stringify(buildStampJson(stamp), null, 2)}\n`,
        'utf8',
      );
    },
    closeBundle() {
      if (!isAdminWeb) {
        return;
      }
      console.log(formatBuildStampLine('[WebAdmin] Built', stamp));
      console.log('[WebAdmin] Output: apps/desktop/dist — hard-refresh admin UI after deploy');
    },
  };
}

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  define: {
    'import.meta.env.VITE_ADMIN_WEB': JSON.stringify(isAdminWeb),
    'import.meta.env.VITE_BUILD_GIT_SHA': JSON.stringify(stamp.gitSha),
    'import.meta.env.VITE_BUILD_GIT_BRANCH': JSON.stringify(stamp.gitBranch),
    'import.meta.env.VITE_BUILD_COMMIT_TIME': JSON.stringify(stamp.commitTime),
    'import.meta.env.VITE_BUILD_COMMIT_AT': JSON.stringify(stamp.commitAt),
    'import.meta.env.VITE_BUILD_TIME': JSON.stringify(stamp.buildTime),
    'import.meta.env.VITE_BUILD_AT': JSON.stringify(stamp.buildAt),
  },
  plugins: [react(), mcpmuxBuildBannerPlugin()],

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
