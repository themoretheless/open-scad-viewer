use super::*;
use crate::surface_shape_operator::FlatPoleCertificate;
use polynomial::{Homogeneous, Pair};
fn bezier(knots: &[f64], degree: usize, n: usize) -> bool {
    n == degree + 1
        && knots[..=degree].iter().all(|&x| x == knots[degree])
        && knots[n..].iter().all(|&x| x == knots[n])
}
fn linear(pc: &Curve) -> Result<()> {
    pc.validate()?;
    check(
        pc.degree == 1
            && !pc.periodic
            && pc.control_points.len() == 2
            && pc.control_points[0].len() == 2,
        "Extrusion seam needs one original linear rational pcurve",
    )
}
pub(super) struct Segment {
    pub pair: Pair,
    pub range: [f64; 2],
    pub ends: [I; 2],
}
pub(super) struct Prepared<'a> {
    pub segments: Vec<Segment>,
    pub source: source::Source<'a>,
    pub candidate_sign: i8,
}
fn endpoint(
    s: &Surface,
    span: usize,
    row: usize,
    end: bool,
    axis: usize,
    sv: i8,
    cert: Option<&FlatPoleCertificate<'_>>,
) -> Result<()> {
    let cert = cert
        .ok_or_else(|| crate::input("Collapsed exterior endpoint needs original pole ownership"))?;
    check(
        std::ptr::eq(cert.source(), s) && cert.is_end() == end,
        "Pole certificate belongs to another original surface/end",
    )?;
    let p = s.degree_u;
    let i = if end { span } else { span - p };
    let neighbor = if end { i - 1 } else { i + 1 };
    let a = &s.control_points[i][row];
    let b = &s.control_points[neighbor][row];
    check(
        a.as_slice() == cert.pole().as_slice(),
        "Extrusion endpoint differs from owned pole",
    )?;
    let axes: Vec<_> = (0..3).filter(|&k| k != axis && a[k] != b[k]).collect();
    check(
        axes.len() == 1,
        "Pole extrusion profile needs a literal nonzero axial tangent",
    )?;
    let k = axes[0];
    let tangent = if (b[k] > a[k]) != end { 1. } else { -1. };
    let mut normal = [0.; 3];
    let n = 3 - axis - k;
    let permutation = if (k + 1) % 3 == axis { 1. } else { -1. };
    normal[n] = permutation * tangent * sv as f64;
    check(
        normal == cert.normal(),
        "Extrusion profile does not approach owned pole normal",
    )
}
impl<'a> Prepared<'a> {
    pub fn new(
        s: [&'a Surface; 2],
        pc: [&Curve; 2],
        axis: usize,
        poles: [Option<&FlatPoleCertificate<'_>>; 2],
    ) -> Result<Self> {
        check(axis < 3, "Invalid original extrusion world axis")?;
        for surface in s {
            surface.validate()?;
            check(
                !surface.periodic_u && !surface.periodic_v,
                "Extrusion proof needs original nonperiodic charts",
            )?;
        }
        for p in pc {
            linear(p)?;
        }
        let source = s[0];
        let candidate = s[1];
        let (p, q, n, m) = (
            candidate.degree_u,
            candidate.degree_v,
            candidate.control_points.len(),
            candidate.control_points[0].len(),
        );
        check(
            (2..=16).contains(&p)
                && (2..=16).contains(&q)
                && bezier(&candidate.knots_v, q, m)
                && pc[1].weights[0] == pc[1].weights[1],
            "Candidate extrusion needs original C0 U by Bernstein V and an affine U clock",
        )?;
        let spans: Vec<_> = (p..n)
            .filter(|&i| candidate.knots_u[i] < candidate.knots_u[i + 1])
            .collect();
        check(
            candidate.knots_u[..=p]
                .iter()
                .all(|&u| u == candidate.knots_u[p])
                && candidate.knots_u[n..]
                    .iter()
                    .all(|&u| u == candidate.knots_u[n])
                && spans.iter().skip(1).all(|&i| {
                    candidate
                        .knots_u
                        .iter()
                        .filter(|&&u| u == candidate.knots_u[i])
                        .count()
                        == p
                }),
            "Candidate U controls must be literal original Bernstein spans",
        )?;
        let v = pc[1].control_points[0][1];
        check(
            pc[1].control_points[1][1] == v
                && (v == candidate.knots_v[q] || v == candidate.knots_v[m]),
            "Candidate pcurve must be its actual V boundary",
        )?;
        let end = v == candidate.knots_v[m];
        let row = if end { q } else { 0 };
        let next = if end { q - 1 } else { 1 };
        let second = if end { q - 2 } else { 2 };
        let u0 = pc[1].control_points[0][0];
        let u1 = pc[1].control_points[1][0];
        check(
            u0 != u1 && u0.min(u1) >= candidate.knots_u[p] && u0.max(u1) <= candidate.knots_u[n],
            "Candidate extrusion clock leaves its original domain",
        )?;
        let origin: [f64; 3] = std::array::from_fn(|k| candidate.control_points[0][row][k]);
        let source_proof = source::Source::new(source, pc[0], origin)?;
        let composed = source_proof.trace.clone();
        let mut segments = Vec::new();
        let mut candidate_sign = None;
        for span in spans {
            let range = [
                candidate.knots_u[span].max(u0.min(u1)),
                candidate.knots_u[span + 1].min(u0.max(u1)),
            ];
            if range[0] >= range[1] {
                continue;
            }
            let mut positive = false;
            let mut negative = false;
            for i in span - p..=span {
                let a = &candidate.control_points[i][row];
                for j in [next, second] {
                    check(
                        candidate.weights[i][j] == candidate.weights[i][row]
                            && (0..3)
                                .all(|k| k == axis || candidate.control_points[i][j][k] == a[k]),
                        "Candidate boundary is not a literal axial 2-jet",
                    )?;
                }
                let b = &candidate.control_points[i][next];
                positive |= if end {
                    b[axis] < a[axis]
                } else {
                    b[axis] > a[axis]
                };
                negative |= if end {
                    b[axis] > a[axis]
                } else {
                    b[axis] < a[axis]
                };
            }
            check(
                positive != negative,
                "Candidate extrusion transverse coefficients do not have one strict interior sign",
            )?;
            let sv = if positive { 1 } else { -1 };
            check(
                candidate_sign.is_none_or(|sign| sign == sv),
                "Candidate extrusion orientation changes between original spans",
            )?;
            candidate_sign = Some(sv);
            for e in [false, true] {
                let i = if e { span } else { span - p };
                if candidate.control_points[i][row][axis] == candidate.control_points[i][next][axis]
                {
                    let global_end = if e { span == n - 1 } else { span == p };
                    check(
                        global_end,
                        "Candidate extrusion has an unowned singular internal endpoint",
                    )?;
                    endpoint(candidate, span, row, e, axis, sv, poles[usize::from(e)])?;
                }
            }
            let mut ch: Homogeneous = std::array::from_fn(|_| Vec::new());
            for i in span - p..=span {
                let w = I::point(candidate.weights[i][row]);
                for k in 0..3 {
                    ch[k].push(if k == axis {
                        I::point(0.)
                    } else {
                        I::point(candidate.control_points[i][row][k])
                            .sub(I::point(origin[k]))?
                            .mul(w)?
                    });
                }
                ch[3].push(w);
            }
            let actual = [candidate.knots_u[span], candidate.knots_u[span + 1]];
            let ends = [0, 1].map(|i| {
                I::point(actual[i])
                    .sub(I::point(u0))?
                    .div_signed(I::point(u1).sub(I::point(u0))?)?
                    .intersect(0., 1.)
            });
            // Each selected original span lies wholly in this seam. Refuse a
            // partial clock rather than extrapolating the projected profile.
            check(
                actual == range,
                "Extrusion seam must own whole original boundary spans",
            )?;
            let ends = [ends[0].clone()?, ends[1].clone()?];
            let mut sh = polynomial::restrict(&composed, ends)?;
            sh[axis].fill(I::point(0.));
            let degree = p.max(composed[0].len() - 1);
            let pair = Pair::new(
                polynomial::elevate(sh, degree)?,
                polynomial::elevate(ch, degree)?,
            )?;
            segments.push(Segment {
                pair,
                range: [0., 1.],
                ends,
            });
        }
        check(!segments.is_empty(), "No original extrusion boundary spans")?;
        Ok(Self {
            segments,
            source: source_proof,
            candidate_sign: candidate_sign.unwrap(),
        })
    }
}
