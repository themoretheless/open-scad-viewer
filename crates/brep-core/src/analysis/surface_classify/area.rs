use super::*;

/// |Su × Sv| at one UV point; zero at poles. Budget ticks per evaluation.
pub(super) fn area_flux(surface: &Surface, u: f64, v: f64, guard: &mut BudgetGuard) -> Result<f64> {
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

/// Knot spans of `knots` strictly inside `domain` (endpoints included).
pub(super) fn knot_spans(knots: &[f64], degree: usize, upto: f64) -> Vec<[f64; 2]> {
    let start = knots[degree];
    let mut breaks: Vec<f64> = knots
        .iter()
        .copied()
        .filter(|&k| k > start && k < upto)
        .collect();
    breaks.sort_by(f64::total_cmp);
    breaks.dedup();
    let mut out = Vec::with_capacity(breaks.len() + 1);
    let mut a = start;
    for b in breaks {
        if b > a {
            out.push([a, b]);
        }
        a = b;
    }
    if upto > a {
        out.push([a, upto]);
    }
    out
}

/// One Green's-theorem area pass with `divisions` subdivisions per span:
/// area = ∮ (∫_{u0}^{u(t)} |N| du) · v'(t) dt over each trim pcurve. Holes
/// contribute with opposite sign automatically because pcurves follow their
/// loop orientation.
pub(super) fn area_pass(model: &Model, face_index: usize, divisions: usize, guard: &mut BudgetGuard) -> Result<f64> {
    let face = &model.faces[face_index];
    let surface = &face.surface;
    let mut total = 0.;
    for &wire in std::iter::once(&face.outer).chain(&face.holes) {
        for coedge in &model.loops[wire].coedges {
            let curve = &coedge.pcurve;
            let domain = curve.domain();
            require_finite_f64(domain[0], "pcurve.domain[0]")?;
            require_finite_f64(domain[1], "pcurve.domain[1]")?;
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
                    let mut inner = 0.;
                    for [low, high] in knot_spans(&surface.knots_u, surface.degree_u, u) {
                        for piece in 0..divisions {
                            let l = low + (high - low) * piece as f64 / divisions as f64;
                            let h = low + (high - low) * (piece + 1) as f64 / divisions as f64;
                            for (ix, iw) in GAUSS {
                                let flux =
                                    area_flux(surface, (l + h) / 2. + ix * (h - l) / 2., v, guard)?;
                                inner += iw * (h - l) / 2. * flux;
                            }
                        }
                    }
                    total += w * (b - a) / 2. * dv * inner;
                }
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
    let mut previous = area_pass(model, face_index, 1, &mut guard)?;
    for divisions in [2usize, 4, 8, 16, 32] {
        let current = area_pass(model, face_index, divisions, &mut guard)?;
        let delta = (current - previous).abs();
        if delta <= 1e-9 * current.abs().max(1e-300) {
            return Ok(current.abs());
        }
        previous = current;
    }
    Ok(previous.abs())
}
