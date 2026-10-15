//! Authored-source error for two complete annular end caps. The source map,
//! profile domain and retained face domains are all recomputed natively.
use super::{
    reference_filling::{self, Limits},
    refused,
};
use crate::Model;
use nurbs_core::{
    Result,
    curve::Curve,
    interval_eval::Interval as I,
    progressive_miter::{Sweep, profile_domain_certificate::EndpointDomains},
    progressive_sweep::MultiSweep,
    retained_wall_coefficients,
    retained_wall_domain_certificate::{self, Coedge},
    surface::Surface,
};
mod projective;
#[cfg(test)]
mod smooth_tests;

// Both owners provide original profiles and original endpoint affine maps.
// Keeping dispatch outside the product/domain arithmetic gives both modes the
// same whole-filled-domain theorem and the same shared work ledger.
enum Source<'a, 'b> {
    Miter(&'a Sweep<'b>),
    Smooth(&'a MultiSweep<'b>),
}
impl Source<'_, '_> {
    fn profiles(&self) -> Vec<&Curve> {
        match self {
            Self::Miter(s) => s.source_profiles().iter().collect(),
            Self::Smooth(s) => s.source_profiles(),
        }
    }
    fn domains(&self, sizes: &[usize], tolerance: f64, limits: Limits) -> Result<EndpointDomains> {
        match self {
            Self::Miter(s) => s.certify_ideal_cap_domains(
                sizes,
                tolerance,
                limits.trim_pairs,
                limits.trim_cells,
                limits.exact_work,
            ),
            Self::Smooth(s) => {
                let report = s.certify_ideal_endpoint_domains(
                    sizes,
                    tolerance,
                    limits.trim_pairs,
                    limits.trim_cells,
                    limits.exact_work,
                )?;
                // Annular filling uses original region ownership and endpoint
                // point maps, not a source-to-retained plane projection. No
                // endpoint normal is inferred from a frame axis here.
                Ok(EndpointDomains {
                    ideal_cap_domains_certified: report.ideal_endpoint_domains_certified,
                    source: report.source,
                    endpoint_normals: None,
                    cells: report.cells,
                    reason: report.reason,
                })
            }
        }
    }
    fn endpoint_points(
        &self,
        points: &[[f64; 3]],
        cells: usize,
        law_cells: usize,
    ) -> Result<nurbs_core::progressive_miter::endpoint_point_certificate::Report> {
        match self {
            Self::Miter(s) => s.certify_endpoint_points(points, cells, law_cells),
            Self::Smooth(s) => {
                let report = s.certify_endpoint_points(points, cells)?;
                Ok(
                    nurbs_core::progressive_miter::endpoint_point_certificate::Report {
                        points: report.points,
                        cells: report.cells,
                        reason: report.reason,
                    },
                )
            }
        }
    }
}

pub struct Report {
    pub filled_cap_error_upper: Option<[f64; 2]>,
    pub within_budget: bool,
    pub source_filling_covered: bool,
    pub exact_work: u64,
    pub products: usize,
    pub endpoint_cells: usize,
    pub projective_reparameterized_patches: usize,
    pub domains: EndpointDomains,
    pub source_uv_faces: Option<crate::face_injectivity::Report>,
    pub source_uv_pairs: Option<crate::face_contacts::Report>,
    pub reason: &'static str,
}

fn source_spans(
    sweep: &Source<'_, '_>,
    sizes: &[usize],
    work: &mut u64,
    max_work: u64,
) -> Option<[Vec<Curve>; 2]> {
    let mut rings = [Vec::new(), Vec::new()];
    let mut at = 0;
    let profiles = sweep.profiles();
    for (ring, &count) in rings.iter_mut().zip(sizes) {
        for curve in &profiles[at..at + count] {
            let rows = (curve.degree..curve.control_points.len())
                .filter(|&i| curve.knots[i] < curve.knots[i + 1])
                .count()
                .checked_mul(curve.degree + 1)?;
            if rows as u64 > max_work - *work {
                return None;
            }
            *work += rows as u64;
            let parts = retained_wall_coefficients::segmented_bezier_controls(curve, rows)
                .or_else(|| {
                    if rows as u64 > max_work - *work {
                        return None;
                    }
                    *work += rows as u64;
                    retained_wall_coefficients::exact_bezier_controls(curve, rows)
                })?;
            // Exact Bezier extraction may keep the original span's [a,b].
            // Copying its poles/weights to [0,1] is the exact affine parameter
            // homeomorphism; no floating knot arithmetic changes its image.
            ring.extend(parts.into_iter().map(|mut c| {
                c.knots = std::iter::repeat_n(0., c.degree + 1)
                    .chain(std::iter::repeat_n(1., c.degree + 1))
                    .collect();
                c
            }));
        }
        at += count;
    }
    rings[1].reverse();
    for c in &mut rings[1] {
        *c = c.reverse().ok()?;
    }
    if rings[0].is_empty() || rings[0].len() != rings[1].len() || rings[0].len() > 64 {
        return None;
    }
    Some(rings)
}
fn norm_upper(radii: [f64; 3]) -> Result<f64> {
    let mut square = I::point(0.);
    for r in radii {
        square = square.add(I::point(r).mul(I::point(r))?)?;
    }
    let result = square.hi.max(0.).sqrt().next_up();
    if !result.is_finite() {
        return Err(refused("Authored cap error enclosure overflow"));
    }
    Ok(result)
}

/// B_i are the nonnegative tensor Bernstein basis. Symmetrizing the complete
/// (X*V-Y*W) expansion retains cancellation when source/target weights agree,
/// and also handles independently changed positive weights. No equality of
/// sampled curve images is substituted for pointwise rational correspondence.
fn patch_error(
    source_weights: &[f64],
    ideal: &[[[f64; 2]; 3]],
    retained: &Surface,
    max_products: usize,
) -> Result<Option<(f64, usize)>> {
    patch_error_intervals(
        &source_weights
            .iter()
            .copied()
            .map(I::point)
            .collect::<Vec<_>>(),
        ideal,
        retained,
        max_products,
    )
}
fn patch_error_intervals(
    source_weights: &[I],
    ideal: &[[[f64; 2]; 3]],
    retained: &Surface,
    max_products: usize,
) -> Result<Option<(f64, usize)>> {
    let target = retained
        .control_points
        .iter()
        .flat_map(|r| r.iter())
        .collect::<Vec<_>>();
    let weights = retained
        .weights
        .iter()
        .flatten()
        .copied()
        .collect::<Vec<_>>();
    let n = source_weights.len();
    if n != target.len() || n != weights.len() || n != ideal.len() {
        return Err(refused("Authored cap tensor correspondence mismatch"));
    }
    let Some(products) = n.checked_mul(n).filter(|&n| n <= max_products) else {
        return Ok(None);
    };
    let mut radii = [0_f64; 3];
    for i in 0..n {
        for j in 0..n {
            for k in 0..3 {
                let a = I::new(ideal[i][k][0], ideal[i][k][1])?
                    .sub(I::point(target[j][k]))?
                    .mul(source_weights[i])?
                    .mul(I::point(weights[j]))?;
                let b = I::new(ideal[j][k][0], ideal[j][k][1])?
                    .sub(I::point(target[i][k]))?
                    .mul(source_weights[j])?
                    .mul(I::point(weights[i]))?;
                let coefficient = a.add(b)?.mul(I::point(0.5))?;
                radii[k] = radii[k].max(coefficient.lo.abs().max(coefficient.hi.abs()));
            }
        }
    }
    let source_min = source_weights
        .iter()
        .map(|w| w.lo)
        .fold(f64::INFINITY, f64::min);
    let target_min = weights.iter().copied().fold(f64::INFINITY, f64::min);
    let denominator = I::point(source_min).mul(I::point(target_min))?;
    Ok(Some((
        I::point(norm_upper(radii)?).div(denominator)?.hi,
        products,
    )))
}

pub fn inspect(
    sweep: &Sweep<'_>,
    model: &Model,
    cap_faces: &[Vec<usize>; 2],
    loop_sizes: &[usize],
    tolerance: f64,
    tolerance_uv: f64,
    law_cells: usize,
    limits: Limits,
) -> Result<Report> {
    inspect_source(
        Source::Miter(sweep),
        model,
        cap_faces,
        loop_sizes,
        tolerance,
        tolerance_uv,
        law_cells,
        limits,
    )
}

/// Compare retained annular caps to the original smooth sweep's complete
/// endpoint fillings. This does not admit the enclosing Solid or wall error.
pub fn inspect_smooth(
    sweep: &MultiSweep<'_>,
    model: &Model,
    cap_faces: &[Vec<usize>; 2],
    loop_sizes: &[usize],
    tolerance: f64,
    tolerance_uv: f64,
    limits: Limits,
) -> Result<Report> {
    inspect_source(
        Source::Smooth(sweep),
        model,
        cap_faces,
        loop_sizes,
        tolerance,
        tolerance_uv,
        0,
        limits,
    )
}

fn inspect_source(
    sweep: Source<'_, '_>,
    model: &Model,
    cap_faces: &[Vec<usize>; 2],
    loop_sizes: &[usize],
    tolerance: f64,
    tolerance_uv: f64,
    law_cells: usize,
    limits: Limits,
) -> Result<Report> {
    model.validate()?;
    if loop_sizes.len() != 2
        || loop_sizes.iter().any(|&v| v == 0)
        || loop_sizes.iter().sum::<usize>() != sweep.profiles().len()
        || !tolerance.is_finite()
        || tolerance < 0.
        || limits.products > 100_000
    {
        return Err(refused(
            "Authored annular filling requires two complete original rings and bounded work",
        ));
    }
    let domains = sweep.domains(loop_sizes, tolerance_uv, limits)?;
    let mut out = Report {
        filled_cap_error_upper: None,
        within_budget: false,
        source_filling_covered: false,
        exact_work: domains.source.exact_work,
        products: 0,
        endpoint_cells: 0,
        projective_reparameterized_patches: 0,
        domains,
        source_uv_faces: None,
        source_uv_pairs: None,
        reason: "authored-cap-domain-unproved",
    };
    if !out.domains.ideal_cap_domains_certified {
        return Ok(out);
    }
    let Some(rings) = source_spans(&sweep, loop_sizes, &mut out.exact_work, limits.exact_work)
    else {
        out.reason = "source-annular-decomposition-unproved";
        return Ok(out);
    };
    let n = rings[0].len();
    if cap_faces.iter().any(|f| f.len() != n) {
        return Err(refused(
            "Complete authored end-cap face coverage is required",
        ));
    }
    let mut ids = cap_faces.iter().flatten().copied().collect::<Vec<_>>();
    ids.sort_unstable();
    ids.dedup();
    if ids.len() != 2 * n || ids.last().is_some_and(|&i| i >= model.faces.len()) {
        return Err(refused("End-cap faces must have unique valid ownership"));
    }
    let axis = out.domains.source.source_plane_axis.unwrap();
    let free = match axis {
        0 => [1, 2],
        1 => [0, 2],
        _ => [0, 1],
    };
    let projected = rings.each_ref().map(|ring| {
        ring.iter()
            .map(|c| Curve {
                degree: c.degree,
                knots: c.knots.clone(),
                control_points: c
                    .control_points
                    .iter()
                    .map(|p| vec![p[free[0]], p[free[1]]])
                    .collect(),
                weights: c.weights.clone(),
                periodic: c.periodic,
            })
            .collect()
    });
    let (uv, used) = reference_filling::uv_sheet(
        &projected,
        limits.exact_work - out.exact_work,
        model.tolerance_mm,
    )?;
    out.exact_work += used;
    let faces =
        crate::face_injectivity::inspect_with_linear(&uv, limits.spans, limits.linear_cells)?;
    let pairs = crate::face_contacts::inspect(&uv, tolerance_uv, limits.contacts)?;
    let covered = faces.all_faces_injective && pairs.all_pairs_classified;
    out.source_uv_faces = Some(faces);
    out.source_uv_pairs = Some(pairs);
    out.source_filling_covered = covered;
    if !covered {
        out.reason = "source-annular-filling-coverage-unproved";
        return Ok(out);
    }
    let total_poles: usize = rings[0].iter().map(|a| a.control_points.len() * 2).sum();
    let mut points = Vec::with_capacity(total_poles);
    let mut source_weights = Vec::with_capacity(total_poles);
    let mut offsets = Vec::with_capacity(n + 1);
    offsets.push(0);
    for (a, b) in rings[0].iter().zip(&rings[1]) {
        if a.degree != b.degree || a.knots != b.knots {
            out.reason = "source-ring-tensor-correspondence-unproved";
            return Ok(out);
        }
        for i in 0..a.control_points.len() {
            let pa = &a.control_points[i];
            let pb = &b.control_points[i];
            points.push([pa[0], pa[1], pa[2]]);
            points.push([pb[0], pb[1], pb[2]]);
            source_weights.extend([a.weights[i], b.weights[i]]);
        }
        offsets.push(points.len());
    }
    let endpoint =
        sweep.endpoint_points(&points, limits.trim_cells - out.domains.cells, law_cells)?;
    out.endpoint_cells = endpoint.cells;
    let Some(ideal) = endpoint.points else {
        out.reason = endpoint
            .reason
            .unwrap_or("endpoint-control-enclosure-unproved");
        return Ok(out);
    };
    let mut upper = [0_f64; 2];
    let mut edges = Vec::with_capacity(4);
    let mut authored = Vec::new();
    for end in 0..2 {
        for span in 0..n {
            let f = &model.faces[cap_faces[end][span]];
            let source = &rings[0][span];
            if !f.holes.is_empty()
                || f.surface.degree_u != source.degree
                || f.surface.degree_v != 1
                || f.surface.knots_u != source.knots
                || f.surface.knots_v != [0., 0., 1., 1.]
                || f.surface.control_points.len() != source.control_points.len()
                || f.surface.control_points.iter().any(|r| r.len() != 2)
                || f.surface.periodic_u
                || f.surface.periodic_v
            {
                out.reason = "retained-annular-tensor-correspondence-unproved";
                return Ok(out);
            }
            if out.exact_work == limits.exact_work {
                out.reason = "retained-cap-domain-work-exhausted";
                return Ok(out);
            }
            edges.clear();
            edges.extend(model.loops[f.outer].coedges.iter().map(|c| Coedge {
                world: &model.edges[c.edge].curve,
                uv: &c.pcurve,
                reversed: c.reversed,
            }));
            let domain = retained_wall_domain_certificate::inspect(
                &f.surface,
                &edges,
                false,
                limits.exact_work - out.exact_work,
            )?;
            out.exact_work += domain.work;
            if !domain.domain_certified {
                out.reason = "retained-cap-domain-unproved";
                return Ok(out);
            }
            let range = offsets[span]..offsets[span + 1];
            authored.clear();
            authored.extend(ideal[range.clone()].iter().map(|p| p[end]));
            let Some(bound) = projective::inspect(
                &source_weights[range],
                &authored,
                &f.surface,
                tolerance,
                limits.products - out.products,
            )?
            else {
                out.reason = "authored-cap-product-budget-exhausted";
                return Ok(out);
            };
            out.products += bound.products;
            out.projective_reparameterized_patches += usize::from(bound.reparameterized);
            upper[end] = upper[end].max(bound.upper);
        }
    }
    out.filled_cap_error_upper = Some(upper);
    out.within_budget = upper.iter().all(|&v| v <= tolerance);
    out.reason = if out.within_budget {
        "authored-filled-annular-caps-certified"
    } else {
        "authored-filled-annular-caps-exceed-budget"
    };
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn weighted_product_enclosure_retains_cancellation_and_charges_all_pairs() {
        let mut s = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1., 2.], vec![3., 4.]],
            periodic_u: false,
            periodic_v: false,
        };
        let ideal = s
            .control_points
            .iter()
            .flatten()
            .map(|p| std::array::from_fn(|k| [p[k]; 2]))
            .collect::<Vec<_>>();
        let w = s.weights.iter().flatten().copied().collect::<Vec<_>>();
        let (same, used) = patch_error(&w, &ideal, &s, 16).unwrap().unwrap();
        assert!(same < 1e-12);
        assert_eq!(used, 16);
        assert!(patch_error(&w, &ideal, &s, 15).unwrap().is_none());
        for weight in s.weights.iter_mut().flatten() {
            *weight *= 2.;
        }
        let (equivalent, _) = patch_error(&w, &ideal, &s, 16).unwrap().unwrap();
        assert!(
            equivalent < 1e-12,
            "Uniform homogeneous scaling preserves the full surface image"
        );
        for weight in s.weights.iter_mut().flatten() {
            *weight *= 0.5;
        }
        s.weights[0][1] *= 2.;
        let (changed, _) = patch_error(&w, &ideal, &s, 16).unwrap().unwrap();
        assert!(
            changed >= 65_f64.sqrt() / 60.,
            "Different weights must not reuse a same-basis coefficient bound"
        );
        // At u=v=1/2 the two rational points are (7/10,3/5) and
        // (7/12,2/3), with independently computed squared distance 65/3600.
    }
}
