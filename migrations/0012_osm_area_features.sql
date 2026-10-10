-- Preserve the OSM object type for POIs represented by ways/relations, and
-- expose one authoritative outdoor waypoint source to the map tile server.

ALTER TABLE waypoints
    ADD COLUMN osm_type CHAR(1),
    ADD COLUMN osm_id BIGINT;

UPDATE waypoints
SET osm_type = 'N',
    osm_id = osm_node_id
WHERE osm_node_id IS NOT NULL;

CREATE INDEX idx_waypoints_osm_identity ON waypoints (osm_type, osm_id);

ALTER TABLE regions
    ADD COLUMN boundary_kind TEXT NOT NULL DEFAULT 'administrative';

CREATE TABLE outdoor_waypoints (
    osm_type   CHAR(1) NOT NULL CHECK (osm_type IN ('N', 'W', 'R')),
    osm_id     BIGINT NOT NULL,
    kind       TEXT NOT NULL,
    name       TEXT,
    elevation_m DOUBLE PRECISION,
    geom       GEOMETRY(Point, 4326) NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (osm_type, osm_id)
);

CREATE INDEX idx_outdoor_waypoints_geom ON outdoor_waypoints USING GIST (geom);
CREATE INDEX idx_outdoor_waypoints_kind ON outdoor_waypoints (kind);
