use brep_core::Model;
use nurbs_core::curve::Curve;
fn warp(mut m: Model, axis: usize, a: f64, b: f64, c: f64) -> Model {
    let point = |p: &[f64]| {
        let mut q = p.to_vec();
        q[axis] += a * p[0] * p[0] + b * p[0] + c;
        q
    };
    for v in &mut m.vertices {
        v.point = point(&v.point).try_into().unwrap();
    }
    for e in &mut m.edges {
        let old = &e.curve;
        let d = old.domain();
        let p = &old.control_points[0];
        let q = old.control_points.last().unwrap();
        let start = point(p);
        let end = point(q);
        let mut mid = (0..3)
            .map(|k| (start[k] + end[k]) * 0.5)
            .collect::<Vec<_>>();
        mid[axis] -= a * (q[0] - p[0]) * (q[0] - p[0]) * 0.5;
        e.curve = Curve {
            degree: 2,
            knots: [vec![d[0]; 3], vec![d[1]; 3]].concat(),
            control_points: vec![start, mid, end],
            weights: vec![1.; 3],
            periodic: false,
        };
    }
    for f in &mut m.faces {
        let old = f.surface.clone();
        let origin = &old.control_points[0][0];
        let u = (0..3)
            .map(|k| old.control_points[1][0][k] - origin[k])
            .collect::<Vec<_>>();
        let v = (0..3)
            .map(|k| old.control_points[0][1][k] - origin[k])
            .collect::<Vec<_>>();
        let mut cp = vec![vec![vec![0.; 3]; 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                let s = i as f64 / 2.;
                let t = j as f64 / 2.;
                let mut p = (0..3)
                    .map(|k| origin[k] + u[k] * s + v[k] * t)
                    .collect::<Vec<_>>();
                let square = origin[0] * origin[0]
                    + 2. * origin[0] * (u[0] * s + v[0] * t)
                    + u[0] * u[0] * if i == 2 { 1. } else { 0. }
                    + v[0] * v[0] * if j == 2 { 1. } else { 0. }
                    + 2. * u[0] * v[0] * s * t;
                p[axis] += a * square + b * p[0] + c;
                cp[i][j] = p;
            }
        }
        f.surface.degree_u = 2;
        f.surface.degree_v = 2;
        f.surface.knots_u = [
            vec![old.knots_u[old.degree_u]; 3],
            vec![old.knots_u[old.control_points.len()]; 3],
        ]
        .concat();
        f.surface.knots_v = [
            vec![old.knots_v[old.degree_v]; 3],
            vec![old.knots_v[old.control_points[0].len()]; 3],
        ]
        .concat();
        f.surface.control_points = cp;
        f.surface.weights = vec![vec![1.; 3]; 3];
    }
    m.rebuild_topology_ids();
    m.validate().unwrap();
    m
}
pub fn solids() -> (Model, Model) {
    let mut a = warp(
        brep_core::cuboid([0.; 3], [1.; 3]).unwrap(),
        1,
        1. / 16.,
        0.,
        0.,
    );
    // Extend the four vertical carriers, retaining EXACT actual trims. Full
    // carrier graphs may traverse outside those trims between seam events.
    for face in 2..6 {
        let s = &mut a.faces[face].surface;
        let lo = s.knots_v[s.degree_v];
        let hi = s.knots_v[s.control_points[0].len()];
        for p in s.control_points.iter_mut().flatten() {
            p[2] = -1. + 4. * p[2];
        }
        let wires = std::iter::once(a.faces[face].outer)
            .chain(a.faces[face].holes.iter().copied())
            .collect::<Vec<_>>();
        for w in wires {
            for c in &mut a.loops[w].coedges {
                for p in &mut c.pcurve.control_points {
                    p[1] = lo + (p[1] - lo + hi - lo) / 4.;
                }
            }
        }
    }
    a.rebuild_topology_ids();
    a.validate().unwrap();
    // Top: z=x²-x+1/8; seam roots x=1/2 +- sqrt(1/8) are irrational.
    let b = warp(
        brep_core::cuboid([-0.25, -0.25, -0.5], [1.25, 1.5, 0.5]).unwrap(),
        2,
        1.,
        -1.,
        -0.375,
    );
    (a, b)
}

/// Independent cylindrical graph slab, with exact dyadic intersection traces.
/// Unlike `solids`, this deliberately has representable ordinary-Model edges.
pub fn layered_solids() -> (Model, Model) {
    let (a, _) = solids();
    let b = warp(
        brep_core::cuboid([-0.5, -0.5, 0.25], [1.5, 1.5, 0.75]).unwrap(),
        2,
        1. / 16.,
        0.,
        0.,
    );
    (a, b)
}
