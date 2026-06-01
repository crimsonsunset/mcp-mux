/** Local timezone for build/commit display (matches generAIt frontend). */
const BUILD_TIMEZONE = 'America/Denver';

const buildDateFormatter = new Intl.DateTimeFormat('en-US', {
  timeZone: BUILD_TIMEZONE,
  weekday: 'short',
  month: 'short',
  day: 'numeric',
  year: 'numeric',
});

const buildTimeFormatter = new Intl.DateTimeFormat('en-US', {
  timeZone: BUILD_TIMEZONE,
  hour: 'numeric',
  minute: '2-digit',
  second: '2-digit',
  hour12: true,
  timeZoneName: 'short',
});

/**
 * Format a date for build metadata (e.g. "Wed, May 29 2026").
 * @param {Date} date
 * @returns {string}
 */
export function formatBuildDate(date) {
  return buildDateFormatter.format(date);
}

/**
 * Format a time for build metadata (e.g. "06:32:19 PM MDT").
 * @param {Date} date
 * @returns {string}
 */
export function formatBuildTime(date) {
  return buildTimeFormatter.format(date);
}

/**
 * generAIt-style combined stamp: "Wed, May 29 2026 at 06:32:19 PM MDT".
 * @param {Date} date
 * @returns {string}
 */
export function formatBuiltAt(date) {
  return `${formatBuildDate(date)} at ${formatBuildTime(date)}`;
}

/**
 * Parse git `%ci` commit time or Rust UTC build strings.
 * @param {string} raw
 * @returns {Date | null}
 */
export function parseStampInstant(raw) {
  const trimmed = raw.trim();
  if (!trimmed || trimmed === 'unknown') {
    return null;
  }
  if (trimmed.endsWith(' UTC')) {
    const iso = trimmed.replace(' UTC', 'Z').replace(' ', 'T');
    const parsed = Date.parse(iso);
    return Number.isNaN(parsed) ? null : new Date(parsed);
  }
  const parsed = Date.parse(trimmed);
  return Number.isNaN(parsed) ? null : new Date(parsed);
}

/**
 * Format a raw git/Rust timestamp for display.
 * @param {string} raw
 * @param {string} [fallback]
 * @returns {string}
 */
export function formatStampInstant(raw, fallback = 'unknown') {
  const date = parseStampInstant(raw);
  return date ? formatBuiltAt(date) : fallback;
}
