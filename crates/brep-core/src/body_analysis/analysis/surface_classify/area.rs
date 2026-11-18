use super::*;
use nurbs_core::curve::CurveSampler;

/// |Su × Sv| at one UV point; zero at poles. Budget ticks per evaluation.
pub(super) fn area_flux(
    surface: &SurfaceRefSampler<'_>,
    u: f64,
    v: f64,
    guard: &mut BudgetGuard,
) -> Result<f64> {
    guard.tick()?;
    let e = surface.evaluate(u, v)?;
    let Some((du, dv)) = e.first_derivatives() else {
        return Err(Error::new(
            "BREP_ANALYSIS_INDETERMINATE",
            "Undefined surface derivative in face-area quadrature",
        ));
    };
    Ok(norm(cross(du, dv)))
}

///Sorted deduplicated interior knot breaks strictly greater than `start`.
fn interior_knot_breaks(knots: &[f64], start: f64) -> Vec<f64> {
    let mut breaks: Vec<f64> = knots.iter().copied().filter(|&k| k > start).collect();
    breaks.sort_by(f64::total_cmp);
    breaks.dedup();
    breaks
}

/// Evaluate `∑_{[low, high]} ...` over knot spans in `[start, upto]` without
/// heap allocation.
fn integrate_u_spans(
    surface: &SurfaceRefSampler<'_>,
    start: f64,
    breaks: &[f64],
    upto: f64,
    v: f64,
    divisions: usize,
    guard: &mut BudgetGuard,
) -> Result<f64> {
    let mut inner = 0.;
    let mut a = start;
    let inv_div = 1. / divisions as f64;
    for &b in breaks {
        if b >= upto {
            break;
        }
        if b > a {
            for piece in 0..divisions {
                let l = a + (b - a) * (piece as f64 * inv_div);
                let h = a + (b - a) * ((piece + 1) as f64 * inv_div);
                for (ix, iw) in GAUSS {
                    let flux = area_flux(surface, (l + h) / 2. + ix * (h - l) / 2., v, guard)?;
                    inner += iw * (h - l) / 2. * flux;
                }
            }
            a = b;
        }
    }
    if upto > a {
        for piece in 0..divisions {
            let l = a + (upto - a) * (piece as f64 * inv_div);
            let h = a + (upto - a) * ((piece + 1) as f64 * inv_div);
            for (ix, iw) in GAUSS {
                let flux = area_flux(surface, (l + h) / 2. + ix * (h - l) / 2., v, guard)?;
                inner += iw * (h - l) / 2. * flux;
            }
        }
    }
    Ok(inner)
}

/// One Green's-theorem area pass with `divisions` subdivisions per span:
/// area = ∮ (∫_{u0}^{u(t)} |N| du) · v'(t) dt over each trim pcurve. Holes
/// contribute with opposite sign automatically because pcurves follow their
/// loop orientation.
pub(super) fn area_pass(
    surface: &SurfaceRefSampler<'_>,
    start_u: f64,
    u_breaks: &[f64],
    pcurves: &[CurveSampler<'_>],
    divisions: usize,
    guard: &mut BudgetGuard,
) -> Result<f64> {
    let mut total = 0.;
    for curve in pcurves {
        let domain = curve.domain();
        for part in 0..divisions {
            let a = domain[0] + (domain[1] - domain[0]) * part as f64 / divisions as f64;
            let b = domain[0] + (domain[1] - domain[0]) * (part + 1) as f64 / divisions as f64;
            for (x, w) in GAUSS {
                let t = (a + b) / 2. + x * (b - a) / 2.;
                let eval = curve.evaluate(t)?;
                let Some(d1) = eval.d1 else {
                    return Err(Error::new(
                        "BREP_ANALYSIS_INDETERMINATE",
                        "Undefined trim derivative in face-area quadrature",
                    ));
                };
                let dv = d1[1];
                if dv == 0. {
                    continue;
                }
                let (u, v) = (eval.point[0], eval.point[1]);
                let inner =
                    integrate_u_spans(surface, start_u, u_breaks, u, v, divisions, guard)?;
                total += w * (b - a) / 2. * dv * inner;
            }
        }
    }
    Ok(total)
}

/// Trimmed surface area of one face (outer wire minus holes), converging to
/// the `mass_properties` integral estimate. Adaptive subdivision refines until
/// the relative change between passes drops below 1e-9 (or 6 refinements),
/// bounded by `budget`.
pub fn face_area(model: &Model, face_index: usize, budget: &Budget) -> Result<f64> {
    if face_index >= model.faces.len() {
        return Err(classify_error("face index out of range"));
    }
    let mut guard: BudgetGuard = budget.guard("face-area");
    guard.check()?;
    let face = &model.faces[face_index];
    let surface_def = &face.surface;
    let surface = SurfaceRefSampler::new(surface_def)?;
    let start_u = surface_def.knots_u[surface_def.degree_u];
    let u_breaks = interior_knot_breaks(&surface_def.knots_u, start_u);
    let mut pcurves = Vec::new();
    for &wire in std::iter::once(&face.outer).chain(&face.holes) {
        for coedge in &model.loops[wire].coedges {
            let domain = coedge.pcurve.domain();
            require_finite_f64(domain[0], "pcurve.domain[0]")?;
            require_finite_f64(domain[1], "pcurve.domain[1]")?;
            pcurves.push(CurveSampler::new(&coedge.pcurve)?);
        }
    }
    let mut previous = area_pass(&surface, start_u, &u_breaks, &pcurves, 1, &mut guard)?;
    for divisions in [2usize, 4, 8, 16, 32] {
        let current = area_pass(&surface, start_u, &u_breaks, &pcurves, divisions, &mut guard)?;
        let delta = (current - previous).abs();
        if delta <= 1e-9 * current.abs().max(1e-300) {
            return Ok(current.abs());
        }
        previous = current;
    }
    Ok(previous.abs())
}
