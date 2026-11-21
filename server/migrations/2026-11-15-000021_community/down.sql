DROP TABLE event_interests;
DROP TABLE server_events;
DROP TABLE onboarding_answers;
DROP TABLE onboarding_questions;
DROP TABLE thread_tags;
ALTER TABLE messages DROP COLUMN thread_locked, DROP COLUMN thread_archived, DROP COLUMN thread_root_id;
ALTER TABLE channels DROP COLUMN is_forum;
