-- Actual coordinates for the graph-analysis markers stitch.rs already
-- computes internally (open ends, branch points, disconnected pieces)
-- but previously discarded after using them to decide how to walk.
-- This is what lets the map draw real dots at real locations, the same
-- idea as Waymarked Trails' "Analyze relation" markers.

CREATE TABLE stitch_diagnostic_points (
    id       UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    trail_id UUID NOT NULL REFERENCES trails(id) ON DELETE CASCADE,
    kind     TEXT NOT NULL,  -- 'open_end' | 'branch' | 'disconnected'
    geom     GEOMETRY(Point, 4326) NOT NULL
);

CREATE INDEX idx_stitch_diagnostic_points_trail ON stitch_diagnostic_points (trail_id);