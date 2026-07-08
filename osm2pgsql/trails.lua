-- osm2pgsql flex-output config: RAW STAGING IMPORT.
--
-- This does the first half of the pipeline: pull hiking-relevant ways
-- and relations straight out of the filtered .osm.pbf into two staging
-- tables (raw_ways, raw_relations). It does NOT stitch a relation's
-- member ways into an ordered trail route yet -- that stitching step
-- reads from these staging tables afterwards (in SQL or Rust) and
-- populates the real `trails` / `segments` tables from migration 0001.
--
-- Run with:
--   osm2pgsql -d hiking --slim --output=flex --style=osm2pgsql/trails.lua hp-trails.osm.pbf
--
-- Note: flex output does NOT support the -l/--latlong command-line
-- flag (that's legacy pgsql-output only) -- projection is set here,
-- per column, via `projection = 4326` below. Our schema is 4326
-- throughout, so this must match.

local srid = 4326

local raw_ways = osm2pgsql.define_way_table('raw_ways', {
    { column = 'osm_id', type = 'bigint', not_null = true },
    { column = 'tags',   type = 'jsonb', not_null = true },
    { column = 'geom',   type = 'linestring', projection = srid, not_null = true },
})

local raw_relations = osm2pgsql.define_relation_table('raw_relations', {
    { column = 'osm_id',     type = 'bigint', not_null = true },
    { column = 'tags',       type = 'jsonb', not_null = true },
    { column = 'member_ids', type = 'jsonb', not_null = true },
})

-- Waypoints: huts, springs, campsites, peaks -- these come from OSM
-- *nodes*, a completely separate code path from ways/relations above.
local raw_waypoints = osm2pgsql.define_node_table('raw_waypoints', {
    { column = 'osm_id', type = 'bigint', not_null = true },
    { column = 'tags',   type = 'jsonb', not_null = true },
    { column = 'geom',   type = 'point', projection = srid, not_null = true },
})

-- Keep this list in sync with the tag list in the `osmium tags-filter`
-- command in the README.
local WAYPOINT_TAGS = {
    { key = 'tourism', value = 'alpine_hut' },
    { key = 'tourism', value = 'wilderness_hut' },
    { key = 'tourism', value = 'camp_site' },
    { key = 'natural', value = 'peak' },
    { key = 'natural', value = 'spring' },
}

local function is_waypoint(tags)
    for _, rule in ipairs(WAYPOINT_TAGS) do
        if tags[rule.key] == rule.value then
            return true
        end
    end
    return false
end

function osm2pgsql.process_node(object)
    if is_waypoint(object.tags) then
        raw_waypoints:insert({
            osm_id = object.id,
            tags = object.tags,
            geom = object:as_point(),
        })
    end
end

-- Tags we care about for path/track geometry.
local TRAIL_HIGHWAY_TYPES = {
    path = true,
    footway = true,
    track = true,
    residential = true,
}

function osm2pgsql.process_way(object)
    if object.tags.highway and TRAIL_HIGHWAY_TYPES[object.tags.highway] then
        raw_ways:insert({
            osm_id = object.id,
            tags = object.tags,
            geom = object:as_linestring(),
        })
    end
end

function osm2pgsql.process_relation(object)
    if object.tags.route == 'hiking' then
        local member_ids = {}
        for _, member in ipairs(object.members) do
            if member.type == 'w' then
                table.insert(member_ids, member.ref)
            end
        end

        if #member_ids > 0 then
            raw_relations:insert({
                osm_id = object.id,
                tags = object.tags,
                member_ids = member_ids,
            })
        end
    end
end