use super::*;

/// Certified divergence-theorem volume for general NURBS solids: variable
/// rational weights, lofted walls and curved sweep walls.
pub const CERTIFIED_FREEFORM_VOLUME_CAPABILITY: &str = "certified-freeform-divergence-volume/1";

#[derive(Clone, Debug)]
pub struct CertifiedFreeformVolume {
    pub capability: &'static str,
    /// Outward-rounded enclosure of the exact enclosed volume.
    pub volume_mm3: CertifiedInterval,
    /// `None`: field `F = x`. `Some(e)`: field `F = (x·e) e`, chosen
    /// orthogonal to every trimmed planar face so those faces carry no flux.
    pub field_axis: Option<[f64; 3]>,
    pub integrated_faces: usize,
    pub orthogonal_planar_faces: usize,
    pub cells: usize,
    pub audit: SolidAuditCertificate,
    pub proof: &'static str,
}

fn refuse(message: impl Into<String>) -> Error {
    Error::new("BREP_CERTIFIED_MASS_REFUSED", message)
}

fn surface_domain(surface: &Surface) -> [[f64; 2]; 2] {
    [
        [surface.knots_u[surface.degree_u], surface.knots_u[surface.control_points.len()]],
        [surface.knots_v[surface.degree_v], surface.knots_v[surface.control_points[0].len()]],
    ]
}

/// The face is its whole untrimmed surface chart: no holes, and every outer
/// pcurve is an axis-parallel line on the chart boundary whose union spans
/// the full domain rectangle.
fn full_chart(model: &Model, face: &Face) -> bool {
    if !face.holes.is_empty() {
        return false;
    }
    let [[u0, u1], [v0, v1]] = surface_domain(&face.surface);
    let mut min = [f64::INFINITY; 2];
    let mut max = [f64::NEG_INFINITY; 2];
    let on_boundary = |p: &[f64]| p[0] == u0 || p[0] == u1 || p[1] == v0 || p[1] == v1;
    for coedge in &model.loops[face.outer].coedges {
        let c = &coedge.pcurve;
        if c.degree != 1 || c.control_points.len() != 2 {
            return false;
        }
        let (a, b) = (&c.control_points[0], &c.control_points[1]);
        if !(a[0] == b[0] && (a[0] == u0 || a[0] == u1) || a[1] == b[1] && (a[1] == v0 || a[1] == v1))
            || !on_boundary(a)
            || !on_boundary(b)
        {
            return false;
        }
        for p in [a, b] {
            for k in 0..2 {
                min[k] = min[k].min(p[k]);
                max[k] = max[k].max(p[k]);
            }
        }
    }
    min == [u0, v0] && max == [u1, v1]
}

fn unit(v: [f64; 3]) -> Option<[f64; 3]> {
    let n = point_norm(v);
    (n.is_finite() && n > 0.).then(|| v.map(|x| x / n))
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Enclose the volume of a closed NURBS solid whose curved faces are full
/// untrimmed charts. Trimmed faces must be planar; their normals must leave a
/// common orthogonal direction `e` (always true for at most two distinct cap
/// planes, e.g. loft and bent sweep end caps), and the field `(x·e) e` then
/// has zero flux through them. The flux through full charts is enclosed by
/// [`nurbs_core::surface_volume::enclosed_volume_oriented`]. A residual
/// `|n·e|` from rounding is charged as `(|n·e| + 1e-9) · d² · max|x·e|` per
/// trimmed planar face (`d` its control-hull box diagonal). Closure and
/// orientation come from the model validator and solid audit. Centroid and
/// inertia are not certified here.
pub fn certified_freeform_volume(
    model: &Model,
    tolerance_mm3: f64,
    max_cells: usize,
) -> Result<CertifiedFreeformVolume> {
    if !(tolerance_mm3.is_finite() && tolerance_mm3 > 0.) {
        return Err(Error::new(
            "BREP_CERTIFIED_MASS_OPTIONS_INVALID",
            "Volume tolerance must be finite and positive",
        ));
    }
    model.validate()?;
    let audit = audit_solid(model)?;
    if model.bodies.is_empty() || !model.shells.iter().all(|shell| shell.closed) {
        return Err(refuse("Certified freeform volume requires closed solid bodies"));
    }
    let mut patches = Vec::new();
    let mut planar = Vec::new();
    for body in &model.bodies {
        for &shell in std::iter::once(&body.outer_shell).chain(&body.inner_shells) {
            for use_ in &model.shells[shell].faces {
                let face = &model.faces[use_.face];
                if full_chart(model, face) {
                    patches.push((&face.surface, use_.reversed));
                } else if affine_surface(&face.surface, 0.) {
                    let p = |i: usize, j: usize| point3(&face.surface.control_points[i][j]);
                    let (Some(a), Some(b), Some(c)) = (p(0, 0), p(1, 0), p(0, 1)) else {
                        return Err(refuse("Planar face has non-finite control points"));
                    };
                    let normal = unit(cross(point_sub(b, a), point_sub(c, a)))
                        .ok_or_else(|| refuse("Planar face is degenerate"))?;
                    planar.push((use_.face, normal));
                } else {
                    return Err(refuse(format!(
                        "Face {} is a trimmed non-planar face; certified freeform volume admits full charts and trimmed planar faces only",
                        use_.face
                    )));
                }
            }
        }
    }
    if patches.is_empty() {
        return Err(refuse("No full-chart face carries flux"));
    }
    let axis = match planar.first() {
        None => None,
        Some(&(_, first)) => {
            let other = planar
                .iter()
                .map(|&(_, n)| cross(first, n))
                .find(|c| point_norm(*c) > 1e-6);
            Some(match other.and_then(unit) {
                Some(e) => e,
                None => {
                    // Every normal is parallel: any orthogonal direction works.
                    let helper = if first[0].abs() < 0.9 { [1., 0., 0.] } else { [0., 1., 0.] };
                    unit(cross(first, helper)).ok_or_else(|| refuse("No field axis"))?
                }
            })
        }
    };
    let mut residual = 0f64;
    if let Some(e) = axis {
        for &(face, normal) in &planar {
            let leak = dot(normal, e).abs() + 1e-9;
            if leak > 1e-6 {
                return Err(refuse(format!(
                    "Trimmed planar face {face} is not orthogonal to the common field axis"
                )));
            }
            let mut min = [f64::INFINITY; 3];
            let mut max = [f64::NEG_INFINITY; 3];
            let face = &model.faces[face];
            for &wire in std::iter::once(&face.outer).chain(&face.holes) {
                for coedge in &model.loops[wire].coedges {
                    for p in &model.edges[coedge.edge].curve.control_points {
                        let p = point3(p).ok_or_else(|| refuse("Edge control point is not finite"))?;
                        for k in 0..3 {
                            min[k] = min[k].min(p[k]);
                            max[k] = max[k].max(p[k]);
                        }
                    }
                }
            }
            let diagonal = point_norm(point_sub(max, min));
            let reach = (0..8)
                .map(|bits| {
                    let corner = std::array::from_fn(|k| if bits >> k & 1 == 1 { max[k] } else { min[k] });
                    dot(corner, e).abs()
                })
                .fold(0f64, f64::max);
            residual += 2. * leak * diagonal * diagonal * (reach + diagonal);
        }
    }
    let report = nurbs_core::surface_volume::enclosed_volume_oriented(
        &patches,
        axis,
        tolerance_mm3,
        max_cells,
    )?;
    if !report.within_tolerance {
        return Err(Error::new(
            "BREP_CERTIFIED_MASS_BUDGET_EXHAUSTED",
            format!(
                "Volume enclosure width {} exceeds tolerance within {} cells",
                report.error_upper, report.cells
            ),
        ));
    }
    let volume = CertifiedInterval {
        lower: next_down(report.bounds[0] - residual),
        upper: next_up(report.bounds[1] + residual),
    };
    if !(volume.lower > 0. && volume.upper.is_finite()) {
        return Err(refuse("Certified volume enclosure is not positive; orientation is inconsistent"));
    }
    Ok(CertifiedFreeformVolume {
        capability: CERTIFIED_FREEFORM_VOLUME_CAPABILITY,
        volume_mm3: volume,
        field_axis: axis,
        integrated_faces: patches.len(),
        orthogonal_planar_faces: planar.len(),
        cells: report.cells,
        audit,
        proof: "divergence theorem over full NURBS charts with interval jets and trapezoid remainder; trimmed planar faces orthogonal to the field axis",
    })
}
