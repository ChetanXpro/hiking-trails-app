CREATE TABLE trail_landcover_segments (
    trail_id        UUID NOT NULL REFERENCES trails(id) ON DELETE CASCADE,
    sequence        INT NOT NULL,
    start_m         DOUBLE PRECISION NOT NULL,
    end_m           DOUBLE PRECISION NOT NULL,
    landcover_code  INT NOT NULL,
    landcover_label TEXT NOT NULL,
    dem_confidence  TEXT NOT NULL,
    PRIMARY KEY (trail_id, sequence)
);

CREATE INDEX idx_trail_landcover_segments_trail
    ON trail_landcover_segments (trail_id);
