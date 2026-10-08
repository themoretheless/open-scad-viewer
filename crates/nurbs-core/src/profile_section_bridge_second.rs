//! Second free jets of the exact full side-Hermite correction.
use super::*;
impl ProfileSectionBridge<'_, '_, '_, '_> {
    fn second_free_bounds(
        &self,
        free: [f64; 2],
        across: [f64; 2],
        budget: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        self.validate(free, across, budget)?;
        let axis = self.section(false).axis();
        let mut order = [0; 2];
        order[1 - axis] = 2;
        if across == [0.; 2] || across == [1.; 2] {
            let section = self.section(across[0] == 1.);
            return section.profile().higher_derivative_bounds(
                section.rectangle(free)?,
                order,
                budget,
            );
        }
        let t = I::new(across[0], across[1])?;
        let q = I::point(1.).sub(t)?;
        let h = [
            q.mul(q)?.mul(I::point(1.).add(I::point(2.).mul(t)?)?)?,
            t.mul(t)?.mul(I::point(3.).sub(I::point(2.).mul(t)?)?)?,
        ];
        let g = [
            t.mul(q)?.mul(q)?.mul(I::point(self.scales[0]))?,
            I::point(-1.)
                .mul(t)?
                .mul(t)?
                .mul(q)?
                .mul(I::point(self.scales[1]))?,
        ];
        let mut out = [I::point(0.); 3];
        let mut spent = 0;
        for side in 0..2 {
            for transverse in 0..2 {
                if spent == budget {
                    return Ok((None, spent));
                }
                let section = self.section(side == 1);
                let mut order = order;
                order[axis] = transverse;
                let (values, used) = section.profile().higher_derivative_bounds(
                    section.rectangle(free)?,
                    order,
                    budget - spent,
                )?;
                spent += used;
                let Some(values) = values else {
                    return Ok((None, spent));
                };
                let factor = if transverse == 0 { h[side] } else { g[side] };
                for k in 0..3 {
                    out[k] = out[k].add(factor.mul(I::new(values[k][0], values[k][1])?)?)?;
                }
            }
        }
        Ok((Some(out.map(|x| [x.lo, x.hi])), spent))
    }
}
impl RailConstrainedBridge<'_, '_, '_, '_, '_> {
    /// Includes the second derivatives of both correction basis functions.
    /// A natural rail preserves position and first jets, not the second jet.
    pub fn second_free_derivatives_with_transverse_correction(
        &self,
        free: [f64; 2],
        across: [f64; 2],
        budget: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        self.bridge.validate(free, across, budget)?;
        let (base, mut spent) = self.bridge.second_free_bounds(free, across, budget)?;
        let Some(base) = base else {
            return Ok((None, spent));
        };
        if across == [0.; 2] || across == [1.; 2] {
            return Ok((Some(base), spent));
        }
        let range = self.section(false).range();
        let width = I::point(range[1]).sub(I::point(range[0]))?;
        let t = I::new(free[0], free[1])?
            .sub(I::point(range[0]))?
            .div(width)?;
        let ddh = [
            I::point(12.)
                .mul(t)?
                .sub(I::point(6.))?
                .div(width.mul(width)?)?,
            I::point(6.)
                .sub(I::point(12.).mul(t)?)?
                .div(width.mul(width)?)?,
        ];
        let ddg = [
            I::point(6.).mul(t)?.sub(I::point(4.))?.div(width)?,
            I::point(6.).mul(t)?.sub(I::point(2.))?.div(width)?,
        ];
        let mut out = base
            .map(|x| I::new(x[0], x[1]))
            .into_iter()
            .collect::<Result<Vec<_>>>()?;
        for high in [false, true] {
            let mut values = Vec::new();
            for query in 0..4 {
                if spent == budget {
                    return Ok((None, spent));
                }
                let (value, used) = match query {
                    0 => self.source_rail(high, across, budget - spent)?,
                    1 => self.bridge.rail(high).enclose(across, budget - spent)?,
                    2 => {
                        let (jet, used) = self.section(false).profile().derivative_bounds(
                            self.source_rail_domain(high, across)?,
                            budget - spent,
                        )?;
                        (
                            jet.map(|j| j.map(|k| k[1 - self.section(false).axis()])),
                            used,
                        )
                    }
                    _ => {
                        let (jet, used) = self.bridge.derivative_bounds(
                            [range[usize::from(high)]; 2],
                            across,
                            budget - spent,
                        )?;
                        (jet.map(|j| j[0]), used)
                    }
                };
                spent += used;
                let Some(value) = value else {
                    return Ok((None, spent));
                };
                values.push(value);
            }
            let (remainders, used) = self.side_remainders(high, across, budget - spent)?;
            spent += used;
            if remainders.is_none() && spent == budget {
                return Ok((None, spent));
            }
            for k in 0..3 {
                let value = |i: usize| I::new(values[i][k][0], values[i][k][1]);
                let mut position = value(0)?.sub(value(1)?)?;
                let mut tangent = value(2)?.sub(value(3)?)?;
                if let Some(r) = remainders {
                    position = position.intersect(r[0][k][0], r[0][k][1])?;
                    tangent = tangent.intersect(r[2][k][0], r[2][k][1])?;
                }
                out[k] = out[k]
                    .add(ddh[usize::from(high)].mul(position)?)?
                    .add(ddg[usize::from(high)].mul(tangent)?)?;
            }
        }
        Ok((Some(std::array::from_fn(|k| [out[k].lo, out[k].hi])), spent))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn full_hermite_second_free_jet_matches_independent_quartic_surface() {
        // S(u,v)=(u,v,u^4 v^2). Across interpolation has a quartic remainder.
        // The exact free-side Hermite correction restores v^2 times that
        // remainder, so the corrected result is S and S_vv=(0,0,2u^4).
        for axis in [0, 1] {
            for rational in [false, true] {
                let mut source = crate::surface::Surface {
                    degree_u: 4,
                    degree_v: 2,
                    knots_u: vec![0., 0., 0., 0., 0., 1., 1., 1., 1., 1.],
                    knots_v: vec![0., 0., 0., 1., 1., 1.],
                    control_points: (0..5)
                        .map(|u| {
                            (0..3)
                                .map(|v| {
                                    vec![
                                        u as f64 / 4.,
                                        v as f64 / 2.,
                                        if u == 4 && v == 2 { 1. } else { 0. },
                                    ]
                                })
                                .collect()
                        })
                        .collect(),
                    weights: vec![vec![1.; 3]; 5],
                    periodic_u: false,
                    periodic_v: false,
                };
                if rational {
                    source.weights[4].fill(2.);
                }
                if axis == 1 {
                    source.control_points = (0..3)
                        .map(|v| {
                            (0..5)
                                .map(|u| source.control_points[u][v].clone())
                                .collect()
                        })
                        .collect();
                    source.weights = (0..3)
                        .map(|v| (0..5).map(|u| source.weights[u][v]).collect())
                        .collect();
                    std::mem::swap(&mut source.degree_u, &mut source.degree_v);
                    std::mem::swap(&mut source.knots_u, &mut source.knots_v);
                }
                let profile = crate::normal_alignment::qualify_weight_profile(&source, [0, 0], 100)
                    .unwrap()
                    .0
                    .unwrap();
                let bridge = ProfileSectionBridge::new(
                    profile.section(axis, 0.5).unwrap(),
                    profile.section(axis, 0.75).unwrap(),
                    [0.25; 2],
                )
                .unwrap();
                let corrected = bridge.constrain_original_rails(100).unwrap().0.unwrap();
                for free in [0., 0.25, 0.5, 0.75, 1.] {
                    for across in [0., 0.25, 0.5, 0.75, 1.] {
                        let (bounds, _) = corrected
                            .second_free_derivatives_with_transverse_correction(
                                [free; 2],
                                [across; 2],
                                100,
                            )
                            .unwrap();
                        let bounds = bounds.unwrap();
                        let u = 0.5 + 0.25 * across;
                        let fourth = u * u * u * u;
                        let expected = [
                            0.,
                            0.,
                            if rational {
                                4. * fourth / (1. + fourth)
                            } else {
                                2. * fourth
                            },
                        ];
                        for k in 0..3 {
                            assert!(
                                bounds[k][0] <= expected[k] && expected[k] <= bounds[k][1],
                                "free={free} across={across} axis={k}: {:?} vs {}",
                                bounds[k],
                                expected[k]
                            );
                        }
                    }
                }
                // The analytic derivative is independent of free and monotone in
                // across, so its endpoint range is its whole rectangle image.
                for across in [[0., 1.], [0.125, 0.875], [0.25, 0.5]] {
                    let bounds = corrected
                        .second_free_derivatives_with_transverse_correction([0., 1.], across, 100)
                        .unwrap()
                        .0
                        .unwrap();
                    for parameter in across {
                        let u = 0.5 + 0.25 * parameter;
                        let fourth = u.powi(4);
                        let z = if rational {
                            4. * fourth / (1. + fourth)
                        } else {
                            2. * fourth
                        };
                        assert!(bounds[0][0] <= 0. && bounds[0][1] >= 0.);
                        assert!(bounds[1][0] <= 0. && bounds[1][1] >= 0.);
                        assert!(bounds[2][0] <= z && z <= bounds[2][1]);
                    }
                }
                let wide = corrected
                    .second_free_derivatives_with_transverse_correction([0., 1.], [0., 1.], 100)
                    .unwrap()
                    .0
                    .unwrap();
                let narrow = corrected
                    .second_free_derivatives_with_transverse_correction(
                        [0.49999, 0.50001],
                        [0.49999, 0.50001],
                        100,
                    )
                    .unwrap()
                    .0
                    .unwrap();
                assert!(
                    narrow[2][1] - narrow[2][0] < 0.01 * (wide[2][1] - wide[2][0]),
                    "axis={axis} rational={rational}: wide={wide:?}, narrow={narrow:?}"
                );
                let (_, spent) = corrected
                    .second_free_derivatives_with_transverse_correction([0.5; 2], [0.5; 2], 100)
                    .unwrap();
                assert!(corrected
                    .second_free_derivatives_with_transverse_correction(
                        [0.5; 2],
                        [0.5; 2],
                        spent - 1
                    )
                    .unwrap()
                    .0
                    .is_none());
                assert!(corrected
                    .second_free_derivatives_with_transverse_correction([0.5; 2], [0.5; 2], 0)
                    .unwrap()
                    .0
                    .is_none());
            }
        }
    }
}
