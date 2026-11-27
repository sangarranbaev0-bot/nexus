-- Revert server boost indexes added in 2026-11-25-000038_indexes if needed.
DROP INDEX IF EXISTS tickets_server_status_idx;
DROP INDEX IF EXISTS invites_server_id_idx;
