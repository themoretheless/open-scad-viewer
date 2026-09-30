//! Outward interval diagnostics of the represented offset chord chain.
//! These predicates concern the returned binary64 line definitions, without
//! promoting construction coordinates into an authored source certificate.
use crate::{Result, check, curve_offset::Segment, distance_bounds::Interval};
use value_codec::{Value, json};

pub struct Diagnostics {
    pub crossings: Vec<[usize; 2]>,
    pub contacts: Vec<[usize; 2]>,
    pub uncertain: Vec<[usize; 2]>,
    pub degenerate: Vec<usize>,
    pub complete: bool,
    pub checks: usize,
    pub total_pairs: usize,
}
impl Diagnostics {
    pub fn to_value(&self) -> Value {
        json!({"scope":"represented-offset-chain","method":"outward-line-pair-interval/1",
            "crossings":self.crossings,"contacts":self.contacts,"uncertain":self.uncertain,
            "degenerate":self.degenerate,"complete":self.complete,"checks":self.checks,
            "enumerationComplete":self.checks==self.total_pairs,
            "predicatesComplete":self.uncertain.is_empty(),
            "totalPairs":self.total_pairs,"simple":self.complete && self.crossings.is_empty()
                && self.contacts.is_empty() && self.degenerate.is_empty(),
            "originalOffsetTopologyCertified":false})
    }
}

fn orientation(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> Result<Option<i8>> {
    if (a[0] == b[0] && b[0] == c[0]) || (a[1] == b[1] && b[1] == c[1]) {
        return Ok(Some(0));
    }
    let x = |p: [f64; 2], axis| Interval::point(p[axis]).sub(Interval::point(a[axis]));
    let cross = x(b, 0)?.mul(x(c, 1)?)?.sub(x(b, 1)?.mul(x(c, 0)?)?)?;
    Ok(if cross.lo > 0. {
        Some(1)
    } else if cross.hi < 0. {
        Some(-1)
    } else {
        None
    })
}
fn apart(a: &Segment, b: &Segment) -> bool {
    (0..2).any(|k| {
        a.points[0][k].max(a.points[1][k]) < b.points[0][k].min(b.points[1][k])
            || b.points[0][k].max(b.points[1][k]) < a.points[0][k].min(a.points[1][k])
    })
}
fn shared_vertex_only(a: &Segment, b: &Segment) -> Result<bool> {
    for i in 0..2 {
        for j in 0..2 {
            if a.points[i] != b.points[j] {
                continue;
            }
            let common = a.points[i];
            let left = a.points[1 - i];
            let right = b.points[1 - j];
            if (0..2).any(|k| {
                (left[k] < common[k] && right[k] > common[k])
                    || (right[k] < common[k] && left[k] > common[k])
            }) {
                return Ok(true);
            }
            if let Some(sign) = orientation(common, left, right)? {
                if sign != 0 {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}

pub fn inspect_chain(segments: &[Segment], closed: bool, max_pairs: usize) -> Result<Diagnostics> {
    check(
        !segments.is_empty() && segments.len() <= 65536 && (1..=1_000_000).contains(&max_pairs),
        "Chain diagnostics require nonempty bounded segments and 1..1000000 pairs",
    )?;
    check(
        segments.iter().all(|s| {
            s.points
                .iter()
                .flatten()
                .all(|x| x.is_finite() && x.abs() <= 1e9)
        }),
        "Chain diagnostic points exceed coordinate bounds",
    )?;
    check(
        segments.iter().all(|s| {
            s.domain[0].is_finite() && s.domain[1].is_finite() && s.domain[0] < s.domain[1]
        }) && segments
            .windows(2)
            .all(|p| p[0].domain[1] == p[1].domain[0] && p[0].points[1] == p[1].points[0]),
        "Chain diagnostics require connected, ordered parameter cells",
    )?;
    check(
        !closed || segments[0].points[0] == segments[segments.len() - 1].points[1],
        "Closed chain diagnostic requires an exact closing point",
    )?;
    let n = segments.len();
    let mut report = Diagnostics {
        crossings: vec![],
        contacts: vec![],
        uncertain: vec![],
        degenerate: segments
            .iter()
            .enumerate()
            .filter(|(_, s)| s.points[0] == s.points[1])
            .map(|(i, _)| i)
            .collect(),
        complete: true,
        checks: 0,
        total_pairs: n * (n - 1) / 2,
    };
    for i in 0..n {
        for j in i + 1..n {
            if report.checks == max_pairs {
                report.complete = false;
                return Ok(report);
            }
            report.checks += 1;
            let (a, b) = (&segments[i], &segments[j]);
            if apart(a, b) {
                continue;
            }
            let adjacent = j == i + 1 || (closed && i == 0 && j == n - 1);
            if adjacent && shared_vertex_only(a, b)? {
                continue;
            }
            let shared = a.points.iter().any(|p| b.points.contains(p));
            if shared && !adjacent {
                report.contacts.push([i, j]);
                continue;
            }
            let signs = [
                orientation(a.points[0], a.points[1], b.points[0])?,
                orientation(a.points[0], a.points[1], b.points[1])?,
                orientation(b.points[0], b.points[1], a.points[0])?,
                orientation(b.points[0], b.points[1], a.points[1])?,
            ];
            if let [Some(x), Some(y), Some(z), Some(w)] = signs {
                if x * y < 0 && z * w < 0 {
                    report.crossings.push([i, j]);
                    continue;
                }
                if x * y > 0 || z * w > 0 {
                    continue;
                }
            }
            // An unresolved touching/overlap predicate cannot prove simplicity.
            report.uncertain.push([i, j]);
            report.complete = false;
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn chain(points: &[[f64; 2]]) -> Vec<Segment> {
        points
            .windows(2)
            .enumerate()
            .map(|(i, p)| Segment {
                domain: [i as f64, (i + 1) as f64],
                points: [p[0], p[1]],
                error_upper_mm: 0.,
            })
            .collect()
    }
    #[test]
    fn crossing_and_adjacent_vertices_have_distinct_results() {
        let bow = chain(&[[0., 0.], [2., 2.], [0., 2.], [2., 0.]]);
        let report = inspect_chain(&bow, false, 100).unwrap();
        assert!(report.complete);
        assert_eq!(report.crossings, vec![[0, 2]]);
        let square = chain(&[[0., 0.], [2., 0.], [2., 2.], [0., 2.], [0., 0.]]);
        let report = inspect_chain(&square, true, 100).unwrap();
        assert!(report.complete && report.crossings.is_empty() && report.contacts.is_empty());
    }
    #[test]
    fn overlap_resource_and_degeneracy_do_not_prove_simple() {
        let overlap = chain(&[[0., 0.], [2., 0.], [1., 0.]]);
        let report = inspect_chain(&overlap, false, 100).unwrap();
        assert!(!report.complete && report.uncertain == vec![[0, 1]]);
        let square = chain(&[[0., 0.], [2., 0.], [2., 2.], [0., 2.], [0., 0.]]);
        let report = inspect_chain(&square, true, 1).unwrap();
        assert!(!report.complete && report.checks == 1 && report.total_pairs == 6);
        let degenerate = chain(&[[0., 0.], [0., 0.], [1., 0.]]);
        let report = inspect_chain(&degenerate, false, 100).unwrap();
        assert_eq!(report.degenerate, vec![0]);
        assert_eq!(report.to_value()["simple"], false);
    }
}
