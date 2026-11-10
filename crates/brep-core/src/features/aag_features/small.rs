use super::*;
use crate::aag_fillets::FilletChainOptions;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

fn small_error(message: impl Into<String>) -> Error {
    Error::new("BREP_AAG_SMALL_FEATURES_INPUT", message)
}

/// Kind of a small geometric feature reported by [`scan_small_features`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SmallFeatureKind {
    Fillet,
    Hole,
    Pocket,
    Boss,
    ShortFace,
}

/// One small-feature candidate. This is a recognition result only: scanning
/// never edits or removes geometry.
#[derive(Clone, Debug, PartialEq)]
pub struct SmallFeature {
    pub kind: SmallFeatureKind,
    /// Member face indices into `model.faces`, ascending.
    pub faces: Vec<usize>,
    /// Characteristic size: fillet maximum radius, hole diameter, pocket
    /// depth, boss height, or short-face area, respectively.
    pub size: f64,
    /// Local closed-form volume estimate. Hole: πr²·depth; fillet:
    /// (2/π−1/2)·r·blend area (equivalent to (1−π/4)r²L for a constant
    /// quarter-cylinder with L=2A/(πr)); pocket: floor area·depth; boss:
    /// top area·height. Short faces carry zero. These are screening
    /// estimates, not exact removed volumes for drafted or blended shapes.
    pub delta_volume: f64,
}

/// Optional absolute thresholds for [`scan_small_features`]. `None` selects
/// the corresponding bbox-scaled default documented on the report fields.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SmallFeatureOptions {
    pub fillet_radius_max: Option<f64>,
    pub hole_diameter_max: Option<f64>,
    pub volume_max: Option<f64>,
    /// Relative short-face threshold; the absolute limit is this value times
    /// the squared bbox diagonal. Defaults to 1e-3.
    pub short_face_area_rel: Option<f64>,
}

/// Small-feature scan result, including the resolved thresholds used.
#[derive(Clone, Debug, PartialEq)]
pub struct SmallFeatureReport {
    pub candidates: Vec<SmallFeature>,
    pub bbox_diagonal: f64,
    /// Default: 0.05 · bbox diagonal.
    pub fillet_radius_max: f64,
    /// Default: 0.1 · bbox diagonal.
    pub hole_diameter_max: f64,
    /// Default: 0.01 · bbox diagonal³.
    pub volume_max: f64,
    /// Default: 1e-3 · bbox diagonal².
    pub short_face_area_max: f64,
}

fn model_bbox_diagonal(model: &Model) -> f64 {
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for p in model
        .faces
        .iter()
        .flat_map(|f| f.surface.control_points.iter().flatten())
    {
        for i in 0..3.min(p.len()) {
            lo[i] = lo[i].min(p[i]);
            hi[i] = hi[i].max(p[i]);
        }
    }
    let diagonal = norm([hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]]);
    if diagonal.is_finite() && diagonal > 0. { diagonal } else { 1. }
}

fn positive_threshold(value: f64, name: &'static str) -> Result<f64> {
    require_finite_f64(value, name)?;
    if value <= 0. {
        return Err(small_error(format!("{name} must be positive")));
    }
    Ok(value)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BossRole {
    Top,
    Wall,
}

pub(super) struct PendingBoss {
    pub(super) faces: Vec<usize>,
    pub(super) top_center: [f64; 3],
    pub(super) top_normal: [f64; 3],
    pub(super) height: f64,
    pub(super) delta_volume: f64,
    wall_dirs: Vec<([f64; 3], [f64; 3])>,
}

/// Mirror of the simple floor-and-walls pocket template: a planar top grows
/// across convex edges, internal edges are convex/smooth, and the base ring
/// is concave. Expensive material probes run only after graph validation and
/// global face-set deduplication.
pub(super) fn find_boss_candidates(
    model: &Model,
    graph: &Aag,
    budget: &Budget,
    volume_max: f64,
) -> Result<Vec<PendingBoss>> {
    let n = model.faces.len();
    let attrs_of = |face: usize| {
        graph.nodes[face]
            .attrs
            .as_ref()
            .expect("fresh attrs checked")
    };
    let wall_eligible = |face: usize| {
        matches!(
            attrs_of(face).class,
            SurfaceClass::Plane | SurfaceClass::Cone | SurfaceClass::Cylinder
        )
    };
    let mut adjacency = vec![Vec::<(usize, DihedralClass, usize)>::new(); n];
    for (edge_index, edge) in graph.edges.iter().enumerate() {
        if edge.uses.len() == 2 {
            let (a, b) = (edge.uses[0].face, edge.uses[1].face);
            adjacency[a].push((b, edge.class, edge_index));
            adjacency[b].push((a, edge.class, edge_index));
        }
    }

    let linear_tol = (model.tolerance_mm * 100.).max(1e-7);
    let same_plane = |a: usize, b: usize| -> Result<bool> {
        if attrs_of(a).class != SurfaceClass::Plane || attrs_of(b).class != SurfaceClass::Plane {
            return Ok(false);
        }
        let (Some(na), Some(nb)) = (
            super::pockets::outward_normal(model, a, 0.5, 0.5)?,
            super::pockets::outward_normal(model, b, 0.5, 0.5)?,
        ) else {
            return Ok(false);
        };
        let pa = super::pockets::face_point(model, a, 0.5, 0.5)?;
        let pb = super::pockets::face_point(model, b, 0.5, 0.5)?;
        Ok(dot(na, nb) > 1. - AXIS_DOT_TOLERANCE
            && (dot(na, pa) - dot(nb, pb)).abs() <= linear_tol)
    };

    let mut guard = budget.guard("aag-small-bosses");
    guard.check()?;
    let mut seen: BTreeMap<Vec<usize>, usize> = BTreeMap::new();
    let mut pending: Vec<PendingBoss> = Vec::new();
    for seed in 0..n {
        guard.tick()?;
        if attrs_of(seed).class != SurfaceClass::Plane {
            continue;
        }
        let mut roles = BTreeMap::from([(seed, BossRole::Top)]);
        let mut queue = VecDeque::from([seed]);
        while let Some(face) = queue.pop_front() {
            guard.tick()?;
            for &(other, class, _) in &adjacency[face] {
                if roles.contains_key(&other) {
                    continue;
                }
                let add = match class {
                    DihedralClass::Convex
                        if roles[&face] == BossRole::Top && wall_eligible(other) =>
                    {
                        Some(BossRole::Wall)
                    }
                    DihedralClass::Smooth if roles[&face] == BossRole::Top => {
                        if same_plane(seed, other)? {
                            Some(BossRole::Top)
                        } else {
                            None
                        }
                    }
                    _ => None,
                };
                if let Some(role) = add {
                    roles.insert(other, role);
                    queue.push_back(other);
                }
            }
        }
        let walls: Vec<usize> = roles
            .iter()
            .filter_map(|(&face, &role)| (role == BossRole::Wall).then_some(face))
            .collect();
        if walls.is_empty() {
            continue;
        }

        let mut valid = true;
        let mut has_base_ring = false;
        let mut rim_points = Vec::new();
        for &face in roles.keys() {
            for &(other, class, edge_index) in &adjacency[face] {
                guard.tick()?;
                if roles.contains_key(&other) {
                    if !matches!(class, DihedralClass::Convex | DihedralClass::Smooth) {
                        valid = false;
                        break;
                    }
                } else {
                    if class != DihedralClass::Concave {
                        valid = false;
                        break;
                    }
                    has_base_ring = true;
                    let edge = &model.edges[graph.edges[edge_index].edge];
                    let domain = edge.curve.domain();
                    let mid = edge.curve.evaluate(0.5 * (domain[0] + domain[1]))?.point;
                    if mid.len() == 3 {
                        rim_points.push([mid[0], mid[1], mid[2]]);
                    }
                }
            }
            if !valid {
                break;
            }
        }
        if !valid || !has_base_ring || rim_points.is_empty() {
            continue;
        }

        let Some(top_normal) = super::pockets::outward_normal(model, seed, 0.5, 0.5)? else {
            continue;
        };
        let top_center = super::pockets::face_point(model, seed, 0.5, 0.5)?;
        let top_offset = dot(top_normal, top_center);
        let base_offset = rim_points
            .iter()
            .map(|point| dot(top_normal, *point))
            .fold(f64::INFINITY, f64::min);
        let height = top_offset - base_offset;
        if !(height.is_finite() && height > 10. * model.tolerance_mm) {
            continue;
        }

        let mut wall_dirs = Vec::with_capacity(walls.len());
        for &wall in &walls {
            guard.tick()?;
            let (Some(normal), center) = (
                super::pockets::outward_normal(model, wall, 0.5, 0.5)?,
                super::pockets::face_point(model, wall, 0.5, 0.5)?,
            ) else {
                valid = false;
                break;
            };
            let lean = dot(normal, top_normal).clamp(-1., 1.).asin().to_degrees();
            if lean.abs() > 45. {
                valid = false;
                break;
            }
            wall_dirs.push((center, normal));
        }
        if !valid {
            continue;
        }
        let top_faces: Vec<usize> = roles
            .iter()
            .filter_map(|(&face, &role)| (role == BossRole::Top).then_some(face))
            .collect();
        let top_area: f64 = top_faces.iter().map(|&face| attrs_of(face).area).sum();
        let delta_volume = top_area * height;
        if !delta_volume.is_finite() || delta_volume >= volume_max {
            continue;
        }
        let mut faces = top_faces;
        faces.extend_from_slice(&walls);
        faces.sort_unstable();
        let candidate = PendingBoss {
            faces: faces.clone(),
            top_center,
            top_normal,
            height,
            delta_volume,
            wall_dirs,
        };
        match seen.get(&faces).copied() {
            Some(index) if pending[index].height >= height => {}
            Some(index) => pending[index] = candidate,
            None => {
                seen.insert(faces, pending.len());
                pending.push(candidate);
            }
        }
    }

    let mut bosses = Vec::new();
    for candidate in pending {
        guard.tick()?;
        let eps = (1e-3 * candidate.height).max(10. * model.tolerance_mm);
        let above_top = [
            candidate.top_center[0] + candidate.top_normal[0] * eps,
            candidate.top_center[1] + candidate.top_normal[1] * eps,
            candidate.top_center[2] + candidate.top_normal[2] * eps,
        ];
        if super::pockets::probe_inside(model, above_top)? != Some(false) {
            continue;
        }
        // Boss wall: outward is air, the opposite side is material. This
        // ray-parity pair rejects cavities even when their edge signs were
        // partitioned unusually by a boolean.
        let mut walls_open = true;
        for (center, normal) in &candidate.wall_dirs {
            guard.tick()?;
            let outside = [
                center[0] + normal[0] * eps,
                center[1] + normal[1] * eps,
                center[2] + normal[2] * eps,
            ];
            let inside = [
                center[0] - normal[0] * eps,
                center[1] - normal[1] * eps,
                center[2] - normal[2] * eps,
            ];
            if super::pockets::probe_inside(model, outside)? != Some(false)
                || super::pockets::probe_inside(model, inside)? != Some(true)
            {
                walls_open = false;
                break;
            }
        }
        if walls_open {
            bosses.push(candidate);
        }
    }
    bosses.sort_by_key(|feature| feature.faces[0]);
    guard.check()?;
    Ok(bosses)
}

fn find_small_bosses(
    model: &Model,
    graph: &Aag,
    budget: &Budget,
    volume_max: f64,
) -> Result<Vec<SmallFeature>> {
    Ok(find_boss_candidates(model, graph, budget, volume_max)?
        .into_iter()
        .map(|boss| SmallFeature {
            kind: SmallFeatureKind::Boss,
            faces: boss.faces,
            size: boss.height,
            delta_volume: boss.delta_volume,
        })
        .collect())
}

/// List small fillets, holes, pockets, bosses and short faces without changing
/// the model. Comparisons are strict (`size < threshold`, or estimated volume
/// `< volume_max`). `graph` must cover the model and carry fresh attributes;
/// all stages honor `budget` and return typed input/resource errors.
///
/// Faces already claimed by a reported fillet, hole, pocket or boss are not
/// reported again as short faces. Different feature templates may still
/// overlap (for example, a round blind pocket can also be a hole).
pub fn scan_small_features(
    model: &Model,
    graph: &Aag,
    options: &SmallFeatureOptions,
    budget: &Budget,
) -> Result<SmallFeatureReport> {
    require_finite_f64(model.tolerance_mm, "model.tolerance_mm")?;
    if graph.nodes.len() != model.faces.len() {
        return Err(small_error("AAG node count does not match model faces"));
    }
    if !graph.stale_face_attrs(model).is_empty() {
        return Err(small_error(
            "AAG face attributes missing or stale; run attach_face_attrs first",
        ));
    }
    let bbox_diagonal = positive_threshold(model_bbox_diagonal(model), "bbox_diagonal")?;
    let fillet_radius_max = positive_threshold(
        options.fillet_radius_max.unwrap_or(0.05 * bbox_diagonal),
        "fillet_radius_max",
    )?;
    let hole_diameter_max = positive_threshold(
        options.hole_diameter_max.unwrap_or(0.1 * bbox_diagonal),
        "hole_diameter_max",
    )?;
    let volume_max = positive_threshold(
        options
            .volume_max
            .unwrap_or(0.01 * bbox_diagonal * bbox_diagonal * bbox_diagonal),
        "volume_max",
    )?;
    let short_face_area_rel = positive_threshold(
        options.short_face_area_rel.unwrap_or(1e-3),
        "short_face_area_rel",
    )?;
    let short_face_area_max = positive_threshold(
        short_face_area_rel * bbox_diagonal * bbox_diagonal,
        "short_face_area_max",
    )?;

    let mut guard = budget.guard("aag-scan-small");
    guard.check()?;
    let mut candidates = Vec::new();
    let mut claimed = BTreeSet::new();
    let mut push = |feature: SmallFeature| {
        claimed.extend(feature.faces.iter().copied());
        candidates.push(feature);
    };

    let fillet_options = FilletChainOptions {
        radius_max: Some(fillet_radius_max),
        ..FilletChainOptions::default()
    };
    let fillets = crate::aag_fillets::find_fillet_chains_with(
        model,
        graph,
        budget,
        &fillet_options,
    )?;
    for chain in fillets {
        guard.tick()?;
        if chain.radius_max >= fillet_radius_max {
            continue;
        }
        let area: f64 = chain
            .faces
            .iter()
            .map(|&face| {
                graph.nodes[face]
                    .attrs
                    .as_ref()
                    .expect("fresh attrs checked")
                    .area
            })
            .sum();
        push(SmallFeature {
            kind: SmallFeatureKind::Fillet,
            faces: chain.faces,
            size: chain.radius_max,
            delta_volume: (2. / std::f64::consts::PI - 0.5) * chain.radius_max * area,
        });
    }

    for hole in find_holes(model, graph, budget)? {
        guard.tick()?;
        if hole.diameter >= hole_diameter_max {
            continue;
        }
        let radius = 0.5 * hole.diameter;
        push(SmallFeature {
            kind: SmallFeatureKind::Hole,
            faces: hole.faces,
            size: hole.diameter,
            delta_volume: std::f64::consts::PI * radius * radius * hole.depth,
        });
    }

    // With no concave edge neither a pocket nor a boss base ring can exist.
    // Besides being a cheap rejection, this keeps large disjoint-solid scans
    // out of the pocket matcher's all-edges validation path.
    let has_concave = graph
        .edges
        .iter()
        .any(|edge| edge.class == DihedralClass::Concave);
    if has_concave {
        for pocket in find_pockets(model, graph, budget)? {
            guard.tick()?;
            let delta_volume = pocket.floor_area * pocket.depth;
            if delta_volume >= volume_max {
                continue;
            }
            push(SmallFeature {
                kind: SmallFeatureKind::Pocket,
                faces: pocket.faces,
                size: pocket.depth,
                delta_volume,
            });
        }
        for boss in find_small_bosses(model, graph, budget, volume_max)? {
            guard.tick()?;
            push(boss);
        }
    }

    for (face, node) in graph.nodes.iter().enumerate() {
        guard.tick()?;
        let area = node.attrs.as_ref().expect("fresh attrs checked").area;
        require_finite_f64(area, "face area")?;
        if area < short_face_area_max && !claimed.contains(&face) {
            candidates.push(SmallFeature {
                kind: SmallFeatureKind::ShortFace,
                faces: vec![face],
                size: area,
                delta_volume: 0.,
            });
        }
    }
    guard.check()?;
    Ok(SmallFeatureReport {
        candidates,
        bbox_diagonal,
        fillet_radius_max,
        hole_diameter_max,
        volume_max,
        short_face_area_max,
    })
}
