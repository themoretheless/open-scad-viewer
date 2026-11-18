//! Fail-closed operand and face-pair classification at the Boolean boundary.
//! These reports classify work; they do not confer topology-authorship proofs.
use crate::{Model, Result, Surface};
use nurbs_core::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BooleanOperandKind {
    Empty,
    Solid,
    Sheet,
}

/// Distinguish closed solids from open sheets before any material operation.
/// Mixed-dimensional and unowned-shell models are not silently promoted.
pub fn classify_boolean_operand(model: &Model) -> Result<BooleanOperandKind> {
    if model.is_empty() {
        return Ok(BooleanOperandKind::Empty);
    }
    let mut owned = vec![false; model.shells.len()];
    for body in &model.bodies {
        for shell in std::iter::once(body.outer_shell).chain(body.inner_shells.iter().copied()) {
            let slot = owned.get_mut(shell).ok_or_else(|| {
                Error::new(
                    "BREP_BOOLEAN_OPERAND_KIND_UNSUPPORTED",
                    "Body references a missing shell",
                )
            })?;
            *slot = true;
        }
    }
    if !model.bodies.is_empty()
        && owned.into_iter().all(|used| used)
        && model.shells.iter().all(|shell| shell.closed)
    {
        return Ok(BooleanOperandKind::Solid);
    }
    if model.bodies.is_empty()
        && !model.shells.is_empty()
        && model.shells.iter().all(|shell| !shell.closed)
    {
        return Ok(BooleanOperandKind::Sheet);
    }
    Err(Error::new(
        "BREP_BOOLEAN_OPERAND_KIND_UNSUPPORTED",
        "Mixed-dimensional or unowned-shell Boolean operand is unsupported",
    ))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SheetBooleanSemantics {
    /// Trim a solid by a sheet and retain one explicitly authored material side.
    TrimSolidBySheet,
    /// Regularized two-dimensional intersection of two sheets.
    RegularizedSheetIntersection,
    /// Sew two sheets along a certified common boundary; no solid is implied.
    SewnSheetUnion,
}

/// Explicit sheet-operation boundary. The semantics are intentionally not
/// folded into solid CSG. Until a corresponding arrangement/closure proof is
/// available, every request is a typed refusal rather than a guessed solid.
pub fn boolean_with_sheet_semantics(
    a: &Model,
    b: &Model,
    semantics: SheetBooleanSemantics,
) -> Result<Model> {
    a.validate()?;
    b.validate()?;
    let kinds = [classify_boolean_operand(a)?, classify_boolean_operand(b)?];
    let shape_matches = match semantics {
        SheetBooleanSemantics::TrimSolidBySheet => matches!(
            kinds,
            [BooleanOperandKind::Solid, BooleanOperandKind::Sheet]
                | [BooleanOperandKind::Sheet, BooleanOperandKind::Solid]
        ),
        SheetBooleanSemantics::RegularizedSheetIntersection
        | SheetBooleanSemantics::SewnSheetUnion => {
            kinds == [BooleanOperandKind::Sheet, BooleanOperandKind::Sheet]
        }
    };
    if !shape_matches {
        return Err(Error::new(
            "BREP_SHEET_BOOLEAN_OPERAND_MISMATCH",
            "Operand dimensions do not match the requested sheet Boolean semantics",
        ));
    }
    Err(Error::new(
        "BREP_SHEET_BOOLEAN_UNIMPLEMENTED",
        "Sheet arrangement and material-side certification are not implemented for this request",
    ))
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UvEquivalence {
    pub transpose: bool,
    pub reverse_u: bool,
    pub reverse_v: bool,
    pub opposite_normals: bool,
}

pub fn surface_uv_domains(surface: &Surface) -> [[f64; 2]; 2] {
    [
        [
            surface.knots_u[surface.degree_u],
            surface.knots_u[surface.control_points.len()],
        ],
        [
            surface.knots_v[surface.degree_v],
            surface.knots_v[surface.control_points[0].len()],
        ],
    ]
}

impl UvEquivalence {
    /// Map a UV point on the second carrier into the first carrier's chart.
    pub fn second_to_first(self, _second: &Surface, first: &Surface, uv: [f64; 2]) -> [f64; 2] {
        let mut out = if self.transpose { [uv[1], uv[0]] } else { uv };
        let domains = surface_uv_domains(first);
        let du = domains[0];
        let dv = domains[1];
        if self.reverse_u {
            out[0] = du[0] + du[1] - out[0];
        }
        if self.reverse_v {
            out[1] = dv[0] + dv[1] - out[1];
        }
        out
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CoincidentFaceClass {
    /// Literal whole-surface identity, including parameterization and weights.
    IdenticalCarrier,
    /// Exact normalized tensor-product representation under an explicit UV map.
    EquivalentNurbs(UvEquivalence),
    /// Both complete control nets certify one plane within the input tolerance.
    CoincidentPlane { opposite_normals: bool },
    /// Coaxial cylindrical carriers with a convex-hull positional deviation bound.
    CoincidentCylinder {
        opposite_normals: bool,
        max_deviation_mm: f64,
    },
    /// Parallel/nearby within the arbitration band, but not admitted coincident.
    NearCoincident,
    /// Separated, transverse, or lacking a sufficient coincidence proof.
    NonCoincident,
}

#[derive(Clone, Copy)]
struct Plane {
    normal: [f64; 3],
    offset: f64,
    residual: f64,
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn plane(surface: &Surface) -> Option<Plane> {
    let points = surface.control_points.iter().flatten().collect::<Vec<_>>();
    let first = points.first()?.as_slice();
    let origin = <[f64; 3]>::try_from(first).ok()?;
    let distance_squared = |p: &Vec<f64>| -> Option<f64> {
        let q = <[f64; 3]>::try_from(p.as_slice()).ok()?;
        Some((0..3).map(|i| (q[i] - origin[i]).powi(2)).sum())
    };
    let a = points.iter().skip(1).max_by(|p, q| {
        distance_squared(p)
            .unwrap_or(f64::NAN)
            .total_cmp(&distance_squared(q).unwrap_or(f64::NAN))
    })?;
    let a = <[f64; 3]>::try_from(a.as_slice()).ok()?;
    let u: [f64; 3] = std::array::from_fn(|i| a[i] - origin[i]);
    let mut normal = [0.; 3];
    let mut best = 0.;
    // Two linear scans choose a well-conditioned triple without a quadratic
    // preclassification cost on dense NURBS control nets.
    for p in points.iter().skip(1) {
        let b = <[f64; 3]>::try_from(p.as_slice()).ok()?;
        let v: [f64; 3] = std::array::from_fn(|i| b[i] - origin[i]);
        let n = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        let length = dot(n, n).sqrt();
        if length > best {
            best = length;
            normal = n;
        }
    }
    if !best.is_finite() || best <= f64::EPSILON {
        return None;
    }
    normal = normal.map(|x| x / best);
    let offset = dot(normal, origin);
    let residual = points
        .iter()
        .map(|p| {
            <[f64; 3]>::try_from(p.as_slice())
                .ok()
                .map(|q| (dot(normal, q) - offset).abs())
        })
        .collect::<Option<Vec<_>>>()?
        .into_iter()
        .fold(0., f64::max);
    residual.is_finite().then_some(Plane {
        normal,
        offset,
        residual,
    })
}

fn normalized_exact(surface: &Surface) -> Option<Surface> {
    surface.validate().ok()?;
    let mut out = surface.clone();
    for (degree, count, knots) in [
        (out.degree_u, out.control_points.len(), &mut out.knots_u),
        (out.degree_v, out.control_points[0].len(), &mut out.knots_v),
    ] {
        let a = knots[degree];
        let b = knots[count];
        let scale = b - a;
        if !scale.is_finite() || scale <= 0. {
            return None;
        }
        for knot in knots.iter_mut() {
            *knot = (*knot - a) / scale;
        }
    }
    let scale = out.weights[0][0];
    if !scale.is_finite() || scale <= 0. {
        return None;
    }
    for weight in out.weights.iter_mut().flatten() {
        *weight /= scale;
    }
    Some(out)
}

/// Exact representation equivalence, not a hash or a proximity comparison.
/// Periodicity is part of the evidence and is never silently unwrapped.
pub fn certify_nurbs_uv_equivalence(a: &Surface, b: &Surface) -> Option<UvEquivalence> {
    use nurbs_core::surface::Axis;
    for transpose in [false, true] {
        for reverse_u in [false, true] {
            for reverse_v in [false, true] {
                let mut mapped = b.clone();
                if transpose {
                    mapped = nurbs_core::surface_edit::transpose(&mapped).ok()?;
                }
                if reverse_u {
                    mapped = nurbs_core::surface_edit::reverse(&mapped, Axis::U).ok()?;
                }
                if reverse_v {
                    mapped = nurbs_core::surface_edit::reverse(&mapped, Axis::V).ok()?;
                }
                if mapped.periodic_u != a.periodic_u || mapped.periodic_v != a.periodic_v {
                    continue;
                }
                if normalized_exact(a)? == normalized_exact(&mapped)? {
                    return Some(UvEquivalence {
                        transpose,
                        reverse_u,
                        reverse_v,
                        opposite_normals: transpose ^ reverse_u ^ reverse_v,
                    });
                }
            }
        }
    }
    None
}

#[derive(Clone, Copy)]
struct Cylinder {
    axis: [f64; 3],
    point: [f64; 3],
    radius: f64,
    residual: f64,
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| a[i] - b[i])
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn norm(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}

fn cylinder(surface: &Surface) -> Option<Cylinder> {
    let points = surface
        .control_points
        .iter()
        .flatten()
        .map(|p| <[f64; 3]>::try_from(p.as_slice()).ok())
        .collect::<Option<Vec<_>>>()?;
    let origin = *points.first()?;
    // Surface normals of a cylinder are perpendicular to its axis. Candidates
    // come only from original surface derivatives; every control pole is then
    // checked, giving a convex-hull bound for the entire rational surface.
    let domains = surface_uv_domains(surface);
    let du = domains[0];
    let dv = domains[1];
    let mut normals = Vec::new();
    for i in 0..3 {
        for j in 0..3 {
            let evaluation = surface
                .evaluate(
                    du[0] + (du[1] - du[0]) * (i as f64 + 0.25) / 3.,
                    dv[0] + (dv[1] - dv[0]) * (j as f64 + 0.25) / 3.,
                )
                .ok()?;
            let (a, b) = evaluation.first_derivatives()?;
            let n = cross(a, b);
            let length = norm(n);
            if length > 1e-12 && length.is_finite() {
                normals.push(n.map(|x| x / length));
            }
        }
    }
    let mut axis = None;
    for (i, n) in normals.iter().enumerate() {
        for m in normals.iter().skip(i + 1) {
            let candidate = cross(*n, *m);
            let length = norm(candidate);
            if length <= 1e-10 {
                continue;
            }
            let candidate = candidate.map(|x| x / length);
            if normals.iter().any(|n| dot(*n, candidate).abs() > 1e-8) {
                continue;
            }
            let radial = points
                .iter()
                .map(|p| {
                    let d = sub(*p, origin);
                    norm(sub(d, candidate.map(|x| x * dot(d, candidate))))
                })
                .collect::<Vec<_>>();
            let mean = radial.iter().sum::<f64>() / radial.len() as f64;
            let residual = radial.iter().map(|r| (r - mean).abs()).fold(0., f64::max);
            if axis
                .as_ref()
                .is_none_or(|old: &Cylinder| residual < old.residual)
            {
                axis = Some(Cylinder {
                    axis: candidate,
                    point: origin,
                    radius: mean,
                    residual,
                });
            }
        }
    }
    let mut result = axis?;
    // Project the axis point to the centroid to remove arbitrary axial offset.
    let centroid = points.iter().fold([0.; 3], |a, p| {
        std::array::from_fn(|i| a[i] + p[i] / points.len() as f64)
    });
    result.point = centroid;
    result.residual = points
        .iter()
        .map(|p| {
            let d = sub(*p, centroid);
            (norm(sub(d, result.axis.map(|x| x * dot(d, result.axis)))) - result.radius).abs()
        })
        .fold(0., f64::max);
    (result.radius > 0. && result.residual.is_finite()).then_some(result)
}

/// Preclassify a face pair before any SSI marching. Exact normalized NURBS
/// equivalence carries a UV map; analytic coincidence and near coincidence
/// remain distinct evidence classes.
pub fn preclassify_coincident_faces(
    a: &Model,
    face_a: usize,
    b: &Model,
    face_b: usize,
) -> Result<CoincidentFaceClass> {
    let (fa, fb) = match (a.faces.get(face_a), b.faces.get(face_b)) {
        (Some(fa), Some(fb)) => (fa, fb),
        _ => {
            return Err(Error::new(
                "BREP_BOOLEAN_FACE_CLASSIFICATION_INPUT",
                "Coincident-face preclassification index is out of range",
            ));
        }
    };
    if fa.surface == fb.surface {
        return Ok(CoincidentFaceClass::IdenticalCarrier);
    }
    if let Some(map) = certify_nurbs_uv_equivalence(&fa.surface, &fb.surface) {
        return Ok(CoincidentFaceClass::EquivalentNurbs(map));
    }
    let tolerance = a.tolerance_mm.max(b.tolerance_mm);
    if !tolerance.is_finite() || tolerance <= 0. {
        return Err(Error::new(
            "BREP_BOOLEAN_FACE_CLASSIFICATION_INPUT",
            "Face classification needs a finite positive tolerance",
        ));
    }
    if let (Some(ca), Some(cb)) = (cylinder(&fa.surface), cylinder(&fb.surface)) {
        let alignment = dot(ca.axis, cb.axis);
        let axis_distance = norm(sub(
            sub(ca.point, cb.point),
            ca.axis.map(|x| x * dot(sub(ca.point, cb.point), ca.axis)),
        ));
        // Rational-cylinder poles need not lie on the cylinder. When both
        // surfaces use the identical weighted parameter basis, corresponding
        // pole distances nevertheless bound their mutual positional deviation
        // by the convex-hull property; do not mistake pole radius for surface radius.
        let same_basis = fa.surface.degree_u == fb.surface.degree_u
            && fa.surface.degree_v == fb.surface.degree_v
            && fa.surface.knots_u == fb.surface.knots_u
            && fa.surface.knots_v == fb.surface.knots_v
            && fa.surface.weights == fb.surface.weights
            && fa.surface.periodic_u == fb.surface.periodic_u
            && fa.surface.periodic_v == fb.surface.periodic_v
            && fa.surface.control_points.len() == fb.surface.control_points.len()
            && fa
                .surface
                .control_points
                .iter()
                .zip(&fb.surface.control_points)
                .all(|(a, b)| a.len() == b.len());
        let pole_deviation = same_basis.then(|| {
            fa.surface
                .control_points
                .iter()
                .flatten()
                .zip(fb.surface.control_points.iter().flatten())
                .map(|(a, b)| {
                    if a.len() != b.len() || a.iter().chain(b).any(|v| !v.is_finite()) {
                        return f64::INFINITY;
                    }
                    a.iter()
                        .zip(b)
                        .map(|(a, b)| (a - b) * (a - b))
                        .sum::<f64>()
                        .sqrt()
                })
                .fold(0., f64::max)
        });
        let deviation = pole_deviation.unwrap_or(f64::INFINITY);
        if alignment.abs() >= 1. - 1e-12
            && axis_distance <= tolerance
            && (ca.radius - cb.radius).abs() <= tolerance
        {
            if deviation <= tolerance {
                let domains = surface_uv_domains(&fa.surface);
                let uv = [
                    (domains[0][0] + domains[0][1]) / 2.,
                    (domains[1][0] + domains[1][1]) / 2.,
                ];
                let normal = |surface: &Surface| {
                    surface
                        .evaluate(uv[0], uv[1])
                        .ok()
                        .and_then(|e| e.first_derivatives())
                        .map(|(a, b)| cross(a, b))
                };
                let opposite_normals = match (normal(&fa.surface), normal(&fb.surface)) {
                    (Some(a), Some(b)) => dot(a, b) < 0.,
                    _ => return Ok(CoincidentFaceClass::NonCoincident),
                };
                return Ok(CoincidentFaceClass::CoincidentCylinder {
                    opposite_normals,
                    max_deviation_mm: deviation,
                });
            }
            if deviation <= tolerance * 4. {
                return Ok(CoincidentFaceClass::NearCoincident);
            }
        }
    }
    let (Some(pa), Some(pb)) = (plane(&fa.surface), plane(&fb.surface)) else {
        return Ok(CoincidentFaceClass::NonCoincident);
    };
    let alignment = dot(pa.normal, pb.normal);
    let distance = if alignment >= 0. {
        (pa.offset - pb.offset).abs()
    } else {
        (pa.offset + pb.offset).abs()
    };
    if alignment.abs() < 1. - 1e-12 {
        return Ok(CoincidentFaceClass::NonCoincident);
    }
    if pa.residual <= tolerance && pb.residual <= tolerance && distance <= tolerance {
        Ok(CoincidentFaceClass::CoincidentPlane {
            opposite_normals: alignment < 0.,
        })
    } else if pa.residual <= tolerance * 4.
        && pb.residual <= tolerance * 4.
        && distance <= tolerance * 4.
    {
        Ok(CoincidentFaceClass::NearCoincident)
    } else {
        Ok(CoincidentFaceClass::NonCoincident)
    }
}
