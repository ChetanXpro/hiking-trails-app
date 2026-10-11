-- These standalone indexes duplicate the leftmost key of a primary, unique,
-- or newer composite index. Removing them reduces import write amplification
-- without removing an access path used by the application.
DROP INDEX IF EXISTS idx_segments_trail;
DROP INDEX IF EXISTS idx_trail_landcover_segments_trail;
DROP INDEX IF EXISTS idx_trails_region;
DROP INDEX IF EXISTS idx_trails_name_trgm;
