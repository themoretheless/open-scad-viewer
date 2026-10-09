//! Correlated homogeneous coefficients of the exact rail-corrected expression.
//! Local single-span recipes only. No rounded replacement Surface is created.
use crate::{
    bernstein_interpolation, check, distance_bounds::Interval as I,
    profile_section_bridge::RailConstrainedBridge, Result,
};
type Net = Vec<Vec<[I; 4]>>;
fn single_span(knots: &[f64], degree: usize, count: usize, domain: [f64; 2]) -> Option<usize> {
    (degree..count)
        .find(|&i| knots[i] < knots[i + 1] && knots[i] <= domain[0] && domain[1] <= knots[i + 1])
}
impl RailConstrainedBridge<'_, '_, '_, '_, '_> {
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
mod tests {
    use super::*;
    fn evaluate(mut values: Vec<[I; 4]>, t: f64) -> [I; 4] {
        let q = I::point(1.).sub(I::point(t)).unwrap();
        while values.len() > 1 {
            values = values
                .windows(2)
                .map(|p| {
                    std::array::from_fn(|k| {
                        p[0][k]
                            .mul(q)
                            .unwrap()
                            .add(p[1][k].mul(I::point(t)).unwrap())
                            .unwrap()
                    })
                })
                .collect();
        }
        values[0]
    }
    #[test]
    fn corrected_cylinder_coefficients_cover_independent_rational_positions() {
        let w = std::f64::consts::FRAC_1_SQRT_2;
        let surface = crate::surface::Surface {
            degree_u: 1,
            degree_v: 2,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: [0., 1.]
                .into_iter()
                .map(|x| vec![vec![x, 1., 0.], vec![x, 1., 1.], vec![x, 0., 1.]])
                .collect(),
            weights: vec![vec![1., w, 1.]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let profile = crate::normal_alignment::qualify_weight_profile(&surface, [0, 0], 6)
            .unwrap()
            .0
            .unwrap();
        let bridge = crate::normal_alignment::ProfileSectionBridge::new(
            profile.section(0, 0.).unwrap(),
            profile.section(0, 1.).unwrap(),
            [1.; 2],
        )
        .unwrap();
        let corrected = bridge.constrain_original_rails(100).unwrap().0.unwrap();
        assert!(corrected
            .homogeneous_coefficients([[0., 1.]; 2], 0, 1000000)
            .unwrap()
            .0
            .is_none());
        assert!(corrected
            .homogeneous_coefficients([[0., 1.]; 2], 100000, 0)
            .unwrap()
            .0
            .is_none());
        let (net, spans, work) = corrected
            .homogeneous_coefficients([[0., 1.]; 2], 100000, 1000000)
            .unwrap();
        let net = net.unwrap();
        assert!(spans <= 100000 && work <= 1000000);
        let centers = vec![
            vec![I::point(0.), I::point(0.), I::point(0.), I::point(1.)],
            vec![I::point(1.), I::point(0.), I::point(0.), I::point(1.)],
        ];
        let radius = vec![vec![I::point(1.), I::point(0.), I::point(1.)]];
        let mut radius_work = 0;
        let mut reason = "unverified";
        let residual = crate::moving_radius::bound_coefficient_nets(
            &net,
            &centers,
            &radius,
            1,
            1.,
            &mut radius_work,
            100000000,
            &mut reason,
        )
        .unwrap()
        .unwrap();
        assert!(residual.0[0] <= 0. && residual.0[1] >= 0.);
        assert!(residual.1 < 1e-6, "coefficient radius error {}", residual.1);
        let wrong = vec![vec![I::point(2.), I::point(0.), I::point(1.)]];
        let mismatch = crate::moving_radius::bound_coefficient_nets(
            &net,
            &centers,
            &wrong,
            1,
            2.,
            &mut radius_work,
            100000000,
            &mut reason,
        )
        .unwrap()
        .unwrap();
        assert!(mismatch.0[0] <= -3. && mismatch.0[1] >= -3. && mismatch.0[1] < 0.);
        assert!(crate::moving_radius::bound_coefficient_nets(
            &net,
            &centers,
            &radius,
            1,
            1.,
            &mut 0,
            0,
            &mut reason
        )
        .unwrap()
        .is_none());
        assert_eq!([net.len(), net[0].len()], [6, 5]);
        for free in [0.1, 0.3, 0.7, 0.9] {
            for across in [0.2, 0.5, 0.8] {
                let rows: Vec<_> = net
                    .iter()
                    .map(|row| evaluate(row.clone(), across))
                    .collect();
                let h = evaluate(rows, free);
                let q = 1. - free;
                let denominator = q * q + 2. * w * q * free + free * free;
                let expected = [
                    across,
                    (q * q + 2. * w * q * free) / denominator,
                    (2. * w * q * free + free * free) / denominator,
                ];
                for k in 0..3 {
                    let position = h[k].div(h[3]).unwrap();
                    assert!(
                        position.contains(expected[k]),
                        "{free} {across} axis{k} {position:?}"
                    );
                    assert!(position.width() < 1e-7);
                }
            }
        }
    }
}
