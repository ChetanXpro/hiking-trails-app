-- Cover the region listing's filter and stable display order without sorting
-- every route in a large region.
CREATE INDEX IF NOT EXISTS idx_trails_region_name
    ON trails (region_id, name, id);
