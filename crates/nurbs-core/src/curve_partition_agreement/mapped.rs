//! Complete original-to-actual affine clock comparison on exact rational cells.
use super::*;
use crate::DistanceStopReason;
use num_rational::BigRational as Q;
use num_traits::{One, ToPrimitive, Zero};
use std::{cmp::Ordering, collections::BinaryHeap};
#[derive(Clone, Debug)]
pub struct Report {
    pub error_upper: f64,
    pub accepted: bool,
    pub cells: usize,
    pub reason: DistanceStopReason,
}
fn q(x: f64) -> Q {
    Q::from_float(x).unwrap()
}
fn enclosure(x: &Q) -> Result<I> {
    let v = x
        .to_f64()
        .ok_or_else(|| crate::numeric_err("Mapped parameter overflow"))?;
    check(v.is_finite(), "Mapped parameter overflow")?;
    let r = q(v);
    I::new(
        if r > *x { v.next_down() } else { v },
        if r < *x { v.next_up() } else { v },
    )
}
struct Clock {
    start: Q,
    end: Q,
    wrap: Option<Q>,
}
impl Clock {
    fn raw(&self, t: &Q) -> Q {
        &self.start + t * (&self.end - &self.start)
    }
    fn ends(&self, lo: &Q, hi: &Q, c: &Curve) -> [Q; 2] {
        let mid = (lo + hi) / Q::from_integer(2.into());
        let shift = if let Some(period) = &self.wrap {
            if self.raw(&mid) > q(c.domain()[1]) {
                period.clone()
            } else {
                Q::zero()
            }
        } else {
            Q::zero()
        };
        [self.raw(lo) - &shift, self.raw(hi) - shift]
    }
    fn cuts(&self, c: &Curve, out: &mut Vec<Q>) {
        let d = c.domain();
        for &k in &c.knots {
            if k < d[0] || k > d[1] {
                continue;
            }
            let mut knot = q(k);
            if let Some(period) = &self.wrap {
                if knot < self.start {
                    knot += period;
                }
            }
            let t = (knot - &self.start) / (&self.end - &self.start);
            if t > Q::zero() && t < Q::one() {
                out.push(t);
            }
        }
    }
}
struct Cell {
    lo: Q,
    hi: Q,
    upper: f64,
    depth: usize,
}
impl PartialEq for Cell {
    fn eq(&self, b: &Self) -> bool {
        self.upper == b.upper
    }
}
impl Eq for Cell {}
impl PartialOrd for Cell {
    fn partial_cmp(&self, b: &Self) -> Option<Ordering> {
        Some(self.cmp(b))
    }
}
impl Ord for Cell {
    fn cmp(&self, b: &Self) -> Ordering {
        self.upper.total_cmp(&b.upper)
    }
}
struct Comparison<'a> {
    a: &'a Curve,
    b: &'a Curve,
    ac: Clock,
    bc: Clock,
}
impl Comparison<'_> {
    fn cell(&self, lo: Q, hi: Q, depth: usize) -> Result<Cell> {
        let ad = self.ac.ends(&lo, &hi, self.a);
        let bd = self.bc.ends(&lo, &hi, self.b);
        let span = |c: &Curve, d: &[Q; 2]| {
            (c.degree..c.control_points.len()).find(|&i| {
                c.knots[i] < c.knots[i + 1]
                    && q(c.knots[i]) <= *d.iter().min().unwrap()
                    && *d.iter().max().unwrap() <= q(c.knots[i + 1])
            })
        };
        let sa =
            span(self.a, &ad).ok_or_else(|| crate::input("Mapped cell has no original span"))?;
        let sb =
            span(self.b, &bd).ok_or_else(|| crate::input("Mapped cell has no candidate span"))?;
        let upper = super::correlated::upper_between(
            self.a,
            sa,
            [enclosure(&ad[0])?, enclosure(&ad[1])?],
            self.b,
            sb,
            [enclosure(&bd[0])?, enclosure(&bd[1])?],
        )?
        .unwrap_or(f64::INFINITY);
        Ok(Cell {
            lo,
            hi,
            upper,
            depth,
        })
    }
    fn verify(self, tolerance: f64, max_cells: usize) -> Result<Option<Report>> {
        let mut cuts = vec![Q::zero(), Q::one()];
        self.ac.cuts(self.a, &mut cuts);
        self.bc.cuts(self.b, &mut cuts);
        cuts.sort();
        cuts.dedup();
        if cuts.len() - 1 > max_cells {
            return Ok(None);
        }
        let mut heap = BinaryHeap::new();
        for w in cuts.windows(2) {
            heap.push(self.cell(w[0].clone(), w[1].clone(), 0)?);
        }
        let mut work = heap.len();
        loop {
            let c = heap.pop().unwrap();
            if c.upper <= tolerance || work + 2 > max_cells || c.depth == 64 {
                let accepted = c.upper <= tolerance;
                return Ok(Some(Report {
                    error_upper: c.upper,
                    accepted,
                    cells: work,
                    reason: if accepted {
                        DistanceStopReason::Tolerance
                    } else if c.depth == 64 {
                        DistanceStopReason::PrecisionLimit
                    } else {
                        DistanceStopReason::WorkLimit
                    },
                }));
            }
            let mid = (&c.lo + &c.hi) / Q::from_integer(2.into());
            heap.push(self.cell(c.lo, mid.clone(), c.depth + 1)?);
            heap.push(self.cell(mid, c.hi, c.depth + 1)?);
            work += 2;
        }
    }
}
/// Bound the complete correspondence between independently authored physical
/// clocks. Descending endpoints reverse traversal; original definitions stay
/// unchanged. Refusal is not proof that the tolerance was exceeded.
pub fn verify(
    a: &Curve,
    b: &Curve,
    source_parameters: [f64; 2],
    candidate_parameters: [f64; 2],
    tolerance: f64,
    max_cells: usize,
) -> Result<Report> {
    a.validate()?;
    b.validate()?;
    check(
        a.control_points[0].len() == b.control_points[0].len() && continuous(a) && continuous(b),
        "Mapped comparison needs matching dimensions and continuous original definitions",
    )?;
    for (c, d) in [(a, source_parameters), (b, candidate_parameters)] {
        let range = c.domain();
        check(
            d[0] != d[1]
                && d.iter()
                    .all(|v| v.is_finite() && *v >= range[0] && *v <= range[1]),
            "Mapped clocks must be nonempty original physical subranges",
        )?;
    }
    check(
        tolerance.is_finite() && tolerance > 0. && (1..=100000).contains(&max_cells),
        "Mapped comparison needs bounded positive tolerance and work",
    )?;
    let ac = Clock {
        start: q(source_parameters[0]),
        end: q(source_parameters[1]),
        wrap: None,
    };
    let bc = Clock {
        start: q(candidate_parameters[0]),
        end: q(candidate_parameters[1]),
        wrap: None,
    };
    Ok(Comparison { a, b, ac, bc }
        .verify(tolerance, max_cells)?
        .unwrap_or(Report {
            error_upper: f64::INFINITY,
            accepted: false,
            cells: 0,
            reason: DistanceStopReason::WorkLimit,
        }))
}
pub(crate) fn wrapped(
    a: &Curve,
    b: &Curve,
    tolerance: f64,
    max_cells: usize,
) -> Result<Option<Report>> {
    let start = q(b.domain()[0]);
    let period = q(a.domain()[1]) - q(a.domain()[0]);
    let ac = Clock {
        start: start.clone(),
        end: &start + &period,
        wrap: Some(period),
    };
    let bc = Clock {
        start,
        end: q(b.domain()[1]),
        wrap: None,
    };
    Comparison { a, b, ac, bc }.verify(tolerance, max_cells)
}
