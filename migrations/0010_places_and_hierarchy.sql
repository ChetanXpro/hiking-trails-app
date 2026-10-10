ALTER TABLE regions
    ALTER COLUMN country_code DROP NOT NULL;

CREATE TABLE places (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    osm_type         TEXT NOT NULL,
    osm_id           BIGINT NOT NULL,
    region_id        UUID REFERENCES regions(id),
    name             TEXT NOT NULL,
    place_kind       TEXT NOT NULL,
    country_code     CHAR(2),
    geom             GEOMETRY(Point, 4326) NOT NULL,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (osm_type, osm_id)
);

CREATE INDEX idx_places_geom       ON places USING GIST (geom);
CREATE INDEX idx_places_region     ON places (region_id);
CREATE INDEX idx_places_kind       ON places (place_kind);
CREATE INDEX idx_places_name_trgm  ON places USING GIN (name gin_trgm_ops);
