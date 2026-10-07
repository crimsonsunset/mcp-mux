-- Auth now defaults to off while the gateway is loopback-only. Pin installs
-- that are already in use to "auth required" so the upgrade keeps their auth.
-- An explicit choice already stored is left alone.
INSERT OR IGNORE INTO app_settings (key, value, updated_at)
SELECT 'gateway.auth_disabled', 'false', datetime('now')
 WHERE EXISTS (SELECT 1 FROM inbound_clients)
    OR EXISTS (SELECT 1 FROM installed_servers);
