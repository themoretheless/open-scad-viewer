//! Exact source-plane and transverse homothety theorem. No rounded
//! normal, fitted plane or transformed profile supplies geometric authority.
use super::*;
use num_rational::BigRational as Q;
use num_traits::{One, ToPrimitive, Zero};
pub(super) struct Frame {
    pub source: [[f64; 3]; 4],
    pub axes: Vec<[usize; 2]>,
    pub axial_coordinate: Option<usize>,
    pub base_end: usize,
    pub scale: [f64; 2],
    pub collapsed_end: Option<usize>,
}
pub(super) fn derive(
    s: &Surface,
    axis: usize,
    n: usize,
) -> core::result::Result<Frame, &'static str> {
    match from_end(s, axis, n, 0) {
        Err("degenerate-original-profile") => from_end(s, axis, n, 1),
        result => result,
    }
}
fn from_end(
    s: &Surface,
    axis: usize,
    n: usize,
    base_end: usize,
) -> core::result::Result<Frame, &'static str> {
    let pole = |i: usize, j: usize| {
        if axis == 0 {
            &s.control_points[i][j]
        } else {
            &s.control_points[j][i]
        }
    };
    let q = |x: f64| Q::from_float(x).unwrap();
    let difference = |i: usize| {
        std::array::from_fn::<_, 3, _>(|k| q(pole(i, base_end)[k]) - q(pole(0, base_end)[k]))
    };
    let first = (1..n)
        .find(|&i| difference(i).iter().any(|v| !v.is_zero()))
        .ok_or("degenerate-original-profile")?;
    let a = difference(first);
    let cross = |b: &[Q; 3]| {
        std::array::from_fn::<_, 3, _>(|k| {
            &a[(k + 1) % 3] * &b[(k + 2) % 3] - &a[(k + 2) % 3] * &b[(k + 1) % 3]
        })
    };
    let second = (1..n)
        .find(|&i| cross(&difference(i)).iter().any(|v| !v.is_zero()))
        .ok_or("degenerate-original-profile")?;
    let normal = cross(&difference(second));
    let dot = |v: &[Q; 3]| (0..3).map(|k| &normal[k] * &v[k]).sum::<Q>();
    if (0..n).any(|i| !dot(&difference(i)).is_zero()) {
        return Err("nonplanar-original-profile");
    }
    let other = 1 - base_end;
    let coordinate = (0..3).find(|&k| !a[k].is_zero()).unwrap();
    let scale =
        (q(pole(first, other)[coordinate]) - q(pole(0, other)[coordinate])) / &a[coordinate];
    if scale < Q::zero() {
        return Err("interior-profile-scale-collapse");
    }
    if (0..n).any(|i| {
        (0..3).any(|k| {
            q(pole(i, other)[k]) - q(pole(0, other)[k]) != &scale * difference(i)[k].clone()
        })
    }) {
        return Err(if scale == Q::one() {
            "nonconstant-original-translation"
        } else {
            "nonhomothetic-original-profile"
        });
    }
    let delta = std::array::from_fn::<_, 3, _>(|k| q(pole(0, other)[k]) - q(pole(0, base_end)[k]));
    if dot(&delta).is_zero() {
        return Err("tangent-extrusion-direction");
    }
    let nonzero = (0..3).filter(|&k| !normal[k].is_zero()).collect::<Vec<_>>();
    let axial_coordinate = (nonzero.len() == 1).then_some(nonzero[0]);
    let axes = (0..3)
        .rev()
        .filter(|k| !normal[*k].is_zero())
        .map(|omit| {
            let v = (0..3).filter(|k| *k != omit).collect::<Vec<_>>();
            [v[0], v[1]]
        })
        .collect();
    let copy = |i, j| {
        let p = pole(i, j);
        [p[0], p[1], p[2]]
    };
    Ok(Frame {
        source: [
            copy(0, base_end),
            copy(first, base_end),
            copy(second, base_end),
            copy(0, other),
        ],
        base_end,
        scale: {
            let v = scale
                .to_f64()
                .filter(|v| v.is_finite())
                .ok_or("profile-scale-overflow")?;
            let represented = Q::from_float(v).unwrap();
            [
                if represented > scale {
                    v.next_down()
                } else {
                    v
                },
                if represented < scale { v.next_up() } else { v },
            ]
        },
        collapsed_end: scale.is_zero().then_some(other),
        axes,
        axial_coordinate,
    })
}
