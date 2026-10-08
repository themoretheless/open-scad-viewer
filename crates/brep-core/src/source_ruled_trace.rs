//! Exact projection of a ruled source support onto one unchanged NURBS curve.
//! This reduction alone does not prove contact ownership or exclusion.
use nurbs_core::{Error, Result, curve::Curve, surface::Surface};
pub(crate) struct Certificate<'a> {
    source: &'a Surface,
    axes: [usize; 2],
    linear: usize,
    curve: Curve,
}
impl Certificate<'_> {
    pub(crate) fn source(&self) -> &Surface {
        self.source
    }
    pub(crate) fn axes(&self) -> [usize; 2] {
        self.axes
    }
    pub(crate) fn linear_axis(&self) -> usize {
        self.linear
    }
    pub(crate) fn curve(&self) -> &Curve {
        &self.curve
    }
}
pub(crate) fn certify(
    source: &Surface,
    axes: [usize; 2],
    linear: usize,
    max_controls: usize,
) -> Result<Option<Certificate<'_>>> {
    if linear > 1 || axes.iter().any(|&a| a > 2) || axes[0] == axes[1] || max_controls > 1000000 {
        return Err(Error::new(
            "BREP_RULED_TRACE",
            "Choose distinct projection axes and bounded source work",
        ));
    }
    let counts = [
        source.control_points.len(),
        source.control_points.first().map_or(0, Vec::len),
    ];
    if counts.iter().any(|&n| n == 0)
        || counts[0]
            .checked_mul(counts[1])
            .is_none_or(|n| n > max_controls)
    {
        return Ok(None);
    }
    let degrees = [source.degree_u, source.degree_v];
    let knots = [&source.knots_u, &source.knots_v];
    if source
        .control_points
        .iter()
        .any(|row| row.len() != counts[1] || row.iter().any(|p| p.len() != 3))
        || source.weights.len() != counts[0]
        || source.weights.iter().any(|row| row.len() != counts[1])
        || (0..2).any(|axis| {
            degrees[axis] >= counts[axis] || knots[axis].len() != counts[axis] + degrees[axis] + 1
        })
    {
        return Ok(None);
    }
    source.validate()?;
    let periodic = [source.periodic_u, source.periodic_v];
    let k = knots[linear];
    if degrees[linear] != 1
        || counts[linear] != 2
        || periodic[linear]
        || k.len() != 4
        || k[0] != k[1]
        || k[2] != k[3]
        || k[1] >= k[2]
    {
        return Ok(None);
    }
    let mut controls = Vec::new();
    let mut weights = Vec::new();
    for i in 0..counts[1 - linear] {
        let (a, b) = if linear == 1 {
            ([i, 0], [i, 1])
        } else {
            ([0, i], [1, i])
        };
        let p = &source.control_points[a[0]][a[1]];
        let q = &source.control_points[b[0]][b[1]];
        let w = source.weights[a[0]][a[1]];
        if w <= 0. || w != source.weights[b[0]][b[1]] || axes.iter().any(|&axis| p[axis] != q[axis])
        {
            return Ok(None);
        }
        controls.push(axes.map(|axis| p[axis]).to_vec());
        weights.push(w);
    }
    // Tensor-product partition of unity removes only the linear basis. The
    // remaining basis, coordinates, weights, and domain are copied exactly.
    let curve = Curve {
        degree: degrees[1 - linear],
        knots: knots[1 - linear].clone(),
        control_points: controls,
        weights,
        periodic: periodic[1 - linear],
    };
    curve.validate()?;
    Ok(Some(Certificate {
        source,
        axes,
        linear,
        curve,
    }))
}
/// A positive-weight rational Bezier curve with strictly ordered scalar
/// controls is strictly monotone on its complete original parameter domain.
/// This certifies a simple trace, not ownership of any transition contact.
pub(crate) struct MonotoneCoordinate<'t, 's> {
    trace: &'t Certificate<'s>,
    coordinate: usize,
    increasing: bool,
}
impl<'t, 's> MonotoneCoordinate<'t, 's> {
    pub(crate) fn trace(&self) -> &'t Certificate<'s> {
        self.trace
    }
    pub(crate) fn coordinate(&self) -> usize {
        self.coordinate
    }
    pub(crate) fn increasing(&self) -> bool {
        self.increasing
    }
    /// Exact coordinate image of the complete original trace. Clamping
    /// preserves endpoint coordinates even with unequal positive weights.
    pub(crate) fn coordinate_range(&self) -> [f64; 2] {
        let c = self.trace.curve();
        let a = c.control_points[0][self.coordinate];
        let b = c.control_points[c.control_points.len() - 1][self.coordinate];
        [a.min(b), a.max(b)]
    }
    pub(crate) fn parameter_domain(&self) -> [f64; 2] {
        self.trace.curve().domain()
    }
}
pub(crate) fn certify_monotone_coordinate<'t, 's>(
    trace: &'t Certificate<'s>,
    coordinate: usize,
    max_edges: usize,
) -> Result<Option<MonotoneCoordinate<'t, 's>>> {
    if coordinate > 1 || max_edges > 1000000 {
        return Err(Error::new(
            "BREP_RULED_TRACE",
            "Choose projected coordinate and bounded monotonicity work",
        ));
    }
    let c = trace.curve();
    let n = c.control_points.len();
    if c.periodic
        || c.degree == 0
        || n != c.degree + 1
        || n - 1 > max_edges
        || c.knots.len() != 2 * n
        || !c.knots[..n].iter().all(|k| *k == c.knots[0])
        || !c.knots[n..].iter().all(|k| *k == c.knots[n])
        || c.knots[0] >= c.knots[n]
        || c.weights.iter().any(|w| *w <= 0.)
    {
        return Ok(None);
    }
    let increasing = c.control_points[1][coordinate] > c.control_points[0][coordinate];
    if c.control_points.windows(2).any(|pair| {
        let a = pair[0][coordinate];
        let b = pair[1][coordinate];
        if increasing { b <= a } else { b >= a }
    }) {
        return Ok(None);
    }
    // The rational derivative numerator is a positive sum over ordered
    // control pairs: wi*wj*(xj-xi)*(j-i)*Bi*Bj/[t*(1-t)].
    // Endpoint derivatives inherit the strict adjacent-control order.
    Ok(Some(MonotoneCoordinate {
        trace,
        coordinate,
        increasing,
    }))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn monotone_trace_covers_nonunit_domain_and_refuses_folds_and_work_limits() {
        let s = source(1);
        let trace = certify(&s, [0, 1], 1, 6).unwrap().unwrap();
        let proof = certify_monotone_coordinate(&trace, 1, 2).unwrap().unwrap();
        assert!(std::ptr::eq(proof.trace(), &trace));
        assert_eq!(proof.coordinate(), 1);
        assert!(proof.increasing());
        assert_eq!(proof.coordinate_range(), [0., 4.]);
        assert_eq!(proof.parameter_domain(), [2., 5.]);
        assert_eq!(proof.trace().curve().knots, vec![2., 2., 2., 5., 5., 5.]);
        assert!(certify_monotone_coordinate(&trace, 1, 1).unwrap().is_none());
        assert!(certify_monotone_coordinate(&trace, 2, 2).is_err());
        let mut reversed = s.clone();
        reversed.control_points.reverse();
        reversed.weights.reverse();
        let reverse = certify(&reversed, [0, 1], 1, 6).unwrap().unwrap();
        assert!(
            !certify_monotone_coordinate(&reverse, 1, 2)
                .unwrap()
                .unwrap()
                .increasing()
        );
        let mut folded = s.clone();
        for p in &mut folded.control_points[1] {
            p[1] = 10.;
        }
        let fold = certify(&folded, [0, 1], 1, 6).unwrap().unwrap();
        assert!(certify_monotone_coordinate(&fold, 1, 2).unwrap().is_none());
    }
    fn source(linear: usize) -> Surface {
        let mut s = Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![2., 2., 2., 5., 5., 5.],
            knots_v: vec![7., 7., 9., 9.],
            control_points: (0..3)
                .map(|i| {
                    (0..2)
                        .map(|j| vec![i as f64, (i * i) as f64, (i + j) as f64])
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 2], vec![0.75; 2], vec![1.25; 2]],
            periodic_u: false,
            periodic_v: false,
        };
        if linear == 0 {
            s.control_points = (0..2)
                .map(|j| (0..3).map(|i| s.control_points[i][j].clone()).collect())
                .collect();
            s.weights = (0..2)
                .map(|j| (0..3).map(|i| s.weights[i][j]).collect())
                .collect();
            std::mem::swap(&mut s.degree_u, &mut s.degree_v);
            std::mem::swap(&mut s.knots_u, &mut s.knots_v);
        }
        s
    }
    #[test]
    fn ruled_trace_preserves_original_basis_and_borrowed_owner() {
        for linear in [0, 1] {
            let s = source(linear);
            let proof = certify(&s, [0, 1], linear, 6).unwrap().unwrap();
            assert!(std::ptr::eq(proof.source(), &s));
            assert_eq!(proof.axes(), [0, 1]);
            assert_eq!(proof.linear_axis(), linear);
            assert_eq!(proof.curve().degree, 2);
            assert_eq!(proof.curve().knots, vec![2., 2., 2., 5., 5., 5.]);
            assert_eq!(proof.curve().weights, vec![1., 0.75, 1.25]);
            assert_eq!(
                proof.curve().control_points,
                vec![vec![0., 0.], vec![1., 1.], vec![2., 4.]]
            );
            for parameter in [2., 2.75, 3.5, 4.25, 5.] {
                let projected = proof.curve().evaluate(parameter).unwrap().point;
                for along in [7., 7.5, 8., 8.5, 9.] {
                    let uv = if linear == 1 {
                        [parameter, along]
                    } else {
                        [along, parameter]
                    };
                    let original = s.evaluate(uv[0], uv[1]).unwrap().point;
                    for axis in 0..2 {
                        assert!((original[axis] - projected[axis]).abs() < 1e-12);
                    }
                }
            }
            let reversed = certify(&s, [1, 0], linear, 6).unwrap().unwrap();
            assert_eq!(
                reversed.curve().control_points,
                proof
                    .curve()
                    .control_points
                    .iter()
                    .map(|p| vec![p[1], p[0]])
                    .collect::<Vec<_>>()
            );
            assert!(certify(&s, [0, 1], linear, 5).unwrap().is_none());
            assert!(certify(&s, [0, 1], linear, 0).unwrap().is_none());
            assert!(certify(&s, [0, 0], linear, 6).is_err());
        }
    }
    #[test]
    fn ruled_trace_refuses_projection_and_weight_changes() {
        let s = source(1);
        let mut changed = s.clone();
        changed.control_points[1][1][0] = f64::from_bits(1f64.to_bits() + 1);
        assert!(certify(&changed, [0, 1], 1, 6).unwrap().is_none());
        changed = s.clone();
        changed.weights[1][1] = f64::from_bits(0.75f64.to_bits() + 1);
        assert!(certify(&changed, [0, 1], 1, 6).unwrap().is_none());
        changed = s.clone();
        changed.control_points[1][1][2] += 100.;
        assert!(certify(&changed, [0, 1], 1, 6).unwrap().is_some());
        changed = s;
        changed.weights[1].fill(-0.75);
        assert!(certify(&changed, [0, 1], 1, 6).map_or(true, |proof| proof.is_none()));
    }
}
