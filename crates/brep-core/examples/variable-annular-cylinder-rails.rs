//! Original boundary normals with exact removal of a vanishing U² factor.
//! Reports whole regular rail angles and one-sided endpoint limits, not pole G1.
use cad_predicates::{
    AuthoredScalar, Limits, PredicateContext, Sign, SourceArena, ToleranceContext,
};
use nurbs_core::{interval_eval::Interval as I, surface::Surface};
fn normals(s: &Surface, reverse: bool, factor: bool) -> Vec<[I; 3]> {
    let mut s = s.clone();
    if reverse {
        s.control_points.reverse();
        s.weights.reverse();
    }
    let weights = &s.weights;
    let values = s
        .control_points
        .iter()
        .enumerate()
        .flat_map(|(u, r)| {
            r.iter()
                .enumerate()
                .flat_map(move |(v, p)| [p[0], p[1], p[2], weights[u][v]])
        })
        .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
        .collect();
    let arena = SourceArena::authored("original-cylinder-rail", 1, values).unwrap();
    let leaves = s
        .control_points
        .iter()
        .enumerate()
        .map(|(u, r)| {
            r.iter()
                .enumerate()
                .map(|(v, _)| {
                    std::array::from_fn(|k| arena.leaf((u * r.len() + v) * 4 + k).unwrap())
                })
                .collect()
        })
        .collect::<Vec<_>>();
    let tolerance = ToleranceContext::default_valid();
    let n = 3 * s.degree_u - 1;
    let mut net = vec![[I::point(0.); 3]; n + 1];
    for (axis, axes) in [[1, 2], [2, 0], [0, 1]].into_iter().enumerate() {
        for row in 0..=n {
            let mut ctx = PredicateContext::new(
                &arena,
                &tolerance,
                Limits {
                    max_work: cad_predicates::MAX_WORK,
                    ..Limits::default()
                },
                None,
            );
            let d = cad_predicates::rational_surface_projected_jacobian_coefficient_bounds(
                &mut ctx,
                &leaves,
                axes,
                [row, 3 * s.degree_v - 1],
            )
            .unwrap();
            if factor && row < 2 {
                assert_eq!(d.signs.as_ref().unwrap()[0][0], Sign::Zero);
            }
            let b = d.coefficient_bounds.unwrap()[0][0];
            net[row][axis] = I::new(b[0], b[1]).unwrap();
        }
    }
    if factor {
        net = (2..=n)
            .map(|i| {
                let scale = I::point((n * (n - 1)) as f64)
                    .div(I::point((i * (i - 1)) as f64))
                    .unwrap();
                net[i].map(|x| x.mul(scale).unwrap())
            })
            .collect();
    }
    let scale = net
        .iter()
        .flatten()
        .map(|x| x.lo.abs().max(x.hi.abs()))
        .fold(0_f64, f64::max);
    assert!(scale > 0.);
    net.iter_mut().for_each(|p| {
        p.iter_mut()
            .for_each(|x| *x = x.div(I::point(scale)).unwrap())
    });
    net
}
fn split(net: &[[I; 3]]) -> (Vec<[I; 3]>, Vec<[I; 3]>) {
    let mut row = net.to_vec();
    let mut a = vec![row[0]];
    let mut b = vec![*row.last().unwrap()];
    while row.len() > 1 {
        row = row
            .windows(2)
            .map(|p| {
                std::array::from_fn(|k| p[0][k].add(p[1][k]).unwrap().mul(I::point(0.5)).unwrap())
            })
            .collect();
        a.push(row[0]);
        b.push(*row.last().unwrap());
    }
    b.reverse();
    (a, b)
}
fn square(x: I) -> I {
    let lo = if x.lo > 0. {
        x.lo
    } else if x.hi < 0. {
        -x.hi
    } else {
        0.
    };
    let hi = x.lo.abs().max(x.hi.abs());
    I::new((lo * lo).next_down().max(0.), (hi * hi).next_up()).unwrap()
}
fn norm(x: [I; 3]) -> I {
    x.into_iter()
        .map(square)
        .fold(I::point(0.), |a, b| a.add(b).unwrap())
}
fn angle(a: &[[I; 3]], b: &[[I; 3]]) -> Option<f64> {
    let hull = |net: &[[I; 3]]| {
        std::array::from_fn(|k| {
            I::new(
                net.iter().map(|p| p[k].lo).fold(f64::INFINITY, f64::min),
                net.iter()
                    .map(|p| p[k].hi)
                    .fold(f64::NEG_INFINITY, f64::max),
            )
            .unwrap()
        })
    };
    let a = hull(a);
    let b = hull(b);
    let den = norm(a).mul(norm(b)).unwrap();
    if den.lo <= 0. {
        return None;
    }
    let cross = std::array::from_fn(|k| {
        a[(k + 1) % 3]
            .mul(b[(k + 2) % 3])
            .unwrap()
            .sub(a[(k + 2) % 3].mul(b[(k + 1) % 3]).unwrap())
            .unwrap()
    });
    Some(norm(cross).div(den).unwrap().hi)
}
fn main() {
    let output = std::env::args().nth(1).expect("output JSON path");
    let mut cases = Vec::new();
    for radii in [[0.5, 1.25], [1.25, 0.5]] {
        for direction in [1., -1.] {
            let model = brep_core::circular_blend::partial_annular_variable_arc(
                20.,
                5.,
                6.,
                radii,
                [0.3, 0.7, 0.4].map(|x| x * direction),
                1e-7,
            )
            .unwrap();
            for base in [0, 5, 10] {
                let blend = &model.faces[base].surface;
                let wall = &model.faces[base + 2].surface;
                assert_eq!(blend.degree_u, wall.degree_u);
                for source in [blend, wall] {
                    let p = source.degree_u;
                    assert_eq!(source.control_points.len(), p + 1);
                    assert_eq!(source.knots_u.len(), 2 * (p + 1));
                    assert!(source.knots_u[..=p].iter().all(|x| *x == source.knots_u[0]));
                    assert!(source.knots_u[p + 1..]
                        .iter()
                        .all(|x| *x == source.knots_u[p + 1]));
                    assert!(source.knots_u[0] < source.knots_u[p + 1]);
                }
                for u in 0..blend.control_points.len() {
                    assert_eq!(
                        blend.control_points[u][2], wall.control_points[u][1],
                        "Original rail positions must have identical U controls"
                    );
                    assert_eq!(
                        blend.weights[u][2], wall.weights[u][1],
                        "Original rail weights must match"
                    );
                }

                let a = normals(&model.faces[base].surface, base == 10, base != 5);
                let b = normals(&model.faces[base + 2].surface, base == 10, false);
                let mut queue = vec![(a, b, [0., 1.])];
                let mut cells = 0;
                let mut accepted = Vec::new();
                let mut unresolved = Vec::new();
                while let Some((a, b, t)) = queue.pop() {
                    cells += 1;
                    let bound = angle(&a, &b);
                    if bound.is_some_and(|x| x <= 1e-4) {
                        accepted.push(value_codec::json!({"interval":t,"sineSquaredUpper":bound}));
                    } else if cells + queue.len() >= 10000 || t[1] - t[0] < 1e-10 {
                        unresolved
                            .push(value_codec::json!({"interval":t,"sineSquaredUpper":bound}));
                    } else {
                        let (a0, a1) = split(&a);
                        let (b0, b1) = split(&b);
                        let m = (t[0] + t[1]) * 0.5;
                        queue.push((a1, b1, [m, t[1]]));
                        queue.push((a0, b0, [t[0], m]));
                    }
                }
                cases.push(value_codec::json!({"radii":radii,"direction":direction,"faces":[base,base+2],"reversed":base==10,"exactUFactor":if base==5 {0}else{2},"proven":unresolved.is_empty(),"cells":cells,"accepted":accepted,"unresolved":unresolved}));
            }
        }
    }
    std::fs::write(output,value_codec::to_string_pretty(&value_codec::json!({"schema":"cad-annular-cylinder-rails/1","scope":"Original normal directions over full regular rail and one-sided endpoint limits; no unique pole plane or solid admission","maxSineSquared":1e-4,"cases":cases})).unwrap()).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn angle_bounds_reject_oblique_and_singular_normals() {
        let z = vec![[I::point(0.), I::point(0.), I::point(1.)]; 3];
        let reverse = vec![[I::point(0.), I::point(0.), I::point(-2.)]; 2];
        let x = vec![[I::point(1.), I::point(0.), I::point(0.)]; 2];
        assert!(angle(&z, &reverse).unwrap() < 1e-4);
        assert!(angle(&z, &x).unwrap() > 0.99);
        assert!(angle(&z, &vec![[I::point(0.); 3]; 2]).is_none());
    }
    #[test]
    fn subdivision_keeps_original_curve_endpoints_and_shared_midpoint() {
        let net = vec![[I::point(0.); 3], [I::point(4.); 3], [I::point(0.); 3]];
        let (a, b) = split(&net);
        assert_eq!(a[0][0].lo, 0.);
        assert_eq!(b[2][0].hi, 0.);
        assert!(a[2][0].lo <= 2. && a[2][0].hi >= 2.);
        assert_eq!(a[2][0].lo, b[0][0].lo);
        assert_eq!(a[2][0].hi, b[0][0].hi);
    }
}
