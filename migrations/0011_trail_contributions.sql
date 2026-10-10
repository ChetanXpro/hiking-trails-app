CREATE TABLE contributors (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    provider          TEXT NOT NULL,
    provider_user_id  TEXT NOT NULL,
    display_name      TEXT NOT NULL,
    profile_url       TEXT,
    metadata          JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (provider, provider_user_id)
);

CREATE INDEX idx_contributors_display_name_trgm
    ON contributors USING GIN (display_name gin_trgm_ops);

CREATE TABLE trail_contributions (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    trail_id            UUID NOT NULL REFERENCES trails(id) ON DELETE CASCADE,
    contributor_id      UUID NOT NULL REFERENCES contributors(id) ON DELETE CASCADE,
    contribution_kind   TEXT NOT NULL,
    attribution_basis   TEXT NOT NULL,
    source_kind         TEXT NOT NULL,
    osm_object_type     TEXT,
    osm_object_id       BIGINT,
    osm_version         INTEGER,
    osm_changeset_id    BIGINT,
    effective_at        TIMESTAMPTZ,
    observed_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    distance_m          DOUBLE PRECISION,
    segment_count       INTEGER,
    impact_score        DOUBLE PRECISION NOT NULL DEFAULT 0,
    metadata            JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_trail_contributions_trail
    ON trail_contributions (trail_id, contribution_kind);
CREATE INDEX idx_trail_contributions_contributor
    ON trail_contributions (contributor_id, contribution_kind);
CREATE INDEX idx_trail_contributions_osm_object
    ON trail_contributions (osm_object_type, osm_object_id);
CREATE INDEX idx_trail_contributions_effective_at
    ON trail_contributions (effective_at DESC NULLS LAST);
CREATE INDEX idx_trail_contributions_basis
    ON trail_contributions (attribution_basis, source_kind);

CREATE VIEW contributor_leaderboard AS
SELECT
    contributors.id AS contributor_id,
    contributors.provider,
    contributors.provider_user_id,
    contributors.display_name,
    contributors.profile_url,
    COUNT(*) AS contribution_events,
    COUNT(DISTINCT trail_contributions.trail_id) AS trail_count,
    COUNT(DISTINCT CASE
        WHEN trail_contributions.contribution_kind = 'relation_curation' THEN trail_contributions.trail_id
        ELSE NULL
    END) AS curated_trail_count,
    COALESCE(SUM(CASE
        WHEN trail_contributions.contribution_kind = 'way_mapping' THEN COALESCE(trail_contributions.distance_m, 0)
        ELSE 0
    END), 0) AS mapped_distance_m,
    COALESCE(SUM(trail_contributions.impact_score), 0) AS impact_score,
    MAX(trail_contributions.effective_at) AS last_contribution_at
FROM contributors
JOIN trail_contributions ON trail_contributions.contributor_id = contributors.id
GROUP BY
    contributors.id,
    contributors.provider,
    contributors.provider_user_id,
    contributors.display_name,
    contributors.profile_url;
