DROP VIEW IF EXISTS trail_lines;
DROP VIEW IF EXISTS trail_start_points;
DROP VIEW IF EXISTS trail_end_points;

CREATE TABLE IF NOT EXISTS trail_lines (
    osm_relation_id BIGINT PRIMARY KEY,
    id UUID NOT NULL,
    name TEXT,
    network TEXT,
    difficulty TEXT,
    hike_type TEXT,
    geom geometry(MultiLineString, 4326) NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_trail_lines_geom ON trail_lines USING GIST (geom);
CREATE UNIQUE INDEX IF NOT EXISTS idx_trail_lines_id ON trail_lines (id);

INSERT INTO trail_lines (osm_relation_id, id, name, network, difficulty, hike_type, geom)
SELECT
    osm_relation_id,
    id,
    name,
    network,
    difficulty,
    hike_type,
    geom::geometry(MultiLineString, 4326)
FROM trails
WHERE geom IS NOT NULL
ON CONFLICT (osm_relation_id) DO UPDATE
SET id = EXCLUDED.id,
    name = EXCLUDED.name,
    network = EXCLUDED.network,
    difficulty = EXCLUDED.difficulty,
    hike_type = EXCLUDED.hike_type,
    geom = EXCLUDED.geom;

CREATE TABLE IF NOT EXISTS trail_start_points (
    osm_relation_id BIGINT PRIMARY KEY,
    id UUID NOT NULL,
    name TEXT,
    geom geometry(Point, 4326) NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_trail_start_points_geom ON trail_start_points USING GIST (geom);
CREATE UNIQUE INDEX IF NOT EXISTS idx_trail_start_points_id ON trail_start_points (id);

INSERT INTO trail_start_points (osm_relation_id, id, name, geom)
SELECT
    osm_relation_id,
    id,
    name,
    start_point::geometry(Point, 4326)
FROM trails
WHERE start_point IS NOT NULL
ON CONFLICT (osm_relation_id) DO UPDATE
SET id = EXCLUDED.id,
    name = EXCLUDED.name,
    geom = EXCLUDED.geom;

CREATE TABLE IF NOT EXISTS trail_end_points (
    osm_relation_id BIGINT PRIMARY KEY,
    id UUID NOT NULL,
    name TEXT,
    geom geometry(Point, 4326) NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_trail_end_points_geom ON trail_end_points USING GIST (geom);
CREATE UNIQUE INDEX IF NOT EXISTS idx_trail_end_points_id ON trail_end_points (id);

INSERT INTO trail_end_points (osm_relation_id, id, name, geom)
SELECT
    osm_relation_id,
    id,
    name,
    end_point::geometry(Point, 4326)
FROM trails
WHERE end_point IS NOT NULL
ON CONFLICT (osm_relation_id) DO UPDATE
SET id = EXCLUDED.id,
    name = EXCLUDED.name,
    geom = EXCLUDED.geom;
