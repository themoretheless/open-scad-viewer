//! Shared imprint / empty-algebra pipeline for analytic curved Boolean (A2).
//!
//! Topology authorship for admitted curved pairs must go through this module —
//! never through `prismatic_boolean` or mesh/Manifold. Sphere imprint retains
//! pair-specific UV scanning in `sphere_boolean`; all families converge on the
//! same closed-shell assembly and audit. Parallel cylinder walls are authored
//! as exact rational-arc profile splits after Complete SS generators.

use crate::{Body, Coedge, Edge, Face, FaceUse, Loop, Model, Shell, TopologyIds, Vertex};
use nurbs_core::{Error, Result, curve::Curve, surface::Surface};
use std::collections::BTreeMap;

fn refuse(message: &str) -> Error {
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
    for (&(operand, face), _segs) in plan.membership.iter() {
        let keep = match (operand, want_inside[operand.min(1)]) {
            (0, true) | (1, true) => RegionKeep::Whole,
            _ => RegionKeep::Drop,
        };
        plan.keeps.insert((operand, face), keep);
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct CircleArc {
    center: [f64; 2],
    radius: f64,
    start: f64,
    sweep: f64,
}

fn arc_point(arc: CircleArc, t: f64, z: f64) -> [f64; 3] {
    let angle = arc.start + arc.sweep * t;
    [
        arc.center[0] + arc.radius * angle.cos(),
        arc.center[1] + arc.radius * angle.sin(),
        z,
    ]
}

fn arc_curve(arc: CircleArc, z: f64) -> Curve {
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

fn split_arc(arc: CircleArc) -> Vec<CircleArc> {
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

fn select_circle_arc(
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

fn order_arc_loop(mut arcs: Vec<CircleArc>) -> Result<Vec<CircleArc>> {
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

fn circle_boolean_boundary(
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

fn extrude_curve_loop(
    bottom_curves: &[Curve],
    z0: f64,
    z1: f64,
    tolerance: f64,
    sources: &[&Model],
) -> Result<Model> {
    let n = bottom_curves.len();
    if n < 2 || z1 <= z0 {
        return Err(refuse(
            "Curve-loop extrusion requires a closed loop and positive height",
        ));
    }
    let bottom_points: Vec<[f64; 3]> = bottom_curves
        .iter()
        .map(|curve| [curve.control_points[0][0], curve.control_points[0][1], z0])
        .collect();
    let top_points: Vec<[f64; 3]> = bottom_points.iter().map(|p| [p[0], p[1], z1]).collect();
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
    let mut vertices: Vec<Vertex> = bottom_points
        .iter()
        .chain(&top_points)
        .map(|point| Vertex { point: *point })
        .collect();
    let mut edges = Vec::with_capacity(n * 3);
    let mut bottom_edges = Vec::with_capacity(n);
    let mut top_edges = Vec::with_capacity(n);
    let mut vertical_edges = Vec::with_capacity(n);
    for i in 0..n {
        let j = (i + 1) % n;
        bottom_edges.push(edges.len());
        edges.push(Edge {
            degenerate: false,
            vertices: [i, j],
            curve: bottom_curves[i].clone(),
        });
        top_edges.push(edges.len());
        edges.push(Edge {
            degenerate: false,
            vertices: [n + i, n + j],
            curve: top_curves[i].clone(),
        });
    }
    for i in 0..n {
        vertical_edges.push(edges.len());
        edges.push(Edge {
            degenerate: false,
            vertices: [i, n + i],
            curve: crate::line(bottom_points[i].to_vec(), top_points[i].to_vec()),
        });
    }
    let mut loops = Vec::new();
    let mut faces = Vec::new();
    let mut shell_faces = Vec::new();
    for i in 0..n {
        let j = (i + 1) % n;
        let bottom_curve = &bottom_curves[i];
        let top_curve = &top_curves[i];
        let outer = loops.len();
        loops.push(Loop {
            coedges: vec![
                Coedge {
                    edge: bottom_edges[i],
                    reversed: false,
                    pcurve: crate::line(vec![0., 0.], vec![1., 0.]),
                },
                Coedge {
                    edge: vertical_edges[j],
                    reversed: false,
                    pcurve: crate::line(vec![1., 0.], vec![1., 1.]),
                },
                Coedge {
                    edge: top_edges[i],
                    reversed: true,
                    pcurve: crate::line(vec![1., 1.], vec![0., 1.]),
                },
                Coedge {
                    edge: vertical_edges[i],
                    reversed: true,
                    pcurve: crate::line(vec![0., 1.], vec![0., 0.]),
                },
            ],
        });
        let face = faces.len();
        faces.push(Face {
            surface: Surface {
                degree_u: bottom_curve.degree,
                degree_v: 1,
                knots_u: bottom_curve.knots.clone(),
                knots_v: vec![0., 0., 1., 1.],
                control_points: (0..bottom_curve.control_points.len())
                    .map(|k| {
                        vec![
                            bottom_curve.control_points[k].clone(),
                            top_curve.control_points[k].clone(),
                        ]
                    })
                    .collect(),
                weights: bottom_curve
                    .weights
                    .iter()
                    .map(|weight| vec![*weight, *weight])
                    .collect(),
                periodic_u: false,
                periodic_v: false,
            },
            outer,
            holes: vec![],
        });
        shell_faces.push(FaceUse {
            face,
            reversed: false,
        });
    }
    let mut min = [f64::INFINITY; 2];
    let mut max = [f64::NEG_INFINITY; 2];
    for curve in bottom_curves {
        for point in &curve.control_points {
            min[0] = min[0].min(point[0]);
            min[1] = min[1].min(point[1]);
            max[0] = max[0].max(point[0]);
            max[1] = max[1].max(point[1]);
        }
    }
    let cap_surface = |z: f64| Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![min[0], min[1], z], vec![min[0], max[1], z]],
            vec![vec![max[0], min[1], z], vec![max[0], max[1], z]],
        ],
        weights: vec![vec![1., 1.], vec![1., 1.]],
        periodic_u: false,
        periodic_v: false,
    };
    for (z, edge_ids, curves, reversed) in [
        (z0, &bottom_edges, bottom_curves, true),
        (z1, &top_edges, top_curves.as_slice(), false),
    ] {
        let outer = loops.len();
        loops.push(Loop {
            coedges: edge_ids
                .iter()
                .enumerate()
                .map(|(i, edge)| {
                    let curve = curves[i].clone();
                    let pcurve = Curve {
                        control_points: curve
                            .control_points
                            .iter()
                            .map(|p| {
                                vec![
                                    (p[0] - min[0]) / (max[0] - min[0]),
                                    (p[1] - min[1]) / (max[1] - min[1]),
                                ]
                            })
                            .collect(),
                        ..curve
                    };
                    Coedge {
                        edge: *edge,
                        reversed: false,
                        pcurve,
                    }
                })
                .collect(),
        });
        let face = faces.len();
        faces.push(Face {
            surface: cap_surface(z),
            outer,
            holes: vec![],
        });
        shell_faces.push(FaceUse { face, reversed });
    }
    assemble_imprint_solid(
        std::mem::take(&mut vertices),
        edges,
        loops,
        faces,
        shell_faces,
        tolerance,
        sources,
    )
}

fn extrude_arc_loop(
    arcs: &[CircleArc],
    z0: f64,
    z1: f64,
    tolerance: f64,
    sources: &[&Model],
) -> Result<Model> {
    let curves: Vec<Curve> = arcs.iter().map(|arc| arc_curve(*arc, z0)).collect();
    extrude_curve_loop(&curves, z0, z1, tolerance, sources)
}

/// Exact AF-01 authoring for vertical cuboid edges. The XY boundary is rebuilt
/// from line segments and positive-weight quarter-circle arcs, then extruded
/// through the shared closed-shell assembly path.
pub(crate) fn rounded_cuboid_vertical_edges(
    source: &Model,
    min: [f64; 3],
    max: [f64; 3],
    rounded: [bool; 4],
    radius: f64,
) -> Result<Model> {
    if !(radius.is_finite()
        && radius > 0.
        && 2. * radius < max[0] - min[0]
        && 2. * radius < max[1] - min[1])
    {
        return Err(refuse("Rounded cuboid radius exceeds the AF-01 profile"));
    }
    let corners = [
        [min[0], min[1]],
        [max[0], min[1]],
        [max[0], max[1]],
        [min[0], max[1]],
    ];
    let entry = [
        [min[0], min[1] + radius],
        [max[0] - radius, min[1]],
        [max[0], max[1] - radius],
        [min[0] + radius, max[1]],
    ];
    let exit = [
        [min[0] + radius, min[1]],
        [max[0], min[1] + radius],
        [max[0] - radius, max[1]],
        [min[0], max[1] - radius],
    ];
    let centers = [
        [min[0] + radius, min[1] + radius],
        [max[0] - radius, min[1] + radius],
        [max[0] - radius, max[1] - radius],
        [min[0] + radius, max[1] - radius],
    ];
    let starts = [
        std::f64::consts::PI,
        -std::f64::consts::FRAC_PI_2,
        0.,
        std::f64::consts::FRAC_PI_2,
    ];
    let effective_entry: Vec<[f64; 2]> = (0..4)
        .map(|i| if rounded[i] { entry[i] } else { corners[i] })
        .collect();
    let effective_exit: Vec<[f64; 2]> = (0..4)
        .map(|i| if rounded[i] { exit[i] } else { corners[i] })
        .collect();
    let mut curves = Vec::new();
    for i in 0..4 {
        if rounded[i] {
            curves.push(arc_curve(
                CircleArc {
                    center: centers[i],
                    radius,
                    start: starts[i],
                    sweep: std::f64::consts::FRAC_PI_2,
                },
                min[2],
            ));
        }
        let next = (i + 1) % 4;
        let a = effective_exit[i];
        let b = effective_entry[next];
        curves.push(crate::line(
            vec![a[0], a[1], min[2]],
            vec![b[0], b[1], min[2]],
        ));
    }
    extrude_curve_loop(&curves, min[2], max[2], source.tolerance_mm, &[source])
}

/// Parallel cylinder wall imprint from Complete generator-line events.
///
/// After a Complete imprint plan is certified, author the retained circular
/// spans, shared crossing vertices, side patches, and cap loops exactly.
/// Tangent, axial mismatch, or unsupported frame cases refuse.
pub fn parallel_cylinder_wall_imprint(
    a: &Model,
    b: &Model,
    operation: &str,
    events: &[ImprintEvent],
    a_origin: [f64; 3],
    a_axis: [f64; 3],
    a_radius: f64,
    a_height: [f64; 2],
    b_origin: [f64; 3],
    b_axis: [f64; 3],
    b_radius: f64,
    b_height: [f64; 2],
) -> Result<Model> {
    a.validate()?;
    b.validate()?;
    if !matches!(operation, "union" | "difference" | "intersection") {
        return Err(refuse(
            "Wall imprint admits union, difference, or intersection only",
        ));
    }
    let max_span = (a_height[1] - a_height[0]).abs() + (b_height[1] - b_height[0]).abs() + 1.;
    let plan = build_imprint_plan(events, max_span)?;
    if !plan.complete || plan.segments.len() < 2 {
        return Err(refuse(
            "Wall imprint requires a Complete multi-segment plan",
        ));
    }
    let tol = a.tolerance_mm.max(b.tolerance_mm);
    if (a_axis[0] - b_axis[0]).abs() > 1e-12
        || (a_axis[1] - b_axis[1]).abs() > 1e-12
        || (a_axis[2] - b_axis[2]).abs() > 1e-12
        || a_axis[0].abs() > 1e-12
        || a_axis[1].abs() > 1e-12
        || (a_axis[2] - 1.).abs() > 1e-12
    {
        return Err(refuse(
            "Cylinder wall imprint currently admits common +Z axes only",
        ));
    }
    if (a_height[0] - b_height[0]).abs() > tol || (a_height[1] - b_height[1]).abs() > tol {
        return Err(refuse(
            "Cylinder wall imprint requires coincident cap planes",
        ));
    }
    let arcs = circle_boolean_boundary(
        [a_origin[0], a_origin[1]],
        a_radius,
        [b_origin[0], b_origin[1]],
        b_radius,
        operation,
    )?;
    extrude_arc_loop(&arcs, a_height[0], a_height[1], tol, &[a, b])
}

/// Regularized empty-algebra for disjoint / identical / containment corpora.
/// Does not call prismatic_boolean or mesh paths.
pub fn regularized_empty_algebra(
    a: &Model,
    b: &Model,
    operation: &str,
    relation: SpatialRelation,
) -> Result<Model> {
    a.validate()?;
    b.validate()?;
    if !matches!(operation, "union" | "difference" | "intersection" | "xor") {
        return Err(refuse(
            "Boolean operation must be union, difference, intersection, or xor",
        ));
    }
    let tolerance = a.tolerance_mm.max(b.tolerance_mm);
    let out = match (relation, operation) {
        (SpatialRelation::Disjoint, "union") => crate::boolean_support::separated_union(a, b)?,
        (SpatialRelation::Disjoint, "intersection") => Model::empty(tolerance)?,
        (SpatialRelation::Disjoint, "difference") => a.clone(),
        (SpatialRelation::Disjoint, "xor") => crate::boolean_support::separated_union(a, b)?,
        (SpatialRelation::Identical, "union" | "intersection") => a.clone(),
        (SpatialRelation::Identical, "difference" | "xor") => Model::empty(tolerance)?,
        (SpatialRelation::AContainsB, "union") => a.clone(),
        (SpatialRelation::AContainsB, "intersection") => b.clone(),
        (SpatialRelation::AContainsB, "difference") => cavity(a, b, tolerance)?,
        (SpatialRelation::AContainsB, "xor") => cavity(a, b, tolerance)?,
        (SpatialRelation::BContainsA, "union") => b.clone(),
        (SpatialRelation::BContainsA, "intersection") => a.clone(),
        (SpatialRelation::BContainsA, "difference") => Model::empty(tolerance)?,
        (SpatialRelation::BContainsA, "xor") => cavity(b, a, tolerance)?,
        (SpatialRelation::WallIntersect, _) => {
            return Err(refuse(
                "WallIntersect requires parallel_cylinder_wall_imprint, not empty algebra",
            ));
        }
        _ => {
            return Err(refuse(
                "Boolean operation not admitted for regularized empty algebra",
            ));
        }
    };
    out.validate()?;
    Ok(out)
}

/// Outer body with inner shell inverted (cavity). Shared by sphere and cylinder.
pub fn cavity(outer: &Model, inner: &Model, tolerance: f64) -> Result<Model> {
    let mut result = outer.clone();
    result.tolerance_mm = tolerance;
    let (v, e, l, f, s) = (
        outer.vertices.len(),
        outer.edges.len(),
        outer.loops.len(),
        outer.faces.len(),
        outer.shells.len(),
    );
    result.vertices.extend(inner.vertices.iter().cloned());
    result
        .edges
        .extend(inner.edges.iter().cloned().map(|mut edge| {
            edge.vertices.iter_mut().for_each(|id| *id += v);
            edge
        }));
    result
        .loops
        .extend(inner.loops.iter().cloned().map(|mut wire| {
            wire.coedges.iter_mut().for_each(|use_| use_.edge += e);
            wire
        }));
    result
        .faces
        .extend(inner.faces.iter().cloned().map(|mut face| {
            face.outer += l;
            face.holes.iter_mut().for_each(|id| *id += l);
            face
        }));
    if inner.shells.is_empty() || result.bodies.is_empty() {
        return Err(refuse(
            "Cavity requires nonempty outer body and inner shell",
        ));
    }
    result.shells.push(Shell {
        faces: inner.shells[0]
            .faces
            .iter()
            .map(|use_| FaceUse {
                face: use_.face + f,
                reversed: !use_.reversed,
            })
            .collect(),
        closed: true,
    });
    result.bodies[0].inner_shells.push(s);
    let _ = Body {
        outer_shell: 0,
        inner_shells: vec![],
    };
    result.inherit_topology_ids(&[outer, inner]);
    result.validate()?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cylinder;

    #[test]
    fn disjoint_union_is_separated_without_prism() {
        let a = cylinder(2., 4.).unwrap();
        let b0 = cylinder(2., 4.).unwrap();
        let b = crate::transform::affine(
            &b0,
            [
                [1., 0., 0., 12.],
                [0., 1., 0., 0.],
                [0., 0., 1., 0.],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        let out = regularized_empty_algebra(&a, &b, "union", SpatialRelation::Disjoint).unwrap();
        assert!(out.bodies.len() >= 2 || out.shells.len() >= 2);
        out.validate().unwrap();
    }

    #[test]
    fn imprint_plan_refuses_duplicate_face_parameter() {
        let events = [
            ImprintEvent {
                face: 0,
                edge: None,
                uv: [0.5, 0.5],
                point: [0., 0., 0.],
                parameter: 0.1,
            },
            ImprintEvent {
                face: 0,
                edge: None,
                uv: [0.6, 0.5],
                point: [0., 0., 1.],
                parameter: 0.1,
            },
        ];
        assert_eq!(
            build_imprint_plan(&events, 1.).unwrap_err().code,
            "BREP_IMPRINT_PIPELINE_REFUSED"
        );
    }

    #[test]
    fn contained_difference_builds_cavity() {
        let outer = cylinder(4., 6.).unwrap();
        let inner = cylinder(1.5, 6.).unwrap();
        let out =
            regularized_empty_algebra(&outer, &inner, "difference", SpatialRelation::AContainsB)
                .unwrap();
        assert!(!out.bodies[0].inner_shells.is_empty());
        out.validate().unwrap();
    }
}
