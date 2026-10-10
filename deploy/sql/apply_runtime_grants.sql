\set ON_ERROR_STOP on

-- This file runs after every production migration. New tables are private by
-- default; only the exact serving surface below is granted to runtime roles.
BEGIN;

ALTER DEFAULT PRIVILEGES IN SCHEMA public
    REVOKE SELECT ON TABLES FROM hiking_api, hiking_tiles;

DO $grants$
DECLARE
    object_record RECORD;
    table_name TEXT;
    api_tables CONSTANT TEXT[] := ARRAY[
        'country_bounds',
        'elevation_profiles',
        'places',
        'regions',
        'stitch_diagnostic_points',
        'stitch_warnings',
        'trail_features',
        'trail_surface_breakdown',
        'trails',
        'waypoints'
    ];
    tile_tables CONSTANT TEXT[] := ARRAY[
        'outdoor_waypoints',
        'trail_end_points',
        'trail_lines',
        'trail_start_points'
    ];
BEGIN
    IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'hiking_api')
       OR NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'hiking_tiles') THEN
        RAISE EXCEPTION 'production runtime roles are missing';
    END IF;

    -- Revoke prior broad grants from every application object owned by the
    -- migration role. Extension-owned objects are intentionally left alone.
    FOR object_record IN
        SELECT namespace.nspname AS schema_name, relation.relname AS object_name
        FROM pg_class relation
        JOIN pg_namespace namespace ON namespace.oid = relation.relnamespace
        WHERE namespace.nspname = 'public'
          AND relation.relkind IN ('r', 'p', 'v', 'm', 'f')
          AND relation.relowner = (SELECT oid FROM pg_roles WHERE rolname = current_user)
    LOOP
        EXECUTE format(
            'REVOKE ALL PRIVILEGES ON TABLE %I.%I FROM hiking_api, hiking_tiles',
            object_record.schema_name,
            object_record.object_name
        );
    END LOOP;

    FOREACH table_name IN ARRAY api_tables LOOP
        IF to_regclass(format('public.%I', table_name)) IS NULL THEN
            RAISE EXCEPTION 'required API table public.% is missing', table_name;
        END IF;
        EXECUTE format('GRANT SELECT ON TABLE public.%I TO hiking_api', table_name);
    END LOOP;

    FOREACH table_name IN ARRAY tile_tables LOOP
        IF to_regclass(format('public.%I', table_name)) IS NULL THEN
            RAISE EXCEPTION 'required tile table public.% is missing', table_name;
        END IF;
        EXECUTE format('GRANT SELECT ON TABLE public.%I TO hiking_tiles', table_name);
    END LOOP;
END
$grants$;

COMMIT;
