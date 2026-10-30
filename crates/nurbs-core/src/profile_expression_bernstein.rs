//! Correlated homogeneous coefficients of the exact rail-corrected expression.
//! Local single-span recipes only. No rounded replacement Surface is created.
use crate::{
    Result, bernstein_interpolation, check, distance_bounds::Interval as I,
    profile_section_bridge::RailConstrainedBridge,
};
type Net = Vec<Vec<[I; 4]>>;
fn single_span(knots: &[f64], degree: usize, count: usize, domain: [f64; 2]) -> Option<usize> {
    (degree..count)
        .find(|&i| knots[i] < knots[i + 1] && knots[i] <= domain[0] && domain[1] <= knots[i + 1])
}
fn law_coefficients(
    curve: &crate::curve::Curve,
    law: [f64; 2],
    across: [f64; 2],
    max_spans: usize,
    max_work: usize,
) -> Result<(Option<Vec<Vec<I>>>, usize, usize)> {
    curve.validate()?;
    let natural = curve.domain();
    let d = curve.control_points[0].len();
    check(
        d <= 3
            && law.iter().chain(&across).all(|x| x.is_finite())
            && law[0] < law[1]
            && law[0] >= natural[0]
            && law[1] <= natural[1]
            && across[0] < across[1]
            && across[0] >= 0.
            && across[1] <= 1.,
        "Choose original law domains",
    )?;
    if curve.degree > 16 || max_spans == 0 {
        return Ok((None, 0, 0));
    }
    let Some(span) = single_span(&curve.knots, curve.degree, curve.control_points.len(), law)
    else {
        return Ok((None, 0, 0));
    };
    let n = curve.degree + 1;
    let nodes: Vec<_> = (0..n)
        .map(|i| {
            if i == 0 {
                across[0]
            } else if i + 1 == n {
                across[1]
            } else {
                let t = i as f64 / (n - 1) as f64;
                across[0] * (1. - t) + across[1] * t
            }
        })
        .collect();
    if nodes.windows(2).any(|w| w[0] >= w[1]) {
        return Ok((None, 0, 0));
    }
    let (basis, mut work) = bernstein_interpolation::basis_on(&nodes, across, max_work)?;
    let Some(basis) = basis else {
        return Ok((None, 0, work));
    };
    let reconstruction = (d + 1) * n * n;
    if reconstruction > max_work - work || n > max_spans {
        return Ok((None, 0, work));
    }
    let mut samples = Vec::new();
    for &node in &nodes {
        let parameter = I::point(law[0])
            .add(I::point(node).mul(I::point(law[1]).sub(I::point(law[0]))?)?)?
            .intersect(law[0], law[1])?;
        let raw = crate::curve_distance::restricted_controls(curve, span, parameter)?;
        let mut h = vec![I::point(0.); d + 1];
        for k in 0..=d {
            h[k] = I::new(
                raw.iter().map(|p| p[k].lo).fold(f64::INFINITY, f64::min),
                raw.iter()
                    .map(|p| p[k].hi)
                    .fold(f64::NEG_INFINITY, f64::max),
            )?;
        }
        for k in 0..d {
            h[k] = h[k].add(I::point(curve.control_points[span - curve.degree][k]).mul(h[d])?)?;
        }
        samples.push(h);
    }
    let mut coefficients = vec![vec![I::point(0.); d + 1]; n];
    for k in 0..=d {
        let component: Vec<_> = samples.iter().map(|h| h[k]).collect();
        for (i, value) in bernstein_interpolation::reconstruct(&component, &basis)?
            .into_iter()
            .enumerate()
        {
            coefficients[i][k] = value;
        }
    }
    work += reconstruction;
    Ok((Some(coefficients), n, work))
}
/// A whole-cell interval bound, not a caller-issued radius certificate.
pub struct CoefficientRadiusBound {
    pub squared_residual: Option<[f64; 2]>,
    pub error_upper: Option<f64>,
    /// Whole-cell lower bound on absolute radial error; zero is inconclusive.
    pub error_lower: Option<f64>,
    pub spans: usize,
    pub coefficient_work: usize,
    pub reason: &'static str,
}
pub struct CoefficientLineReport {
    pub intersections: Option<crate::ray_surface::Report>,
    pub spans: usize,
    pub coefficient_work: usize,
    pub reason: &'static str,
}
impl RailConstrainedBridge<'_, '_, '_, '_, '_> {
    /// Isolate roots of the actual corrected expression, without a rounded
    /// replacement Surface. UV trim and material crossing parity are separate.
    pub fn inspect_line_intersections(
        &self,
        domain: [[f64; 2]; 2],
        origin: [f64; 3],
        direction: [f64; 3],
        tolerance_uv: f64,
        cells: usize,
        spans: usize,
        coefficient_work: usize,
    ) -> Result<CoefficientLineReport> {
        check(
            origin.iter().chain(&direction).all(|v| v.is_finite())
                && direction.iter().any(|v| *v != 0.)
                && tolerance_uv.is_finite()
                && tolerance_uv > 0.
                && cells <= 100000,
            "Choose a finite line and bounded expression root work",
        )?;
        let (net, spans, coefficient_work) =
            self.homogeneous_coefficients(domain, spans, coefficient_work)?;
        let Some(net) = net else {
            return Ok(CoefficientLineReport {
                intersections: None,
                spans,
                coefficient_work,
                reason: "expression-line-coefficients-unresolved",
            });
        };
        let roots = crate::ray_surface::intersections_coefficients(
            &net,
            domain,
            origin,
            direction,
            tolerance_uv,
            cells,
        )?;
        let reason = if roots.complete {
            "expression-line-roots-isolated"
        } else {
            "expression-line-roots-unresolved"
        };
        Ok(CoefficientLineReport {
            intersections: Some(roots),
            spans,
            coefficient_work,
            reason,
        })
    }
    pub fn inspect_radius_coefficients(
        &self,
        centers: &crate::curve::Curve,
        radius: &crate::curve::Curve,
        laws: [[f64; 2]; 2],
        domain: [[f64; 2]; 2],
        max_spans: usize,
        max_work: usize,
    ) -> Result<CoefficientRadiusBound> {
        centers.validate()?;
        radius.validate()?;
        check(
            centers.control_points[0].len() == 3
                && radius.control_points[0].len() == 2
                && radius.control_points.iter().all(|p| p[0] >= 0.),
            "Choose center and nonnegative radius laws",
        )?;
        let (net, spans, work) = self.homogeneous_coefficients(domain, max_spans, max_work)?;
        let mut out = CoefficientRadiusBound {
            squared_residual: None,
            error_upper: None,
            error_lower: None,
            spans,
            coefficient_work: work,
            reason: "expression-radius-coefficients-unresolved",
        };
        let Some(net) = net else {
            return Ok(out);
        };
        let (centers, spent, work) = law_coefficients(
            centers,
            laws[0],
            domain[1],
            max_spans - out.spans,
            max_work - out.coefficient_work,
        )?;
        out.spans += spent;
        out.coefficient_work += work;
        let Some(centers) = centers else {
            out.reason = "expression-radius-center-coefficients-unresolved";
            return Ok(out);
        };
        let (radius, spent, work) = law_coefficients(
            radius,
            laws[1],
            domain[1],
            max_spans - out.spans,
            max_work - out.coefficient_work,
        )?;
        out.spans += spent;
        out.coefficient_work += work;
        let Some(radius) = radius else {
            out.reason = "expression-radius-law-coefficients-unresolved";
            return Ok(out);
        };
        let rlo = if radius.iter().all(|h| h[2].lo > 0.) {
            radius
                .iter()
                .map(|h| h[0].div(h[2]).map(|r| r.lo))
                .collect::<Result<Vec<_>>>()?
                .into_iter()
                .fold(f64::INFINITY, f64::min)
                .max(0.)
        } else {
            0.
        };
        let mut used = out.coefficient_work as u64;
        let bound = crate::moving_radius::bound_coefficient_nets(
            &net,
            &centers,
            &radius,
            1,
            rlo,
            &mut used,
            max_work as u64,
            &mut out.reason,
        )?;
        out.coefficient_work = used as usize;
        if let Some((residual, error)) = bound {
            out.squared_residual = Some(residual);
            out.error_upper = Some(error);
            if radius.iter().all(|h| h[2].lo > 0.) {
                let radius_upper = radius.iter().map(|h| h[0].div(h[2]).map(|r| r.hi))
                    .collect::<Result<Vec<_>>>()?.into_iter().fold(0., f64::max);
                let minimum = if residual[0] > 0. { residual[0] }
                    else if residual[1] < 0. { -residual[1] } else { 0. };
                // |S-C| <= r + error_upper, hence |S-C|+r <= 2*r_upper+error_upper.
                // Divide the signed residual's minimum magnitude outward.
                let denominator = I::point(2.).mul(I::point(radius_upper))?
                    .add(I::point(error))?.hi;
                out.error_lower = Some(if minimum > 0. && denominator > 0. {
                    I::point(minimum).div(I::point(denominator))?.lo.max(0.)
                } else { 0. });
            }
            out.reason = "expression-radius-coefficient-bound";
        }
        Ok(out)
    }
    /// Multiply the expression by its factored source denominator before
    /// interpolation. Sections and their across jets have free denominator Wf;
    /// fixed-free source rails and their free jets have across denominator Wa.
    /// The cubic Hermite corrections therefore have common denominator Wf*Wa
    /// and numerator degree at most [free_degree+3,across_degree+3].
    /// Degree admission belongs to this immutable recipe, never caller samples.
    pub(crate) fn homogeneous_coefficients(
        &self,
        domain: [[f64; 2]; 2],
        max_spans: usize,
        max_work: usize,
    ) -> Result<(Option<Net>, usize, usize)> {
        let first = self.section(false);
        let last = self.section(true);
        let range = first.range();
        check(
            domain
                .iter()
                .all(|r| r.iter().all(|x| x.is_finite()) && r[0] < r[1])
                && domain[0][0] >= range[0]
                && domain[0][1] <= range[1]
                && domain[1][0] >= 0.
                && domain[1][1] <= 1.
                && max_spans <= 1000000
                && max_work <= 100000000,
            "Bound exact expression coefficient domains and work",
        )?;
        let profile = first.profile();
        if max_spans == 0 || max_work == 0 {
            return Ok((None, 0, 0));
        }
        let source = profile.source();
        let axis = first.axis();
        let degrees = [source.degree_u, source.degree_v];
        if degrees.iter().any(|&d| d > 13) {
            return Ok((None, 0, 0));
        }
        let cuts = [
            first.parameter().min(last.parameter()),
            first.parameter().max(last.parameter()),
        ];
        let source_domain = if axis == 0 {
            [cuts, domain[0]]
        } else {
            [domain[0], cuts]
        };
        let counts = [source.control_points.len(), source.control_points[0].len()];
        let knots = [&source.knots_u, &source.knots_v];
        let mut span = [0; 2];
        for a in 0..2 {
            let Some(index) = single_span(knots[a], degrees[a], counts[a], source_domain[a]) else {
                return Ok((None, 0, 0));
            };
            span[a] = index;
        }
        let sizes = [degrees[1 - axis] + 4, degrees[axis] + 4];
        let nodes: [Vec<f64>; 2] = std::array::from_fn(|a| {
            (0..sizes[a])
                .map(|i| {
                    if i == 0 {
                        domain[a][0]
                    } else if i + 1 == sizes[a] {
                        domain[a][1]
                    } else {
                        let t = i as f64 / (sizes[a] - 1) as f64;
                        domain[a][0] * (1. - t) + domain[a][1] * t
                    }
                })
                .collect()
        });
        if nodes.iter().any(|n| n.windows(2).any(|w| w[0] >= w[1])) {
            return Ok((None, 0, 0));
        }
        let (free_basis, mut work) =
            bernstein_interpolation::basis_on(&nodes[0], domain[0], max_work)?;
        let Some(free_basis) = free_basis else {
            return Ok((None, 0, work));
        };
        let (across_basis, used) =
            bernstein_interpolation::basis_on(&nodes[1], domain[1], max_work - work)?;
        work += used;
        let Some(across_basis) = across_basis else {
            return Ok((None, 0, work));
        };
        let reconstruction = 4 * sizes[0] * sizes[1] * (sizes[0] + sizes[1]);
        if reconstruction > max_work - work {
            return Ok((None, 0, work));
        }
        let mut spans = 0;
        let mut samples = vec![vec![[I::point(0.); 4]; sizes[1]]; sizes[0]];
        for (i, &free) in nodes[0].iter().enumerate() {
            for (j, &across) in nodes[1].iter().enumerate() {
                let (position, used) = self.enclose_with_transverse_correction(
                    [free; 2],
                    [across; 2],
                    (max_spans - spans).min(100000),
                )?;
                spans += used;
                let Some(position) = position else {
                    return Ok((None, spans, work));
                };
                if spans == max_spans {
                    return Ok((None, spans, work));
                }
                let coordinate = I::point(first.parameter())
                    .add(
                        I::point(across)
                            .mul(I::point(last.parameter()).sub(I::point(first.parameter()))?)?,
                    )?
                    .intersect(cuts[0], cuts[1])?;
                let mut uv = [[free; 2]; 2];
                uv[axis] = [coordinate.lo, coordinate.hi];
                let homogeneous = profile.homogeneous_on(span, uv)?;
                spans += 1;
                let weight = I::new(
                    homogeneous
                        .iter()
                        .flatten()
                        .map(|h| h[3].lo)
                        .fold(f64::INFINITY, f64::min),
                    homogeneous
                        .iter()
                        .flatten()
                        .map(|h| h[3].hi)
                        .fold(f64::NEG_INFINITY, f64::max),
                )?;
                for k in 0..3 {
                    samples[i][j][k] = I::new(position[k][0], position[k][1])?.mul(weight)?;
                }
                samples[i][j][3] = weight;
            }
        }
        let mut net = vec![vec![[I::point(0.); 4]; sizes[1]]; sizes[0]];
        for k in 0..4 {
            let values: Vec<Vec<_>> = samples
                .iter()
                .map(|r| r.iter().map(|h| h[k]).collect())
                .collect();
            let (coefficients, used) = bernstein_interpolation::reconstruct_tensor(
                &values,
                [&free_basis, &across_basis],
                max_work - work,
            )?;
            work += used;
            let Some(coefficients) = coefficients else {
                return Ok((None, spans, work));
            };
            for i in 0..sizes[0] {
                for j in 0..sizes[1] {
                    net[i][j][k] = coefficients[i][j];
                }
            }
        }
        Ok((Some(net), spans, work))
    }
}

#[cfg(test)]
#[path = "profile_expression_bernstein/tests/mod.rs"]
mod tests;

impl crate::surface_weight_profile::WeightProfile<'_> {
    /// Isolate on the original exact separable-weight recipe, including a
    /// restricted retained rectangle. Multiple knot spans remain explicit.
    pub fn inspect_line_intersections(
        &self,
        domain: [[f64; 2]; 2],
        origin: [f64; 3],
        direction: [f64; 3],
        tolerance_uv: f64,
        max_cells: usize,
        max_spans: usize,
        max_work: usize,
    ) -> Result<CoefficientLineReport> {
        let s = self.source();
        check(
            domain
                .iter()
                .all(|d| d.iter().all(|x| x.is_finite()) && d[0] < d[1])
                && origin.iter().chain(&direction).all(|x| x.is_finite())
                && direction.iter().any(|x| *x != 0.)
                && tolerance_uv.is_finite()
                && tolerance_uv > 0.
                && max_cells <= 100000
                && max_spans <= 1000000
                && max_work <= 100000000,
            "Choose bounded profile line isolation parameters",
        )?;
        let knots = [&s.knots_u, &s.knots_v];
        let degrees = [s.degree_u, s.degree_v];
        let counts = [s.control_points.len(), s.control_points[0].len()];
        check(
            (0..2).all(|k| {
                domain[k][0] >= knots[k][degrees[k]] && domain[k][1] <= knots[k][counts[k]]
            }),
            "Keep profile isolation in the natural domain",
        )?;
        if s.degree_u > 16 || s.degree_v > 16 {
            return Ok(CoefficientLineReport {
                intersections: None,
                spans: 0,
                coefficient_work: 0,
                reason: "profile-line-degree-limit",
            });
        }
        let work = (s.degree_u + 1)
            .saturating_mul(s.degree_v + 1)
            .saturating_mul(4);
        let mut spans_used = 0;
        let mut work_used = 0;
        let mut report = crate::ray_surface::Report {
            roots: Vec::new(),
            unresolved: Vec::new(),
            cells: 0,
            complete: false,
            profile_parameterization: None,
        };
        for u in s.degree_u..counts[0] {
            for v in s.degree_v..counts[1] {
                let rect = [
                    [
                        domain[0][0].max(knots[0][u]),
                        domain[0][1].min(knots[0][u + 1]),
                    ],
                    [
                        domain[1][0].max(knots[1][v]),
                        domain[1][1].min(knots[1][v + 1]),
                    ],
                ];
                if rect.iter().any(|d| d[0] >= d[1]) {
                    continue;
                }
                if spans_used == max_spans
                    || work > max_work - work_used
                    || report.cells == max_cells
                {
                    report.unresolved.push(rect);
                    continue;
                }
                let raw = self.homogeneous_on([u, v], rect)?;
                spans_used += 1;
                work_used += work;
                let net = raw
                    .into_iter()
                    .map(|row| {
                        row.into_iter()
                            .map(|p| p.map(|x| I { lo: x.lo, hi: x.hi }))
                            .collect()
                    })
                    .collect::<Vec<Vec<_>>>();
                let isolated = crate::ray_surface::intersections_coefficients(
                    &net,
                    rect,
                    origin,
                    direction,
                    tolerance_uv,
                    max_cells - report.cells,
                )?;
                report.cells += isolated.cells;
                report.roots.extend(isolated.roots);
                report.unresolved.extend(isolated.unresolved);
            }
        }
        report.complete = report.unresolved.is_empty();
        let reason = if report.complete {
            "profile-line-roots-isolated"
        } else {
            "profile-line-roots-unresolved"
        };
        Ok(CoefficientLineReport {
            intersections: Some(report),
            spans: spans_used,
            coefficient_work: work_used,
            reason,
        })
    }
}
