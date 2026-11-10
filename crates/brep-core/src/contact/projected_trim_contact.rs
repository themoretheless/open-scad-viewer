//! Exact affine chart projections separate trims even on different carriers.
use super::same_support_contact::{self as common, Contact, Polygon, Work};
use crate::Model;
use num_rational::BigRational as Q;
use num_traits::{Signed, ToPrimitive, Zero};
use nurbs_core::surface::Surface;
#[derive(Clone, Debug)]
pub struct Certificate {
    pub faces: [usize; 2],
    pub axes: [usize; 2],
    pub determinant_signs: [i8; 2],
    pub contact: Contact,
    pub cells: usize,
}
impl Certificate {
    pub fn edge(&self) -> Option<usize> {
        if let Contact::Edge(e) = self.contact {
            Some(e)
        } else {
            None
        }
    }
    pub fn vertex(&self) -> Option<usize> {
        if let Contact::Vertex(v) = self.contact {
            Some(v)
        } else {
            None
        }
    }
}
pub struct Attempt {
    pub certificate: Option<Certificate>,
    pub cells: usize,
}
fn q(x: f64) -> Option<Q> {
    Q::from_float(x)
}
fn charge(w: &mut Work, v: &Q) -> Option<()> {
    if w.used == w.limit {
        return None;
    }
    w.used += 1;
    (v.numer().bits() <= 8192 && v.denom().bits() <= 8192).then_some(())
}
pub(super) fn project(s: &Surface, p: &Polygon, axes: [usize; 2], w: &mut Work) -> Option<(Polygon, i8)> {
    let (du, dv) = (s.degree_u, s.degree_v);
    if s.periodic_u
        || s.periodic_v
        || !(1..=8).contains(&du)
        || !(1..=8).contains(&dv)
        || s.control_points.len() != du + 1
        || s.control_points[0].len() != dv + 1
        || s.weights.iter().flatten().any(|v| *v != s.weights[0][0])
    {
        return None;
    }
    let domains = [
        [s.knots_u[du], s.knots_u[du + 1]],
        [s.knots_v[dv], s.knots_v[dv + 1]],
    ];
    for (knots, d, n) in [(&s.knots_u, domains[0], du), (&s.knots_v, domains[1], dv)] {
        if knots[..=n].iter().any(|v| *v != d[0]) || knots[n + 1..].iter().any(|v| *v != d[1]) {
            return None;
        }
    }
    let origin = [
        q(s.control_points[0][0][axes[0]])?,
        q(s.control_points[0][0][axes[1]])?,
    ];
    let u = [
        q(s.control_points[du][0][axes[0]])? - &origin[0],
        q(s.control_points[du][0][axes[1]])? - &origin[1],
    ];
    let v = [
        q(s.control_points[0][dv][axes[0]])? - &origin[0],
        q(s.control_points[0][dv][axes[1]])? - &origin[1],
    ];
    let det = &u[0] * &v[1] - &u[1] * &v[0];
    charge(w, &det)?;
    if det.is_zero() {
        return None;
    }
    // A constant rational weight cancels exactly. Bernstein linear reproduction
    // proves this affine projection on the ENTIRE original parameter rectangle.
    for i in 0..=du {
        for j in 0..=dv {
            for k in 0..2 {
                let expected = &origin[k]
                    + &u[k] * Q::from_integer(i.into()) / Q::from_integer(du.into())
                    + &v[k] * Q::from_integer(j.into()) / Q::from_integer(dv.into());
                charge(w, &expected)?;
                if expected != q(s.control_points[i][j][axes[k]])? {
                    return None;
                }
            }
        }
    }
    let mut points = Vec::new();
    for point in &p.points {
        let t = [
            (q(point[0])? - q(domains[0][0])?) / (q(domains[0][1])? - q(domains[0][0])?),
            (q(point[1])? - q(domains[1][0])?) / (q(domains[1][1])? - q(domains[1][0])?),
        ];
        let mut output = [0.; 2];
        for k in 0..2 {
            let value = &origin[k] + &u[k] * &t[0] + &v[k] * &t[1];
            charge(w, &value)?;
            let f = value.to_f64().filter(|v| v.is_finite())?;
            if q(f)? != value {
                return None;
            }
            output[k] = f;
        }
        points.push(output);
    }
    Some((
        Polygon {
            points,
            vertices: p.vertices.clone(),
            edges: p.edges.clone(),
        },
        if det.is_positive() { 1 } else { -1 },
    ))
}
pub fn certify(m: &Model, faces: [usize; 2], max_cells: usize) -> nurbs_core::Result<Attempt> {
    let mut out = Attempt {
        certificate: None,
        cells: 0,
    };
    if max_cells == 0 || max_cells > 100_000 {
        return Ok(out);
    }
    let (Some(a), Some(b)) = (common::polygon(m, faces[0]), common::polygon(m, faces[1])) else {
        return Ok(out);
    };
    let mut work = Work {
        used: 0,
        limit: max_cells,
    };
    for axes in [[0, 1], [0, 2], [1, 2]] {
        let Some((pa, sa)) = project(&m.faces[faces[0]].surface, &a, axes, &mut work) else {
            continue;
        };
        let Some((pb, sb)) = project(&m.faces[faces[1]].surface, &b, axes, &mut work) else {
            continue;
        };
        if let Some(contact) = common::contact(&pa, &pb, &mut work) {
            out.certificate = Some(Certificate {
                faces,
                axes,
                determinant_signs: [sa, sb],
                contact,
                cells: work.used,
            });
            break;
        }
    }
    out.cells = work.used;
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_affine_projection_refuses_a_small_nonlinear_term_and_exhaustion() {
        let mut s = Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: (0..3)
                .map(|i| (0..2).map(|j| vec![i as f64 / 2., j as f64, 0.]).collect())
                .collect(),
            weights: vec![vec![1.; 2]; 3],
            periodic_u: false,
            periodic_v: false,
        };
        let p = Polygon {
            points: vec![[0., 0.], [1., 0.], [0., 1.]],
            vertices: vec![0, 1, 2],
            edges: vec![0, 1, 2],
        };
        assert!(
            project(
                &s,
                &p,
                [0, 1],
                &mut Work {
                    used: 0,
                    limit: 100
                }
            )
            .is_some()
        );
        s.control_points[1][1][0] += 2f64.powi(-40);
        assert!(
            project(
                &s,
                &p,
                [0, 1],
                &mut Work {
                    used: 0,
                    limit: 100
                }
            )
            .is_none()
        );
        assert!(project(&s, &p, [0, 1], &mut Work { used: 0, limit: 1 }).is_none());
    }
}
