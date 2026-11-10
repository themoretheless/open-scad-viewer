use super::*;
mod trim;
pub(super) type Box3 = [[Q; 2]; 3];
fn separated(a: &[[Q; 2]], b: &[[Q; 2]]) -> bool {
    (0..a.len()).any(|i| a[i][1] < b[i][0] || b[i][1] < a[i][0])
}
fn bounds(points: &[P]) -> Box3 {
    std::array::from_fn(|i| {
        [
            points.iter().map(|p| p[i].clone()).min().unwrap(),
            points.iter().map(|p| p[i].clone()).max().unwrap(),
        ]
    })
}
pub(super) fn restrict_box(p: &Bi, window: &[[Q; 2]; 2], work: &mut Work) -> Result<Bi> {
    polynomial::compose(
        p,
        &[
            [
                window[0][0].clone(),
                &window[0][1] - &window[0][0],
                Q::zero(),
            ],
            [
                window[1][0].clone(),
                Q::zero(),
                &window[1][1] - &window[1][0],
            ],
        ],
        work,
    )
}
pub(super) fn swept_bounds(
    powers: &[[Bi; 4]; 2],
    window: &[[Q; 2]; 2],
    work: &mut Work,
) -> Result<Box3> {
    let mut points = vec![];
    for h in powers {
        let mut nets: [Bi; 4] = std::array::from_fn(|_| vec![]);
        // All homogeneous coordinates need the SAME tensor degree; elevate
        // by padding power coefficient rectangles before Bernstein conversion.
        let restricted = h
            .iter()
            .map(|p| restrict_box(p, window, work))
            .collect::<Result<Vec<_>>>()?;
        let n = restricted.iter().map(Vec::len).max().unwrap().max(1);
        let m = restricted
            .iter()
            .flatten()
            .map(Vec::len)
            .max()
            .unwrap_or(1)
            .max(1);
        for axis in 0..4 {
            let mut p = restricted[axis].clone();
            p.resize(n, vec![]);
            for row in &mut p {
                row.resize(m, Q::zero());
            }
            nets[axis] = polynomial::bernstein(&p, work)?;
        }
        for j in 0..n {
            for i in 0..m {
                let w = &nets[3][j][i];
                if w <= &Q::zero() {
                    return Err(fail(
                        "pocket rational control hull has no proved positive denominator",
                    ));
                }
                let p = std::array::from_fn(|axis| &nets[axis][j][i] / w);
                guard(&p)?;
                points.push(p);
            }
        }
    }
    Ok(bounds(&points))
}
fn curve_avoids(h: &[sb::H], window: &[[Q; 2]; 2], depth: usize, work: &mut Work) -> Result<()> {
    work.charge(h.len())?;
    if h.iter().any(|p| p[3] <= Q::zero()) {
        return Err(fail("pocket trim guard requires positive rational weights"));
    }
    let points = h.iter().map(sb::point).collect::<Vec<_>>();
    let bb = bounds(&points);
    if separated(&bb[..2], window) {
        return Ok(());
    }
    if depth == 12 {
        return Err(fail(
            "pocket window meets an original trim or trim separation is unproved",
        ));
    }
    let mut row = h.to_vec();
    let mut a = vec![row[0].clone()];
    let mut b = vec![row.last().unwrap().clone()];
    while row.len() > 1 {
        row = row
            .windows(2)
            .map(|p| std::array::from_fn(|i| (&p[0][i] + &p[1][i]) / integer(2)))
            .collect();
        a.push(row[0].clone());
        b.push(row.last().unwrap().clone());
    }
    b.reverse();
    curve_avoids(&a, window, depth + 1, work)?;
    curve_avoids(&b, window, depth + 1, work)
}
pub(super) fn face_contains_window(
    model: &Model,
    face: usize,
    window: &[[Q; 2]; 2],
    work: &mut Work,
) -> Result<crate::profile_certificate::PointWindingCertificate> {
    let f = &model.faces[face];
    let s = &f.surface;
    let ranges = [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ];
    let real: [[Q; 2]; 2] = std::array::from_fn(|axis| {
        window[axis].each_ref().map(|t| {
            Q::from_float(ranges[axis][0]).unwrap()
                + t * (Q::from_float(ranges[axis][1]).unwrap()
                    - Q::from_float(ranges[axis][0]).unwrap())
        })
    });
    for id in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
        for c in &model.loops[id].coedges {
            if c.trim().is_some_and(|t| {
                t.pcurve_parameter != c.pcurve.domain() || t.periodic_lift != [[0; 2]; 2]
            }) {
                return Err(fail("pocket trim guard needs complete authored pcurve domains without periodic lifts"));
            }
        }
    }
    let loops = std::iter::once(f.outer)
        .chain(f.holes.iter().copied())
        .map(|l| {
            model.loops[l]
                .coedges
                .iter()
                .map(|c| c.pcurve.clone())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    for c in loops.iter().flatten() {
        for k in c.degree..c.control_points.len() {
            if c.knots[k] < c.knots[k + 1] {
                let h = sb::curve(
                    c,
                    &Q::from_float(c.knots[k]).unwrap(),
                    &Q::from_float(c.knots[k + 1]).unwrap(),
                )?;
                curve_avoids(&h, &real, 0, work)?;
            }
        }
    }
    let point = real
        .each_ref()
        .map(|r| roots::ratio(&((&r[0] + &r[1]) / integer(2))));
    let cert = crate::profile_certificate::certify_rational_point_winding(&loops, point.clone())?;
    let checked = crate::profile_certificate::verify_rational_point_winding(&loops, point, &cert)?;
    if checked.windings()[0] != 1 || checked.windings().iter().skip(1).any(|w| *w != 0) {
        return Err(fail(
            "pocket window is outside the original face or inside an authored hole",
        ));
    }
    Ok(cert)
}
fn surface_avoids(
    s: &Surface,
    lo: [Q; 2],
    hi: [Q; 2],
    pocket: &Box3,
    depth: usize,
    work: &mut Work,
) -> Result<()> {
    work.charge(1)?;
    let points = sb::controls(s, &lo, &hi)?;
    for p in &points {
        guard(p)?;
    }
    work.charge(points.len())?;
    if separated(&bounds(&points), pocket) {
        return Ok(());
    }
    if depth == 12 {
        return Err(fail(
            "pocket swept volume touches another carrier or spatial separation is unproved",
        ));
    }
    let axis = depth % 2;
    let middle = (&lo[axis] + &hi[axis]) / integer(2);
    let mut left_hi = hi.clone();
    left_hi[axis] = middle.clone();
    let mut right_lo = lo.clone();
    right_lo[axis] = middle;
    surface_avoids(s, lo, left_hi, pocket, depth + 1, work)?;
    surface_avoids(s, right_lo, hi, pocket, depth + 1, work)
}
pub(super) fn other_faces_avoid(
    model: &Model,
    face: usize,
    pocket: &Box3,
    work: &mut Work,
) -> Result<()> {
    for (i, f) in model.faces.iter().enumerate() {
        if i != face {
            let s = &f.surface;
            if s.weights.iter().flatten().any(|w| *w <= 0.) {
                return Err(fail(
                    "retained carrier spatial bounds need positive source weights",
                ));
            }
            let lo = [
                Q::from_float(s.knots_u[s.degree_u]).unwrap(),
                Q::from_float(s.knots_v[s.degree_v]).unwrap(),
            ];
            let hi = [
                Q::from_float(s.knots_u[s.control_points.len()]).unwrap(),
                Q::from_float(s.knots_v[s.control_points[0].len()]).unwrap(),
            ];
            if let Err(error) = surface_avoids(s, lo, hi, pocket, 0, work) {
                if error.code != "MINKOWSKI_3D_REFUSED" || !trim::avoids(model, i, pocket, work)? {
                    return Err(error);
                }
            }
        }
    }
    Ok(())
}
