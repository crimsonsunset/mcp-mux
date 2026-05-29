#!/usr/bin/env node
/**
 * Build Cloudflare Access HTTP headers from environment variables.
 *
 * Reads `MCPMUX_CF_ACCESS_CLIENT_ID` and `MCPMUX_CF_ACCESS_CLIENT_SECRET`.
 * Returns an empty object when either is unset.
 */

/**
 * @returns {Record<string, string>}
 */
export function cfAccessHeadersFromEnv() {
  const clientId = process.env.MCPMUX_CF_ACCESS_CLIENT_ID?.trim();
  const clientSecret = process.env.MCPMUX_CF_ACCESS_CLIENT_SECRET?.trim();
  if (!clientId || !clientSecret) {
    return {};
  }
  return {
    'CF-Access-Client-Id': clientId,
    'CF-Access-Client-Secret': clientSecret,
  };
}

/**
 * @returns {string[]}
 */
export function cfAccessCurlFlagsFromEnv() {
  const headers = cfAccessHeadersFromEnv();
  return Object.entries(headers).flatMap(([name, value]) => ['-H', `${name}: ${value}`]);
}

/**
 * Load `.env` from the repo root when present (optional; no dependency on dotenv).
 */
export function loadRepoDotEnv(repoRoot) {
  try {
    const fs = require('node:fs');
    const path = require('node:path');
    const dotenvPath = path.join(repoRoot, '.env');
    if (!fs.existsSync(dotenvPath)) {
      return;
    }
    const text = fs.readFileSync(dotenvPath, 'utf8');
    for (const line of text.split('\n')) {
      const trimmed = line.trim();
      if (!trimmed || trimmed.startsWith('#')) {
        continue;
      }
      const eq = trimmed.indexOf('=');
      if (eq <= 0) {
        continue;
      }
      const key = trimmed.slice(0, eq).trim();
      let value = trimmed.slice(eq + 1).trim();
      if (
        (value.startsWith('"') && value.endsWith('"')) ||
        (value.startsWith("'") && value.endsWith("'"))
      ) {
        value = value.slice(1, -1);
      }
      if (process.env[key] === undefined) {
        process.env[key] = value;
      }
    }
  } catch {
    // optional convenience only
  }
}
