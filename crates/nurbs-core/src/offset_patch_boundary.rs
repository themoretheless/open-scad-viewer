//! Canonical contact/end curves of a clamped transition patch.
//! Coedge traversal is metadata: original knots and controls are never reversed.
//! These are patch boundaries, not completed end-transition faces or a solid.
use crate::{
    Result,
    curve::Curve,
    surface::{Axis, Surface},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    FirstContact,
    EndSection,
    SecondContact,
    StartSection,
}
#[derive(Clone)]
pub struct Coedge {
    pub role: Role,
    pub curve: Curve,
    pub reversed: bool,
    /// Fixed source chart axis and exact natural-domain endpoint.
    pub fixed_axis: Axis,
    pub fixed_parameter: f64,
}
impl Coedge {
    pub fn endpoints(&self) -> [&[f64]; 2] {
        let first = self.curve.control_points.first().unwrap().as_slice();
        let last = self.curve.control_points.last().unwrap().as_slice();
        if self.reversed {
            [last, first]
        } else {
            [first, last]
        }
    }
}
pub struct Boundary {
    pub coedges: [Coedge; 4],
}

/// Only endpoint basis selection supplies exact original row/column identity.
/// Unclamped or periodic patches receive no canonical boundary authority.
pub fn extract(patch: &Surface) -> Result<Option<Boundary>> {
    patch.validate()?;
    let nu = patch.control_points.len();
    let nv = patch.control_points[0].len();
    for (knots, degree, n, periodic) in [
        (&patch.knots_u, patch.degree_u, nu, patch.periodic_u),
        (&patch.knots_v, patch.degree_v, nv, patch.periodic_v),
    ] {
        if periodic
            || !knots[..=degree].iter().all(|k| *k == knots[degree])
            || !knots[n..].iter().all(|k| *k == knots[n])
        {
            return Ok(None);
        }
    }
    let u = [patch.knots_u[patch.degree_u], patch.knots_u[nu]];
    let v = [patch.knots_v[patch.degree_v], patch.knots_v[nv]];
    let make = |role, axis, parameter, reversed| -> Result<Coedge> {
        Ok(Coedge {
            role,
            curve: patch.iso(axis, parameter)?,
            reversed,
            fixed_axis: axis,
            fixed_parameter: parameter,
        })
    };
    let coedges = [
        make(Role::FirstContact, Axis::V, v[0], false)?,
        make(Role::EndSection, Axis::U, u[1], false)?,
        make(Role::SecondContact, Axis::V, v[1], true)?,
        make(Role::StartSection, Axis::U, u[0], true)?,
    ];
    for i in 0..4 {
        if coedges[i].endpoints()[1] != coedges[(i + 1) % 4].endpoints()[0] {
            return Ok(None);
        }
    }
    Ok(Some(Boundary { coedges }))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn patch() -> Surface {
        let z = [0., 0.4, 1.];
        Surface {
            degree_u: 1,
            degree_v: 2,
            knots_u: vec![2., 2., 2.3, 3., 3.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: z
                .iter()
                .map(|&z| vec![vec![1., 0., z], vec![1., 1., z], vec![0., 1., z]])
                .collect(),
            weights: vec![vec![1., 0.5f64.sqrt(), 1.]; 3],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn end_sections_and_contacts_share_exact_corners_without_knot_reflection() {
        let p = patch();
        let before = p.clone();
        let b = extract(&p).unwrap().unwrap();
        assert_eq!(p, before);
        assert_eq!(b.coedges[0].curve.knots, p.knots_u);
        assert_eq!(b.coedges[2].curve.knots, p.knots_u);
        assert!(b.coedges[2].reversed);
        assert!(b.coedges[3].reversed);
        for i in 0..4 {
            assert_eq!(
                b.coedges[i].endpoints()[1],
                b.coedges[(i + 1) % 4].endpoints()[0]
            );
        }
        assert_eq!(b.coedges[1].curve.control_points, p.control_points[2]);
        assert_eq!(b.coedges[3].curve.weights, p.weights[0]);
        for end in [1usize, 3] {
            for t in [0., 0.13, 0.5, 0.87, 1.] {
                let q = b.coedges[end].curve.evaluate(t).unwrap().point;
                assert!((q[0] * q[0] + q[1] * q[1] - 1.).abs() < 1e-12);
                assert_eq!(q[2], if end == 1 { 1. } else { 0. });
            }
        }
    }
    #[test]
    fn rotated_patch_preserves_authored_boundary_controls_and_weights() {
        let mut p = patch();
        for row in &mut p.control_points {
            for q in row {
                let [x, y, z] = [q[0], q[1], q[2]];
                *q = vec![z + 12., x - 3., y + 7.];
            }
        }
        let b = extract(&p).unwrap().unwrap();
        for i in 0..3 {
            assert_eq!(b.coedges[0].curve.control_points[i], p.control_points[i][0]);
            assert_eq!(b.coedges[2].curve.control_points[i], p.control_points[i][2]);
        }
        assert_eq!(b.coedges[1].curve.weights, p.weights[2]);
    }
    #[test]
    fn valid_unclamped_patch_cannot_gain_original_boundary_identity() {
        let mut p = patch();
        p.knots_u = vec![1., 2., 2.3, 3., 4.];
        p.validate().unwrap();
        assert!(extract(&p).unwrap().is_none());
        p.weights[0][1] = -1.;
        assert!(extract(&p).is_err());
    }
}
