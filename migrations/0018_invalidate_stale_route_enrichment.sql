-- Elevation, duration and land cover describe an ordered geometry. Never keep
-- them when that geometry changes (including a reversal of the route).
CREATE FUNCTION invalidate_changed_route_enrichment() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF ST_AsEWKB(OLD.geom) IS DISTINCT FROM ST_AsEWKB(NEW.geom) THEN
        DELETE FROM elevation_profiles WHERE trail_id = NEW.id;
        DELETE FROM trail_landcover_segments WHERE trail_id = NEW.id;
        NEW.ascent_m := NULL;
        NEW.descent_m := NULL;
        NEW.estimated_duration_min := NULL;
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER invalidate_route_enrichment
BEFORE UPDATE OF geom ON trails
FOR EACH ROW EXECUTE FUNCTION invalidate_changed_route_enrichment();

-- Repair provably incompatible legacy profiles. Allow normal differences
-- between geodesic route length and the sampled profile's distance model.
WITH invalid AS (
    DELETE FROM elevation_profiles e USING trails t
    WHERE e.trail_id = t.id
      AND abs((e.profile_points -> -1 ->> 'pos_m')::double precision - t.distance_m)
          > greatest(100, t.distance_m * 0.05)
    RETURNING e.trail_id
), removed_landcover AS (
    DELETE FROM trail_landcover_segments WHERE trail_id IN (SELECT trail_id FROM invalid)
)
UPDATE trails SET ascent_m = NULL, descent_m = NULL, estimated_duration_min = NULL
WHERE id IN (SELECT trail_id FROM invalid);
