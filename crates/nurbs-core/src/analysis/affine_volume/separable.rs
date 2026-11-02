//! Exact tensor-weight cancellation and original coordinate dependence.
//! W_ij W_00=W_i0 W_0j and literal row/column identities reduce a coordinate
//! to one ORIGINAL rational basis. No rounded weight factor is admitted.
use super::*;
#[derive(Clone, Copy)]
enum Axis {
    Constant(f64),
    U,
    V,
}
pub(super) struct Carrier {
    axes: [Axis; 3],
    groups: [Option<Surface>; 2],
}
fn exact_product(a: f64, b: f64) -> Option<f64> {
    if a == 0. || b == 0. {
        return Some(0.);
    }
    if a == 1. {
        return Some(b);
    }
    if b == 1. {
        return Some(a);
    }
    let v = a * b;
    (v.is_finite()
        && a.abs() >= 1e-100
        && b.abs() >= 1e-100
        && v.abs() >= 1e-100
        && a.mul_add(b, -v) == 0.)
        .then_some(v)
}
pub(super) fn prepare(
    s: &Surface,
    work: &mut u64,
    max: u64,
) -> std::result::Result<Option<Carrier>, ()> {
    macro_rules! charge {
        ($n:expr) => {
            if max - *work < $n {
                return Err(());
            }
            *work += $n;
        };
    }
    for i in 0..s.control_points.len() {
        for j in 0..s.control_points[0].len() {
            charge!(3);
            let lhs = exact_product(s.weights[i][j], s.weights[0][0]);
            let rhs = exact_product(s.weights[i][0], s.weights[0][j]);
            if lhs.is_none() || lhs != rhs {
                return Ok(None);
            }
        }
    }
    let mut axes = [Axis::U; 3];
    for k in 0..3 {
        let mut u = true;
        let mut v = true;
        for i in 0..s.control_points.len() {
            for j in 0..s.control_points[0].len() {
                charge!(2);
                u &= s.control_points[i][j][k] == s.control_points[i][0][k];
                v &= s.control_points[i][j][k] == s.control_points[0][j][k];
            }
        }
        axes[k] = match (u, v) {
            (true, true) => Axis::Constant(s.control_points[0][0][k]),
            (true, false) => Axis::U,
            (false, true) => Axis::V,
            _ => return Ok(None),
        };
    }
    let make = |axis: usize| {
        if !axes
            .iter()
            .any(|a| matches!((axis, a), (0, Axis::U) | (1, Axis::V)))
        {
            return None;
        }
        let (degree, knots, n) = if axis == 0 {
            (s.degree_u, s.knots_u.clone(), s.control_points.len())
        } else {
            (s.degree_v, s.knots_v.clone(), s.control_points[0].len())
        };
        let controls = (0..n)
            .map(|i| {
                let p = (0..3)
                    .map(|k| match (axis, axes[k]) {
                        (0, Axis::U) => s.control_points[i][0][k],
                        (1, Axis::V) => s.control_points[0][i][k],
                        _ => 0.,
                    })
                    .collect::<Vec<_>>();
                vec![p.clone(), p]
            })
            .collect();
        let weights = (0..n)
            .map(|i| {
                vec![
                    if axis == 0 {
                        s.weights[i][0]
                    } else {
                        s.weights[0][i]
                    };
                    2
                ]
            })
            .collect();
        Some(Surface {
            degree_u: degree,
            degree_v: 1,
            knots_u: knots,
            knots_v: vec![0., 0., 1., 1.],
            control_points: controls,
            weights,
            periodic_u: false,
            periodic_v: false,
        })
    };
    Ok(Some(Carrier {
        axes,
        groups: [make(0), make(1)],
    }))
}
impl Carrier {
    pub fn enclose(
        &self,
        uv: [[f64; 2]; 2],
        spent: &mut usize,
        max: usize,
    ) -> Result<Option<[[I; 3]; 3]>> {
        let mut out = [[I::point(0.); 3]; 3];
        for k in 0..3 {
            if let Axis::Constant(value) = self.axes[k] {
                out[0][k] = I::point(value);
            }
        }
        for axis in 0..2 {
            let Some(s) = &self.groups[axis] else {
                continue;
            };
            let Some(jets) = charts::cap_jets(s, [uv[axis], [0., 0.]], spent, max)? else {
                return Ok(None);
            };
            for k in 0..3 {
                if matches!((axis, self.axes[k]), (0, Axis::U) | (1, Axis::V)) {
                    out[0][k] = jets[0][k];
                    out[axis + 1][k] = jets[1][k];
                }
            }
        }
        Ok(Some(out))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn separated_weights_and_coordinates_need_exact_original_identities() {
        let mut s = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 1.], vec![1., 1., 1.]],
            ],
            weights: vec![vec![1., 2.], vec![3., 6.]],
            periodic_u: false,
            periodic_v: false,
        };
        let mut work = 0;
        assert!(prepare(&s, &mut work, 1000).unwrap().is_some());
        assert!(prepare(&s, &mut 0, work - 1).is_err());
        s.weights[1][1] = s.weights[1][1].next_up();
        assert!(prepare(&s, &mut 0, 1000).unwrap().is_none());
        s.weights[1][1] = 6.;
        s.control_points[1][1][2] = 2.;
        assert!(prepare(&s, &mut 0, 1000).unwrap().is_none());
    }
}
