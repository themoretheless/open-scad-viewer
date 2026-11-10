//! Jordan certificate for the exact factorized profile; original weights are not used.
//! Global interior injectivity from exact orientation and a simple boundary.
//! Degree is ±1 inside a Jordan boundary and zero outside. Every interior
//! preimage has the same nonzero local degree, so there is exactly one inside
//! and none outside. Openness excludes interior images on the boundary.
use crate::{
    Result, check, curve::Curve, curve_quadratic_separator as separator,
    profile_projection_jacobian as jacobian, surface_weight_profile::WeightProfile,
    trim_simplicity,
};
pub struct Certificate<'p, 's> {
    orientation: jacobian::Certificate<'p, 's>,
    boundary: Vec<Curve>,
    boundary_indices: Vec<usize>,
    collapsed: Vec<usize>,
    separators: Vec<separator::Certificate>,
}
impl<'p, 's> Certificate<'p, 's> {
    pub fn profile(&self) -> &'p WeightProfile<'s> {
        self.orientation.profile()
    }
    pub fn axes(&self) -> [usize; 2] {
        self.orientation.axes()
    }
    pub fn boundary(&self) -> &[Curve] {
        &self.boundary
    }
    /// Natural rectangle edges: v-low, u-high, v-high, u-low.
    pub fn boundary_indices(&self) -> &[usize] {
        &self.boundary_indices
    }
    pub fn collapsed_boundaries(&self) -> &[usize] {
        &self.collapsed
    }
    pub fn separators(&self) -> &[separator::Certificate] {
        &self.separators
    }
}
pub struct Report<'p, 's> {
    pub certificate: Option<Certificate<'p, 's>>,
    pub exact_work: u64,
    pub boundary_cells: usize,
    pub reason: &'static str,
}
pub fn certify<'p, 's>(
    profile: &'p WeightProfile<'s>,
    axes: [usize; 2],
    tolerance: f64,
    max_work: u64,
    max_cells: usize,
) -> Result<Report<'p, 's>> {
    check(
        tolerance.is_finite() && tolerance > 0. && (1..=100000).contains(&max_cells),
        "Choose bounded projected boundary work",
    )?;
    let orientation = jacobian::certify(profile, axes, max_work)?;
    let s = profile.source();
    let (row_weights, column_weights, _) = profile.factors();
    let mut out = Report {
        certificate: None,
        exact_work: orientation.exact_work,
        boundary_cells: 0,
        reason: "projected-orientation-unproven",
    };
    let Some(orientation) = orientation.certificate else {
        return Ok(out);
    };
    let (p, q) = (s.degree_u, s.degree_v);
    let natural = [
        ((0..=p).map(|i| (i, 0)).collect::<Vec<_>>(), p),
        ((0..=q).map(|j| (p, j)).collect(), q),
        ((0..=p).rev().map(|i| (i, q)).collect(), p),
        ((0..=q).rev().map(|j| (0, j)).collect(), q),
    ];
    let mut boundary = Vec::new();
    let mut boundary_indices = Vec::new();
    let mut collapsed = Vec::new();
    for (index, (indices, degree)) in natural.into_iter().enumerate() {
        let c = Curve {
            degree,
            knots: std::iter::repeat_n(0., degree + 1)
                .chain(std::iter::repeat_n(1., degree + 1))
                .collect(),
            control_points: indices
                .iter()
                .map(|&(i, j)| {
                    vec![
                        s.control_points[i][j][axes[0]],
                        s.control_points[i][j][axes[1]],
                    ]
                })
                .collect(),
            // Fixed-axis positive factors cancel exactly from the rational curve.
            weights: indices
                .iter()
                .map(|&(i, j)| {
                    if index % 2 == 0 {
                        row_weights[i]
                    } else {
                        column_weights[j]
                    }
                })
                .collect(),
            periodic: false,
        };
        if c.control_points.iter().all(|p| p == &c.control_points[0]) {
            collapsed.push(index);
        } else {
            boundary_indices.push(index);
            boundary.push(c);
        }
    }
    out.reason = "projected-boundary-simplicity-unproven";
    if boundary.len() < 3 {
        return Ok(out);
    }
    let audit = trim_simplicity::inspect(&boundary, tolerance, 6, max_cells)?;
    out.boundary_cells = audit.cells;
    if audit.exact_joins != Some(true)
        || audit.injective.iter().any(|b| !*b)
        || audit.pairs.len() != audit.total_pairs
    {
        return Ok(out);
    }
    let mut separators = Vec::new();
    for pair in audit.pairs {
        if pair.proven {
            continue;
        }
        let [a, b] = pair.curves;
        let ends = if b == a + 1 {
            [1, 0]
        } else if a == 0 && b == boundary.len() - 1 {
            [0, 1]
        } else {
            return Ok(out);
        };
        let curves = [&boundary[a], &boundary[b]];
        let frame_index = if ends[0] == 0 {
            1
        } else {
            curves[0].degree - 1
        };
        let join = &curves[0].control_points[ends[0] * curves[0].degree];
        let t = &curves[0].control_points[frame_index];
        let direction = [t[0] - join[0], t[1] - join[1]];
        let ratio = |c: &Curve, u: f64| -> Result<f64> {
            let point = c.evaluate(u)?.point;
            let delta = [point[0] - join[0], point[1] - join[1]];
            let dot = direction[0] * delta[0] + direction[1] * delta[1];
            Ok((direction[0] * delta[1] - direction[1] * delta[0]) / (dot * dot))
        };
        let mut proof = None;
        for fraction in [0.1, 0.25, 0.5, 0.75, 0.9] {
            let parameters = ends.map(|end| if end == 0 { fraction } else { 1. - fraction });
            let beta = (ratio(curves[0], parameters[0])? + ratio(curves[1], parameters[1])?) * 0.5;
            if !beta.is_finite() {
                continue;
            }
            let remaining = max_work
                .saturating_sub(out.exact_work)
                .min(cad_predicates::MAX_WORK);
            if remaining == 0 {
                return Ok(out);
            }
            let r = separator::certify(curves, ends, frame_index, beta, remaining)?;
            out.exact_work += r.exact_work;
            if let Some(c) = r.certificate {
                proof = Some(c);
                break;
            }
        }
        let Some(proof) = proof else {
            return Ok(out);
        };
        separators.push(proof);
    }
    out.certificate = Some(Certificate {
        orientation,
        boundary,
        boundary_indices,
        collapsed,
        separators,
    });
    out.reason = "profile-projection-jordan-interior-injective";
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{surface::Surface, surface_weight_profile::qualify_weight_profile};
    #[test]
    fn exact_factor_boundary_and_collapsed_pole_have_profile_ownership() {
        for collapse in [false, true] {
            let s = Surface {
                degree_u: 1,
                degree_v: 1,
                knots_u: vec![2., 2., 5., 5.],
                knots_v: vec![-3., -3., 4., 4.],
                control_points: vec![
                    vec![
                        vec![0., 0., 0.],
                        vec![0., if collapse { 0. } else { 1. }, 0.],
                    ],
                    vec![vec![1., 0., 0.], vec![1., 1., 0.]],
                ],
                weights: vec![vec![1., 1.1], vec![1.1, 1.21]],
                periodic_u: false,
                periodic_v: false,
            };
            let profile = qualify_weight_profile(&s, [0, 0], 4).unwrap().0.unwrap();
            let r = certify(&profile, [0, 1], 1e-9, 1_000_000, 1000).unwrap();
            let c = r.certificate.expect(r.reason);
            assert!(std::ptr::eq(c.profile(), &profile));
            assert_eq!(c.axes(), [0, 1]);
            assert_eq!(
                c.collapsed_boundaries(),
                if collapse { &[3][..] } else { &[][..] }
            );
            assert_eq!(c.boundary()[0].weights, vec![1., 1.1]);
            assert_eq!(c.boundary()[1].weights, vec![1., 1.1]);
            assert_eq!(c.boundary()[2].weights, vec![1.1, 1.]);
            let mut folded = s.clone();
            folded.control_points[0][1][0] = 1.;
            folded.control_points[1][1][0] = 0.;
            let folded_profile = qualify_weight_profile(&folded, [0, 0], 4)
                .unwrap()
                .0
                .unwrap();
            assert!(
                certify(&folded_profile, [0, 1], 1e-9, 1_000_000, 1000)
                    .unwrap()
                    .certificate
                    .is_none()
            );
            let denied = certify(&profile, [0, 1], 1e-9, 1, 1000).unwrap();
            assert!(denied.certificate.is_none());
            assert!(denied.exact_work <= 1);
        }
    }
}
