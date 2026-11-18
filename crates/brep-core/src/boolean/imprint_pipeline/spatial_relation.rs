use super::*;

pub(super) fn refuse(message: &str) -> Error {
    Error::new("BREP_IMPRINT_PIPELINE_REFUSED", message)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpatialRelation {
    Disjoint,
    Identical,
    AContainsB,
    BContainsA,
    /// Parallel cylinders with Complete transverse wall generators.
    WallIntersect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegionKeep {
    Drop,
    Whole,
    Fwd,
    Back,
    RingIn,
    RingOut,
}

#[derive(Clone, Debug)]
pub struct ImprintEvent {
    pub face: usize,
    pub edge: Option<(usize, f64)>,
    pub uv: [f64; 2],
    pub point: [f64; 3],
    pub parameter: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SegmentId(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CrossingId(pub usize);

#[derive(Clone, Debug, Default)]
pub struct ImprintPlan {
    pub crossings: Vec<CrossingId>,
    pub segments: Vec<SegmentId>,
    pub membership: BTreeMap<(usize, usize), Vec<SegmentId>>,
    pub keeps: BTreeMap<(usize, usize), RegionKeep>,
    pub complete: bool,
}

/// Shared final topology assembly for all admitted curved imprint families.
/// Pair-specific code may discover and classify UV sections, but it cannot
/// bypass the common closed-shell construction, lineage rebuild, and validation.
pub(crate) fn assemble_imprint_solid(
    vertices: Vec<Vertex>,
    edges: Vec<Edge>,
    loops: Vec<Loop>,
    faces: Vec<Face>,
    shell_faces: Vec<FaceUse>,
    tolerance: f64,
    sources: &[&Model],
) -> Result<Model> {
    let mut model = Model(
        brep_topology::Model {
            vertices,
            edges,
            loops,
            faces,
            shells: vec![Shell {
                faces: shell_faces,
                closed: true,
            }],
            bodies: vec![Body {
                outer_shell: 0,
                inner_shells: vec![],
            }],
            tolerance_mm: tolerance,
        },
        TopologyIds::default(),
    );
    model.inherit_topology_ids(sources);
    model.validate()?;
    Ok(model)
}

/// Like `assemble_imprint_solid`, but the authored faces may form several
/// edge-connected components; each becomes its own closed outer shell and
/// body. Used when a regularized difference splits an operand into pieces.
/// Nested (cavity) components are not produced by imprint families, since
/// every component touches the shared intersection network.
pub(crate) fn assemble_imprint_components(
    vertices: Vec<Vertex>,
    edges: Vec<Edge>,
    loops: Vec<Loop>,
    faces: Vec<Face>,
    shell_faces: Vec<FaceUse>,
    tolerance: f64,
    sources: &[&Model],
) -> Result<Model> {
    let mut parent: Vec<usize> = (0..shell_faces.len()).collect();
    pub(super) fn find(parent: &mut [usize], i: usize) -> usize {
        let mut i = i;
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    let mut owner_of_edge: BTreeMap<usize, usize> = BTreeMap::new();
    for (use_index, face_use) in shell_faces.iter().enumerate() {
        let face = &faces[face_use.face];
        for loop_id in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
            for coedge in &loops[loop_id].coedges {
                match owner_of_edge.get(&coedge.edge) {
                    Some(&other) => {
                        let a = find(&mut parent, other);
                        let b = find(&mut parent, use_index);
                        parent[a] = b;
                    }
                    None => {
                        owner_of_edge.insert(coedge.edge, use_index);
                    }
                }
            }
        }
    }
    let mut groups: BTreeMap<usize, Vec<FaceUse>> = BTreeMap::new();
    for (use_index, face_use) in shell_faces.iter().enumerate() {
        let root = find(&mut parent, use_index);
        groups.entry(root).or_default().push(face_use.clone());
    }
    if groups.len() == 1 {
        return assemble_imprint_solid(
            vertices,
            edges,
            loops,
            faces,
            shell_faces,
            tolerance,
            sources,
        );
    }
    let mut shells = Vec::with_capacity(groups.len());
    let mut bodies = Vec::with_capacity(groups.len());
    for (_, group) in groups {
        bodies.push(Body {
            outer_shell: shells.len(),
            inner_shells: vec![],
        });
        shells.push(Shell {
            faces: group,
            closed: true,
        });
    }
    let mut model = Model(
        brep_topology::Model {
            vertices,
            edges,
            loops,
            faces,
            shells,
            bodies,
            tolerance_mm: tolerance,
        },
        TopologyIds::default(),
    );
    model.inherit_topology_ids(sources);
    model.validate()?;
    Ok(model)
}

/// Validate imprint events and build an ordered plan skeleton.
/// Pair families supply Complete SS-derived events; this refuses empty /
/// non-finite / duplicate-parameter strata without a stratum label.
pub fn build_imprint_plan(events: &[ImprintEvent], max_span: f64) -> Result<ImprintPlan> {
    if !(max_span.is_finite() && max_span > 0.) {
        return Err(refuse("Imprint max_span must be finite and positive"));
    }
    if events.is_empty() {
        return Err(refuse("Imprint plan requires at least one section event"));
    }
    if events.len() > 4096 {
        return Err(refuse("Imprint event budget exceeded"));
    }
    let mut sorted = events.to_vec();
    sorted.sort_by(|a, b| {
        a.parameter
            .partial_cmp(&b.parameter)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.face.cmp(&b.face))
    });
    for window in sorted.windows(2) {
        let a = &window[0];
        let b = &window[1];
        if !a.point.iter().chain(&b.point).all(|x| x.is_finite())
            || !a.uv.iter().chain(&b.uv).all(|x| x.is_finite())
            || !a.parameter.is_finite()
            || !b.parameter.is_finite()
        {
            return Err(refuse("Imprint events must be finite"));
        }
        if (a.parameter - b.parameter).abs() <= 1e-15 && a.face == b.face {
            return Err(refuse(
                "Duplicate imprint event on the same face without stratum label",
            ));
        }
        if (b.parameter - a.parameter).abs() > max_span + 1e-9 && a.face == b.face {
            let _ = max_span;
        }
    }
    let mut plan = ImprintPlan {
        complete: true,
        ..ImprintPlan::default()
    };
    for (i, event) in sorted.iter().enumerate() {
        plan.crossings.push(CrossingId(i));
        let seg = SegmentId(i);
        plan.segments.push(seg);
        plan.membership
            .entry((0, event.face))
            .or_default()
            .push(seg);
    }
    Ok(plan)
}

pub fn classify_regions(plan: &mut ImprintPlan, want_inside: [bool; 2]) -> Result<()> {
    if !plan.complete {
        return Err(refuse("Cannot classify an incomplete imprint plan"));
    }
    for &(operand, face) in plan.membership.keys() {
        let keep = match (operand, want_inside[operand.min(1)]) {
            (0, true) | (1, true) => RegionKeep::Whole,
            _ => RegionKeep::Drop,
        };
        plan.keeps.insert((operand, face), keep);
    }
    Ok(())
}

#[derive(Clone, Copy)]
pub(super) struct CircleArc {
    pub(super) center: [f64; 2],
    pub(super) radius: f64,
    pub(super) start: f64,
    pub(super) sweep: f64,
}

pub(super) fn arc_point(arc: CircleArc, t: f64, z: f64) -> [f64; 3] {
    let angle = arc.start + arc.sweep * t;
    [
        arc.center[0] + arc.radius * angle.cos(),
        arc.center[1] + arc.radius * angle.sin(),
        z,
    ]
}

pub(super) fn arc_curve(arc: CircleArc, z: f64) -> Curve {
    let middle = arc.start + arc.sweep * 0.5;
    let weight = (arc.sweep.abs() * 0.5).cos();
    let start = arc_point(arc, 0., z);
    let end = arc_point(arc, 1., z);
    Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![
            start.to_vec(),
            vec![
                arc.center[0] + arc.radius * middle.cos() / weight,
                arc.center[1] + arc.radius * middle.sin() / weight,
                z,
            ],
            end.to_vec(),
        ],
        weights: vec![1., weight, 1.],
        periodic: false,
    }
}

pub(super) fn split_arc(arc: CircleArc) -> Vec<CircleArc> {
    let count = (arc.sweep.abs() / std::f64::consts::FRAC_PI_2)
        .ceil()
        .max(1.) as usize;
    let step = arc.sweep / count as f64;
    (0..count)
        .map(|i| CircleArc {
            start: arc.start + step * i as f64,
            sweep: step,
            ..arc
        })
        .collect()
}

pub(super) fn select_circle_arc(
    center: [f64; 2],
    radius: f64,
    first: [f64; 2],
    second: [f64; 2],
    other_center: [f64; 2],
    other_radius: f64,
    want_inside: bool,
) -> Result<CircleArc> {
    let a0 = (first[1] - center[1]).atan2(first[0] - center[0]);
    let a1 = (second[1] - center[1]).atan2(second[0] - center[0]);
    for (start, end) in [(a0, a1), (a1, a0)] {
        let sweep = (end - start).rem_euclid(std::f64::consts::TAU);
        let mid = start + sweep * 0.5;
        let p = [
            center[0] + radius * mid.cos(),
            center[1] + radius * mid.sin(),
        ];
        let inside = (p[0] - other_center[0]).hypot(p[1] - other_center[1]) <= other_radius + 1e-9;
        if inside == want_inside {
            return Ok(CircleArc {
                center,
                radius,
                start,
                sweep,
            });
        }
    }
    Err(refuse("Could not classify a circle Boolean boundary arc"))
}

pub(super) fn order_arc_loop(mut arcs: Vec<CircleArc>) -> Result<Vec<CircleArc>> {
    if arcs.is_empty() {
        return Err(refuse("Circle Boolean boundary is empty"));
    }
    let mut ordered = vec![arcs.remove(0)];
    while !arcs.is_empty() {
        let end = arc_point(*ordered.last().unwrap(), 1., 0.);
        let Some(index) = arcs.iter().position(|arc| {
            let start = arc_point(*arc, 0., 0.);
            (start[0] - end[0]).hypot(start[1] - end[1]) <= 1e-7
        }) else {
            return Err(refuse("Circle Boolean arcs do not form one oriented loop"));
        };
        ordered.push(arcs.remove(index));
    }
    let first = arc_point(ordered[0], 0., 0.);
    let last = arc_point(*ordered.last().unwrap(), 1., 0.);
    if (first[0] - last[0]).hypot(first[1] - last[1]) > 1e-7 {
        return Err(refuse("Circle Boolean arc loop is not closed"));
    }
    Ok(ordered)
}

pub(super) fn circle_boolean_boundary(
    a_center: [f64; 2],
    a_radius: f64,
    b_center: [f64; 2],
    b_radius: f64,
    operation: &str,
) -> Result<Vec<CircleArc>> {
    let dx = b_center[0] - a_center[0];
    let dy = b_center[1] - a_center[1];
    let distance = dx.hypot(dy);
    if distance <= 1e-12
        || distance >= a_radius + b_radius - 1e-9
        || distance <= (a_radius - b_radius).abs() + 1e-9
    {
        return Err(refuse(
            "Circle wall imprint requires two regular transverse crossings",
        ));
    }
    let along = (distance * distance + a_radius * a_radius - b_radius * b_radius) / (2. * distance);
    let height_sq = a_radius * a_radius - along * along;
    if height_sq <= 1e-16 {
        return Err(refuse(
            "Circle wall contact is tangent or numerically singular",
        ));
    }
    let height = height_sq.sqrt();
    let ux = dx / distance;
    let uy = dy / distance;
    let base = [a_center[0] + along * ux, a_center[1] + along * uy];
    let first = [base[0] - height * uy, base[1] + height * ux];
    let second = [base[0] + height * uy, base[1] - height * ux];
    let (a_inside, b_inside, reverse_b) = match operation {
        "union" => (false, false, false),
        "intersection" => (true, true, false),
        "difference" => (false, true, true),
        _ => return Err(refuse("Circle wall imprint operation is unsupported")),
    };
    let a_arc = select_circle_arc(
        a_center, a_radius, first, second, b_center, b_radius, a_inside,
    )?;
    let mut b_arc = select_circle_arc(
        b_center, b_radius, first, second, a_center, a_radius, b_inside,
    )?;
    if reverse_b {
        b_arc.start += b_arc.sweep;
        b_arc.sweep = -b_arc.sweep;
    }
    order_arc_loop(
        split_arc(a_arc)
            .into_iter()
            .chain(split_arc(b_arc))
            .collect(),
    )
}

pub(super) fn extrude_curve_loop(
    bottom_curves: &[Curve],
    z0: f64,
    z1: f64,
    tolerance: f64,
    sources: &[&Model],
) -> Result<Model> {
    let top_curves: Vec<Curve> = bottom_curves
        .iter()
        .cloned()
        .map(|mut curve| {
            for point in &mut curve.control_points {
                point[2] = z1;
            }
            curve
        })
        .collect();
    loft_curve_loops(bottom_curves, &top_curves, z0, z1, tolerance, sources)
}
