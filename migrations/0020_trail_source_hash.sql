-- Add source hash to detect unchanged trails and optimize incremental imports.
-- source_hash is computed from relation tags, members, way geometries, importer version, and DEM config.
-- data_revision is derived from source_hash instead of updated_at, preventing spurious cache invalidation.

ALTER TABLE trails ADD COLUMN source_hash TEXT;
ALTER TABLE trails ADD COLUMN data_revision TEXT;

-- Backfill data_revision from existing MD5 hashes (will be recomputed on next import)
UPDATE trails SET data_revision = md5(geom::text || COALESCE(updated_at::text, ''));

-- Mark all existing trails as needing recomputation on next import
UPDATE trails SET source_hash = NULL;

-- Create index for data_revision lookups
CREATE INDEX IF NOT EXISTS idx_trails_data_revision ON trails(data_revision);
