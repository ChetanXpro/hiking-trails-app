-- Keep serving tables self-contained: the API must not depend on osm2pgsql's
-- disposable raw staging tables after an import has completed.

ALTER TABLE trails
    ADD COLUMN IF NOT EXISTS network_type TEXT,
    ADD COLUMN IF NOT EXISTS route_tag TEXT,
    ADD COLUMN IF NOT EXISTS route_from TEXT,
    ADD COLUMN IF NOT EXISTS route_to TEXT,
    ADD COLUMN IF NOT EXISTS route_via TEXT,
    ADD COLUMN IF NOT EXISTS website_url TEXT,
    ADD COLUMN IF NOT EXISTS video_url TEXT;

-- Preserve metadata for existing databases before raw_relations is removed
-- from the online serving path. Dynamic SQL keeps this migration valid for a
-- clean production database that never contains staging tables.
DO $migration$
BEGIN
    IF to_regclass('public.raw_relations') IS NOT NULL THEN
        EXECUTE $backfill$
            UPDATE trails
            SET network_type = NULLIF(btrim(raw.tags->>'network:type'), ''),
                route_tag = NULLIF(btrim(raw.tags->>'tag'), ''),
                route_from = NULLIF(btrim(raw.tags->>'from'), ''),
                route_to = NULLIF(btrim(raw.tags->>'to'), ''),
                route_via = NULLIF(btrim(raw.tags->>'via'), ''),
                website_url = NULLIF(btrim(raw.tags->>'website'), ''),
                video_url = NULLIF(btrim(raw.tags->>'video'), '')
            FROM raw_relations raw
            WHERE raw.osm_id = trails.osm_relation_id
        $backfill$;
    END IF;
END
$migration$;

ALTER TABLE trails
    ADD COLUMN IF NOT EXISTS search_text TEXT
    GENERATED ALWAYS AS (
        COALESCE(name, '') || ' ' ||
        COALESCE(summary, '') || ' ' ||
        COALESCE(route_from, '') || ' ' ||
        COALESCE(route_to, '') || ' ' ||
        COALESCE(route_via, '')
    ) STORED;

CREATE INDEX IF NOT EXISTS idx_trails_search_text_trgm
    ON trails USING GIN (search_text gin_trgm_ops);

-- Country extents change only during an import. Caching four numbers avoids
-- aggregating every administrative polygon on each search keystroke.
CREATE TABLE IF NOT EXISTS country_bounds (
    country_code CHAR(2) PRIMARY KEY,
    min_lon DOUBLE PRECISION NOT NULL,
    min_lat DOUBLE PRECISION NOT NULL,
    max_lon DOUBLE PRECISION NOT NULL,
    max_lat DOUBLE PRECISION NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (min_lon >= -180 AND max_lon <= 180 AND min_lon <= max_lon),
    CHECK (min_lat >= -90 AND max_lat <= 90 AND min_lat <= max_lat)
);

INSERT INTO country_bounds (country_code, min_lon, min_lat, max_lon, max_lat)
SELECT
    country_code,
    MIN(ST_XMin(ST_Envelope(boundary))),
    MIN(ST_YMin(ST_Envelope(boundary))),
    MAX(ST_XMax(ST_Envelope(boundary))),
    MAX(ST_YMax(ST_Envelope(boundary)))
FROM regions
WHERE country_code IS NOT NULL
  AND boundary IS NOT NULL
GROUP BY country_code
ON CONFLICT (country_code) DO UPDATE
SET min_lon = EXCLUDED.min_lon,
    min_lat = EXCLUDED.min_lat,
    max_lon = EXCLUDED.max_lon,
    max_lat = EXCLUDED.max_lat,
    updated_at = now();
