-- Competitive contributor ranking was intentionally removed. Keep only one
-- stable, non-competitive provenance record on each trail: the OpenStreetMap
-- user who created version 1 of the hiking route relation. This is map
-- authorship, not evidence that the person physically recorded the trail.

DROP VIEW IF EXISTS contributor_leaderboard;
DROP TABLE IF EXISTS trail_contributions;
DROP TABLE IF EXISTS contributors;

ALTER TABLE trails
    ADD COLUMN original_mapper_name TEXT,
    ADD COLUMN original_mapper_provider TEXT,
    ADD COLUMN original_mapper_user_id TEXT,
    ADD COLUMN original_mapper_profile_url TEXT,
    ADD COLUMN original_mapped_at TIMESTAMPTZ,
    ADD COLUMN original_mapper_source TEXT;

