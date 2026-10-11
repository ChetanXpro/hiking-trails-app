-- Run against a migrated development database with psql -v ON_ERROR_STOP=1.
-- The fixture and all mutations are rolled back.
BEGIN;
DO $$
DECLARE route_id uuid := gen_random_uuid();
BEGIN
    INSERT INTO trails(id, osm_relation_id, name, geom, distance_m, ascent_m, descent_m, estimated_duration_min)
    VALUES(route_id, -987654321, 'Enrichment regression fixture',
        ST_GeomFromText('MULTILINESTRING((77 32,77.01 32))',4326), 945, 100, 30, 25);
    INSERT INTO elevation_profiles(id, trail_id, min_elevation_m, max_elevation_m, profile_points)
    VALUES(gen_random_uuid(), route_id, 1000, 1100, '[{"pos_m":0,"ele_m":1000},{"pos_m":945,"ele_m":1100}]');
    INSERT INTO trail_landcover_segments VALUES(route_id,0,0,945,10,'Trees','high');

    UPDATE trails SET geom = geom, name = 'Same route' WHERE id = route_id;
    IF NOT EXISTS(SELECT FROM elevation_profiles WHERE trail_id = route_id) THEN
        RAISE EXCEPTION 'identical geometry discarded a valid profile';
    END IF;

    -- Reversal keeps length identical but makes the old profile incorrect.
    UPDATE trails SET geom = ST_Reverse(geom) WHERE id = route_id;
    IF EXISTS(SELECT FROM elevation_profiles WHERE trail_id = route_id)
       OR EXISTS(SELECT FROM trail_landcover_segments WHERE trail_id = route_id)
       OR EXISTS(SELECT FROM trails WHERE id = route_id AND
           (ascent_m IS NOT NULL OR descent_m IS NOT NULL OR estimated_duration_min IS NOT NULL)) THEN
        RAISE EXCEPTION 'changed geometry retained stale enrichment';
    END IF;
END $$;
ROLLBACK;
