import { invoke } from '@tauri-apps/api/core';

import { apiCall } from './transport';

/** Startup and system tray settings. */
export interface StartupSettings {
  autoLaunch: boolean;
  startMinimized: boolean;
  closeToTray: boolean;
}

/** Persisted gateway port override, default, and currently active port. */
export interface GatewayPortSettings {
  configuredPort: number | null;
  defaultPort: number;
  activePort: number | null;
}

/**
 * Load startup and system tray preferences.
 */
export async function getStartupSettings(): Promise<StartupSettings> {
  return apiCall('get_startup_settings');
}

/**
 * Persist startup and system tray preferences.
 */
export async function updateStartupSettings(settings: StartupSettings): Promise<void> {
  return apiCall('update_startup_settings', { settings });
}

/**
 * Load gateway port settings (configured override, default, active).
 */
export async function getGatewayPortSettings(): Promise<GatewayPortSettings> {
  return apiCall('get_gateway_port_settings');
}

/**
 * Persist a custom gateway port. Takes effect on the next gateway start.
 */
export async function setGatewayPort(port: number): Promise<void> {
  return apiCall('set_gateway_port', { port });
}

/**
 * Clear the persisted gateway port override.
 */
export async function resetGatewayPort(): Promise<void> {
  return apiCall('reset_gateway_port');
}

/**
 * Resolve the on-disk application logs directory path.
 */
export async function getLogsPath(): Promise<string> {
  return apiCall('get_logs_path');
}

/**
 * Open the application logs folder in the system file manager.
 */
export async function openLogsFolder(): Promise<void> {
  return invoke('open_logs_folder');
}
