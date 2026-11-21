DROP TABLE reaction_roles;
ALTER TABLE servers
  DROP COLUMN announcement_channel_id,
  DROP COLUMN banner_url,
  DROP COLUMN rules,
  DROP COLUMN description;
