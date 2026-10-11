use actix_web::{error, get, web, HttpResponse};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
struct DiscoveryQuery {
    bbox: String,
    max_distance_km: Option<f64>,
    max_ascent_m: Option<f64>,
    difficulty: Option<String>,
    loops_only: Option<bool>,
    sort: Option<String>,
    offset: Option<i64>,
}

#[derive(FromRow, Serialize)]
struct DiscoveryTrail {
    id: Uuid,
    name: Option<String>,
    difficulty: Option<String>,
    distance_m: Option<f64>,
    ascent_m: Option<f64>,
    estimated_duration_min: Option<i32>,
    hike_type: Option<String>,
    region_name: Option<String>,
    incomplete: bool,
}

fn parse_bounds(value: &str) -> Result<[f64; 4], &'static str> {
    let values: Vec<f64> = value
        .split(',')
        .map(str::parse)
        .collect::<Result<_, _>>()
        .map_err(|_| "bbox must contain west,south,east,north")?;
    let bounds: [f64; 4] = values
        .try_into()
        .map_err(|_| "bbox requires four coordinates")?;
    let [west, south, east, north] = bounds;
    if bounds.iter().any(|v| !v.is_finite())
        || !(-180.0..=180.0).contains(&west)
        || !(-180.0..=180.0).contains(&east)
        || !(-90.0..=90.0).contains(&south)
        || !(-90.0..=90.0).contains(&north)
        || south >= north
        || west == east
    {
        return Err("bbox is outside geographic bounds or has no area");
    }
    Ok(bounds)
}

#[get("/trails")]
pub(super) async fn discover_trails(
    pool: web::Data<PgPool>,
    query: web::Query<DiscoveryQuery>,
) -> actix_web::Result<HttpResponse> {
    let [west, south, east, north] = parse_bounds(&query.bbox).map_err(error::ErrorBadRequest)?;
    for value in [query.max_distance_km, query.max_ascent_m]
        .into_iter()
        .flatten()
    {
        if !value.is_finite() || value <= 0.0 {
            return Err(error::ErrorBadRequest(
                "filter limits must be positive finite numbers",
            ));
        }
    }
    let difficulty = query.difficulty.as_deref().unwrap_or("all");
    if !["all", "hiking", "mountain", "alpine", "unrated"].contains(&difficulty) {
        return Err(error::ErrorBadRequest("unknown difficulty filter"));
    }
    let sort = query.sort.as_deref().unwrap_or("nearest");
    if !["nearest", "shortest", "longest", "ascent", "name"].contains(&sort) {
        return Err(error::ErrorBadRequest("unknown sort order"));
    }
    let center_lon = if west <= east {
        (west + east) / 2.0
    } else {
        ((west + east + 360.0) / 2.0 + 180.0).rem_euclid(360.0) - 180.0
    };
    let center_lat = (south + north) / 2.0;
    let offset = query.offset.unwrap_or(0);
    if !(0..=10_000).contains(&offset) {
        return Err(error::ErrorBadRequest("invalid offset"));
    }
    // PostGIS uses the GiST geometry index for the viewport; exact intersection
    // excludes routes whose bounding boxes alone overlap. Split at the dateline.
    let mut trails = sqlx::query_as::<_, DiscoveryTrail>(r#"
        WITH viewport AS (
            SELECT CASE WHEN $1 <= $3 THEN ST_MakeEnvelope($1,$2,$3,$4,4326)
                ELSE ST_Union(ST_MakeEnvelope($1,$2,180,$4,4326),
                              ST_MakeEnvelope(-180,$2,$3,$4,4326)) END AS geom
        )
        SELECT t.id, t.name, t.difficulty, t.distance_m, t.ascent_m,
               t.estimated_duration_min, t.hike_type, r.name AS region_name,
               EXISTS (SELECT 1 FROM stitch_warnings w WHERE w.trail_id = t.id AND
                   (w.message LIKE '%relation splits into%' OR
                    w.message LIKE '%could not be reached by a simple walk%' OR
                    w.message LIKE '%relation member way(s) were missing%')) AS incomplete
        FROM trails t CROSS JOIN viewport v LEFT JOIN regions r ON r.id = t.region_id
        WHERE t.geom && v.geom AND ST_Intersects(t.geom, v.geom)
          AND ($5::double precision IS NULL OR t.distance_m <= $5 * 1000)
          AND ($6::double precision IS NULL OR t.ascent_m <= $6)
          AND ($7 = 'all' OR ($7 = 'hiking' AND t.difficulty = 'hiking')
               OR ($7 = 'mountain' AND t.difficulty IN ('mountain_hiking','demanding_mountain_hiking'))
               OR ($7 = 'alpine' AND t.difficulty IN ('alpine_hiking','demanding_alpine_hiking','difficult_alpine_hiking'))
               OR ($7 = 'unrated' AND (t.difficulty IS NULL OR t.difficulty NOT IN
                    ('hiking','mountain_hiking','demanding_mountain_hiking','alpine_hiking','demanding_alpine_hiking','difficult_alpine_hiking'))))
          AND (NOT $8 OR t.hike_type = 'loop')
        ORDER BY CASE WHEN $9 = 'nearest' THEN ST_Distance(t.start_point::geography, ST_SetSRID(ST_MakePoint($11, $12), 4326)::geography) END ASC NULLS LAST,
                 CASE WHEN $9 = 'shortest' THEN t.distance_m END ASC NULLS LAST,
                 CASE WHEN $9 = 'longest' THEN t.distance_m END DESC NULLS LAST,
                 CASE WHEN $9 = 'ascent' THEN t.ascent_m END ASC NULLS LAST,
                 t.name ASC NULLS LAST, t.id
        LIMIT 25 OFFSET $10
    "#)
    .bind(west).bind(south).bind(east).bind(north)
    .bind(query.max_distance_km).bind(query.max_ascent_m).bind(difficulty)
    .bind(query.loops_only.unwrap_or(false)).bind(sort).bind(offset).bind(center_lon).bind(center_lat)
    .fetch_all(pool.get_ref()).await
    .map_err(|e| super::internal_error("discovering trails", e))?;
    let has_more = trails.len() > 24;
    trails.truncate(24);
    Ok(super::cached_json(
        serde_json::json!({
            "trails": trails, "next_offset": if has_more { Some(offset + 24) } else { None }
        }),
        super::CACHE_SEARCH,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_bounds_including_dateline() {
        assert_eq!(
            parse_bounds("170,-10,-170,10").unwrap(),
            [170.0, -10.0, -170.0, 10.0]
        );
        for bad in [
            "",
            "0,0,1",
            "0,0,1,2,3",
            "NaN,0,1,2",
            "0,2,1,1",
            "181,0,1,2",
            "0,-91,1,2",
            "0,0,0,1",
        ] {
            assert!(parse_bounds(bad).is_err(), "{bad}");
        }
    }
}
