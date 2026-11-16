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

/// Minimum boss height, in multiples of `model.tolerance_mm`.
const MIN_BOSS_HEIGHT_TOLERANCE_FACTOR: f64 = 10.;
/// A boss wall leaning further than this off perpendicular faces the top.
const MAX_BOSS_WALL_LEAN_DEG: f64 = 45.;

/// One face adjacency: (other face, class, aag edge index).
type FaceLink = (usize, DihedralClass, usize);

/// Center and outward normal of one boss wall.
type WallDir = ([f64; 3], [f64; 3]);

/// Top plane and height of a boss candidate.
struct BossTop {
    center: [f64; 3],
    normal: [f64; 3],
    height: f64,
}

/// Boss search state shared by every seed.
struct BossSearch<'a> {
    model: &'a Model,
    graph: &'a Aag,
    attrs: Vec<&'a FaceAttrs>,
    adjacency: Vec<Vec<FaceLink>>,
    linear_tol: f64,
    volume_max: f64,
    seen: BTreeMap<Vec<usize>, usize>,
    pending: Vec<PendingBoss>,
}

impl BossSearch<'_> {
    fn wall_eligible(&self, face: usize) -> bool {
        matches!(
            self.attrs[face].class,
            SurfaceClass::Plane | SurfaceClass::Cone | SurfaceClass::Cylinder
        )
    }

    fn same_plane(&self, a: usize, b: usize) -> Result<bool> {
        if self.attrs[a].class != SurfaceClass::Plane || self.attrs[b].class != SurfaceClass::Plane
        {
            return Ok(false);
        }
        let model = self.model;
        let (Some(na), Some(nb)) = (
            super::pockets::outward_normal(model, a, 0.5, 0.5)?,
            super::pockets::outward_normal(model, b, 0.5, 0.5)?,
        ) else {
            return Ok(false);
        };
        let pa = super::pockets::face_point(model, a, 0.5, 0.5)?;
        let pb = super::pockets::face_point(model, b, 0.5, 0.5)?;
        Ok(dot(na, nb) > 1. - AXIS_DOT_TOLERANCE
            && (dot(na, pa) - dot(nb, pb)).abs() <= self.linear_tol)
    }

    /// Planar top grows across convex edges (walls) and coplanar smooth
    /// siblings (more top).
    fn grow_roles(
        &self,
        seed: usize,
        guard: &mut BudgetGuard,
    ) -> Result<BTreeMap<usize, BossRole>> {
        let mut roles = BTreeMap::from([(seed, BossRole::Top)]);
        let mut queue = VecDeque::from([seed]);
        while let Some(face) = queue.pop_front() {
            guard.tick()?;
            for &(other, class, _) in &self.adjacency[face] {
                if roles.contains_key(&other) {
                    continue;
                }
                let add = match class {
                    DihedralClass::Convex
                        if roles[&face] == BossRole::Top && self.wall_eligible(other) =>
                    {
                        Some(BossRole::Wall)
                    }
                    DihedralClass::Smooth if roles[&face] == BossRole::Top => {
                        if self.same_plane(seed, other)? {
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
        Ok(roles)
    }

    /// Internal edges convex/smooth, outside edges a concave base ring.
    /// Returns the base-ring edge midpoints; `None` rejects the candidate.
    fn base_ring_points(
        &self,
        roles: &BTreeMap<usize, BossRole>,
        guard: &mut BudgetGuard,
    ) -> Result<Option<Vec<[f64; 3]>>> {
        let mut has_base_ring = false;
        let mut rim_points = Vec::new();
        for &face in roles.keys() {
            for &(other, class, edge_index) in &self.adjacency[face] {
                guard.tick()?;
                if roles.contains_key(&other) {
                    if !matches!(class, DihedralClass::Convex | DihedralClass::Smooth) {
                        return Ok(None);
                    }
                    continue;
                }
                if class != DihedralClass::Concave {
                    return Ok(None);
                }
                has_base_ring = true;
                let edge = &self.model.edges[self.graph.edges[edge_index].edge];
                let domain = edge.curve.domain();
                let mid = edge.curve.evaluate(0.5 * (domain[0] + domain[1]))?.point;
                if mid.len() == 3 {
                    rim_points.push([mid[0], mid[1], mid[2]]);
                }
            }
        }
        Ok((has_base_ring && !rim_points.is_empty()).then_some(rim_points))
    }

    /// Top plane and top-to-base height; `None` rejects the candidate.
    fn boss_top(&self, seed: usize, rim_points: &[[f64; 3]]) -> Result<Option<BossTop>> {
        let model = self.model;
        let Some(normal) = super::pockets::outward_normal(model, seed, 0.5, 0.5)? else {
            return Ok(None);
        };
        let center = super::pockets::face_point(model, seed, 0.5, 0.5)?;
        let top_offset = dot(normal, center);
        let base_offset = rim_points
            .iter()
            .map(|point| dot(normal, *point))
            .fold(f64::INFINITY, f64::min);
        let height = top_offset - base_offset;
        if !(height.is_finite() && height > MIN_BOSS_HEIGHT_TOLERANCE_FACTOR * model.tolerance_mm) {
            return Ok(None);
        }
        Ok(Some(BossTop {
            center,
            normal,
            height,
        }))
    }

    /// (center, outward normal) per wall; `None` when a wall is singular or
    /// leans past the limit.
    fn wall_dirs(
        &self,
        walls: &[usize],
        top_normal: [f64; 3],
        guard: &mut BudgetGuard,
    ) -> Result<Option<Vec<WallDir>>> {
        let mut wall_dirs = Vec::with_capacity(walls.len());
        for &wall in walls {
            guard.tick()?;
            let (Some(normal), center) = (
                super::pockets::outward_normal(self.model, wall, 0.5, 0.5)?,
                super::pockets::face_point(self.model, wall, 0.5, 0.5)?,
            ) else {
                return Ok(None);
            };
            let lean = dot(normal, top_normal).clamp(-1., 1.).asin().to_degrees();
            if lean.abs() > MAX_BOSS_WALL_LEAN_DEG {
                return Ok(None);
            }
            wall_dirs.push((center, normal));
        }
        Ok(Some(wall_dirs))
    }

    /// Grow and validate one planar seed; records a new or taller reading.
    fn try_seed(&mut self, seed: usize, guard: &mut BudgetGuard) -> Result<()> {
        if self.attrs[seed].class != SurfaceClass::Plane {
            return Ok(());
        }
        let roles = self.grow_roles(seed, guard)?;
        let with_role = |want: BossRole| -> Vec<usize> {
            roles
                .iter()
                .filter_map(|(&face, &role)| (role == want).then_some(face))
                .collect()
        };
        let walls = with_role(BossRole::Wall);
        if walls.is_empty() {
            return Ok(());
        }
        let Some(rim_points) = self.base_ring_points(&roles, guard)? else {
            return Ok(());
        };
        let Some(top) = self.boss_top(seed, &rim_points)? else {
            return Ok(());
        };
        let Some(wall_dirs) = self.wall_dirs(&walls, top.normal, guard)? else {
            return Ok(());
        };
        let top_faces = with_role(BossRole::Top);
        let top_area: f64 = top_faces.iter().map(|&face| self.attrs[face].area).sum();
        let delta_volume = top_area * top.height;
        if !delta_volume.is_finite() || delta_volume >= self.volume_max {
            return Ok(());
        }
        let mut faces = top_faces;
        faces.extend_from_slice(&walls);
        faces.sort_unstable();
        let candidate = PendingBoss {
            faces: faces.clone(),
            top_center: top.center,
            top_normal: top.normal,
            height: top.height,
            delta_volume,
            wall_dirs,
        };
        match self.seen.get(&faces).copied() {
            Some(index) if self.pending[index].height >= top.height => {}
            Some(index) => self.pending[index] = candidate,
            None => {
                self.seen.insert(faces, self.pending.len());
                self.pending.push(candidate);
            }
        }
        Ok(())
    }
}

/// Material probes for one deduplicated candidate: void above the top, and
/// at every wall air outside with material on the opposite side. This
/// ray-parity pair rejects cavities even when their edge signs were
/// partitioned unusually by a boolean.
fn boss_probes_clear(
    model: &Model,
    candidate: &PendingBoss,
    guard: &mut BudgetGuard,
) -> Result<bool> {
    let eps = probe_offset(model, candidate.height);
    let above_top = [
        candidate.top_center[0] + candidate.top_normal[0] * eps,
        candidate.top_center[1] + candidate.top_normal[1] * eps,
        candidate.top_center[2] + candidate.top_normal[2] * eps,
    ];
    if super::pockets::probe_inside(model, above_top)? != Some(false) {
        return Ok(false);
    }
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
            return Ok(false);
        }
    }
    Ok(true)
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
    let attrs = graph
        .nodes
        .iter()
        .map(|node| {
            node.attrs
                .as_ref()
                .ok_or_else(|| small_error(STALE_ATTRS_MESSAGE))
        })
        .collect::<Result<Vec<_>>>()?;
    let mut adjacency = vec![Vec::<FaceLink>::new(); n];
    for (edge_index, edge) in graph.edges.iter().enumerate() {
        if edge.uses.len() == 2 {
            let (a, b) = (edge.uses[0].face, edge.uses[1].face);
            adjacency[a].push((b, edge.class, edge_index));
            adjacency[b].push((a, edge.class, edge_index));
        }
    }

    let mut guard = budget.guard("aag-small-bosses");
    guard.check()?;
    let mut search = BossSearch {
        model,
        graph,
        attrs,
        adjacency,
        linear_tol: linear_tolerance(model),
        volume_max,
        seen: BTreeMap::new(),
        pending: Vec::new(),
    };
    for seed in 0..n {
        guard.tick()?;
        search.try_seed(seed, &mut guard)?;
    }

    let mut bosses = Vec::new();
    for candidate in search.pending {
        guard.tick()?;
        if boss_probes_clear(model, &candidate, &mut guard)? {
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

/// Default fillet radius limit, relative to the bbox diagonal.
const DEFAULT_FILLET_RADIUS_REL: f64 = 0.05;
/// Default hole diameter limit, relative to the bbox diagonal.
const DEFAULT_HOLE_DIAMETER_REL: f64 = 0.1;
/// Default volume limit, relative to the cubed bbox diagonal.
const DEFAULT_VOLUME_REL: f64 = 0.01;
/// Default short-face area limit, relative to the squared bbox diagonal.
const DEFAULT_SHORT_FACE_AREA_REL: f64 = 1e-3;

/// Resolved absolute thresholds of one scan.
struct SmallThresholds {
    bbox_diagonal: f64,
    fillet_radius_max: f64,
    hole_diameter_max: f64,
    volume_max: f64,
    short_face_area_max: f64,
}

fn resolve_thresholds(model: &Model, options: &SmallFeatureOptions) -> Result<SmallThresholds> {
    let bbox_diagonal = positive_threshold(model_bbox_diagonal(model), "bbox_diagonal")?;
    let fillet_radius_max = positive_threshold(
        options
            .fillet_radius_max
            .unwrap_or(DEFAULT_FILLET_RADIUS_REL * bbox_diagonal),
        "fillet_radius_max",
    )?;
    let hole_diameter_max = positive_threshold(
        options
            .hole_diameter_max
            .unwrap_or(DEFAULT_HOLE_DIAMETER_REL * bbox_diagonal),
        "hole_diameter_max",
    )?;
    let volume_max = positive_threshold(
        options
            .volume_max
            .unwrap_or(DEFAULT_VOLUME_REL * bbox_diagonal * bbox_diagonal * bbox_diagonal),
        "volume_max",
    )?;
    let short_face_area_rel = positive_threshold(
        options
            .short_face_area_rel
            .unwrap_or(DEFAULT_SHORT_FACE_AREA_REL),
        "short_face_area_rel",
    )?;
    let short_face_area_max = positive_threshold(
        short_face_area_rel * bbox_diagonal * bbox_diagonal,
        "short_face_area_max",
    )?;
    Ok(SmallThresholds {
        bbox_diagonal,
        fillet_radius_max,
        hole_diameter_max,
        volume_max,
        short_face_area_max,
    })
}

/// Trimmed area of one face from its fresh attributes.
fn fresh_area(graph: &Aag, face: usize) -> Result<f64> {
    graph.nodes[face]
        .attrs
        .as_ref()
        .map(|attrs| attrs.area)
        .ok_or_else(|| small_error(STALE_ATTRS_MESSAGE))
}

/// Candidates found so far and the faces they claim.
struct SmallScan<'a> {
    model: &'a Model,
    graph: &'a Aag,
    budget: &'a Budget,
    limits: SmallThresholds,
    candidates: Vec<SmallFeature>,
    claimed: BTreeSet<usize>,
}

impl SmallScan<'_> {
    fn push(&mut self, feature: SmallFeature) {
        self.claimed.extend(feature.faces.iter().copied());
        self.candidates.push(feature);
    }

    fn collect_fillets(&mut self, guard: &mut BudgetGuard) -> Result<()> {
        let fillet_radius_max = self.limits.fillet_radius_max;
        let fillet_options = FilletChainOptions {
            radius_max: Some(fillet_radius_max),
            ..FilletChainOptions::default()
        };
        let fillets = crate::aag_fillets::find_fillet_chains_with(
            self.model,
            self.graph,
            self.budget,
            &fillet_options,
        )?;
        for chain in fillets {
            guard.tick()?;
            if chain.radius_max >= fillet_radius_max {
                continue;
            }
            let mut area = 0f64;
            for &face in &chain.faces {
                area += fresh_area(self.graph, face)?;
            }
            self.push(SmallFeature {
                kind: SmallFeatureKind::Fillet,
                faces: chain.faces,
                size: chain.radius_max,
                delta_volume: (2. / std::f64::consts::PI - 0.5) * chain.radius_max * area,
            });
        }
        Ok(())
    }

    fn collect_holes(&mut self, guard: &mut BudgetGuard) -> Result<()> {
        for hole in find_holes(self.model, self.graph, self.budget)? {
            guard.tick()?;
            if hole.diameter >= self.limits.hole_diameter_max {
                continue;
            }
            let radius = 0.5 * hole.diameter;
            self.push(SmallFeature {
                kind: SmallFeatureKind::Hole,
                faces: hole.faces,
                size: hole.diameter,
                delta_volume: std::f64::consts::PI * radius * radius * hole.depth,
            });
        }
        Ok(())
    }

    fn collect_pockets_and_bosses(&mut self, guard: &mut BudgetGuard) -> Result<()> {
        // With no concave edge neither a pocket nor a boss base ring can exist.
        // Besides being a cheap rejection, this keeps large disjoint-solid scans
        // out of the pocket matcher's all-edges validation path.
        let has_concave = self
            .graph
            .edges
            .iter()
            .any(|edge| edge.class == DihedralClass::Concave);
        if !has_concave {
            return Ok(());
        }
        let volume_max = self.limits.volume_max;
        for pocket in find_pockets(self.model, self.graph, self.budget)? {
            guard.tick()?;
            let delta_volume = pocket.floor_area * pocket.depth;
            if delta_volume >= volume_max {
                continue;
            }
            self.push(SmallFeature {
                kind: SmallFeatureKind::Pocket,
                faces: pocket.faces,
                size: pocket.depth,
                delta_volume,
            });
        }
        for boss in find_small_bosses(self.model, self.graph, self.budget, volume_max)? {
            guard.tick()?;
            self.push(boss);
        }
        Ok(())
    }

    fn collect_short_faces(&mut self, guard: &mut BudgetGuard) -> Result<()> {
        for face in 0..self.graph.nodes.len() {
            guard.tick()?;
            let area = fresh_area(self.graph, face)?;
            require_finite_f64(area, "face area")?;
            if area < self.limits.short_face_area_max && !self.claimed.contains(&face) {
                self.candidates.push(SmallFeature {
                    kind: SmallFeatureKind::ShortFace,
                    faces: vec![face],
                    size: area,
                    delta_volume: 0.,
                });
            }
        }
        Ok(())
    }
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
        return Err(small_error(STALE_ATTRS_MESSAGE));
    }
    let limits = resolve_thresholds(model, options)?;

    let mut guard = budget.guard("aag-scan-small");
    guard.check()?;
    let mut scan = SmallScan {
        model,
        graph,
        budget,
        limits,
        candidates: Vec::new(),
        claimed: BTreeSet::new(),
    };
    scan.collect_fillets(&mut guard)?;
    scan.collect_holes(&mut guard)?;
    scan.collect_pockets_and_bosses(&mut guard)?;
    scan.collect_short_faces(&mut guard)?;
    guard.check()?;
    let limits = scan.limits;
    Ok(SmallFeatureReport {
        candidates: scan.candidates,
        bbox_diagonal: limits.bbox_diagonal,
        fillet_radius_max: limits.fillet_radius_max,
        hole_diameter_max: limits.hole_diameter_max,
        volume_max: limits.volume_max,
        short_face_area_max: limits.short_face_area_max,
    })
}
