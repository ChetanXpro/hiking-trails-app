-- Persists the warnings stitch.rs already computes (disconnected
-- pieces, branch points, stranded ways) instead of only printing them
-- to the terminal during `cargo run -p hiking-import` and losing them
-- immediately after. This is a QA/data-quality table -- not something
-- shown to hikers on the main map, which deliberately stays free of
-- OSM-internal debugging detail.

CREATE TABLE stitch_warnings (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    trail_id   UUID NOT NULL REFERENCES trails(id) ON DELETE CASCADE,
    message    TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_stitch_warnings_trail ON stitch_warnings (trail_id);