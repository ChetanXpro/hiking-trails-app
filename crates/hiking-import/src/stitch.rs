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

fn meters_between(a: (f64, f64), b: (f64, f64)) -> f64 {
    let lat_rad = a.1.to_radians();
    let m_per_deg_lat = 111_320.0;
    let m_per_deg_lon = 111_320.0 * lat_rad.cos();
    let dx = (a.0 - b.0) * m_per_deg_lon;
    let dy = (a.1 - b.1) * m_per_deg_lat;
    (dx * dx + dy * dy).sqrt()
}

struct NodeAssigner {
    nodes: Vec<(f64, f64)>,
}

impl NodeAssigner {
    fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    fn node_id_for(&mut self, coord: (f64, f64)) -> usize {
        for (id, existing) in self.nodes.iter().enumerate() {
            if meters_between(*existing, coord) <= SNAP_TOLERANCE_METERS {
                return id;
            }
        }
        self.nodes.push(coord);
        self.nodes.len() - 1
    }
}

#[derive(Debug, Clone)]
pub struct WayRecord {
    pub osm_way_id: i64,
    pub name: Option<String>,
    pub surface: Option<String>,
    pub highway_type: Option<String>,
    pub length_m: f64,
    pub start: (f64, f64),
    pub end: (f64, f64),
    pub geom_wkb: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticKind {
    OpenEnd,
    Branch,
    Disconnected,
}

impl DiagnosticKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            DiagnosticKind::OpenEnd => "open_end",
            DiagnosticKind::Branch => "branch",
            DiagnosticKind::Disconnected => "disconnected",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DiagnosticPoint {
    pub kind: DiagnosticKind,
    pub coord: (f64, f64),
}

#[derive(Debug)]
pub struct StitchResult {
    pub ordered_ways: Vec<(i64, bool)>,
    pub warnings: Vec<String>,
    pub diagnostic_points: Vec<DiagnosticPoint>,
}

pub fn stitch_ways(ways: &[WayRecord]) -> StitchResult {
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

    fn find(parent: &mut HashMap<usize, usize>, x: usize) -> usize {
        let p = *parent.entry(x).or_insert(x);
        if p == x {
            x
        } else {
            let root = find(parent, p);
            parent.insert(x, root);
            root
        }
    }
    fn union(parent: &mut HashMap<usize, usize>, a: usize, b: usize) {
        let ra = find(parent, a);
        let rb = find(parent, b);
        if ra != rb {
            parent.insert(ra, rb);
        }
    }

    for &(a, b, _) in &edges {
        union(&mut parent, a, b);
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
        .max_by_key(|(_, edges)| edges.len())
        .map(|(&root, edges)| (root, edges.clone()))
        .unwrap_or((0, Vec::new()));

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

    let start_node = main_component
        .iter()
        .flat_map(|&e| {
            let (a, b, _) = edges[e];
            [a, b]
        })
        .find(|n| degree.get(n).copied().unwrap_or(0) == 1)
        .unwrap_or_else(|| edges[main_component[0]].0);

    let mut used = vec![false; edges.len()];
    let mut ordered_ways = Vec::new();
    let mut current = start_node;

    loop {
        let Some(candidates) = adjacency.get(&current) else {
            break;
        };
        let Some(&edge_idx) = candidates.iter().find(|&&e| !used[e]) else {
            break;
        };
        used[edge_idx] = true;

        let (a, b, way_idx) = edges[edge_idx];
        let reversed = a != current;
        ordered_ways.push((ways[way_idx].osm_way_id, reversed));

        current = if a == current { b } else { a };

        if degree.get(&current).copied().unwrap_or(0) >= 3 {
            warnings.push(
                "branch point at a shared node -- this route has a junction, not a simple path; took the first available continuation rather than guessing the intended branch".to_string()
            );
        }
    }

    let stranded = main_component.iter().filter(|&&e| !used[e]).count();
    if stranded > 0 {
        warnings.push(format!(
            "{stranded} way(s) in this relation could not be reached by a simple walk from the start -- likely a spur or out-and-back branch that needs manual review"
        ));
    }

    StitchResult {
        ordered_ways,
        warnings,
        diagnostic_points,
    }
}