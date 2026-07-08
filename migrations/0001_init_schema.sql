-- Core schema for the hiking trails app.
-- Scoped to run against a single Postgres/PostGIS instance.
-- Works unchanged whether you've imported Himachal Pradesh only,
-- all of India, or the whole world -- scope is controlled entirely
-- by which rows exist, not by the schema.

CREATE EXTENSION IF NOT EXISTS postgis;
CREATE EXTENSION IF NOT EXISTS pg_trgm;   -- fuzzy name search fallback
CREATE EXTENSION IF NOT EXISTS pgcrypto;  -- gen_random_uuid()

-- ============================================================
-- regions: administrative boundaries used to scope trails
-- (state, district, or any custom polygon you import)
-- ============================================================
CREATE TABLE regions (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    osm_relation_id  BIGINT UNIQUE,
    name             TEXT NOT NULL,
    admin_level      SMALLINT,
    country_code     CHAR(2) NOT NULL,
    parent_region_id UUID REFERENCES regions(id),
    boundary         GEOMETRY(MultiPolygon, 4326),
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_regions_boundary ON regions USING GIST (boundary);
CREATE INDEX idx_regions_country  ON regions (country_code);
CREATE INDEX idx_regions_parent   ON regions (parent_region_id);

-- ============================================================
-- trails: one row per named/waymarked hiking route
-- (an OSM relation, e.g. "Lamadugh Circuit")
-- ============================================================
CREATE TABLE trails (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    osm_relation_id BIGINT NOT NULL UNIQUE,
    region_id       UUID REFERENCES regions(id),
    name            TEXT,
    network         TEXT,              -- lwn / rwn / nwn / iwn
    difficulty      TEXT,              -- sac_scale value; nullable, often untagged
    distance_m      DOUBLE PRECISION,
    ascent_m        DOUBLE PRECISION,
    descent_m       DOUBLE PRECISION,
    geom            GEOMETRY(MultiLineString, 4326),  -- merged route geometry
    last_synced_at  TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_trails_geom       ON trails USING GIST (geom);
CREATE INDEX idx_trails_region     ON trails (region_id);
CREATE INDEX idx_trails_name_trgm  ON trails USING GIN (name gin_trgm_ops);

-- ============================================================
-- segments: individual OSM ways that make up a trail, in order.
-- A trail's route is reconstructed from these, not stored as
-- one flat geometry from the start -- this is what lets us fix
-- ordering/stitching ourselves instead of trusting OSM's raw
-- member order (the "route is not sorted" problem you saw in
-- Waymarked Trails' debug panel).
-- ============================================================
CREATE TABLE segments (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    trail_id     UUID NOT NULL REFERENCES trails(id) ON DELETE CASCADE,
    osm_way_id   BIGINT NOT NULL,
    sequence     INT NOT NULL,
    name         TEXT,
    surface      TEXT,
    highway_type TEXT,               -- path / footway / track / residential / ...
    length_m     DOUBLE PRECISION,
    geom         GEOMETRY(LineString, 4326) NOT NULL,
    UNIQUE (trail_id, sequence)
);

CREATE INDEX idx_segments_geom  ON segments USING GIST (geom);
CREATE INDEX idx_segments_trail ON segments (trail_id);

-- ============================================================
-- waypoints: points of interest along or near a trail
-- (huts, springs, campsites, peaks, viewpoints, ...)
-- ============================================================
CREATE TABLE waypoints (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    trail_id    UUID REFERENCES trails(id) ON DELETE SET NULL,
    osm_node_id BIGINT,
    type        TEXT NOT NULL,
    name        TEXT,
    elevation_m DOUBLE PRECISION,
    geom        GEOMETRY(Point, 4326) NOT NULL
);

CREATE INDEX idx_waypoints_geom  ON waypoints USING GIST (geom);
CREATE INDEX idx_waypoints_trail ON waypoints (trail_id);

-- ============================================================
-- elevation_profiles: precomputed elevation samples along a trail.
-- Kept separate from `trails` so the common "get trail details"
-- query stays light -- this only gets joined in when a client
-- actually opens the elevation chart.
-- ============================================================
CREATE TABLE elevation_profiles (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    trail_id        UUID NOT NULL UNIQUE REFERENCES trails(id) ON DELETE CASCADE,
    min_elevation_m DOUBLE PRECISION,
    max_elevation_m DOUBLE PRECISION,
    profile_points  JSONB NOT NULL,   -- [{ "pos_m": 0, "ele_m": 2052 }, ...]
    computed_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ============================================================
-- sync_state: tracks OSM replication progress, one row per
-- source you're syncing against (e.g. a Geofabrik region feed).
-- ============================================================
CREATE TABLE sync_state (
    id                   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_name          TEXT NOT NULL UNIQUE,  -- e.g. 'india'
    last_sequence_number BIGINT,
    last_synced_at       TIMESTAMPTZ,
    replication_url      TEXT
);
