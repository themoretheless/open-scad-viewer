//! Positional error bound for an exact separable original rational weight profile.
use crate::{Result, check, interval_eval::Interval as I, surface::Surface};
/// A proposal certificate, not a replacement Surface or a B-rep admission.
/// The target weights are the exact expressions row[i]*column[j]/pivot;
/// flattening those expressions to binary64 is not covered by this certificate.
pub struct WeightProfile<'a> {
    source: &'a Surface,
    row: Vec<f64>,
    column: Vec<f64>,
    pivot: f64,
    axis_deviation_upper: [f64; 3],
    weight_deviation_upper: f64,
}
/// Direction certificate for the exact proposed profile, not its original Surface.
pub struct ProfileDirection<'a, 's> {
    profile: &'a WeightProfile<'s>,
    endpoints: [[f64; 2]; 2],
    axis: usize,
    boundary: f64,
    factor: Option<(bool, usize)>,
    components: [[f64; 2]; 3],
}
impl ProfileDirection<'_, '_> {
    pub fn profile(&self) -> &WeightProfile<'_> {
        self.profile
    }
    pub fn endpoints(&self) -> [[f64; 2]; 2] {
        self.endpoints
    }
    pub fn axis(&self) -> usize {
        self.axis
    }
    pub fn boundary(&self) -> f64 {
        self.boundary
    }
    pub fn factor(&self) -> Option<(bool, usize)> {
        self.factor
    }
    pub fn components(&self) -> [[f64; 2]; 3] {
        self.components
    }
}
/// Pointwise equality of one original natural boundary and the proposal.
pub struct ProfileBoundary<'a, 's> {
    profile: &'a WeightProfile<'s>,
    axis: usize,
    boundary: f64,
}
impl ProfileBoundary<'_, '_> {
    pub fn profile(&self) -> &WeightProfile<'_> {
        self.profile
    }
    pub fn axis(&self) -> usize {
        self.axis
    }
    pub fn boundary(&self) -> f64 {
        self.boundary
    }
}
fn choose(n: usize, k: usize) -> f64 {
    let mut x = 1u64;
    for i in 0..k.min(n - k) {
        x = x * (n - i) as u64 / (i + 1) as u64;
    }
    x as f64
}
impl<'source> WeightProfile<'source> {
    pub fn source(&self) -> &Surface {
        self.source
    }
    pub fn factors(&self) -> (&[f64], &[f64], f64) {
        (&self.row, &self.column, self.pivot)
    }
    pub fn axis_deviation_upper(&self) -> [f64; 3] {
        self.axis_deviation_upper
    }
    pub fn weight_deviation_upper(&self) -> f64 {
        self.weight_deviation_upper
    }
    /// Certified enclosure of the exact proposed profile over a complete
    /// original parameter rectangle. No target weight is rounded to f64.
    pub fn enclose(
        &self,
        domain: [[f64; 2]; 2],
        max_spans: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        crate::normal_alignment::validate_rectangle(self.source, domain, max_spans)?;
        let s = self.source;
        let degrees = [s.degree_u, s.degree_v];
        let counts = [s.control_points.len(), s.control_points[0].len()];
        let knots = [&s.knots_u, &s.knots_v];
        let mut hull = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
        let mut spent = 0;
        for u in degrees[0]..counts[0] {
            for v in degrees[1]..counts[1] {
                let span = [
                    [knots[0][u], knots[0][u + 1]],
                    [knots[1][v], knots[1][v + 1]],
                ];
                if span.iter().any(|a| a[0] == a[1]) {
                    continue;
                }
                let section = std::array::from_fn(|a| {
                    [span[a][0].max(domain[a][0]), span[a][1].min(domain[a][1])]
                });
                if section.iter().any(|a| a[0] > a[1]) {
                    continue;
                }
                if spent == max_spans {
                    return Ok((None, spent));
                }
                let net = self.homogeneous_on([u, v], span)?;
                let values = crate::normal_alignment::restrict_hull(&net, degrees, span, section)?;
                for k in 0..3 {
                    let r = values[k].div(values[3])?;
                    hull[k][0] = hull[k][0].min(r.lo);
                    hull[k][1] = hull[k][1].max(r.hi);
                }
                spent += 1;
            }
        }
        if spent == 0 {
            return Ok((None, 0));
        }
        Ok((Some(hull), spent))
    }
    pub(crate) fn homogeneous_on(
        &self,
        span: [usize; 2],
        domain: [[f64; 2]; 2],
    ) -> Result<Vec<Vec<[I; 4]>>> {
        crate::curve_surface_composition::surface_net_on_with_weights(
            self.source,
            span,
            domain,
            |i, j| {
                I::point(self.row[i])
                    .mul(I::point(self.column[j]))?
                    .div(I::point(self.pivot))
            },
        )
    }
    /// Original control positions with exact separable weights. Constant endpoint
    /// rows may be different points: separability cancels each row's transverse
    /// chord exactly. Removed normalized free factors are nonnegative.
    pub fn boundary_direction<'profile>(
        &'profile self,
        endpoints: [[f64; 2]; 2],
        axis: usize,
        boundary: f64,
        max_spans: usize,
    ) -> Result<(Option<ProfileDirection<'profile, 'source>>, usize)> {
        check(axis < 2, "Choose a proposed profile secant coordinate")?;
        crate::normal_alignment::validate_rectangle(self.source, endpoints, max_spans)?;
        let s = self.source;
        let degrees = [s.degree_u, s.degree_v];
        let counts = [s.control_points.len(), s.control_points[0].len()];
        let knots = [&s.knots_u, &s.knots_v];
        let free = 1 - axis;
        let p = degrees[free];
        let q = degrees[axis];
        let span = [
            [knots[free][p], knots[free][counts[free]]],
            [knots[axis][q], knots[axis][counts[axis]]],
        ];
        check(
            boundary == span[1][0] || boundary == span[1][1],
            "Use a natural proposed profile boundary",
        )?;
        if p == 0
            || p > 8
            || q == 0
            || q > 8
            || counts[free] != p + 1
            || counts[axis] != q + 1
            || !(0..2).all(|a| {
                knots[a][..=degrees[a]]
                    .iter()
                    .all(|x| *x == knots[a][degrees[a]])
                    && knots[a][degrees[a] + 1..]
                        .iter()
                        .all(|x| *x == knots[a][counts[a]])
            })
        {
            return Ok((None, 0));
        }
        let cp = |i: usize, j: usize| {
            if axis == 1 {
                &s.control_points[i][j]
            } else {
                &s.control_points[j][i]
            }
        };
        let mut factor = None;
        for high in [false, true] {
            let mut power = 0;
            for step in 0..=p {
                let i = if high { p - step } else { step };
                if !(0..=q).all(|j| cp(i, j) == cp(i, 0)) {
                    break;
                }
                power += 1;
            }
            if power > p {
                return Ok((None, 0));
            }
            if power > 0 {
                factor = Some((high, power));
                break;
            }
        }
        let high_boundary = boundary == span[1][1];
        let b = if high_boundary { q } else { 0 };
        let mut net = vec![vec![[I::point(0.); 4]; q]; p + 1];
        for i in 0..=p {
            for j in 0..=q {
                if j == b {
                    continue;
                }
                let (u, v) = if axis == 1 { (i, j) } else { (j, i) };
                let weight = I::point(self.row[u])
                    .mul(I::point(self.column[v]))?
                    .div(I::point(self.pivot))?;
                let division =
                    I::point(q as f64)
                        .div(I::point(if high_boundary { q - j } else { j } as f64))?;
                let column = if high_boundary { j } else { j - 1 };
                for k in 0..3 {
                    net[i][column][k] = if cp(i, j)[k] == cp(i, b)[k] {
                        I::point(0.)
                    } else {
                        I::point(cp(i, j)[k])
                            .sub(I::point(cp(i, b)[k]))?
                            .mul(weight)?
                            .mul(division)?
                    };
                }
            }
        }
        let mut degree = p;
        if let Some((high, power)) = factor {
            let mut reduced = Vec::new();
            for row in 0..=p - power {
                let original = if high { row } else { row + power };
                let scale = I::point(choose(p, original)).div(I::point(choose(p - power, row)))?;
                let mut values = net[original].clone();
                for h in &mut values {
                    for k in 0..3 {
                        h[k] = h[k].mul(scale)?;
                    }
                }
                reduced.push(values);
            }
            net = reduced;
            degree -= power;
        }
        let domain = [endpoints[free], endpoints[axis]];
        let numerator =
            crate::normal_alignment::restrict_hull(&net, [degree, q - 1], span, domain)?;
        let original_span = [
            [s.knots_u[s.degree_u], s.knots_u[counts[0]]],
            [s.knots_v[s.degree_v], s.knots_v[counts[1]]],
        ];
        let homogeneous = self.homogeneous_on(degrees, original_span)?;
        let values = crate::normal_alignment::restrict_hull(
            &homogeneous,
            degrees,
            original_span,
            endpoints,
        )?;
        let scale = I::point(if high_boundary { -1. } else { 1. })
            .div(I::point(span[1][1]).sub(I::point(span[1][0]))?)?;
        let mut components = [[0.; 2]; 3];
        for k in 0..3 {
            let r = numerator[k].mul(scale)?.div(values[3])?;
            components[k] = [r.lo, r.hi];
        }
        Ok((
            Some(ProfileDirection {
                profile: self,
                endpoints,
                axis,
                boundary,
                factor,
                components,
            }),
            1,
        ))
    }
    /// Prove a complete natural boundary unchanged using original coefficient
    /// identity or its constant position. No nearby weight or sampled point is
    /// an identity. Unclamped boundaries and incomplete work refuse.
    pub fn preserved_boundary<'profile>(
        &'profile self,
        axis: usize,
        high: bool,
        max_controls: usize,
    ) -> Result<(Option<ProfileBoundary<'profile, 'source>>, usize)> {
        check(
            axis < 2 && max_controls <= 1_000_000,
            "Choose an original bounded boundary proof",
        )?;
        let s = self.source;
        let knots = [&s.knots_u, &s.knots_v];
        let degrees = [s.degree_u, s.degree_v];
        let counts = [s.control_points.len(), s.control_points[0].len()];
        let boundary = if high {
            knots[axis][counts[axis]]
        } else {
            knots[axis][degrees[axis]]
        };
        let clamp = if high {
            &knots[axis][counts[axis]..]
        } else {
            &knots[axis][..=degrees[axis]]
        };
        if !clamp.iter().all(|x| *x == boundary) {
            return Ok((None, 0));
        }
        let fixed = if high { counts[axis] - 1 } else { 0 };
        let position = if axis == 0 {
            &s.control_points[fixed][0]
        } else {
            &s.control_points[0][fixed]
        };
        let mut constant = true;
        let mut weights_equal = true;
        let mut spent = 0;
        for free in 0..counts[1 - axis] {
            if spent == max_controls {
                return Ok((None, spent));
            }
            let (i, j) = if axis == 0 {
                (fixed, free)
            } else {
                (free, fixed)
            };
            constant &= s.control_points[i][j] == *position;
            weights_equal &= if axis == 0 {
                self.row[fixed] == self.pivot && s.weights[i][j] == self.column[j]
            } else {
                self.column[fixed] == self.pivot && s.weights[i][j] == self.row[i]
            };
            spent += 1;
        }
        if !constant && !weights_equal {
            return Ok((None, spent));
        }
        Ok((
            Some(ProfileBoundary {
                profile: self,
                axis,
                boundary,
            }),
            spent,
        ))
    }
    pub fn within_position_tolerance(&self, tolerance: f64) -> Result<bool> {
        check(
            tolerance.is_finite() && tolerance >= 0.,
            "Use a finite nonnegative positional tolerance",
        )?;
        let mut square = I::point(0.);
        for x in self.axis_deviation_upper {
            square = square.add(I::point(x).mul(I::point(x))?)?;
        }
        let required = I::point(tolerance).mul(I::point(tolerance))?;
        Ok(square.hi <= required.lo)
    }
}
/// Cover every original control. Whole-domain positional deviation follows from
/// positive rational basis weights, without sampling or derivative assumptions.
pub fn qualify_weight_profile<'a>(
    s: &'a Surface,
    anchor: [usize; 2],
    max_controls: usize,
) -> Result<(Option<WeightProfile<'a>>, usize)> {
    s.validate()?;
    check(
        max_controls <= 1_000_000 && anchor[0] < s.weights.len() && anchor[1] < s.weights[0].len(),
        "Choose an original profile anchor and bounded control work",
    )?;
    if s.periodic_u || s.periodic_v {
        return Ok((None, 0));
    }
    let row: Vec<_> = s.weights.iter().map(|r| r[anchor[1]]).collect();
    let column = s.weights[anchor[0]].clone();
    let pivot = s.weights[anchor[0]][anchor[1]];
    let mut delta = 0_f64;
    let mut lower = f64::INFINITY;
    let mut ranges = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
    let mut spent = 0;
    for i in 0..row.len() {
        for j in 0..column.len() {
            if spent == max_controls {
                return Ok((None, spent));
            }
            let target = I::point(row[i])
                .mul(I::point(column[j]))?
                .div(I::point(pivot))?;
            let difference = target.sub(I::point(s.weights[i][j]))?;
            delta = delta.max(difference.lo.abs()).max(difference.hi.abs());
            lower = lower.min(target.lo);
            for k in 0..3 {
                let coordinate = s.control_points[i][j][k];
                ranges[k][0] = ranges[k][0].min(coordinate);
                ranges[k][1] = ranges[k][1].max(coordinate);
            }
            spent += 1;
        }
    }
    if lower <= 0. {
        return Ok((None, spent));
    }
    // T-S = sum B_ij (targetWeight-originalWeight)(P_ij-S)/targetW.
    // Positive original weights place S in the original control convex hull;
    // sum B_ij=1 and targetW>=min(targetWeight). No chart samples enter proof.
    let mut axis_deviation_upper = [0.; 3];
    for k in 0..3 {
        let extent = I::point(ranges[k][1]).sub(I::point(ranges[k][0]))?;
        axis_deviation_upper[k] = extent.mul(I::point(delta))?.div(I::point(lower))?.hi;
    }
    Ok((
        Some(WeightProfile {
            source: s,
            row,
            column,
            pivot,
            axis_deviation_upper,
            weight_deviation_upper: delta,
        }),
        spent,
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn surface() -> Surface {
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn tiny_weight_change_is_bounded_without_mutating_source() {
        let mut s = surface();
        s.weights[1][1] = 1_f64.next_up();
        let original = s.weights.clone();
        let (proof, spent) = qualify_weight_profile(&s, [0, 0], 4).unwrap();
        let proof = proof.unwrap();
        assert_eq!(spent, 4);
        assert!(std::ptr::eq(proof.source(), &s));
        assert!(proof.within_position_tolerance(1e-12).unwrap());
        assert!(!proof.within_position_tolerance(0.).unwrap());
        assert!(proof.within_position_tolerance(f64::NAN).is_err());
        let (row, column, pivot) = proof.factors();
        assert_eq!(row, &[1., 1.]);
        assert_eq!(column, &[1., 1.]);
        assert_eq!(pivot, 1.);
        let bound = proof.axis_deviation_upper();
        for u in [0.1, 0.4, 0.9] {
            for v in [0.1, 0.4, 0.9] {
                let p = s.evaluate(u, v).unwrap().point;
                assert!((p[0] - u).abs() <= bound[0] && (p[1] - v).abs() <= bound[1]);
            }
        }
        assert_eq!(s.weights, original);
    }
    #[test]
    fn missing_controls_and_large_changes_do_not_pass_tolerance() {
        let mut s = surface();
        s.weights[1][1] = 2.;
        let (proof, spent) = qualify_weight_profile(&s, [0, 0], 3).unwrap();
        assert!(proof.is_none());
        assert_eq!(spent, 3);
        let proof = qualify_weight_profile(&s, [0, 0], 4).unwrap().0.unwrap();
        assert!(!proof.within_position_tolerance(1e-7).unwrap());
        assert!(qualify_weight_profile(&s, [2, 0], 4).is_err());
        assert!(qualify_weight_profile(&s, [0, 0], 0).unwrap().0.is_none());
    }
    #[test]
    fn exact_profile_geometry_matches_independent_separable_formula() {
        let mut s = surface();
        s.weights = vec![vec![1., 3.], vec![2., 6.]];
        let proof = qualify_weight_profile(&s, [0, 0], 4).unwrap().0.unwrap();
        for u in [0.2, 0.37, 0.4] {
            for v in [0.3, 0.41, 0.5] {
                let (box_, spent) = proof.enclose([[u, u], [v, v]], 1).unwrap();
                let box_ = box_.unwrap();
                assert_eq!(spent, 1);
                let expected = [2. * u / (1. + u), 3. * v / (1. + 2. * v), 0.];
                for (r, x) in box_.into_iter().zip(expected) {
                    assert!(r[0] <= x && x <= r[1], "{r:?} misses {x}");
                }
            }
        }
        assert!(proof.enclose([[-0.1, 0.2], [0., 1.]], 1).is_err());
        assert!(proof.enclose([[0., 1.]; 2], 0).is_err());
    }
    #[test]
    fn profile_geometry_never_returns_partial_span_union() {
        let mut s = surface();
        s.knots_u = vec![0., 0., 0.5, 1., 1.];
        s.control_points
            .insert(1, vec![vec![0.5, 0., 0.], vec![0.5, 1., 0.]]);
        s.weights = vec![vec![1.; 2]; 3];
        let proof = qualify_weight_profile(&s, [0, 0], 6).unwrap().0.unwrap();
        let (missing, spent) = proof.enclose([[0.2, 0.8], [0.3, 0.7]], 1).unwrap();
        assert!(missing.is_none());
        assert_eq!(spent, 1);
        let (bounds, spent) = proof.enclose([[0.2, 0.8], [0.3, 0.7]], 2).unwrap();
        let bounds = bounds.unwrap();
        assert_eq!(spent, 2);
        assert!(
            bounds[0][0] <= 0.2
                && bounds[0][1] >= 0.8
                && bounds[1][0] <= 0.3
                && bounds[1][1] >= 0.7
        );
    }
    #[test]
    fn exact_profile_separability_removes_constant_rows_at_different_points() {
        let mut s = surface();
        s.degree_u = 2;
        s.knots_u = vec![0., 0., 0., 1., 1., 1.];
        s.control_points = vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![0.5, 0., 0.]; 2],
            vec![vec![1., 0., 0.]; 2],
        ];
        s.weights = vec![vec![1., 0.7], vec![1., 0.7_f64.next_up()], vec![1., 0.7]];
        let profile = qualify_weight_profile(&s, [0, 0], 6).unwrap().0.unwrap();
        let proof = profile
            .boundary_direction([[0.99, 1.], [0.2, 0.3]], 1, 1., 1)
            .unwrap()
            .0
            .unwrap();
        assert!(std::ptr::eq(proof.profile(), &profile));
        assert_eq!(proof.factor(), Some((true, 2)));
        let r = proof.components()[1];
        assert!(r[0] > 0.);
        // Profile y=(1-u)^2 * 0.7v/(1-0.3v); its boundary secant
        // divided by (1-u)^2 equals 1/(1-0.3v), including u=1.
        for v in [0.2, 0.25, 0.3] {
            let expected = 1. / (1. - 0.3 * v);
            assert!(r[0] <= expected && expected <= r[1]);
        }
        assert!(
            profile
                .boundary_direction([[0.99, 1.], [0.2, 0.3]], 1, 1., 0)
                .is_err()
        );
    }
    #[test]
    fn preserved_boundaries_are_exact_and_budgeted() {
        let mut s = surface();
        s.weights[1][1] = 1_f64.next_up();
        let proof = qualify_weight_profile(&s, [0, 0], 4).unwrap().0.unwrap();
        for axis in [0, 1] {
            let (boundary, spent) = proof.preserved_boundary(axis, false, 2).unwrap();
            let boundary = boundary.unwrap();
            assert_eq!(spent, 2);
            assert!(std::ptr::eq(boundary.profile(), &proof));
            assert_eq!(boundary.axis(), axis);
            assert_eq!(boundary.boundary(), 0.);
            assert!(proof.preserved_boundary(axis, true, 2).unwrap().0.is_none());
            assert!(
                proof
                    .preserved_boundary(axis, false, 1)
                    .unwrap()
                    .0
                    .is_none()
            );
        }
        assert!(proof.preserved_boundary(2, false, 2).is_err());
        s.control_points[1] = vec![vec![1., 0., 0.]; 2];
        let proof = qualify_weight_profile(&s, [0, 0], 4).unwrap().0.unwrap();
        assert!(proof.preserved_boundary(0, true, 2).unwrap().0.is_some());
    }
}
