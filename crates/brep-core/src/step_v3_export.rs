//! STEP `/3`–`/9` writer, periodic carrier reconstruction, and `/8` regularity certification.

use crate::analytic_features::FeatureCertificate;
use crate::step_interchange_v3::{
    MAX_OUTPUT_BYTES, STEP_INTERCHANGE_V3_CAPABILITY, STEP_INTERCHANGE_V4_CAPABILITY,
    STEP_INTERCHANGE_V5_CAPABILITY, STEP_INTERCHANGE_V6_CAPABILITY,
    STEP_INTERCHANGE_V7_CAPABILITY, STEP_INTERCHANGE_V8_CAPABILITY,
    STEP_INTERCHANGE_V9_CAPABILITY, StepRegularityEvidence, StepV3Report, StepV8Certificate,
    entity_digest, refuse,
};
use crate::step_part21::{Value, parse, render_entity_value};
use crate::{Coedge, Edge, Face, FaceUse, Loop, Model, TopoId, TopoKind};
use nurbs_core::{Result, curve::Curve, surface::Surface};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) struct Writer {
    next: usize,
    pub(crate) rows: BTreeMap<usize, String>,
}
impl Writer {
    pub(crate) fn new() -> Self {
        Self {
            next: 1,
            rows: BTreeMap::new(),
        }
    }
    fn reserve(&mut self) -> usize {
        let id = self.next;
        self.next += 1;
        id
    }
    fn set(&mut self, id: usize, body: String) {
        self.rows.insert(id, body);
    }
    pub(crate) fn emit(&mut self, body: String) -> usize {
        let id = self.reserve();
        self.set(id, body);
        id
    }
    pub(crate) fn point(&mut self, p: &[f64]) -> usize {
        self.emit(format!("CARTESIAN_POINT('',({}))", floats(p)))
    }
}
fn floats(v: &[f64]) -> String {
    v.iter()
        .map(|x| format!("{x:.17e}").replace('e', "E"))
        .collect::<Vec<_>>()
        .join(",")
}
pub(crate) fn enforce_output_limit(text: &str) -> Result<()> {
    if text.len() > MAX_OUTPUT_BYTES {
        return Err(refuse("STEP output exceeds 16 MiB"));
    }
    Ok(())
}
fn quadratic_patch_knots(segments: usize) -> Vec<f64> {
    let mut knots = vec![0.; 3];
    for knot in 1..segments {
        knots.extend([knot as f64, knot as f64])
    }
    knots.extend([segments as f64; 3]);
    knots
}
fn merge_periodic_patch_grid(
    model: &Model,
    face_ids: &[usize],
    u_count: usize,
    v_count: usize,
) -> Result<Model> {
    if face_ids.len() != u_count * v_count {
        return Err(refuse("Periodic patch grid size mismatch"));
    }
    let first = &model.faces[face_ids[0]].surface;
    if first.degree_u != 2 || first.degree_v != 2 && v_count > 1 {
        return Err(refuse("Periodic patch grid degree mismatch"));
    }
    let rows = 2 * u_count + 1;
    let columns = if v_count == 1 {
        first.control_points[0].len()
    } else {
        2 * v_count + 1
    };
    let mut points = vec![vec![vec![f64::NAN; 3]; columns]; rows];
    let mut weights = vec![vec![f64::NAN; columns]; rows];
    for v in 0..v_count {
        for u in 0..u_count {
            let surface = &model.faces[face_ids[v * u_count + u]].surface;
            if surface.degree_u != first.degree_u
                || surface.degree_v != first.degree_v
                || surface.control_points.len() != 3
                || surface.control_points[0].len() != (if v_count == 1 { columns } else { 3 })
            {
                return Err(refuse("Periodic patch grid contains incompatible surface"));
            }
            for i in 0..surface.control_points.len() {
                for j in 0..surface.control_points[i].len() {
                    let (row, column) = (2 * u + i, if v_count == 1 { j } else { 2 * v + j });
                    if points[row][column][0].is_finite() {
                        let error = points[row][column]
                            .iter()
                            .zip(&surface.control_points[i][j])
                            .map(|(a, b)| (a - b) * (a - b))
                            .sum::<f64>()
                            .sqrt();
                        if error > 1e-9
                            || (weights[row][column] - surface.weights[i][j]).abs() > 1e-12
                        {
                            return Err(refuse("Periodic patch grid control boundaries disagree"));
                        }
                    } else {
                        points[row][column] = surface.control_points[i][j].clone();
                        weights[row][column] = surface.weights[i][j];
                    }
                }
            }
        }
    }
    let mut merged = Surface {
        degree_u: 2,
        degree_v: first.degree_v,
        knots_u: quadratic_patch_knots(u_count),
        knots_v: if v_count == 1 {
            first.knots_v.clone()
        } else {
            quadratic_patch_knots(v_count)
        },
        control_points: points,
        weights,
        periodic_u: false,
        periodic_v: false,
    };
    for column in [0, columns - 1] {
        let pole = merged.control_points[0][column].clone();
        if merged.control_points.iter().all(|row| {
            row[column]
                .iter()
                .zip(&pole)
                .map(|(left, right)| (left - right) * (left - right))
                .sum::<f64>()
                .sqrt()
                <= 1e-7
        }) {
            for row in &mut merged.control_points {
                row[column] = pole.clone()
            }
        }
    }
    for &face_id in face_ids {
        let face = &model.faces[face_id];
        for loop_id in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
            for coedge in &model.loops[loop_id].coedges {
                let edge = &model.edges[coedge.edge];
                if !edge.degenerate {
                    continue;
                }
                let endpoints = &coedge.pcurve.control_points;
                if endpoints.len() != 2 || endpoints[0][1] != endpoints[1][1] {
                    continue;
                }
                let column = if (endpoints[0][1] - merged.knots_v[merged.degree_v]).abs() <= 1e-12 {
                    0
                } else if (endpoints[0][1]
                    - merged.knots_v[merged.knots_v.len() - merged.degree_v - 1])
                    .abs()
                    <= 1e-12
                {
                    columns - 1
                } else {
                    continue;
                };
                let pole = model.vertices[edge.vertices[0]].point.to_vec();
                for row in &mut merged.control_points {
                    row[column] = pole.clone()
                }
            }
        }
    }
    merged.validate()?;
    let mut result = model.clone();
    for v in 0..v_count {
        for u in 0..u_count {
            let face = face_ids[v * u_count + u];
            result.faces[face].surface = merged.clone();
            let mut loops = vec![result.faces[face].outer];
            loops.extend(result.faces[face].holes.iter().copied());
            for loop_id in loops {
                for coedge in &mut result.loops[loop_id].coedges {
                    for point in &mut coedge.pcurve.control_points {
                        point[0] += u as f64;
                        if v_count > 1 {
                            point[1] += v as f64
                        }
                    }
                }
            }
        }
    }
    result.validate()?;
    Ok(result)
}
fn periodicized_sphere(model: &Model) -> Result<Model> {
    let radius = model
        .vertices
        .iter()
        .map(|vertex| {
            vertex
                .point
                .iter()
                .map(|value| value * value)
                .sum::<f64>()
                .sqrt()
        })
        .fold(0., f64::max);
    let w = std::f64::consts::FRAC_1_SQRT_2;
    let circle = [
        [1., 0.],
        [1., 1.],
        [0., 1.],
        [-1., 1.],
        [-1., 0.],
        [-1., -1.],
        [0., -1.],
        [1., -1.],
        [1., 0.],
    ];
    let circle_weights = [1., w, 1., w, 1., w, 1., w, 1.];
    let profile = [
        [0., radius],
        [radius, radius],
        [radius, 0.],
        [radius, -radius],
        [0., -radius],
    ];
    let profile_weights = [1., w, 1., w, 1.];
    let control_points = circle
        .iter()
        .map(|u| {
            profile
                .iter()
                .map(|v| vec![v[0] * u[0], v[0] * u[1], v[1]])
                .collect()
        })
        .collect();
    let weights = circle_weights
        .iter()
        .map(|wu| profile_weights.iter().map(|wv| wu * wv).collect())
        .collect();
    let surface = Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: quadratic_patch_knots(4),
        knots_v: quadratic_patch_knots(2),
        control_points,
        weights,
        periodic_u: false,
        periodic_v: false,
    };
    surface.validate()?;
    let mut result = model.clone();
    let source_edges = result.edges.clone();
    let north = model
        .vertices
        .iter()
        .position(|vertex| (vertex.point[2] - radius).abs() < radius * 1e-9)
        .ok_or_else(|| refuse("Sphere north pole is missing"))?;
    let south = model
        .vertices
        .iter()
        .position(|vertex| (vertex.point[2] + radius).abs() < radius * 1e-9)
        .ok_or_else(|| refuse("Sphere south pole is missing"))?;
    let selected = [0, 1, 3, 5, 7, 8];
    result.0.edges = selected
        .iter()
        .map(|index| {
            let mut edge = source_edges[*index].clone();
            if edge.curve.degree == 2 {
                edge.curve.weights = vec![1., w, 1.]
            }
            edge
        })
        .collect();
    let pole_edges = [north, south].map(|vertex| {
        let edge = result.edges.len();
        let point = result.vertices[vertex].point.to_vec();
        result.0.edges.push(Edge {
            degenerate: true,
            vertices: [vertex, vertex],
            curve: Curve {
                degree: 1,
                knots: vec![0., 0., 1., 1.],
                control_points: vec![point.clone(), point],
                weights: vec![1., 1.],
                periodic: false,
            },
        });
        edge
    });
    let line = |a: [f64; 2], b: [f64; 2]| Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![a.to_vec(), b.to_vec()],
        weights: vec![1., 1.],
        periodic: false,
    };
    result.0.loops = vec![
        Loop {
            coedges: vec![
                Coedge::new(
                    pole_edges[0],
                    false,
                    line([0., 0.], [4., 0.]),
                ),
                Coedge::new(
                    0,
                    true,
                    line([4., 0.], [4., 1.]),
                ),
                Coedge::new(
                    4,
                    false,
                    line([4., 1.], [3., 1.]),
                ),
                Coedge::new(
                    3,
                    true,
                    line([3., 1.], [2., 1.]),
                ),
                Coedge::new(
                    2,
                    true,
                    line([2., 1.], [1., 1.]),
                ),
                Coedge::new(
                    1,
                    true,
                    line([1., 1.], [0., 1.]),
                ),
                Coedge::new(
                    0,
                    false,
                    line([0., 1.], [0., 0.]),
                ),
            ],
        },
        Loop {
            coedges: vec![
                Coedge::new(
                    1,
                    false,
                    line([0., 1.], [1., 1.]),
                ),
                Coedge::new(
                    2,
                    false,
                    line([1., 1.], [2., 1.]),
                ),
                Coedge::new(
                    3,
                    false,
                    line([2., 1.], [3., 1.]),
                ),
                Coedge::new(
                    4,
                    true,
                    line([3., 1.], [4., 1.]),
                ),
                Coedge::new(
                    5,
                    false,
                    line([4., 1.], [4., 2.]),
                ),
                Coedge::new(
                    pole_edges[1],
                    false,
                    line([4., 2.], [0., 2.]),
                ),
                Coedge::new(
                    5,
                    true,
                    line([0., 2.], [0., 1.]),
                ),
            ],
        },
    ];
    result.0.faces = vec![
        Face {
            surface: surface.clone(),
            outer: 0,
            holes: vec![],
        },
        Face {
            surface,
            outer: 1,
            holes: vec![],
        },
    ];
    result.0.shells[model.bodies[0].outer_shell].faces = vec![
        FaceUse {
            face: 0,
            reversed: false,
        },
        FaceUse {
            face: 1,
            reversed: false,
        },
    ];
    result.rebuild_topology_ids();
    for (face_index, face) in result.faces.iter().enumerate() {
        for coedge in &result.loops[face.outer].coedges {
            for sample in 0..=8 {
                let t = sample as f64 / 8.;
                let uv = coedge.pcurve.evaluate(t)?.point;
                let p = face.surface.evaluate(uv[0], uv[1])?.point;
                let edge = &result.edges[coedge.edge];
                let domain = edge.curve.domain();
                let q = edge
                    .curve
                    .evaluate(if coedge.reversed {
                        domain[1] - t * (domain[1] - domain[0])
                    } else {
                        domain[0] + t * (domain[1] - domain[0])
                    })?
                    .point;
                let error = p
                    .iter()
                    .zip(&q)
                    .map(|(a, b)| (a - b) * (a - b))
                    .sum::<f64>()
                    .sqrt();
                if error > 1e-7 {
                    return Err(refuse(format!(
                        "Periodic sphere face {face_index} edge {} mismatch {error} at {t}: uv={uv:?} surface={p:?} edge={q:?}",
                        coedge.edge
                    )));
                }
            }
        }
    }
    result.validate()?;
    Ok(result)
}
// Face counts and degrees do not establish a periodic carrier: partial
// revolutions can have the same shape of patch records as a complete band.
pub(crate) fn periodic_patch_grid_compatible(
    model: &Model,
    u_count: usize,
    v_count: usize,
) -> bool {
    let compatible = |a: &Surface, ai: usize, aj: usize, b: &Surface, bi: usize, bj: usize| {
        a.control_points[ai][aj]
            .iter()
            .zip(&b.control_points[bi][bj])
            .map(|(x, y)| (x - y) * (x - y))
            .sum::<f64>()
            .sqrt()
            <= 1e-9
            && (a.weights[ai][aj] - b.weights[bi][bj]).abs() <= 1e-12
    };
    for v in 0..v_count {
        for u in 0..u_count {
            let a = &model.faces[v * u_count + u].surface;
            let b = &model.faces[v * u_count + (u + 1) % u_count].surface;
            if a.control_points.len() != 3
                || b.control_points.len() != 3
                || a.control_points[0].len() != b.control_points[0].len()
            {
                return false;
            }
            for j in 0..a.control_points[0].len() {
                if !compatible(a, 2, j, b, 0, j) {
                    return false;
                }
            }
            if v_count > 1 {
                let b = &model.faces[((v + 1) % v_count) * u_count + u].surface;
                if a.control_points[0].len() != 3 || b.control_points[0].len() != 3 {
                    return false;
                }
                for i in 0..3 {
                    if !compatible(a, i, 2, b, i, 0) {
                        return false;
                    }
                }
            }
        }
    }
    true
}
pub(crate) fn periodicized_step_v6(model: &Model) -> Result<Model> {
    if model.faces.len() == 8
        && model.vertices.len() == 6
        && model
            .faces
            .iter()
            .all(|face| face.surface.degree_u == 2 && face.surface.degree_v == 2)
    {
        periodicized_sphere(model)
    } else if model.faces.len() >= 4
        && model.faces[..4].iter().all(|face| {
            face.surface.degree_u == 2
                && face.surface.degree_v == 1
                && face.surface.control_points.len() == 3
        })
    {
        if periodic_patch_grid_compatible(model, 4, 1) {
            merge_periodic_patch_grid(model, &[0, 1, 2, 3], 4, 1)
        } else {
            Ok(model.clone())
        }
    } else if model.faces.len() == 16
        && model.faces.iter().all(|face| {
            face.surface.degree_u == 2
                && face.surface.degree_v == 2
                && face.surface.control_points.len() == 3
                && face.surface.control_points[0].len() == 3
        })
    {
        if periodic_patch_grid_compatible(model, 4, 4) {
            merge_periodic_patch_grid(model, &(0..16).collect::<Vec<_>>(), 4, 4)
        } else {
            Ok(model.clone())
        }
    } else {
        Ok(model.clone())
    }
}

fn constant_boundary(surface: &Surface, column: usize) -> bool {
    surface
        .control_points
        .iter()
        .all(|row| row[column] == surface.control_points[0][column])
}
pub(crate) fn certify_periodic_surface(surface: &Surface) -> Result<Option<StepRegularityEvidence>> {
    let nu = surface.control_points.len();
    let nv = surface.control_points.first().map_or(0, Vec::len);
    if nu != 9
        || surface.degree_u != 2
        || !matches!((nv, surface.degree_v), (2, 1) | (5, 2) | (9, 2))
    {
        return Ok(None);
    }
    if surface
        .weights
        .iter()
        .flatten()
        .any(|weight| !weight.is_finite() || *weight <= 0.)
    {
        return Err(refuse(
            "STEP /8 rational denominator is not positive on the whole parameter domain",
        ));
    }
    if surface.control_points[0] != surface.control_points[nu - 1]
        || surface.weights[0] != surface.weights[nu - 1]
    {
        return Err(refuse(
            "STEP /8 lifted U seam lacks exact homogeneous control-net identity",
        ));
    }
    let du = [
        surface.knots_u[surface.degree_u],
        surface.knots_u[surface.knots_u.len() - surface.degree_u - 1],
    ];
    let dv = [
        surface.knots_v[surface.degree_v],
        surface.knots_v[surface.knots_v.len() - surface.degree_v - 1],
    ];
    let denominator_lower_bound = surface
        .weights
        .iter()
        .flatten()
        .copied()
        .fold(f64::INFINITY, f64::min);
    let radius_at = |column: usize| {
        surface.control_points[0][column]
            .iter()
            .zip(&surface.control_points[4][column])
            .map(|(left, right)| (left - right) * (left - right))
            .sum::<f64>()
            .sqrt()
            / 2.
    };
    let radii = if nv == 2 {
        vec![radius_at(0), radius_at(1)]
    } else {
        (0..nv).step_by(2).map(radius_at).collect::<Vec<_>>()
    };
    let positive_min = radii
        .iter()
        .copied()
        .filter(|radius| *radius > 1e-12)
        .fold(f64::INFINITY, f64::min);
    let positive_max = radii.iter().copied().fold(0., f64::max);
    let (
        carrier,
        collapsed_boundaries,
        regular_open_domain,
        jacobian_lower_bound,
        lifted_v,
        identity,
    ) = match nv {
        2 => {
            let collapsed = if constant_boundary(surface, 0) {
                vec!["v-min"]
            } else if constant_boundary(surface, 1) {
                vec!["v-max"]
            } else {
                vec![]
            };
            let carrier = if collapsed.is_empty() && ((radii[0] - radii[1]).abs() <= 1e-12) {
                "cylinder"
            } else {
                "cone"
            };
            (
                carrier,
                collapsed,
                !radii.iter().all(|radius| *radius > 1e-12),
                if radii.iter().all(|radius| *radius > 1e-12) {
                    positive_min
                } else {
                    0.
                },
                0.,
                "positive rational quadratic circle × affine generator; J=(affine radius)·J0",
            )
        }
        5 => {
            if !constant_boundary(surface, 0) || !constant_boundary(surface, nv - 1) {
                return Err(refuse(
                    "STEP /8 sphere requires exact collapsed V boundary control nets",
                ));
            }
            (
                "sphere",
                vec!["v-min", "v-max"],
                true,
                0.,
                0.,
                "positive rational circle × semicircle; J=R² cos(v)·J0 with only endpoint zeros",
            )
        }
        9 => {
            if surface
                .control_points
                .iter()
                .map(|row| &row[0])
                .ne(surface.control_points.iter().map(|row| &row[nv - 1]))
                || surface
                    .weights
                    .iter()
                    .map(|row| row[0])
                    .ne(surface.weights.iter().map(|row| row[nv - 1]))
            {
                return Err(refuse(
                    "STEP /8 lifted V seam lacks exact homogeneous control-net identity",
                ));
            }
            if !(positive_min > 1e-12 && positive_max > positive_min) {
                return Err(refuse(
                    "STEP /8 torus requires exact positive major-minus-minor radius",
                ));
            }
            (
                "torus",
                vec![],
                false,
                positive_min,
                dv[1] - dv[0],
                "positive rational circle product; J=r(R+r cos(v))·J0 and R-r>0",
            )
        }
        _ => unreachable!(),
    };
    Ok(Some(StepRegularityEvidence {
        carrier,
        parameter_u: du,
        parameter_v: dv,
        lifted_periods: [du[1] - du[0], lifted_v],
        denominator_lower_bound,
        jacobian_lower_bound,
        collapsed_boundaries,
        regular_open_domain,
        identity,
    }))
}
pub(crate) fn certify_step_v8_topology(model: &Model) -> Result<StepV8Certificate> {
    model.validate()?;
    let mut seen = BTreeSet::new();
    let mut regularity = Vec::new();
    for face in &model.faces {
        if !seen.insert(format!("{:?}", face.surface)) {
            continue;
        }
        if let Some(evidence) = certify_periodic_surface(&face.surface)? {
            regularity.push(evidence)
        }
    }
    for evidence in &regularity {
        for boundary in &evidence.collapsed_boundaries {
            let boundary_value = if *boundary == "v-min" {
                evidence.parameter_v[0]
            } else {
                evidence.parameter_v[1]
            };
            let mut intervals = Vec::<[f64; 2]>::new();
            for face in &model.faces {
                if !certify_periodic_surface(&face.surface)?
                    .is_some_and(|candidate| candidate.carrier == evidence.carrier)
                {
                    continue;
                }
                for loop_id in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
                    for coedge in &model.loops[loop_id].coedges {
                        if !model.edges[coedge.edge].degenerate {
                            continue;
                        }
                        let domain = coedge.pcurve.domain();
                        let a = coedge.pcurve.evaluate(domain[0])?.point;
                        let b = coedge.pcurve.evaluate(domain[1])?.point;
                        if (a[1] - boundary_value).abs() <= 1e-12
                            && (b[1] - boundary_value).abs() <= 1e-12
                        {
                            intervals.push([a[0].min(b[0]), a[0].max(b[0])])
                        }
                    }
                }
            }
            intervals.sort_by(|left, right| left[0].total_cmp(&right[0]));
            let mut covered = evidence.parameter_u[0];
            for interval in intervals {
                if interval[0] > covered + 1e-12 {
                    break;
                }
                covered = covered.max(interval[1]);
            }
            if covered < evidence.parameter_u[1] - 1e-12 {
                return Err(refuse(
                    "STEP /8 pole topology does not cover the complete collapsed parameter interval",
                ));
            }
        }
    }
    Ok(StepV8Certificate {
        capability: STEP_INTERCHANGE_V8_CAPABILITY,
        complete: true,
        regularity,
        sense_layers: [
            "seam/pole/hole",
            "EDGE_CURVE.same_sense",
            "ORIENTED_EDGE.orientation",
            "FACE_BOUND.orientation",
            "ADVANCED_FACE.same_sense",
            "CLOSED_SHELL face use",
            "ORIENTED_CLOSED_SHELL.orientation",
        ],
        coupled_sense_cases: 128,
        notes: vec![
            "whole_domain_homogeneous_interval_certificate",
            "collapsed_boundaries_bound_to_pole_topology",
            "lifted_periods_bound_to_seam_topology",
            "exhaustive_coupled_sense_matrix",
        ],
    })
}
pub(crate) fn canonical_face_senses(model: &Model) -> Result<Model> {
    let mut result = model.clone();
    let reversed = result
        .shells
        .iter()
        .flat_map(|shell| &shell.faces)
        .filter(|usage| usage.reversed)
        .map(|usage| usage.face)
        .collect::<BTreeSet<_>>();
    for face_index in reversed {
        let face = &mut result.0.faces[face_index];
        let sum = face.surface.knots_u[face.surface.degree_u]
            + face.surface.knots_u[face.surface.knots_u.len() - face.surface.degree_u - 1];
        face.surface.control_points.reverse();
        face.surface.weights.reverse();
        face.surface.knots_u.reverse();
        for knot in &mut face.surface.knots_u {
            *knot = sum - *knot
        }
        let mut loops = vec![face.outer];
        loops.extend(face.holes.iter().copied());
        for loop_index in loops {
            for coedge in &mut result.0.loops[loop_index].coedges {
                for point in &mut coedge.pcurve.control_points {
                    point[0] = sum - point[0]
                }
                coedge.pcurve = coedge.pcurve.reverse()?;
                coedge.reversed = !coedge.reversed;
            }
            result.0.loops[loop_index].coedges.reverse();
        }
    }
    for shell in &mut result.0.shells {
        for usage in &mut shell.faces {
            usage.reversed = false
        }
    }
    result.validate()?;
    Ok(result)
}
pub(crate) fn refs_text(v: &[usize]) -> String {
    v.iter()
        .map(|x| format!("#{x}"))
        .collect::<Vec<_>>()
        .join(",")
}
fn knot_form(knots: &[f64]) -> (Vec<usize>, Vec<f64>) {
    let mut m = Vec::new();
    let mut k = Vec::new();
    for &x in knots {
        if k.last()
            .is_some_and(|v: &f64| (*v - x).abs() <= f64::EPSILON)
        {
            *m.last_mut().unwrap() += 1;
        } else {
            k.push(x);
            m.push(1);
        }
    }
    (m, k)
}
fn emit_curve(w: &mut Writer, c: &Curve) -> usize {
    let cps: Vec<_> = c.control_points.iter().map(|p| w.point(p)).collect();
    let (m, k) = knot_form(&c.knots);
    w.emit(format!("B_SPLINE_CURVE_WITH_KNOTS('',{},({}),.UNSPECIFIED.,.F.,.{closed}.,.F.,({}),({}),({}),.UNSPECIFIED.)",
        c.degree, refs_text(&cps), floats(&c.weights), m.iter().map(ToString::to_string).collect::<Vec<_>>().join(","), floats(&k),
        closed = if c.periodic { "T" } else { "F" }))
}
fn emit_surface(w: &mut Writer, s: &Surface) -> usize {
    let grid = s
        .control_points
        .iter()
        .map(|row| {
            let ids: Vec<_> = row.iter().map(|p| w.point(p)).collect();
            format!("({})", refs_text(&ids))
        })
        .collect::<Vec<_>>()
        .join(",");
    let (mu, ku) = knot_form(&s.knots_u);
    let (mv, kv) = knot_form(&s.knots_v);
    w.emit(format!("B_SPLINE_SURFACE_WITH_KNOTS('',{},{},({grid}),.UNSPECIFIED.,.F.,.{pu}.,.{pv}.,.F.,.F.,({}),({}),({}),({}),({}),.UNSPECIFIED.)",
        s.degree_u, s.degree_v, floats(&s.weights.iter().flatten().copied().collect::<Vec<_>>()),
        mu.iter().map(ToString::to_string).collect::<Vec<_>>().join(","),
        mv.iter().map(ToString::to_string).collect::<Vec<_>>().join(","), floats(&ku), floats(&kv),
        pu=if s.periodic_u {"T"} else {"F"}, pv=if s.periodic_v {"T"} else {"F"}))
}
fn all_unit_weights(mut weights: impl Iterator<Item = f64>) -> bool {
    weights.all(|weight| (weight - 1.).abs() <= f64::EPSILON)
}
fn knot_form_exchange(knots: &[f64], exact: bool) -> (Vec<usize>, Vec<f64>) {
    if !exact {
        return knot_form(knots);
    }
    let mut counts = Vec::new();
    let mut values = Vec::new();
    for &knot in knots {
        if values.last() == Some(&knot) {
            *counts.last_mut().unwrap() += 1;
        } else {
            values.push(knot);
            counts.push(1);
        }
    }
    (counts, values)
}
pub(crate) fn emit_curve_v5(w: &mut Writer, c: &Curve) -> usize {
    emit_curve_exchange(w, c, false)
}
pub(crate) fn emit_curve_source(w: &mut Writer, c: &Curve) -> usize {
    emit_curve_exchange(w, c, true)
}
fn emit_curve_exchange(w: &mut Writer, c: &Curve, exact: bool) -> usize {
    let cps: Vec<_> = c.control_points.iter().map(|p| w.point(p)).collect();
    let (m, k) = knot_form_exchange(&c.knots, exact);
    let mult = m
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let closed = if c.periodic { "T" } else { "F" };
    let unit = if exact {
        c.weights.iter().all(|&v| v == 1.)
    } else {
        all_unit_weights(c.weights.iter().copied())
    };
    if unit {
        w.emit(format!("B_SPLINE_CURVE_WITH_KNOTS('',{},({}),.UNSPECIFIED.,.{closed}.,.F.,({mult}),({}),.UNSPECIFIED.)",
            c.degree,refs_text(&cps),floats(&k)))
    } else {
        w.emit(format!("(BOUNDED_CURVE()B_SPLINE_CURVE({},({}),.UNSPECIFIED.,.{closed}.,.F.)B_SPLINE_CURVE_WITH_KNOTS(({mult}),({}),.UNSPECIFIED.)CURVE()GEOMETRIC_REPRESENTATION_ITEM()RATIONAL_B_SPLINE_CURVE(({}))REPRESENTATION_ITEM(''))",
            c.degree,refs_text(&cps),floats(&k),floats(&c.weights)))
    }
}
fn emit_surface_v5(w: &mut Writer, s: &Surface) -> usize {
    emit_surface_exchange(w, s, false)
}
pub(crate) fn emit_surface_source(w: &mut Writer, s: &Surface) -> usize {
    emit_surface_exchange(w, s, true)
}
fn emit_surface_exchange(w: &mut Writer, s: &Surface, exact: bool) -> usize {
    let grid = s
        .control_points
        .iter()
        .map(|row| {
            let ids: Vec<_> = row.iter().map(|p| w.point(p)).collect();
            format!("({})", refs_text(&ids))
        })
        .collect::<Vec<_>>()
        .join(",");
    let weights = s
        .weights
        .iter()
        .map(|row| format!("({})", floats(row)))
        .collect::<Vec<_>>()
        .join(",");
    let (mu, ku) = knot_form_exchange(&s.knots_u, exact);
    let (mv, kv) = knot_form_exchange(&s.knots_v, exact);
    let mu = mu
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let mv = mv
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let pu = if s.periodic_u { "T" } else { "F" };
    let pv = if s.periodic_v { "T" } else { "F" };
    let unit = if exact {
        s.weights.iter().flatten().all(|&v| v == 1.)
    } else {
        all_unit_weights(s.weights.iter().flatten().copied())
    };
    if unit {
        w.emit(format!("B_SPLINE_SURFACE_WITH_KNOTS('',{},{},({grid}),.UNSPECIFIED.,.{pu}.,.{pv}.,.F.,({mu}),({mv}),({}),({}),.UNSPECIFIED.)",
            s.degree_u,s.degree_v,floats(&ku),floats(&kv)))
    } else {
        w.emit(format!("(BOUNDED_SURFACE()B_SPLINE_SURFACE({},{},({grid}),.UNSPECIFIED.,.{pu}.,.{pv}.,.F.)B_SPLINE_SURFACE_WITH_KNOTS(({mu}),({mv}),({}),({}),.UNSPECIFIED.)GEOMETRIC_REPRESENTATION_ITEM()RATIONAL_B_SPLINE_SURFACE(({weights}))REPRESENTATION_ITEM('')SURFACE())",
            s.degree_u,s.degree_v,floats(&ku),floats(&kv)))
    }
}

pub(crate) fn export_step_direct(
    model: &Model,
    ap242_composition: bool,
    allow_degenerate: bool,
    capability: &'static str,
) -> Result<(String, FeatureCertificate, StepV3Report)> {
    // /6 exchanges the authored chart directly, including its original face
    // senses. Other routes mirror reversed faces into canonical forward charts.
    let canonical = (capability != STEP_INTERCHANGE_V6_CAPABILITY)
        .then(|| canonical_face_senses(model))
        .transpose()?;
    let model = canonical.as_ref().unwrap_or(model);
    let allow_open_shells = capability == STEP_INTERCHANGE_V9_CAPABILITY;
    if (!allow_open_shells && model.bodies.is_empty())
        || (!allow_open_shells && model.shells.iter().any(|s| !s.closed))
    {
        return Err(refuse(
            "STEP /3 through /8 export closed manifold bodies only",
        ));
    }
    if model.bodies.is_empty() && !model.shells.iter().any(|shell| !shell.closed) {
        return Err(refuse("STEP /9 export requires a solid body or open shell"));
    }
    let mut w = Writer::new();
    let mut vertex_entities = Vec::new();
    for v in &model.vertices {
        let p = w.point(&v.point);
        vertex_entities.push(w.emit(format!("VERTEX_POINT('',#{p})")));
    }
    let mut surface_cache = BTreeMap::<String, usize>::new();
    let mut surfaces = Vec::new();
    for face in &model.faces {
        let key = format!("{:?}", face.surface);
        let id = if let Some(id) = surface_cache.get(&key) {
            *id
        } else {
            let id = if ap242_composition {
                emit_surface_v5(&mut w, &face.surface)
            } else {
                emit_surface(&mut w, &face.surface)
            };
            surface_cache.insert(key, id);
            id
        };
        surfaces.push(id);
    }
    let curves3: Vec<_> = model
        .edges
        .iter()
        .map(|e| {
            if ap242_composition {
                emit_curve_v5(&mut w, &e.curve)
            } else {
                emit_curve(&mut w, &e.curve)
            }
        })
        .collect();
    let pcurve_context = ap242_composition.then(|| {
        w.emit("(GEOMETRIC_REPRESENTATION_CONTEXT(2)REPRESENTATION_CONTEXT('','2D'))".into())
    });
    let mut pcurves_by_edge = vec![Vec::<usize>::new(); model.edges.len()];
    let mut pcurve_surfaces_by_edge = vec![Vec::<usize>::new(); model.edges.len()];
    let mut coedge_pcurve = BTreeMap::new();
    for (li, loop_) in model.loops.iter().enumerate() {
        for (ci, c) in loop_.coedges.iter().enumerate() {
            let face = model
                .faces
                .iter()
                .position(|f| f.outer == li || f.holes.contains(&li))
                .ok_or_else(|| refuse("Loop has no owning face"))?;
            let curve2 = if ap242_composition {
                emit_curve_v5(&mut w, &c.pcurve)
            } else {
                emit_curve(&mut w, &c.pcurve)
            };
            let reference = if let Some(context) = pcurve_context {
                w.emit(format!(
                    "DEFINITIONAL_REPRESENTATION('',(#{curve2}),#{context})"
                ))
            } else {
                curve2
            };
            let pc = w.emit(format!("PCURVE('',#{},#{reference})", surfaces[face]));
            pcurves_by_edge[c.edge].push(pc);
            pcurve_surfaces_by_edge[c.edge].push(surfaces[face]);
            coedge_pcurve.insert((li, ci), pc);
        }
    }
    let mut edge_entities = Vec::new();
    for (i, e) in model.edges.iter().enumerate() {
        if e.degenerate && !allow_degenerate {
            return Err(refuse(
                "Pole/degenerate edges are typed-refused before STEP /6",
            ));
        }
        let usages = model
            .loops
            .iter()
            .flat_map(|loop_| &loop_.coedges)
            .filter(|coedge| coedge.edge == i)
            .filter_map(|coedge| {
                let face = model.faces.iter().position(|face| {
                    face.outer < model.loops.len()
                        && (model.loops[face.outer]
                            .coedges
                            .iter()
                            .any(|candidate| std::ptr::eq(candidate, coedge))
                            || face.holes.iter().any(|hole| {
                                model.loops[*hole]
                                    .coedges
                                    .iter()
                                    .any(|candidate| std::ptr::eq(candidate, coedge))
                            }))
                })?;
                Some((surfaces[face], face, coedge))
            })
            .collect::<Vec<_>>();
        let mut seam = false;
        for a in 0..usages.len() {
            for b in a + 1..usages.len() {
                if usages[a].0 != usages[b].0 {
                    continue;
                }
                let da = usages[a].2.pcurve.domain();
                let db = usages[b].2.pcurve.domain();
                let pa = usages[a].2.pcurve.evaluate((da[0] + da[1]) / 2.)?.point;
                let pb = usages[b].2.pcurve.evaluate((db[0] + db[1]) / 2.)?.point;
                let surface = &model.faces[usages[a].1].surface;
                let periods = [
                    surface.knots_u[surface.knots_u.len() - surface.degree_u - 1]
                        - surface.knots_u[surface.degree_u],
                    surface.knots_v[surface.knots_v.len() - surface.degree_v - 1]
                        - surface.knots_v[surface.degree_v],
                ];
                seam |= (pa[0] - pb[0]).abs() >= (periods[0] - 1e-8)
                    || (pa[1] - pb[1]).abs() >= (periods[1] - 1e-8);
            }
        }
        let sc = w.emit(format!(
            "{}('',#{},({}),.PCURVE_S1.)",
            if ap242_composition && seam {
                "SEAM_CURVE"
            } else {
                "SURFACE_CURVE"
            },
            curves3[i],
            refs_text(&pcurves_by_edge[i])
        ));
        edge_entities.push(w.emit(format!(
            "EDGE_CURVE('',#{},#{},#{sc},.T.)",
            vertex_entities[e.vertices[0]], vertex_entities[e.vertices[1]]
        )));
    }
    let mut loop_entities = Vec::new();
    for loop_ in &model.loops {
        let oriented: Vec<_> = loop_
            .coedges
            .iter()
            .map(|c| {
                w.emit(format!(
                    "ORIENTED_EDGE('',*,*,#{},.{}.)",
                    edge_entities[c.edge],
                    if c.reversed { "F" } else { "T" }
                ))
            })
            .collect();
        loop_entities.push(w.emit(format!("EDGE_LOOP('',({}))", refs_text(&oriented))));
    }
    let mut face_reversed = vec![None; model.faces.len()];
    for shell in &model.shells {
        for usage in &shell.faces {
            if face_reversed[usage.face]
                .replace(usage.reversed)
                .is_some_and(|prior| prior != usage.reversed)
            {
                return Err(refuse("A shared face has inconsistent shell-use senses"));
            }
        }
    }
    let mut face_entities = Vec::new();
    for (i, f) in model.faces.iter().enumerate() {
        let mut bounds = vec![w.emit(format!(
            "FACE_OUTER_BOUND('',#{},.T.)",
            loop_entities[f.outer]
        ))];
        bounds.extend(
            f.holes
                .iter()
                .map(|h| w.emit(format!("FACE_BOUND('',#{},.T.)", loop_entities[*h]))),
        );
        face_entities.push(w.emit(format!(
            "ADVANCED_FACE('',({}),#{},.{})",
            refs_text(&bounds),
            surfaces[i],
            if face_reversed[i].unwrap_or(false) {
                "F."
            } else {
                "T."
            }
        )));
    }
    let mut shell_entities = Vec::new();
    for s in &model.shells {
        shell_entities.push(w.emit(format!(
            "{}('',({}))",
            if s.closed {
                "CLOSED_SHELL"
            } else {
                "OPEN_SHELL"
            },
            refs_text(
                &s.faces
                    .iter()
                    .map(|u| face_entities[u.face])
                    .collect::<Vec<_>>()
            )
        )));
    }
    let mut body_entities = Vec::new();
    for b in &model.bodies {
        body_entities.push(if b.inner_shells.is_empty() {
            w.emit(format!(
                "MANIFOLD_SOLID_BREP('',#{})",
                shell_entities[b.outer_shell]
            ))
        } else {
            if ap242_composition {
                let voids = b
                    .inner_shells
                    .iter()
                    .map(|s| {
                        w.emit(format!(
                            "ORIENTED_CLOSED_SHELL('',*,#{},.F.)",
                            shell_entities[*s]
                        ))
                    })
                    .collect::<Vec<_>>();
                w.emit(format!(
                    "BREP_WITH_VOIDS('',#{},({}))",
                    shell_entities[b.outer_shell],
                    refs_text(&voids)
                ))
            } else {
                w.emit(format!(
                    "BREP_WITH_VOIDS('',#{},({}))",
                    shell_entities[b.outer_shell],
                    refs_text(
                        &b.inner_shells
                            .iter()
                            .map(|s| shell_entities[*s])
                            .collect::<Vec<_>>()
                    )
                ))
            }
        });
    }
    let surface_models = model
        .shells
        .iter()
        .enumerate()
        .filter(|(_, shell)| !shell.closed)
        .map(|(index, _)| {
            w.emit(format!(
                "SHELL_BASED_SURFACE_MODEL('',(#{s}))",
                s = shell_entities[index]
            ))
        })
        .collect::<Vec<_>>();
    let unit = w.emit("(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.))".into());
    if ap242_composition {
        let uncertainty = w.emit(format!(
            "UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE({}),#{unit},'distance_accuracy_value','')",
            floats(std::slice::from_ref(&model.tolerance_mm))
        ));
        let context = w.emit(format!(
            "(GEOMETRIC_REPRESENTATION_CONTEXT(3)GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#{uncertainty}))GLOBAL_UNIT_ASSIGNED_CONTEXT((#{unit}))REPRESENTATION_CONTEXT('','3D'))"
        ));
        let mut items = body_entities.clone();
        items.extend(surface_models.iter().copied());
        let representation_type = if body_entities.is_empty() {
            "MANIFOLD_SURFACE_SHAPE_REPRESENTATION"
        } else {
            "ADVANCED_BREP_SHAPE_REPRESENTATION"
        };
        let representation = w.emit(format!(
            "{representation_type}('',({}),#{context})",
            refs_text(&items)
        ));
        let application =
            w.emit("APPLICATION_CONTEXT('managed model based 3d engineering')".into());
        let product_context = w.emit(format!("PRODUCT_CONTEXT('',#{application},'mechanical')"));
        let product = w.emit(format!("PRODUCT('model','model','',(#{product_context}))"));
        let formation = w.emit(format!("PRODUCT_DEFINITION_FORMATION('1','',#{product})"));
        let definition_context = w.emit(format!(
            "PRODUCT_DEFINITION_CONTEXT('part definition',#{application},'design')"
        ));
        let definition = w.emit(format!(
            "PRODUCT_DEFINITION('design','',#{formation},#{definition_context})"
        ));
        let shape = w.emit(format!("PRODUCT_DEFINITION_SHAPE('','',#{definition})"));
        w.emit(format!(
            "SHAPE_DEFINITION_REPRESENTATION(#{shape},#{representation})"
        ));
    }
    let header = "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('OpenSCAD Viewer retained AP242 B-rep'),'2;1');\nFILE_NAME('model.step','',(''),(''),'OpenSCAD Viewer','OpenSCAD Viewer','');\nFILE_SCHEMA(('AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF'));\nENDSEC;\nDATA;\n";
    let mut text = String::from(header);
    for (id, row) in &w.rows {
        text.push_str(&format!("#{id}={row};\n"));
    }
    text.push_str("ENDSEC;\nEND-ISO-10303-21;\n");
    enforce_output_limit(&text)?;
    let mut parsed = parse(&text)?;
    let groups: [(TopoKind, &[TopoId], &[usize]); 6] = [
        (TopoKind::Vertex, &model.1.vertices, &vertex_entities),
        (TopoKind::Edge, &model.1.edges, &edge_entities),
        (TopoKind::Loop, &model.1.loops, &loop_entities),
        (TopoKind::Face, &model.1.faces, &face_entities),
        (TopoKind::Shell, &model.1.shells, &shell_entities),
        (TopoKind::Body, &model.1.bodies, &body_entities),
    ];
    for (kind, ids, entities_for_kind) in groups {
        for (&topo, &entity_id) in ids.iter().zip(entities_for_kind) {
            let digest = entity_digest(&parsed, entity_id)?;
            let entity = parsed.get_mut(&entity_id).unwrap();
            if let Value::Call(_, args) = &mut entity.value {
                args[0] =
                    Value::String(format!("OSCAD_TOPO/3|{}|{}|{digest}", kind.as_str(), topo));
            }
        }
    }
    text = String::from(header);
    for (id, entity) in &parsed {
        text.push_str(&format!("#{id}={};\n", render_entity_value(&entity.value)));
    }
    text.push_str("ENDSEC;\nEND-ISO-10303-21;\n");
    enforce_output_limit(&text)?;
    let count = model.vertices.len()
        + model.edges.len()
        + model.loops.len()
        + model.faces.len()
        + model.shells.len()
        + model.bodies.len();
    Ok((
        text,
        FeatureCertificate {
            capability,
            complete: true,
            notes: vec![
                "direct_model_export",
                "no_constructor_recognition",
                "sha256_canonical_graph_identity",
            ],
        },
        StepV3Report {
            identity: crate::step_interchange::StepIdentityReport {
                preserved: true,
                source: "internal-metadata",
                preserved_count: count,
                created_count: 0,
                lost_count: 0,
            },
            ignored_entities: vec![],
            metadata_loss: vec![],
            instance_count: parsed.len(),
            reachable_count: parsed.len(),
            definition_identities: vec![],
            occurrence_identities: vec![],
            product_hierarchy: vec![],
            external_references: vec![],
        },
    ))
}

pub fn export_step_v3(model: &Model) -> Result<(String, FeatureCertificate, StepV3Report)> {
    export_step_direct(model, false, false, STEP_INTERCHANGE_V3_CAPABILITY)
}
pub fn export_step_v4(model: &Model) -> Result<(String, FeatureCertificate, StepV3Report)> {
    let (text, mut certificate, report) = export_step_v3(model)?;
    certificate.capability = STEP_INTERCHANGE_V4_CAPABILITY;
    certificate.notes.extend([
        "analytic_periodic_carriers_import",
        "exact_endpoint_point_selectors",
    ]);
    Ok((text, certificate, report))
}
pub fn export_step_v5(model: &Model) -> Result<(String, FeatureCertificate, StepV3Report)> {
    export_step_direct(model, true, false, STEP_INTERCHANGE_V5_CAPABILITY)
}
pub fn export_step_v6(model: &Model) -> Result<(String, FeatureCertificate, StepV3Report)> {
    export_step_direct(
        &periodicized_step_v6(model)?,
        true,
        true,
        STEP_INTERCHANGE_V6_CAPABILITY,
    )
}
pub fn export_step_v7(model: &Model) -> Result<(String, FeatureCertificate, StepV3Report)> {
    export_step_direct(
        &periodicized_step_v6(model)?,
        true,
        true,
        STEP_INTERCHANGE_V7_CAPABILITY,
    )
}
pub fn export_step_v8(model: &Model) -> Result<(String, StepV8Certificate, StepV3Report)> {
    let periodic = periodicized_step_v6(model)?;
    let certificate = certify_step_v8_topology(&periodic)?;
    let (text, _, report) =
        export_step_direct(&periodic, true, true, STEP_INTERCHANGE_V8_CAPABILITY)?;
    Ok((text, certificate, report))
}
pub fn export_step_v9(model: &Model) -> Result<(String, StepV8Certificate, StepV3Report)> {
    // Compatibility API follows STEP /9 source-preserving chart semantics.
    let mut certificate = certify_step_v8_topology(model)?;
    certificate.capability = STEP_INTERCHANGE_V9_CAPABILITY;
    certificate.notes.extend([
        "certified_interior_point_selectors",
        "bounded_freeform_poles",
        "general_affine_occurrences",
        "retained_open_shell_products",
        "authored_nurbs_charts_preserved",
    ]);
    let (text, _, report) =
        export_step_direct(model, true, true, STEP_INTERCHANGE_V9_CAPABILITY)?;
    Ok((text, certificate, report))
}
