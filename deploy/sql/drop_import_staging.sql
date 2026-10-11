\set ON_ERROR_STOP on

-- DESTRUCTIVE, but intentionally limited to regenerable osm2pgsql/import
-- staging. Run this only after hiking-import exits successfully and the map/API
-- have been checked. The next snapshot import recreates these tables.
BEGIN;

DO $$
BEGIN
    IF to_regclass('public.trails') IS NULL
       OR NOT EXISTS (SELECT 1 FROM trails LIMIT 1) THEN
        RAISE EXCEPTION 'refusing cleanup: normalized trails are missing or empty';
    END IF;
END
$$;

DROP TABLE IF EXISTS
    raw_waypoints,
    raw_places,
    raw_regions,
    raw_relations,
    raw_ways,
    planet_osm_rels,
    planet_osm_ways,
    planet_osm_nodes,
    osm2pgsql_properties;

COMMIT;
