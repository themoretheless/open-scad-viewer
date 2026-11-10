//! Exact Booleans of meridian sections of coaxial solids of revolution.
//!
//! A section lives in the (radius, axial) half-plane and is one of the
//! primitive convex shapes of the canonical solids: a polygon with a side on
//! the axis (cylinder, frustum, cone), a disk off the axis (torus) or a half
//! disk on the axis (sphere). Boundaries are line segments and circular arcs.
//!
//! The Boolean splits both boundaries at their mutual intersections (closed
//! forms for line/line, line/circle and circle/circle), classifies every piece
//! against the other section with that section's own analytic inside test,
//! keeps the pieces the operation selects, and chains them into loops with the
//! material on the left. Pieces lying on the other boundary are kept once when
//! the materials agree (union, intersection) or disagree (difference). Arcs
//! leave as rational quadratic spans of at most 90 degrees, so revolving the
//! loops reproduces exact cylinder, cone, sphere, torus and disk patches.
use crate::sphere_mate::unsupported;
use nurbs_core::{Result, curve::Curve};

type P = [f64; 2];
const TAU: f64 = std::f64::consts::TAU;

fn sub(a: P, b: P) -> P {
    [a[0] - b[0], a[1] - b[1]]
}
fn dot(a: P, b: P) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}
fn cross(a: P, b: P) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}
fn len(a: P) -> f64 {
    a[0].hypot(a[1])
}

/// A boundary span: a segment, or an arc of signed sweep (counter-clockwise
/// positive) starting at angle `start` on the circle `(center, radius)`.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Span {
    Line(P, P),
    Arc {
        center: P,
        radius: f64,
        start: f64,
        sweep: f64,
    },
}

impl Span {
    fn at(&self, t: f64) -> P {
        match *self {
            Span::Line(a, b) => [a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])],
            Span::Arc {
                center,
                radius,
                start,
                sweep,
            } => {
                let (s, c) = (start + t * sweep).sin_cos();
                [center[0] + radius * c, center[1] + radius * s]
            }
        }
    }
    /// Unit tangent along the span direction at parameter `t`.
    fn tangent(&self, t: f64) -> P {
        match *self {
            Span::Line(a, b) => {
                let d = sub(b, a);
                let l = len(d);
                [d[0] / l, d[1] / l]
            }
            Span::Arc { start, sweep, .. } => {
                let (s, c) = (start + t * sweep).sin_cos();
                let sign = sweep.signum();
                [-s * sign, c * sign]
            }
        }
    }
    fn piece(&self, t0: f64, t1: f64) -> Span {
        match *self {
            Span::Line(..) => Span::Line(self.at(t0), self.at(t1)),
            Span::Arc {
                center,
                radius,
                start,
                sweep,
            } => Span::Arc {
                center,
                radius,
                start: start + t0 * sweep,
                sweep: (t1 - t0) * sweep,
            },
        }
    }
    fn reversed(&self) -> Span {
        match *self {
            Span::Line(a, b) => Span::Line(b, a),
            Span::Arc {
                center,
                radius,
                start,
                sweep,
            } => Span::Arc {
                center,
                radius,
                start: start + sweep,
                sweep: -sweep,
            },
        }
    }
    /// Parameter of a point known to lie on the span (within rounding).
    fn parameter_of(&self, p: P) -> f64 {
        match *self {
            Span::Line(a, b) => {
                let d = sub(b, a);
                dot(sub(p, a), d) / dot(d, d)
            }
            Span::Arc {
                center,
                start,
                sweep,
                ..
            } => {
                let angle = (p[1] - center[1]).atan2(p[0] - center[0]);
                let mut delta = (angle - start) * sweep.signum();
                delta = delta.rem_euclid(TAU);
                // Near a full turn, a point at the start is parameter 0.
                if delta > sweep.abs() + 1e-9 && TAU - delta < 1e-9 {
                    delta -= TAU;
                }
                delta / sweep.abs()
            }
        }
    }
}

/// A primitive section with an exact inside test.
#[derive(Clone, Debug)]
pub(crate) enum Section {
    /// Convex polygon, counter-clockwise.
    Polygon(Vec<P>),
    /// Disk off the axis (a torus).
    Disk { center: P, radius: f64 },
    /// Half disk `rho >= 0` of a disk centred on the axis (a sphere).
    HalfDisk { center_z: f64, radius: f64 },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Side {
    In,
    Out,
    On,
}

impl Section {
    fn boundary(&self) -> Vec<Span> {
        match self {
            Section::Polygon(ring) => (0..ring.len())
                .map(|i| Span::Line(ring[i], ring[(i + 1) % ring.len()]))
                .collect(),
            Section::Disk { center, radius } => vec![Span::Arc {
                center: *center,
                radius: *radius,
                start: 0.,
                sweep: TAU,
            }],
            Section::HalfDisk { center_z, radius } => vec![
                Span::Arc {
                    center: [0., *center_z],
                    radius: *radius,
                    start: -std::f64::consts::FRAC_PI_2,
                    sweep: std::f64::consts::PI,
                },
                Span::Line([0., center_z + radius], [0., center_z - radius]),
            ],
        }
    }

    /// Signed distance-like measure: negative inside, positive outside.
    fn measure(&self, p: P) -> f64 {
        match self {
            Section::Polygon(ring) => (0..ring.len())
                .map(|i| {
                    let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
                    let d = sub(b, a);
                    -cross(d, sub(p, a)) / len(d)
                })
                .fold(f64::NEG_INFINITY, f64::max),
            Section::Disk { center, radius } => len(sub(p, *center)) - radius,
            Section::HalfDisk { center_z, radius } => {
                (len([p[0], p[1] - center_z]) - radius).max(-p[0])
            }
        }
    }

    fn side(&self, p: P, band: f64) -> Side {
        let m = self.measure(p);
        if m < -band {
            Side::In
        } else if m > band {
            Side::Out
        } else {
            Side::On
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Op {
    Union,
    Intersection,
    Difference,
}

/// Parameters on `a` and `b` of their intersection points, and of the
/// endpoints of each lying on the other (overlaps of collinear segments or of
/// arcs on one circle).
fn crossings(a: &Span, b: &Span, band: f64) -> (Vec<f64>, Vec<f64>) {
    let mut ta = Vec::new();
    let mut tb = Vec::new();
    let mut add = |p: P, ta: &mut Vec<f64>, tb: &mut Vec<f64>| {
        let (u, v) = (a.parameter_of(p), b.parameter_of(p));
        let (ea, eb) = (band / span_length(a), band / span_length(b));
        if (-ea..=1. + ea).contains(&u)
            && (-eb..=1. + eb).contains(&v)
            && len(sub(a.at(u.clamp(0., 1.)), p)) <= 4. * band
            && len(sub(b.at(v.clamp(0., 1.)), p)) <= 4. * band
        {
            ta.push(u.clamp(0., 1.));
            tb.push(v.clamp(0., 1.));
        }
    };
    let mut points = Vec::new();
    match (*a, *b) {
        (Span::Line(p0, p1), Span::Line(q0, q1)) => {
            let (r, s) = (sub(p1, p0), sub(q1, q0));
            let denominator = cross(r, s);
            if denominator.abs() > 1e-12 * len(r) * len(s) {
                let t = cross(sub(q0, p0), s) / denominator;
                points.push(a.at(t));
            } else {
                // Parallel: endpoints of each on the other (collinear overlap).
                points.extend([p0, p1, q0, q1]);
            }
        }
        (Span::Line(..), Span::Arc { center, radius, .. })
        | (Span::Arc { center, radius, .. }, Span::Line(..)) => {
            let line = if let Span::Line(..) = a { a } else { b };
            let Span::Line(p0, p1) = *line else {
                unreachable!()
            };
            let d = sub(p1, p0);
            let f = sub(p0, center);
            let (qa, qb, qc) = (dot(d, d), 2. * dot(f, d), dot(f, f) - radius * radius);
            let disc = qb * qb - 4. * qa * qc;
            if disc >= -1e-12 * qb * qb {
                let root = disc.max(0.).sqrt();
                for t in [(-qb - root) / (2. * qa), (-qb + root) / (2. * qa)] {
                    points.push(line.at(t));
                }
            }
        }
        (
            Span::Arc {
                center: c0,
                radius: r0,
                ..
            },
            Span::Arc {
                center: c1,
                radius: r1,
                ..
            },
        ) => {
            let d = len(sub(c1, c0));
            if d <= band && (r0 - r1).abs() <= band {
                // One circle: endpoints of each arc on the other.
                points.extend([a.at(0.), a.at(1.), b.at(0.), b.at(1.)]);
            } else if d > 0. && d <= r0 + r1 + band && d + band >= (r0 - r1).abs() {
                let along = (d * d + r0 * r0 - r1 * r1) / (2. * d);
                let h = (r0 * r0 - along * along).max(0.).sqrt();
                let u = [(c1[0] - c0[0]) / d, (c1[1] - c0[1]) / d];
                let base = [c0[0] + along * u[0], c0[1] + along * u[1]];
                points.push([base[0] - h * u[1], base[1] + h * u[0]]);
                points.push([base[0] + h * u[1], base[1] - h * u[0]]);
            }
        }
    }
    for p in points {
        add(p, &mut ta, &mut tb);
    }
    (ta, tb)
}

fn span_length(s: &Span) -> f64 {
    match *s {
        Span::Line(a, b) => len(sub(b, a)).max(1e-300),
        Span::Arc { radius, sweep, .. } => (radius * sweep.abs()).max(1e-300),
    }
}

/// Split every span of `spans` at the given sorted parameters.
fn split(spans: &[Span], cuts: &[Vec<f64>], band: f64) -> Vec<Span> {
    let mut out = Vec::new();
    for (span, cut) in spans.iter().zip(cuts) {
        let mut ts: Vec<f64> = cut.clone();
        ts.extend([0., 1.]);
        ts.sort_by(f64::total_cmp);
        let eps = band / span_length(span);
        ts.dedup_by(|x, y| (*x - *y).abs() <= eps);
        for w in ts.windows(2) {
            if w[1] - w[0] > eps {
                out.push(span.piece(w[0], w[1]));
            }
        }
    }
    out
}

/// Boolean of two sections; returns loops of spans with material on the left.
pub(crate) fn boolean(a: &Section, b: &Section, op: Op, band: f64) -> Result<Vec<Vec<Span>>> {
    let (sa, sb) = (a.boundary(), b.boundary());
    let mut cuts_a = vec![Vec::new(); sa.len()];
    let mut cuts_b = vec![Vec::new(); sb.len()];
    for (i, x) in sa.iter().enumerate() {
        for (j, y) in sb.iter().enumerate() {
            let (ta, tb) = crossings(x, y, band);
            cuts_a[i].extend(ta);
            cuts_b[j].extend(tb);
        }
    }
    let pieces_a = split(&sa, &cuts_a, band);
    let pieces_b = split(&sb, &cuts_b, band);
    let mut kept: Vec<Span> = Vec::new();
    // Classify a piece by its midpoint; an `On` piece by a point just to its
    // material (left) side.
    let classify = |piece: &Span, other: &Section| -> Result<(Side, bool)> {
        let mid = piece.at(0.5);
        let side = other.side(mid, band);
        if side != Side::On {
            return Ok((side, false));
        }
        let t = piece.tangent(0.5);
        let step = 64. * band;
        let left = [mid[0] - t[1] * step, mid[1] + t[0] * step];
        Ok((Side::On, other.side(left, band) == Side::In))
    };
    for piece in &pieces_a {
        let (side, same) = classify(piece, b)?;
        let keep = match op {
            Op::Union => side == Side::Out || (side == Side::On && same),
            Op::Intersection => side == Side::In || (side == Side::On && same),
            Op::Difference => side == Side::Out || (side == Side::On && !same),
        };
        if keep {
            kept.push(*piece);
        }
    }
    for piece in &pieces_b {
        let (side, _) = classify(piece, a)?;
        match op {
            Op::Union if side == Side::Out => kept.push(*piece),
            Op::Intersection if side == Side::In => kept.push(*piece),
            Op::Difference if side == Side::In => kept.push(piece.reversed()),
            _ => {}
        }
    }
    chain(kept, band)
}

/// Chain pieces end to start into closed loops.
fn chain(mut pieces: Vec<Span>, band: f64) -> Result<Vec<Vec<Span>>> {
    let close = |p: P, q: P| len(sub(p, q)) <= 16. * band;
    let mut loops = Vec::new();
    while let Some(first) = pieces.pop() {
        let start = first.at(0.);
        let mut current = vec![first];
        loop {
            let end = current.last().unwrap().at(1.);
            if close(end, start) {
                break;
            }
            let Some(index) = pieces.iter().position(|p| close(p.at(0.), end)) else {
                return Err(unsupported(
                    "Revolution section Boolean: boundary pieces do not close into loops",
                ));
            };
            current.push(pieces.swap_remove(index));
        }
        loops.push(current);
    }
    Ok(loops)
}

/// Exact NURBS curves of a loop: lines, and arcs as rational quadratic spans
/// of at most 90 degrees. Coordinates within `snap` of the axis become 0.
pub(crate) fn curves(spans: &[Span], snap: f64) -> Vec<Curve> {
    let fix = |p: P| -> Vec<f64> { vec![if p[0].abs() <= snap { 0. } else { p[0] }, p[1]] };
    // Revolution needs at least three spans per loop: a lens of two arcs, or
    // an arc closed by one segment, splits each arc at least once.
    let quarters = |sweep: f64| (sweep.abs() / std::f64::consts::FRAC_PI_2 - 1e-9).ceil().max(1.) as usize;
    let natural: usize = spans
        .iter()
        .map(|s| match *s {
            Span::Line(..) => 1,
            Span::Arc { sweep, .. } => quarters(sweep),
        })
        .sum();
    let minimum_pieces = if natural < 3 { 2 } else { 1 };
    let mut out = Vec::new();
    for span in spans {
        match *span {
            Span::Line(a, b) => out.push(Curve {
                degree: 1,
                knots: vec![0., 0., 1., 1.],
                control_points: vec![fix(a), fix(b)],
                weights: vec![1., 1.],
                periodic: false,
            }),
            Span::Arc { sweep, .. } => {
                let count = quarters(sweep).max(minimum_pieces);
                for k in 0..count {
                    let piece = span.piece(k as f64 / count as f64, (k + 1) as f64 / count as f64);
                    let Span::Arc {
                        center,
                        radius,
                        start,
                        sweep,
                    } = piece
                    else {
                        unreachable!()
                    };
                    let half = sweep / 2.;
                    let (p0, p2) = (piece.at(0.), piece.at(1.));
                    let mid = start + half;
                    let reach = radius / half.cos();
                    let p1 = [center[0] + reach * mid.cos(), center[1] + reach * mid.sin()];
                    out.push(Curve {
                        degree: 2,
                        knots: vec![0., 0., 0., 1., 1., 1.],
                        control_points: vec![fix(p0), fix(p1), fix(p2)],
                        weights: vec![1., half.cos(), 1.],
                        periodic: false,
                    });
                }
            }
        }
    }
    out
}

/// Signed area of a loop (positive for counter-clockwise).
pub(crate) fn area(spans: &[Span]) -> f64 {
    spans
        .iter()
        .map(|span| match *span {
            Span::Line(a, b) => 0.5 * cross(a, b),
            Span::Arc {
                center,
                radius,
                start,
                sweep,
            } => {
                // Green's theorem on x = cx + r cos, y = cy + r sin.
                let (t0, t1) = (start, start + sweep);
                0.5 * (radius * center[0] * (t1.sin() - t0.sin())
                    - radius * center[1] * (t1.cos() - t0.cos())
                    + radius * radius * sweep)
            }
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    const PI: f64 = std::f64::consts::PI;

    fn total(loops: &[Vec<Span>]) -> f64 {
        loops.iter().map(|l| area(l)).sum()
    }

    #[test]
    fn rectangle_minus_disk_and_disk_union() {
        let rect = Section::Polygon(vec![[0., 0.], [4., 0.], [4., 6.], [0., 6.]]);
        let disk = Section::Disk {
            center: [4., 3.],
            radius: 1.,
        };
        let band = 1e-9;
        // The disk straddles the right side: half inside, half outside.
        let diff = boolean(&rect, &disk, Op::Difference, band).unwrap();
        assert!((total(&diff) - (24. - PI / 2.)).abs() < 1e-9, "{}", total(&diff));
        let union = boolean(&rect, &disk, Op::Union, band).unwrap();
        assert!((total(&union) - (24. + PI / 2.)).abs() < 1e-9);
        let inter = boolean(&rect, &disk, Op::Intersection, band).unwrap();
        assert!((total(&inter) - PI / 2.).abs() < 1e-9);
        // A disk fully inside leaves a hole: two loops, inner clockwise.
        let inner = Section::Disk {
            center: [2., 3.],
            radius: 1.,
        };
        let holed = boolean(&rect, &inner, Op::Difference, band).unwrap();
        assert_eq!(holed.len(), 2);
        assert!((total(&holed) - (24. - PI)).abs() < 1e-9);
    }

    #[test]
    fn half_disk_with_triangle_and_shared_axis_side() {
        // Sphere section of radius 2 about z = 0 and a cone triangle that
        // shares the axis segment from z = -2 to z = 2.
        let half = Section::HalfDisk {
            center_z: 0.,
            radius: 2.,
        };
        let tri = Section::Polygon(vec![[0., -2.], [3., -2.], [0., 2.]]);
        let band = 1e-9;
        let union = boolean(&half, &tri, Op::Union, band).unwrap();
        let inter = boolean(&half, &tri, Op::Intersection, band).unwrap();
        let diff = boolean(&half, &tri, Op::Difference, band).unwrap();
        let (ah, at) = (2. * PI, 6.);
        let (u, i, d) = (total(&union), total(&inter), total(&diff));
        assert!((u + i - ah - at).abs() < 1e-9, "{u} {i}");
        assert!((d + i - ah).abs() < 1e-9, "{d} {i}");
        assert_eq!(union.len(), 1);
    }

    #[test]
    fn two_disks_and_disjoint_pairs() {
        let a = Section::Disk {
            center: [3., 0.],
            radius: 1.,
        };
        let b = Section::Disk {
            center: [3., 1.],
            radius: 1.,
        };
        let band = 1e-9;
        // Lens area of two unit circles one apart.
        let lens = 2. * (0.5f64).acos() - 0.5 * 3f64.sqrt();
        let inter = boolean(&a, &b, Op::Intersection, band).unwrap();
        assert!((total(&inter) - lens).abs() < 1e-9);
        let union = boolean(&a, &b, Op::Union, band).unwrap();
        assert!((total(&union) - (2. * PI - lens)).abs() < 1e-9);
        let far = Section::Disk {
            center: [10., 0.],
            radius: 1.,
        };
        assert!(boolean(&a, &far, Op::Intersection, band).unwrap().is_empty());
        assert_eq!(boolean(&a, &far, Op::Union, band).unwrap().len(), 2);
    }

    #[test]
    fn arcs_become_exact_quarter_spans() {
        let disk = Section::Disk {
            center: [3., 0.],
            radius: 1.,
        };
        let loops = boolean(&disk, &disk, Op::Union, 1e-9).unwrap();
        let curves = curves(&loops[0], 1e-9);
        assert_eq!(curves.len(), 4);
        for curve in &curves {
            for k in 0..=8 {
                let p = curve.evaluate(k as f64 / 8.).unwrap().point;
                assert!(((p[0] - 3.).hypot(p[1]) - 1.).abs() < 1e-12);
            }
        }
    }
}
