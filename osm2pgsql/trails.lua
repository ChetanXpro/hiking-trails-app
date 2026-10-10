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
    { column = 'tags',   type = 'jsonb', not_null = true },
    { column = 'geom',   type = 'linestring', projection = srid, not_null = true },
})

local raw_relations = osm2pgsql.define_relation_table('raw_relations', {
    { column = 'osm_id',     type = 'bigint', not_null = true },
    { column = 'tags',       type = 'jsonb', not_null = true },
    { column = 'members',    type = 'jsonb', not_null = true },
})

local raw_regions = osm2pgsql.define_relation_table('raw_regions', {
    { column = 'osm_id', type = 'bigint', not_null = true },
    { column = 'tags',   type = 'jsonb', not_null = true },
    { column = 'geom',   type = 'multipolygon', projection = srid, not_null = true },
})

-- Outdoor POIs and named places can be mapped as nodes, closed ways, or
-- multipolygon relations. A generic `any` id preserves that OSM identity
-- while the representative point keeps downstream proximity/search queries
-- simple.
local raw_waypoints = osm2pgsql.define_table({
    name = 'raw_waypoints',
    ids = { type = 'any', type_column = 'osm_type', id_column = 'osm_id' },
    columns = {
    { column = 'tags',   type = 'jsonb', not_null = true },
    { column = 'geom',   type = 'point', projection = srid, not_null = true },
    },
})

local raw_places = osm2pgsql.define_table({
    name = 'raw_places',
    ids = { type = 'any', type_column = 'osm_type', id_column = 'osm_id' },
    columns = {
    { column = 'tags',   type = 'jsonb', not_null = true },
    { column = 'geom',   type = 'point', projection = srid, not_null = true },
    },
})

-- Keep this list in sync with the tag list in the `osmium tags-filter`
-- command in the README.
local WAYPOINT_TAGS = {
    { key = 'tourism', value = 'alpine_hut' },
    { key = 'tourism', value = 'wilderness_hut' },
    { key = 'tourism', value = 'camp_site' },
    { key = 'tourism', value = 'viewpoint' },
    { key = 'tourism', value = 'picnic_site' },
    { key = 'tourism', value = 'information' },
    { key = 'amenity', value = 'shelter' },
    { key = 'amenity', value = 'drinking_water' },
    { key = 'amenity', value = 'toilets' },
    { key = 'amenity', value = 'parking' },
    { key = 'amenity', value = 'ranger_station' },
    { key = 'amenity', value = 'hospital' },
    { key = 'amenity', value = 'clinic' },
    { key = 'emergency', value = 'phone' },
    { key = 'emergency', value = 'first_aid_kit' },
    { key = 'emergency', value = 'defibrillator' },
    { key = 'emergency', value = 'mountain_rescue' },
    { key = 'highway', value = 'trailhead' },
    { key = 'natural', value = 'peak' },
    { key = 'natural', value = 'spring' },
    { key = 'natural', value = 'waterfall' },
    { key = 'natural', value = 'cave_entrance' },
    { key = 'natural', value = 'saddle' },
    { key = 'mountain_pass', value = 'yes' },
    { key = 'aerialway', value = 'station' },
}

local PLACE_VALUES = {
    city = true,
    town = true,
    village = true,
    hamlet = true,
    suburb = true,
    neighbourhood = true,
    locality = true,
    isolated_dwelling = true,
}

local ADMIN_LEVELS = {
    ['2'] = true,
    ['3'] = true,
    ['4'] = true,
    ['5'] = true,
    ['6'] = true,
    ['7'] = true,
    ['8'] = true,
}

local function is_waypoint(tags)
    for _, rule in ipairs(WAYPOINT_TAGS) do
        if tags[rule.key] == rule.value then
            return true
        end
    end
    return false
end

local function is_named_place(tags)
    return tags.name and PLACE_VALUES[tags.place] == true
end

local function is_admin_boundary(tags)
    return tags.name and tags.boundary == 'administrative' and ADMIN_LEVELS[tags.admin_level] == true
end

local function is_protected_area(tags)
    return tags.name and tags.boundary == 'protected_area'
end

local function insert_point_features(object, point)
    if point:is_null() then
        return
    end

    if is_waypoint(object.tags) then
        raw_waypoints:insert({
            tags = object.tags,
            geom = point,
        })
    end

    if is_named_place(object.tags) then
        raw_places:insert({
            tags = object.tags,
            geom = point,
        })
    end
end

local function is_main_route_member_role(role)
    return role == nil or role == '' or role == 'forward' or role == 'backward'
end

-- Flex stage 1 sees ways before it sees their relations. Mark the supported
-- members of ordinary hiking routes for stage-2 reprocessing so an untagged
-- connector, bridge outline, or other legitimate non-highway member is still
-- available to the stitcher. This is much smaller than retaining every way in
-- the regional PBF.
function osm2pgsql.select_relation_members(relation)
    if relation.tags.type ~= 'route' or relation.tags.route ~= 'hiking' then
        return
    end

    local ways = {}
    for _, member in ipairs(relation.members) do
        if member.type == 'w' and is_main_route_member_role(member.role) then
            table.insert(ways, member.ref)
        end
    end

    return { ways = ways }
end

function osm2pgsql.process_node(object)
    insert_point_features(object, object:as_point())
end

function osm2pgsql.process_way(object)
    if object.is_closed and (is_waypoint(object.tags) or is_named_place(object.tags)) then
        insert_point_features(object, object:as_polygon():centroid())
    end

    -- Keep the raw staging layer broad: hiking relations often include
    -- ordinary road segments, steps, bridges, and other `highway=*`
    -- members in addition to classic `path`/`track` ways. If we prune
    -- too early here, the stitcher never even sees those relation
    -- members and the final route becomes artificially shorter.
    if object.tags.highway or osm2pgsql.stage == 2 then
        raw_ways:insert({
            tags = object.tags,
            geom = object:as_linestring(),
        })
    end
end

function osm2pgsql.process_relation(object)
    if object.tags.type == 'multipolygon'
        and (is_waypoint(object.tags) or is_named_place(object.tags)) then
        insert_point_features(object, object:as_multipolygon():centroid())
    end

    -- State extracts deliberately stage ordinary routes only. Completing and
    -- publishing a country-scale superroute from one intersecting state would
    -- create misleading coverage and unnecessary storage; national imports
    -- can opt into those through a dedicated full-country pipeline later.
    if object.tags.type == 'route' and object.tags.route == 'hiking' then
        local members = {}
        for _, member in ipairs(object.members) do
            table.insert(members, {
                type = member.type,
                ref = member.ref,
                role = member.role or '',
            })
        end

        if #members > 0 then
            raw_relations:insert({
                osm_id = object.id,
                tags = object.tags,
                members = members,
            })
        end
    end

    if is_admin_boundary(object.tags) or is_protected_area(object.tags) then
        raw_regions:insert({
            osm_id = object.id,
            tags = object.tags,
            geom = object:as_multipolygon(),
        })
    end
end
