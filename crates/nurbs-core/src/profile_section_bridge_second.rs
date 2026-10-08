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
    fn source_first_free_rail(
        &self,
        high: bool,
        across: [f64; 2],
        budget: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        let section = self.section(false);
        let source = section.profile().source();
        let axis = section.axis();
        let free = 1 - axis;
        let degrees = [source.degree_u, source.degree_v];
        let counts = [source.control_points.len(), source.control_points[0].len()];
        let knots = [&source.knots_u, &source.knots_v];
        let q = degrees[axis];
        let p = degrees[free];
        if budget == 0
            || p == 0
            || q > 8
            || counts[axis] != q + 1
            || [source.periodic_u, source.periodic_v][axis]
            || !(0..=q).all(|i| knots[axis][i] == knots[axis][q])
            || !(q + 1..knots[axis].len()).all(|i| knots[axis][i] == knots[axis][counts[axis]])
        {
            return Ok((None, 0));
        }
        let endpoint = if high { counts[free] - 1 } else { 0 };
        let adjacent = if high { endpoint - 1 } else { 1 };
        let range = if high {
            [knots[free][counts[free] - 1], knots[free][counts[free]]]
        } else {
            [knots[free][p], knots[free][p + 1]]
        };
        let (rows, columns, _) = section.profile().factors();
        let factors = [rows, columns];
        let scale = I::point(p as f64)
            .div(I::point(range[1]).sub(I::point(range[0]))?)?
            .mul(I::point(factors[free][adjacent]).div(I::point(factors[free][endpoint]))?)?;
        let cp = |i: usize, j: usize| {
            if axis == 0 {
                &source.control_points[i][j]
            } else {
                &source.control_points[j][i]
            }
        };
        let mut net = Vec::new();
        for i in 0..=q {
            let a = cp(i, endpoint);
            let b = cp(i, adjacent);
            let weight = I::point(factors[axis][i]);
            let mut value = [I::point(0.); 4];
            value[3] = weight;
            for k in 0..3 {
                let difference = if a[k] == b[k] {
                    I::point(0.)
                } else if high {
                    I::point(a[k]).sub(I::point(b[k]))?
                } else {
                    I::point(b[k]).sub(I::point(a[k]))?
                };
                value[k] = difference.mul(weight)?;
            }
            net.push(vec![value]);
        }
        let domain = self.source_rail_domain(high, across)?;
        let span = [[knots[axis][q], knots[axis][counts[axis]]], [0., 1.]];
        let hull =
            crate::normal_alignment::restrict_hull(&net, [q, 0], span, [domain[axis], [0., 1.]])?;
        let mut out = [[0.; 2]; 3];
        for k in 0..3 {
            let value = hull[k].div(hull[3])?.mul(scale)?;
            out[k] = [value.lo, value.hi];
        }
        Ok((Some(out), 1))
    }
    /// Integrate the full second free jet from an exactly preserved source
    /// rail first jet. Returns no bound if a path knot permits a C1 jump.
    pub fn first_free_from_rail_bounds(
        &self,
        free: [f64; 2],
        across: [f64; 2],
        budget: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        let (parts, spent) = self.free_jet_from_rail_parts(free, across, budget)?;
        let Some((first, second, delta)) = parts else {
            return Ok((None, spent));
        };
        let mut out = [[0.; 2]; 3];
        for k in 0..3 {
            let v = I::new(first[k][0], first[k][1])?
                .add(I::new(second[k][0], second[k][1])?.mul(delta)?)?;
            out[k] = [v.lo, v.hi];
        }
        Ok((Some(out), spent))
    }
    pub(super) fn free_jet_from_rail_parts(
        &self,
        free: [f64; 2],
        across: [f64; 2],
        budget: usize,
    ) -> Result<(Option<([[f64; 2]; 3], [[f64; 2]; 3], I)>, usize)> {
        self.bridge.validate(free, across, budget)?;
        let section = self.section(false);
        let range = section.range();
        let high = free[0] + (free[1] - free[0]) * 0.5 > range[0] + (range[1] - range[0]) * 0.5;
        let anchor = range[usize::from(high)];
        let path = [free[0].min(anchor), free[1].max(anchor)];
        let source = section.profile().source();
        let axis = 1 - section.axis();
        let knots = if axis == 0 {
            &source.knots_u
        } else {
            &source.knots_v
        };
        let degree = if axis == 0 {
            source.degree_u
        } else {
            source.degree_v
        };
        if knots.iter().any(|&k| {
            k >= path[0]
                && k <= path[1]
                && k > range[0]
                && k < range[1]
                && knots.iter().filter(|&&v| v == k).count() >= degree
        }) {
            return Ok((None, 0));
        }
        let (direct, mut spent) = self.source_first_free_rail(high, across, budget)?;
        let first = if let Some(direct) = direct {
            direct
        } else {
            let (jet, used) = section
                .profile()
                .derivative_bounds(self.source_rail_domain(high, across)?, budget - spent)?;
            spent += used;
            let Some(jet) = jet else {
                return Ok((None, spent));
            };
            jet.map(|k| k[axis])
        };
        if free == [anchor; 2] {
            return Ok((Some((first, [[0.; 2]; 3], I::point(0.))), spent));
        }
        let (second, used) =
            self.second_free_derivatives_with_transverse_correction(path, across, budget - spent)?;
        spent += used;
        let Some(second) = second else {
            return Ok((None, spent));
        };
        let delta = I::new(free[0], free[1])?.sub(I::point(anchor))?;
        Ok((Some((first, second, delta)), spent))
    }
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

// Keep the common displacement in both rows of the free Jacobian column.
/// Project before multiplying by the shared rail displacement. Coordinate
/// boxes alone lose this correlation when the projection mixes coordinates.
pub(super) fn projected_rail_column(
    projection: [[f64; 3]; 2],
    first: [[f64; 2]; 3],
    second: [[f64; 2]; 3],
    delta: I,
) -> Result<[I; 2]> {
    let mut out = [I::point(0.); 2];
    for row in 0..2 {
        let mut anchor = I::point(0.);
        let mut curvature = I::point(0.);
        for k in 0..3 {
            if projection[row][k] == 0. {
                continue;
            }
            let scale = I::point(projection[row][k]);
            anchor = anchor.add(scale.mul(I::new(first[k][0], first[k][1])?)?)?;
            curvature = curvature.add(scale.mul(I::new(second[k][0], second[k][1])?)?)?;
        }
        out[row] = anchor.add(delta.mul(curvature)?)?;
    }
    Ok(out)
}
pub(super) fn projected_rail_determinant(
    projection: [[f64; 3]; 2],
    first: [[f64; 2]; 3],
    second: [[f64; 2]; 3],
    delta: I,
    across: [I; 2],
) -> Result<I> {
    let mut a = [I::point(0.); 2];
    let mut b = [I::point(0.); 2];
    for row in 0..2 {
        for k in 0..3 {
            if projection[row][k] == 0. {
                continue;
            }
            let scale = I::point(projection[row][k]);
            a[row] = a[row].add(scale.mul(I::new(first[k][0], first[k][1])?)?)?;
            b[row] = b[row].add(scale.mul(I::new(second[k][0], second[k][1])?)?)?;
        }
    }
    let anchor = a[0].mul(across[1])?.sub(across[0].mul(a[1])?)?;
    let curvature = b[0].mul(across[1])?.sub(across[0].mul(b[1])?)?;
    anchor.add(delta.mul(curvature)?)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn projected_rail_column_keeps_coordinate_displacement_shared() {
        let first = [[1e-12; 2], [0.; 2], [0.; 2]];
        let second = [[-2.; 2], [1.; 2], [0.; 2]];
        let delta = I::new(-1., 0.).unwrap();
        let independent = I::point(1e-12)
            .add(delta.mul(I::point(-2.)).unwrap())
            .unwrap()
            .add(delta.mul(I::point(1.)).unwrap())
            .unwrap();
        assert!(independent.lo < 0.);
        let column =
            projected_rail_column([[1., 1., 0.], [0., 0., 1.]], first, second, delta).unwrap();
        assert!(column[0].lo > 0. && column[0].hi >= 1.);
        let crossing = projected_rail_column(
            [[1., 1., 0.], [0., 0., 1.]],
            first,
            [[2.; 2], [-1.; 2], [0.; 2]],
            delta,
        )
        .unwrap();
        assert!(crossing[0].lo < 0. && crossing[0].hi > 0.);
    }
    #[test]
    fn rail_determinant_preserves_common_displacement_and_interior_zeros() {
        let epsilon = 1e-12;
        let first = [[epsilon; 2], [0.1 * epsilon; 2], [0.; 2]];
        let second = [[-1.; 2], [-0.1; 2], [0.; 2]];
        let projection = [[1., 0., 0.], [0., 1., 0.]];
        let delta = I::new(-1., 0.).unwrap();
        let x = I::point(epsilon)
            .add(delta.mul(I::point(-1.)).unwrap())
            .unwrap();
        let y = I::point(0.1 * epsilon)
            .add(delta.mul(I::point(-0.1)).unwrap())
            .unwrap();
        assert!(x.sub(y).unwrap().lo < 0.);
        let bound = projected_rail_determinant(projection, first, second, delta, [I::point(1.); 2])
            .unwrap();
        assert!(bound.lo > 0. && bound.hi >= 0.9);
        let crossing = projected_rail_determinant(
            projection,
            first,
            [[1.; 2], [0.1; 2], [0.; 2]],
            delta,
            [I::point(1.); 2],
        )
        .unwrap();
        assert!(crossing.lo < 0. && crossing.hi > 0.);
    }
    #[test]
    fn first_free_rail_keeps_small_jets_at_large_world_coordinates() {
        let large = 2f64.powi(29);
        let ulp = f64::from_bits(large.to_bits() + 1) - large;
        let source = crate::surface::Surface {
            degree_u: 1,
            degree_v: 2,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: (0..2)
                .map(|u| {
                    vec![
                        vec![large - 1000., u as f64, 0.],
                        vec![large - ulp, u as f64, 0.5],
                        vec![large, u as f64, 1.],
                    ]
                })
                .collect(),
            weights: vec![vec![1.; 3]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let profile = crate::normal_alignment::qualify_weight_profile(&source, [0, 0], 100)
            .unwrap()
            .0
            .unwrap();
        let bridge = ProfileSectionBridge::new(
            profile.section(0, 0.5).unwrap(),
            profile.section(0, 0.75).unwrap(),
            [0.25; 2],
        )
        .unwrap();
        let constrained = bridge.constrain_original_rails(100).unwrap().0.unwrap();
        let value = constrained
            .first_free_from_rail_bounds([1.; 2], [0., 1.], 100)
            .unwrap()
            .0
            .unwrap();
        assert!(value[0][0] > 0. && value[0][0] <= 2. * ulp && value[0][1] >= 2. * ulp);
        assert!(value[1][0] <= 0. && value[1][1] >= 0.);
        assert!(value[2][0] <= 1. && value[2][1] >= 1.);
    }
    #[test]
    fn first_free_rail_bound_refuses_a_derivative_jump_at_path_endpoint() {
        let source = crate::surface::Surface {
            degree_u: 1,
            degree_v: 2,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 0., 0.5, 0.5, 1., 1., 1.],
            control_points: (0..2)
                .map(|u| {
                    (0..5)
                        .map(|v| vec![u as f64, v as f64 / 4., (v * v) as f64])
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 5]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let profile = crate::normal_alignment::qualify_weight_profile(&source, [0, 0], 100)
            .unwrap()
            .0
            .unwrap();
        let bridge = ProfileSectionBridge::new(
            profile.section(0, 0.5).unwrap(),
            profile.section(0, 0.75).unwrap(),
            [0.25; 2],
        )
        .unwrap();
        assert!(bridge.constrain_original_rails(100).unwrap().0.is_none());
        // Exercise the local defensive guard even with the private factory
        // bypassed in this test; production cannot construct this raw value.
        let raw = RailConstrainedBridge { bridge: &bridge };
        for free in [[0.25, 0.5], [0.5, 0.75], [0.49, 0.51]] {
            let refused = raw
                .first_free_from_rail_bounds(free, [0.5; 2], 100)
                .unwrap();
            assert!(refused.0.is_none());
            assert_eq!(refused.1, 0);
        }
    }
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
                        let first = corrected
                            .first_free_from_rail_bounds([free; 2], [across; 2], 100)
                            .unwrap()
                            .0
                            .unwrap();
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
                        let expected_first = [0., 1., expected[2] * free];
                        for k in 0..3 {
                            assert!(
                                first[k][0] <= expected_first[k]
                                    && expected_first[k] <= first[k][1]
                            );
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
                assert!(
                    corrected
                        .second_free_derivatives_with_transverse_correction(
                            [0.5; 2],
                            [0.5; 2],
                            spent - 1
                        )
                        .unwrap()
                        .0
                        .is_none()
                );
                assert!(
                    corrected
                        .second_free_derivatives_with_transverse_correction([0.5; 2], [0.5; 2], 0)
                        .unwrap()
                        .0
                        .is_none()
                );
            }
        }
    }
}
