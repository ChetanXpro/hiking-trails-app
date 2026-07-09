CREATE OR REPLACE VIEW trail_lines AS
SELECT
    id,
    osm_relation_id,
    name,
    network,
    difficulty,
    hike_type,
    geom::geometry(MultiLineString, 4326) AS geom
FROM trails
WHERE geom IS NOT NULL;

CREATE OR REPLACE VIEW trail_start_points AS
SELECT
    id,
    osm_relation_id,
    name,
    start_point::geometry(Point, 4326) AS geom
FROM trails
WHERE start_point IS NOT NULL;

CREATE OR REPLACE VIEW trail_end_points AS
SELECT
    id,
    osm_relation_id,
    name,
    end_point::geometry(Point, 4326) AS geom
FROM trails
WHERE end_point IS NOT NULL;
