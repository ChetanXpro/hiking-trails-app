ALTER TABLE trails
    ADD COLUMN summary TEXT,
    ADD COLUMN hike_type TEXT,
    ADD COLUMN estimated_duration_min INT,
    ADD COLUMN source_kind TEXT NOT NULL DEFAULT 'osm',
    ADD COLUMN start_point GEOMETRY(Point, 4326),
    ADD COLUMN end_point GEOMETRY(Point, 4326);

CREATE INDEX idx_trails_start_point ON trails USING GIST (start_point);
CREATE INDEX idx_trails_end_point   ON trails USING GIST (end_point);

CREATE TABLE trail_surface_breakdown (
    trail_id      UUID NOT NULL REFERENCES trails(id) ON DELETE CASCADE,
    surface_type  TEXT NOT NULL,
    distance_m    DOUBLE PRECISION NOT NULL,
    pct           DOUBLE PRECISION NOT NULL,
    PRIMARY KEY (trail_id, surface_type)
);

CREATE TABLE trail_features (
    trail_id      UUID NOT NULL REFERENCES trails(id) ON DELETE CASCADE,
    feature_slug  TEXT NOT NULL,
    source        TEXT NOT NULL DEFAULT 'derived',
    PRIMARY KEY (trail_id, feature_slug)
);

CREATE INDEX idx_trail_features_slug ON trail_features (feature_slug);
