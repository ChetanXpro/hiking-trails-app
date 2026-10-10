// The stitching problem: a hiking relation's member ways are given by
// OSM in a declared order that isn't always correct (open ends,
// unsorted segments -- exactly what you saw in Waymarked Trails' own
// "Analyze relation" panel). We rebuild the real order from the
// geometry itself instead of trusting it, AND we now keep the actual
// coordinates of every open end / branch / disconnected point we find,
// instead of discarding them the moment the walk decision is made --
// that's what lets the map draw real markers at real locations, same
// idea as WMT's red/blue/grey dots.

use std::collections::{HashMap, HashSet};

const SNAP_TOLERANCE_METERS: f64 = 3.0;

struct NodeAssigner {
    nodes: Vec<(f64, f64)>,
    buckets: HashMap<(i32, i32), Vec<usize>>,
}

impl NodeAssigner {
    fn new() -> Self {
        Self {
            nodes: Vec::new(),
            buckets: HashMap::new(),
        }
    }

    fn bucket_key(coord: (f64, f64)) -> (i32, i32) {
        let lat_rad = coord.1.to_radians();
        let cell_size_deg_lat = SNAP_TOLERANCE_METERS / 111_320.0;
        let cell_size_deg_lon = SNAP_TOLERANCE_METERS / (111_320.0 * lat_rad.cos().max(0.01));
        (
            (coord.0 / cell_size_deg_lon).floor() as i32,
            (coord.1 / cell_size_deg_lat).floor() as i32,
        )
    }

    fn node_id_for(&mut self, coord: (f64, f64)) -> usize {
        let (bucket_x, bucket_y) = Self::bucket_key(coord);
        let lat_rad = coord.1.to_radians();
        let lon_reach = (1.0 / lat_rad.cos().max(0.01)).ceil() as i32;
        for neighbor_x in (bucket_x - lon_reach)..=(bucket_x + lon_reach) {
            for neighbor_y in (bucket_y - 1)..=(bucket_y + 1) {
                if let Some(candidate_ids) = self.buckets.get(&(neighbor_x, neighbor_y)) {
                    for &id in candidate_ids {
                        if crate::approx_distance_m(self.nodes[id], coord) <= SNAP_TOLERANCE_METERS
                        {
                            return id;
                        }
                    }
                }
            }
        }

        let node_id = self.nodes.len();
        self.nodes.push(coord);
        self.buckets
            .entry((bucket_x, bucket_y))
            .or_default()
            .push(node_id);
        node_id
    }
}

#[derive(Debug, Clone)]
pub struct WayRecord {
    pub osm_way_id: i64,
    pub start: (f64, f64),
    pub end: (f64, f64),
    pub member_index: usize,
    pub role: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticKind {
    OpenEnd,
    Branch,
    Disconnected,
}

impl DiagnosticKind {
    #[allow(dead_code)]
    pub fn as_str(&self) -> &'static str {
        match self {
            DiagnosticKind::OpenEnd => "open_end",
            DiagnosticKind::Branch => "branch",
            DiagnosticKind::Disconnected => "disconnected",
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct DiagnosticPoint {
    pub kind: DiagnosticKind,
    pub coord: (f64, f64),
}

#[derive(Debug)]
pub struct StitchResult {
    pub ordered_ways: Vec<(i64, bool)>,
    pub hike_type: String,
    pub default_direction_mode: String,
    pub direction_source: String,
    pub warnings: Vec<String>,
    #[allow(dead_code)]
    pub diagnostic_points: Vec<DiagnosticPoint>,
}

#[derive(Debug)]
struct WalkAttempt {
    ordered_ways: Vec<(i64, bool)>,
    warnings: Vec<String>,
    stranded: usize,
    order_penalty: usize,
    role_mismatches: usize,
}

fn preferred_reversed(role: &str) -> Option<bool> {
    match role.trim().to_ascii_lowercase().as_str() {
        "forward" => Some(false),
        "backward" => Some(true),
        _ => None,
    }
}

fn circular_member_distance(previous: usize, current: usize, total_members: usize) -> usize {
    if total_members <= 1 {
        return 0;
    }

    if current >= previous {
        current - previous
    } else {
        (total_members - previous) + current
    }
}

fn walk_component(
    start_node: usize,
    adjacency: &HashMap<usize, Vec<usize>>,
    edges: &[(usize, usize, usize)],
    degree: &HashMap<usize, usize>,
    ways: &[WayRecord],
    total_members: usize,
) -> WalkAttempt {
    let mut used = vec![false; edges.len()];
    let mut ordered_ways = Vec::new();
    let mut warnings = Vec::new();
    let mut current = start_node;
    let mut previous_member_index: Option<usize> = None;
    let mut order_penalty = 0usize;
    let mut role_mismatches = 0usize;
    let mut encountered_branch = false;

    while let Some(candidates) = adjacency.get(&current) {
        let mut candidate_edges = candidates
            .iter()
            .copied()
            .filter(|&edge_idx| !used[edge_idx])
            .collect::<Vec<_>>();

        if candidate_edges.is_empty() {
            break;
        }

        candidate_edges.sort_by_key(|&edge_idx| {
            let (a, _, way_idx) = edges[edge_idx];
            let reversed = a != current;
            let way = &ways[way_idx];
            let role_penalty = match preferred_reversed(&way.role) {
                Some(preferred) if preferred != reversed => 1usize,
                _ => 0usize,
            };
            let relation_penalty = match previous_member_index {
                Some(previous) => {
                    circular_member_distance(previous, way.member_index, total_members)
                }
                None => way.member_index,
            };

            (role_penalty, relation_penalty, way.member_index, edge_idx)
        });

        let edge_idx = candidate_edges[0];
        used[edge_idx] = true;

        let (a, b, way_idx) = edges[edge_idx];
        let reversed = a != current;
        let way = &ways[way_idx];

        if let Some(previous) = previous_member_index {
            order_penalty += circular_member_distance(previous, way.member_index, total_members);
        } else {
            order_penalty += way.member_index;
        }

        if let Some(preferred) = preferred_reversed(&way.role) {
            if preferred != reversed {
                role_mismatches += 1;
            }
        }

        ordered_ways.push((way.osm_way_id, reversed));
        previous_member_index = Some(way.member_index);
        current = if a == current { b } else { a };

        if degree.get(&current).copied().unwrap_or(0) >= 3 {
            encountered_branch = true;
        }
    }

    if encountered_branch {
        warnings.push(
            "branch point at a shared node -- this route has a junction, not a simple path; took the first available continuation rather than guessing the intended branch".to_string()
        );
    }

    let stranded = adjacency
        .values()
        .flat_map(|edge_indexes| edge_indexes.iter())
        .copied()
        .collect::<HashSet<_>>()
        .into_iter()
        .filter(|&edge_idx| !used[edge_idx])
        .count();

    if stranded > 0 {
        warnings.push(format!(
            "{stranded} way(s) in this relation could not be reached by a simple walk from the start -- likely a spur or out-and-back branch that needs manual review"
        ));
    }

    WalkAttempt {
        ordered_ways,
        warnings,
        stranded,
        order_penalty,
        role_mismatches,
    }
}

pub fn stitch_ways(ways: &[WayRecord], had_direction_roles: bool) -> StitchResult {
    let mut warnings = Vec::new();

    let mut assigner = NodeAssigner::new();
    let mut edges: Vec<(usize, usize, usize)> = Vec::new();
    let mut degree: HashMap<usize, usize> = HashMap::new();

    for (i, way) in ways.iter().enumerate() {
        let a = assigner.node_id_for(way.start);
        let b = assigner.node_id_for(way.end);
        edges.push((a, b, i));
        *degree.entry(a).or_insert(0) += 1;
        *degree.entry(b).or_insert(0) += 1;
    }

    let mut parent: HashMap<usize, usize> = HashMap::new();
    let mut rank: HashMap<usize, usize> = HashMap::new();

    fn find(parent: &mut HashMap<usize, usize>, x: usize) -> usize {
        let mut current = x;
        let mut path = Vec::new();

        loop {
            let next = *parent.entry(current).or_insert(current);
            if next == current {
                break;
            }
            path.push(current);
            current = next;
        }

        for node in path {
            parent.insert(node, current);
        }

        current
    }

    fn union(
        parent: &mut HashMap<usize, usize>,
        rank: &mut HashMap<usize, usize>,
        a: usize,
        b: usize,
    ) {
        let ra = find(parent, a);
        let rb = find(parent, b);
        if ra == rb {
            return;
        }

        let rank_a = *rank.entry(ra).or_insert(0);
        let rank_b = *rank.entry(rb).or_insert(0);

        if rank_a < rank_b {
            parent.insert(ra, rb);
        } else if rank_a > rank_b {
            parent.insert(rb, ra);
        } else {
            parent.insert(rb, ra);
            rank.insert(ra, rank_a + 1);
        }
    }

    for &(a, b, _) in &edges {
        union(&mut parent, &mut rank, a, b);
    }

    let mut components: HashMap<usize, Vec<usize>> = HashMap::new();
    for (idx, &(a, _, _)) in edges.iter().enumerate() {
        let root = find(&mut parent, a);
        components.entry(root).or_default().push(idx);
    }

    if components.len() > 1 {
        warnings.push(format!(
            "relation splits into {} disconnected pieces -- likely a member way is missing or mistagged in OSM; stitching only the largest piece, the rest are left out of this trail's geometry",
            components.len()
        ));
    }

    let (main_root, main_component) = components
        .iter()
        .max_by_key(|(_, component)| {
            // Stable ties keep the same route geometry across repeated imports.
            let first_member = component
                .iter()
                .map(|&i| ways[edges[i].2].member_index)
                .min()
                .unwrap_or(usize::MAX);
            (component.len(), std::cmp::Reverse(first_member))
        })
        .map(|(&root, edges)| (root, edges.clone()))
        .unwrap_or((0, Vec::new()));

    if main_component.is_empty() {
        return StitchResult {
            ordered_ways: Vec::new(),
            hike_type: "unknown".to_string(),
            default_direction_mode: "forward".to_string(),
            direction_source: "topology_fallback".to_string(),
            warnings,
            diagnostic_points: Vec::new(),
        };
    }

    let mut diagnostic_points = Vec::new();

    let main_nodes: HashSet<usize> = main_component
        .iter()
        .flat_map(|&e| {
            let (a, b, _) = edges[e];
            [a, b]
        })
        .collect();

    for &node in &main_nodes {
        match degree.get(&node).copied().unwrap_or(0) {
            1 => diagnostic_points.push(DiagnosticPoint {
                kind: DiagnosticKind::OpenEnd,
                coord: assigner.nodes[node],
            }),
            d if d >= 3 => diagnostic_points.push(DiagnosticPoint {
                kind: DiagnosticKind::Branch,
                coord: assigner.nodes[node],
            }),
            _ => {}
        }
    }

    for (&root, edge_list) in &components {
        if root == main_root {
            continue;
        }
        let nodes: HashSet<usize> = edge_list
            .iter()
            .flat_map(|&e| {
                let (a, b, _) = edges[e];
                [a, b]
            })
            .collect();
        for &node in &nodes {
            if degree.get(&node).copied().unwrap_or(0) == 1 {
                diagnostic_points.push(DiagnosticPoint {
                    kind: DiagnosticKind::Disconnected,
                    coord: assigner.nodes[node],
                });
            }
        }
    }

    let mut adjacency: HashMap<usize, Vec<usize>> = HashMap::new();
    for &edge_idx in &main_component {
        let (a, b, _) = edges[edge_idx];
        adjacency.entry(a).or_default().push(edge_idx);
        adjacency.entry(b).or_default().push(edge_idx);
    }

    let odd_nodes = main_nodes
        .iter()
        .copied()
        .filter(|node| degree.get(node).copied().unwrap_or(0) % 2 == 1)
        .collect::<Vec<_>>();
    let odd_count = odd_nodes.len();

    let hike_type = if odd_count == 0 {
        "loop"
    } else if odd_count == 2 {
        "point_to_point"
    } else {
        "complex"
    };
    let anchor_way_idx = ways
        .iter()
        .enumerate()
        .min_by_key(|(_, way)| way.member_index)
        .map(|(idx, _)| idx)
        .unwrap_or(0);
    let anchor_edge_idx = main_component
        .iter()
        .copied()
        .find(|&edge_idx| edges[edge_idx].2 == anchor_way_idx)
        .unwrap_or(main_component[0]);
    let (anchor_a, anchor_b, _) = edges[anchor_edge_idx];
    let anchor_preferred_start = match preferred_reversed(&ways[anchor_way_idx].role) {
        Some(true) => anchor_b,
        _ => anchor_a,
    };
    let anchor_alternate_start = if anchor_preferred_start == anchor_a {
        anchor_b
    } else {
        anchor_a
    };

    let mut start_candidates = if odd_count > 0 {
        odd_nodes.clone()
    } else {
        Vec::new()
    };
    if start_candidates.is_empty() {
        start_candidates.push(anchor_preferred_start);
        if anchor_alternate_start != anchor_preferred_start {
            start_candidates.push(anchor_alternate_start);
        }
    } else if !start_candidates.contains(&anchor_preferred_start)
        && degree.get(&anchor_preferred_start).copied().unwrap_or(0) % 2 == 1
    {
        start_candidates.insert(0, anchor_preferred_start);
    }

    start_candidates.sort_unstable();
    start_candidates.dedup();

    let mut attempts = start_candidates
        .iter()
        .copied()
        .map(|candidate| {
            let attempt = walk_component(candidate, &adjacency, &edges, &degree, ways, ways.len());
            (candidate, attempt)
        })
        .collect::<Vec<_>>();

    attempts.sort_by_key(|(candidate, attempt)| {
        (
            attempt.stranded,
            attempt.role_mismatches,
            attempt.order_penalty,
            if *candidate == anchor_preferred_start {
                0usize
            } else {
                1usize
            },
        )
    });

    let (_, best_attempt) = attempts.into_iter().next().unwrap_or((
        anchor_preferred_start,
        WalkAttempt {
            ordered_ways: Vec::new(),
            warnings: Vec::new(),
            stranded: 0,
            order_penalty: 0,
            role_mismatches: 0,
        },
    ));
    warnings.extend(best_attempt.warnings);

    let has_explicit_direction_role = ways
        .iter()
        .any(|way| preferred_reversed(&way.role).is_some());
    let direction_source = if had_direction_roles || has_explicit_direction_role {
        "member_role"
    } else if hike_type == "loop" || ways.len() > 1 {
        "member_order"
    } else {
        "topology_fallback"
    };
    let default_direction_mode = if hike_type == "loop" && direction_source == "topology_fallback" {
        "both"
    } else {
        "forward"
    };

    StitchResult {
        ordered_ways: best_attempt.ordered_ways,
        hike_type: hike_type.to_string(),
        default_direction_mode: default_direction_mode.to_string(),
        direction_source: direction_source.to_string(),
        warnings,
        diagnostic_points,
    }
}

#[cfg(test)]
mod tests {
    use super::{stitch_ways, WayRecord};

    #[test]
    fn loop_prefers_relation_member_order_for_forward_direction() {
        let ways = vec![
            WayRecord {
                osm_way_id: 10,
                start: (0.0, 0.0),
                end: (1.0, 0.0),
                member_index: 0,
                role: String::new(),
            },
            WayRecord {
                osm_way_id: 11,
                start: (1.0, 0.0),
                end: (1.0, 1.0),
                member_index: 1,
                role: String::new(),
            },
            WayRecord {
                osm_way_id: 12,
                start: (1.0, 1.0),
                end: (0.0, 1.0),
                member_index: 2,
                role: String::new(),
            },
            WayRecord {
                osm_way_id: 13,
                start: (0.0, 1.0),
                end: (0.0, 0.0),
                member_index: 3,
                role: String::new(),
            },
        ];

        let result = stitch_ways(&ways, false);
        let ordered_ids = result
            .ordered_ways
            .iter()
            .map(|(osm_way_id, _)| *osm_way_id)
            .collect::<Vec<_>>();

        assert_eq!(ordered_ids, vec![10, 11, 12, 13]);
        assert_eq!(result.default_direction_mode, "forward");
        assert_eq!(result.direction_source, "member_order");
    }

    #[test]
    fn explicit_backward_role_flips_the_canonical_start_direction() {
        let ways = vec![
            WayRecord {
                osm_way_id: 20,
                start: (0.0, 0.0),
                end: (1.0, 0.0),
                member_index: 0,
                role: "backward".to_string(),
            },
            WayRecord {
                osm_way_id: 21,
                start: (0.0, 0.0),
                end: (-1.0, 0.0),
                member_index: 1,
                role: String::new(),
            },
        ];

        let result = stitch_ways(&ways, false);

        assert_eq!(result.ordered_ways[0], (20, true));
        assert_eq!(result.ordered_ways[1], (21, false));
        assert_eq!(result.default_direction_mode, "forward");
        assert_eq!(result.direction_source, "member_role");
    }

    #[test]
    fn odd_degree_branch_nodes_are_valid_start_candidates() {
        let ways = vec![
            WayRecord {
                osm_way_id: 100,
                start: (1.0, 0.0),
                end: (2.0, 0.0),
                member_index: 0,
                role: String::new(),
            },
            WayRecord {
                osm_way_id: 101,
                start: (0.0, 0.0),
                end: (1.0, 0.0),
                member_index: 1,
                role: String::new(),
            },
            WayRecord {
                osm_way_id: 102,
                start: (2.0, 0.0),
                end: (0.0, 0.0),
                member_index: 2,
                role: String::new(),
            },
            WayRecord {
                osm_way_id: 103,
                start: (0.0, 0.0),
                end: (10.0, 0.0),
                member_index: 3,
                role: String::new(),
            },
            WayRecord {
                osm_way_id: 104,
                start: (10.0, 0.0),
                end: (11.0, 0.0),
                member_index: 4,
                role: String::new(),
            },
            WayRecord {
                osm_way_id: 105,
                start: (11.0, 0.0),
                end: (12.0, 0.0),
                member_index: 5,
                role: String::new(),
            },
            WayRecord {
                osm_way_id: 106,
                start: (12.0, 0.0),
                end: (10.0, 0.0),
                member_index: 6,
                role: String::new(),
            },
        ];

        let result = stitch_ways(&ways, false);

        assert_eq!(result.hike_type, "point_to_point");
        assert_eq!(result.ordered_ways.len(), 7);
        assert!(!result
            .warnings
            .iter()
            .any(|warning| warning.contains("could not be reached")));
    }
}
