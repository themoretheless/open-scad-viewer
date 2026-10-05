//! Local constant-radius rolling-ball envelope over a certified contact band.
//! Rational in the cross-section parameter; implicit in the center parameter.
//! No fitted tensor NURBS, replacement trims, embedding or topology authority.
use crate::{
    Result, check,
    distance_bounds::Interval as I,
    offset_contact_tangent::{self, Report as Tangent},
    surface::Surface,
    surface_offset::{self, ContactBand},
};
type V = [I; 3];
#[derive(Clone, Debug)]
pub struct Cell {
    pub arc_parameter: [f64; 2],
    pub image: [[f64; 2]; 3],
    pub center_derivative: [[f64; 2]; 3],
    pub arc_derivative: [[f64; 2]; 3],
    pub area_speed: [f64; 2],
    pub regular: bool,
}
pub struct Report {
    pub radius: f64,
    pub tangent: Tangent,
    pub cells: Vec<Cell>,
    pub visited: usize,
    pub regular: bool,
    pub reason: &'static str,
}
impl Report {
    pub fn to_value(&self) -> value_codec::Value {
        use value_codec::json;
        let cells: Vec<_> = self.cells.iter().map(|c| json!({
            "arcParameter":c.arc_parameter,"imageIntervalsMm":c.image,
            "centerDerivativeIntervalsMm":c.center_derivative,"arcDerivativeIntervalsMm":c.arc_derivative,
            "areaSpeedIntervalMm2":c.area_speed,"regularityProven":c.regular
        })).collect();
        json!({"method":"interval-rolling-ball-envelope","scope":"constant-radius-local-contact-band",
            "radiusMm":self.radius,"centerTangent":self.tangent.to_value(),"cells":cells,
            "visitedCells":self.visited,"envelopeRegularityProven":self.regular,"reason":self.reason,
            "wholeCurveComplete":false,"finiteNurbsPatchProven":false,"trimMembershipProven":false,
            "embeddingProven":false,"topologyAuthority":false})
    }
}
fn vec_i(x: [[f64; 2]; 3]) -> Result<V> {
    Ok([
        I::new(x[0][0], x[0][1])?,
        I::new(x[1][0], x[1][1])?,
        I::new(x[2][0], x[2][1])?,
    ])
}
fn dot(a: V, b: V) -> Result<I> {
    let mut r = I::point(0.);
    for k in 0..3 {
        r = r.add(a[k].mul(b[k])?)?;
    }
    Ok(r)
}
fn cross(a: V, b: V) -> Result<V> {
    let mut r = [I::point(0.); 3];
    for k in 0..3 {
        r[k] = a[(k + 1) % 3]
            .mul(b[(k + 2) % 3])?
            .sub(a[(k + 2) % 3].mul(b[(k + 1) % 3])?)?;
    }
    Ok(r)
}
struct Arc {
    center: V,
    center_tangent: V,
    radius: I,
    // Homogeneous relative controls and their center-parameter derivatives.
    controls: [V; 3],
    velocities: [V; 3],
    weights: [I; 3],
    weight_velocities: [I; 3],
}
impl Arc {
    fn cell(&self, range: [f64; 2]) -> Result<Cell> {
        let s = I::new(range[0], range[1])?;
        let a = I::point(1.).sub(s)?;
        let two = I::point(2.);
        let basis = [a.mul(a)?, two.mul(a)?.mul(s)?, s.mul(s)?];
        let deriv = [
            I::point(-2.).mul(a)?,
            two.mul(I::point(1.).sub(two.mul(s)?)?)?,
            two.mul(s)?,
        ];
        let mut w = I::point(0.);
        let mut wt = w;
        let mut ws = w;
        let mut n = [w; 3];
        let mut nt = n;
        let mut ns = n;
        for j in 0..3 {
            w = w.add(basis[j].mul(self.weights[j])?)?;
            wt = wt.add(basis[j].mul(self.weight_velocities[j])?)?;
            ws = ws.add(deriv[j].mul(self.weights[j])?)?;
            for k in 0..3 {
                n[k] = n[k].add(basis[j].mul(self.controls[j][k])?)?;
                nt[k] = nt[k].add(basis[j].mul(self.velocities[j][k])?)?;
                ns[k] = ns[k].add(deriv[j].mul(self.controls[j][k])?)?;
            }
        }
        // Bernstein partition of unity supplies a denominator bound even on
        // the full arc where independent scalar basis intervals are wide.
        w = w.intersect(
            self.weights
                .iter()
                .map(|x| x.lo)
                .fold(f64::INFINITY, f64::min),
            self.weights
                .iter()
                .map(|x| x.hi)
                .fold(f64::NEG_INFINITY, f64::max),
        )?;
        let mut image = [[0.; 2]; 3];
        let mut pt = [I::point(0.); 3];
        let mut ps = pt;
        for k in 0..3 {
            let q = n[k].div(w)?;
            let p = self.center[k].add(self.radius.mul(q)?)?;
            image[k] = [p.lo, p.hi];
            pt[k] = self.center_tangent[k].add(self.radius.mul(nt[k].sub(q.mul(wt)?)?.div(w)?)?)?;
            ps[k] = self.radius.mul(ns[k].sub(q.mul(ws)?)?.div(w)?)?;
        }
        let area = offset_contact_tangent::speed(cross(pt, ps)?)?;
        Ok(Cell {
            arc_parameter: range,
            image,
            center_derivative: pt.map(|x| [x.lo, x.hi]),
            arc_derivative: ps.map(|x| [x.lo, x.hi]),
            area_speed: [area.lo, area.hi],
            regular: area.lo > 0.,
        })
    }
}
/// Constant signed offset distances must have the same nonzero absolute radius.
/// Every retained cell covers the entire driving center-parameter interval.
/// `max_cells` bounds total evaluated arc cells, including failed parent cells.
/// Unresolved leaves remain in the report and prevent a regularity claim.
pub fn certify(
    surfaces: [&Surface; 2],
    distances: [f64; 2],
    fixed_axis: usize,
    fixed_interval: [f64; 2],
    first_other: [f64; 2],
    second: [[f64; 2]; 2],
    max_spans: usize,
    max_cells: usize,
) -> Result<Report> {
    certify_arc(
        surfaces,
        distances,
        fixed_axis,
        fixed_interval,
        first_other,
        second,
        max_spans,
        [0., 1.],
        max_cells,
    )
}
/// Internal rectangle oracle for fitting; a point arc parameter is permitted.
/// Returned cells cover only `arc_range`, never an implied whole arc.
pub(crate) fn certify_arc(
    surfaces: [&Surface; 2],
    distances: [f64; 2],
    fixed_axis: usize,
    fixed_interval: [f64; 2],
    first_other: [f64; 2],
    second: [[f64; 2]; 2],
    max_spans: usize,
    arc_range: [f64; 2],
    max_cells: usize,
) -> Result<Report> {
    check(
        arc_range.iter().all(|x| x.is_finite())
            && arc_range[0] >= 0.
            && arc_range[1] <= 1.
            && arc_range[0] <= arc_range[1],
        "Envelope arc range must lie in [0,1]",
    )?;
    check(
        distances.iter().all(|d| d.is_finite())
            && distances[0] != 0.
            && distances[0].abs() == distances[1].abs(),
        "Envelope requires equal nonzero absolute radii",
    )?;
    check(
        max_cells > 0 && max_cells <= 1_000_000,
        "Envelope cell budget must be between 1 and 1000000",
    )?;
    let tangent = offset_contact_tangent::certify(
        surfaces,
        distances,
        fixed_axis,
        fixed_interval,
        first_other,
        second,
        max_spans,
    )?;
    let mut out = Report {
        radius: distances[0].abs(),
        tangent,
        cells: vec![],
        visited: 0,
        regular: false,
        reason: "center-regularity-unproven",
    };
    if !out.tangent.regular {
        return Ok(out);
    }
    let ContactBand::ContinuousBranch(contact) = &out.tangent.contact else {
        return Ok(out);
    };
    let domains = [contact.first_uv, contact.second_uv];
    let center = vec_i(contact.point)?;
    let center_tangent = vec_i(out.tangent.tangent.unwrap())?;
    let derivative = out
        .tangent
        .derivatives
        .unwrap()
        .map(|x| I::new(x[0], x[1]))
        .into_iter()
        .collect::<Result<Vec<_>>>()?;
    let radius = I::point(distances[0].abs());
    let mut dirs = [[I::point(0.); 3]; 2];
    let mut velocities = dirs;
    for side in 0..2 {
        let bounds =
            surface_offset::bounds(surfaces[side], domains[side], distances[side], max_spans)?;
        let jets = surface_offset::jacobian_bounds(surfaces[side], domains[side], 0., max_spans)?;
        let (Some(normals), Some(jets)) = (bounds.unit_normals, jets.derivatives) else {
            out.reason = "source-jet-enclosure-unresolved";
            return Ok(out);
        };
        let normal = vec_i(normals)?;
        let du = vec_i(jets[0])?;
        let dv = vec_i(jets[1])?;
        for k in 0..3 {
            dirs[side][k] = normal[k].mul(I::point(-distances[side].signum()))?;
            let source_velocity = if side == 0 {
                let axes = [du, dv];
                axes[fixed_axis][k].add(axes[1 - fixed_axis][k].mul(derivative[0])?)?
            } else {
                du[k].mul(derivative[1])?.add(dv[k].mul(derivative[2])?)?
            };
            velocities[side][k] = source_velocity.sub(center_tangent[k])?.div(radius)?;
        }
    }
    let [u, v] = dirs;
    let [ut, vt] = velocities;
    // Minor sphere arc: endpoints C+rU, C+rV; middle control
    // C+r(U+V)/(1+U.V), middle weight sqrt((1+U.V)/2).
    // U,V are unit and orthogonal to C' by the constant-offset contact equation.
    let d = I::point(1.).add(dot(u, v)?)?.intersect(0., 2.)?;
    if d.lo <= 0. {
        out.reason = "antipodal-contact-unresolved";
        return Ok(out);
    }
    let half = d.mul(I::point(0.5))?;
    let w = I::new(half.lo.sqrt().next_down().max(0.), half.hi.sqrt().next_up())?;
    if w.lo <= 0. {
        out.reason = "arc-weight-unresolved";
        return Ok(out);
    }
    let dt = dot(ut, v)?.add(dot(u, vt)?)?;
    let wt = dt.div(I::point(4.).mul(w)?)?;
    let mut middle = [I::point(0.); 3];
    let mut middle_t = middle;
    for k in 0..3 {
        let sum = u[k].add(v[k])?;
        let m = sum.div(d)?;
        let mt = ut[k].add(vt[k])?.sub(m.mul(dt)?)?.div(d)?;
        middle[k] = w.mul(m)?;
        middle_t[k] = wt.mul(m)?.add(w.mul(mt)?)?;
    }
    let arc = Arc {
        center,
        center_tangent,
        radius,
        controls: [u, middle, v],
        velocities: [ut, middle_t, vt],
        weights: [I::point(1.), w, I::point(1.)],
        weight_velocities: [I::point(0.), wt, I::point(0.)],
    };
    let mut pending = vec![arc_range];
    while let Some(range) = pending.pop() {
        let cell = arc.cell(range)?;
        out.visited += 1;
        let mid = range[0] + (range[1] - range[0]) * 0.5;
        // Reserve one evaluation for every pending leaf before subdividing.
        if !cell.regular
            && mid > range[0]
            && mid < range[1]
            && out.visited + pending.len() + 2 <= max_cells
        {
            pending.push([mid, range[1]]);
            pending.push([range[0], mid]);
        } else {
            out.cells.push(cell);
        }
    }
    out.regular = out.cells.iter().all(|c| c.regular);
    out.reason = if out.regular {
        "regular-local-envelope"
    } else {
        "envelope-regularity-unproven"
    };
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn plane() -> Surface {
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
    fn pair() -> [Surface; 2] {
        let a = plane();
        let mut b = a.clone();
        for row in &mut b.control_points {
            for p in row {
                let z = p[1];
                p[1] = 0.5;
                p[2] = z;
            }
        }
        [a, b]
    }
    fn run(a: &Surface, b: &Surface, cells: usize) -> Report {
        certify(
            [a, b],
            [0.2, 0.2],
            0,
            [0.35, 0.39],
            [0.25, 0.35],
            [[0.30, 0.44], [0.15, 0.25]],
            2,
            cells,
        )
        .unwrap()
    }
    #[test]
    fn quarter_cylinder_envelope_covers_entire_band_and_arc() {
        let [a, b] = pair();
        let r = run(&a, &b, 255);
        assert!(r.regular, "{} {:?}", r.reason, r.cells);
        assert_eq!(r.cells.first().unwrap().arc_parameter[0], 0.);
        assert_eq!(r.cells.last().unwrap().arc_parameter[1], 1.);
        for c in &r.cells {
            assert!(c.area_speed[0] > 0.);
        }
        for cells in r.cells.windows(2) {
            assert_eq!(cells[0].arc_parameter[1], cells[1].arc_parameter[0]);
        }
        // Independent analytic circle evaluation at each cell endpoint/midpoint,
        // at both driving endpoints. These are regressions, not the certificate.
        for c in &r.cells {
            for t in [0.35, 0.39] {
                for s in [
                    c.arc_parameter[0],
                    (c.arc_parameter[0] + c.arc_parameter[1]) * 0.5,
                    c.arc_parameter[1],
                ] {
                    let w = std::f64::consts::FRAC_1_SQRT_2;
                    let den = (1. - s).powi(2) + 2. * w * s * (1. - s) + s * s;
                    let y = 0.3 + 0.2 * (2. * w * s * (1. - s) + s * s) / den;
                    let z = 0.2 - 0.2 * ((1. - s).powi(2) + 2. * w * s * (1. - s)) / den;
                    let ny = 2. * w * s * (1. - s) + s * s;
                    let nz = (1. - s).powi(2) + 2. * w * s * (1. - s);
                    let dny = 2. * w * (1. - 2. * s) + 2. * s;
                    let dnz = -2. * (1. - s) + 2. * w * (1. - 2. * s);
                    let dd = dnz + 2. * s;
                    let ps = [
                        0.,
                        0.2 * (dny * den - ny * dd) / (den * den),
                        -0.2 * (dnz * den - nz * dd) / (den * den),
                    ];
                    for (k, x) in [1., 0., 0.].into_iter().enumerate() {
                        assert!(c.center_derivative[k][0] <= x && x <= c.center_derivative[k][1]);
                    }
                    for (k, x) in ps.into_iter().enumerate() {
                        assert!(c.arc_derivative[k][0] <= x && x <= c.arc_derivative[k][1]);
                    }
                    let area = ps[1].hypot(ps[2]);
                    assert!(c.area_speed[0] <= area && area <= c.area_speed[1]);
                    for (k, x) in [t, y, z].into_iter().enumerate() {
                        assert!(
                            c.image[k][0] <= x && x <= c.image[k][1],
                            "{:?} misses {x}",
                            c.image[k]
                        );
                    }
                }
            }
        }
    }
    #[test]
    fn work_stop_retains_full_unresolved_arc() {
        let [a, b] = pair();
        let r = run(&a, &b, 1);
        assert!(!r.regular);
        assert_eq!(r.visited, 1);
        assert_eq!(r.cells.len(), 1);
        assert_eq!(r.cells[0].arc_parameter, [0., 1.]);
        let r = run(&a, &b, 2);
        assert!(r.visited <= 2);
        assert_eq!(r.cells[0].arc_parameter, [0., 1.]);
    }
    #[test]
    fn curved_centerline_has_regular_envelope_in_rotated_frames() {
        let a = Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![[3., 0.], [3., 3.], [0., 3.]]
                .into_iter()
                .map(|p| vec![vec![p[0], p[1], 0.], vec![p[0], p[1], 5.]])
                .collect(),
            weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.]
                .into_iter()
                .map(|w| vec![w; 2])
                .collect(),
            periodic_u: false,
            periodic_v: false,
        };
        let mut b = plane();
        for row in &mut b.control_points {
            for p in row {
                let y = 4. * p[0];
                let z = 5. * p[1];
                *p = vec![2. - z, y, z];
            }
        }
        let x = 2. + 0.2 * 2_f64.sqrt() - 0.5;
        let y = (3.2_f64.powi(2) - x * x).sqrt();
        let mut u = 0.5;
        for _ in 0..8 {
            let e = surface_offset::evaluate(&a, [u, 0.1], 0.2).unwrap();
            u -= (e.point[0] - x) / e.du[0];
        }
        let bu = y / 4.;
        let bv = (0.5 - 0.2 / 2_f64.sqrt()) / 5.;
        for rotated in [false, true] {
            let mut aa = a.clone();
            let mut bb = b.clone();
            if rotated {
                for s in [&mut aa, &mut bb] {
                    for row in &mut s.control_points {
                        for p in row {
                            *p = vec![p[2] + 17., p[0] - 9., p[1] + 23.];
                        }
                    }
                }
            }
            let r = certify(
                [&aa, &bb],
                [0.2, 0.2],
                1,
                [0.099999, 0.100001],
                [u - 1e-4, u + 1e-4],
                [[bu - 1e-4, bu + 1e-4], [bv - 1e-4, bv + 1e-4]],
                2,
                511,
            )
            .unwrap();
            assert!(r.regular, "{} {:?}", r.reason, r.cells);
            assert_eq!(r.cells.first().unwrap().arc_parameter[0], 0.);
            assert_eq!(r.cells.last().unwrap().arc_parameter[1], 1.);
            assert!(r.visited <= 511);
            for c in &r.cells {
                for t in [0.099999, 0.1, 0.100001] {
                    let x = 2. + 0.2 * 2_f64.sqrt() - 5. * t;
                    let y = (3.2_f64.powi(2) - x * x).sqrt();
                    let center = [x, y, 5. * t];
                    let ua = [-x / 3.2, -y / 3.2, 0.];
                    let ub = [
                        -std::f64::consts::FRAC_1_SQRT_2,
                        0.,
                        -std::f64::consts::FRAC_1_SQRT_2,
                    ];
                    let d = 1. + ua.iter().zip(ub).map(|(x, y)| x * y).sum::<f64>();
                    let w = (d / 2.).sqrt();
                    for s in [
                        c.arc_parameter[0],
                        (c.arc_parameter[0] + c.arc_parameter[1]) * 0.5,
                        c.arc_parameter[1],
                    ] {
                        let den = (1. - s).powi(2) + 2. * w * s * (1. - s) + s * s;
                        let p: [f64; 3] = std::array::from_fn(|k| {
                            center[k]
                                + 0.2
                                    * ((1. - s).powi(2) * ua[k]
                                        + 2. * w * s * (1. - s) * (ua[k] + ub[k]) / d
                                        + s * s * ub[k])
                                    / den
                        });
                        let p = if rotated {
                            [p[2] + 17., p[0] - 9., p[1] + 23.]
                        } else {
                            p
                        };
                        for (k, x) in p.into_iter().enumerate() {
                            assert!(c.image[k][0] <= x && x <= c.image[k][1]);
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn both_signed_contact_sides_and_rigid_frames_are_admitted() {
        for da in [-0.2, 0.2] {
            for db in [-0.2, 0.2] {
                for rotated in [false, true] {
                    let [mut a, mut b] = pair();
                    for row in &mut b.control_points {
                        for p in row {
                            p[2] = 2. * p[2] - 1.;
                        }
                    }
                    if rotated {
                        for s in [&mut a, &mut b] {
                            for row in &mut s.control_points {
                                for p in row {
                                    *p = vec![p[2] + 17., p[0] - 9., p[1] + 23.];
                                }
                            }
                        }
                    }
                    let av = 0.5 - db;
                    let bv = (da + 1.) / 2.;
                    let r = certify(
                        [&a, &b],
                        [da, db],
                        0,
                        [0.35, 0.39],
                        [av - 0.05, av + 0.05],
                        [[0.30, 0.44], [bv - 0.05, bv + 0.05]],
                        2,
                        255,
                    )
                    .unwrap();
                    assert!(r.regular, "{da} {db} {rotated}: {}", r.reason);
                    assert!(r.visited <= 255);
                    for c in &r.cells {
                        assert!(c.area_speed[0] > 0.);
                    }
                }
            }
        }
    }
    #[test]
    fn malformed_radius_and_budget_do_not_enter_envelope() {
        let [a, b] = pair();
        for distances in [[0., 0.], [0.2, 0.3], [f64::NAN, 0.2]] {
            assert!(
                certify(
                    [&a, &b],
                    distances,
                    0,
                    [0.35, 0.39],
                    [0.25, 0.35],
                    [[0.30, 0.44], [0.15, 0.25]],
                    2,
                    1
                )
                .is_err()
            );
        }
        assert!(
            certify(
                [&a, &b],
                [0.2, 0.2],
                0,
                [0.35, 0.39],
                [0.25, 0.35],
                [[0.30, 0.44], [0.15, 0.25]],
                2,
                0
            )
            .is_err()
        );
    }
}
