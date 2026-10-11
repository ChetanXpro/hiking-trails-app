CREATE INDEX IF NOT EXISTS idx_regions_name_trgm
    ON regions
    USING gin (name gin_trgm_ops);
