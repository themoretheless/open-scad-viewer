//! Conservative support-pair exclusion. Overlap remains explicitly pending.
use super::*;
/// Parameter product for two supporting expressions, not a contact witness.
pub type SupportProduct = [[[f64; 2]; 2]; 2];
pub struct SupportPairReport {
    pub separated: Vec<SupportProduct>,
    pub pending: Vec<SupportProduct>,
    pub cells: usize,
    pub spans: usize,
}
impl SupportPairReport {
    pub fn complete_separated(&self) -> bool {
        self.pending.is_empty()
    }
}
fn inspect<F>(
    domain: SupportProduct,
    max_cells: usize,
    max_spans: usize,
    mut bounds: F,
) -> Result<SupportPairReport>
where
    F: FnMut(usize, [[f64; 2]; 2], usize) -> Result<(Option<[[f64; 2]; 3]>, usize)>,
{
    if max_cells > 100000
        || max_spans > 100000
        || domain
            .iter()
            .flatten()
            .any(|r| !r.iter().all(|x| x.is_finite()) || r[0] > r[1] || !(r[1] - r[0]).is_finite())
    {
        return Err(Error::new(
            "BREP_PROFILE_CONTACTS",
            "Bound support pair domain and work",
        ));
    }
    let mut out = SupportPairReport {
        separated: Vec::new(),
        pending: Vec::new(),
        cells: 0,
        spans: 0,
    };
    let mut stack = vec![domain];
    while let Some(cell) = stack.pop() {
        if out.cells == max_cells {
            out.pending.push(cell);
            out.pending.extend(stack);
            break;
        }
        out.cells += 1;
        let (a, spent) = bounds(0, cell[0], max_spans - out.spans)?;
        out.spans += spent;
        let Some(a) = a else {
            out.pending.push(cell);
            out.pending.extend(stack);
            break;
        };
        let (b, spent) = bounds(1, cell[1], max_spans - out.spans)?;
        out.spans += spent;
        let Some(b) = b else {
            out.pending.push(cell);
            out.pending.extend(stack);
            break;
        };
        if (0..3).any(|k| a[k][1] < b[k][0] || b[k][1] < a[k][0]) {
            out.separated.push(cell);
            continue;
        }
        let mut choice = None;
        let mut width = -1.;
        for side in 0..2 {
            for axis in 0..2 {
                let r = cell[side][axis];
                let midpoint = r[0] + (r[1] - r[0]) / 2.;
                let original = domain[side][axis];
                let scale = original[1] - original[0];
                if midpoint > r[0] && midpoint < r[1] && scale > 0. {
                    let normalized = (r[1] - r[0]) / scale;
                    if normalized > width {
                        choice = Some((side, axis, midpoint));
                        width = normalized;
                    }
                }
            }
        }
        let Some((side, axis, midpoint)) = choice else {
            out.pending.push(cell);
            continue;
        };
        let mut left = cell;
        let mut right = cell;
        left[side][axis][1] = midpoint;
        right[side][axis][0] = midpoint;
        stack.push(right);
        stack.push(left);
    }
    Ok(out)
}
impl ExpressionShell<'_, '_, '_, '_, '_, '_, '_, '_> {
    /// Excludes intersections only on the supplied support rectangles. Shared
    /// seams and uncertain overlap remain pending; no sampled contact admission.
    pub fn inspect_support_pair(
        &self,
        faces: [usize; 2],
        domain: SupportProduct,
        max_cells: usize,
        max_controls_per_enclosure: usize,
        max_spans: usize,
    ) -> Result<SupportPairReport> {
        if faces[0] == faces[1]
            || faces.iter().any(|i| *i >= self.faces.len())
            || max_controls_per_enclosure > 1000000
        {
            return Err(Error::new(
                "BREP_PROFILE_CONTACTS",
                "Require two distinct owned faces and bounded controls",
            ));
        }
        inspect(domain, max_cells, max_spans, |side, rect, budget| {
            self.enclose_support(faces[side], rect, max_controls_per_enclosure, budget)
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn volume(r: SupportProduct) -> f64 {
        r.iter().flatten().map(|x| x[1] - x[0]).product()
    }
    #[test]
    fn support_pair_covers_budget_remainder_and_real_crossing() {
        let domain = [[[0., 1.]; 2]; 2];
        // Two perpendicular planes sharing a line. Bounds cannot exclude it.
        let report = inspect(domain, 31, 100, |side, r, _| {
            Ok((
                Some(if side == 0 {
                    [r[0], r[1], [0.; 2]]
                } else {
                    [r[0], [0.5; 2], r[1]]
                }),
                1,
            ))
        })
        .unwrap();
        assert!(!report.complete_separated());
        assert_eq!(report.cells, 31);
        let covered = report
            .pending
            .iter()
            .chain(&report.separated)
            .copied()
            .map(volume)
            .sum::<f64>();
        assert_eq!(covered, 1.);
        assert!(report
            .pending
            .iter()
            .any(|r| r[0][1][0] <= 0.5 && r[0][1][1] >= 0.5 && r[1][1][0] == 0.));
        let no_work = inspect(domain, 0, 100, |_, _, _| panic!("must not evaluate")).unwrap();
        assert_eq!(no_work.pending, vec![domain]);
        let no_spans = inspect(domain, 10, 0, |_, _, _| Ok((None, 0))).unwrap();
        assert_eq!(no_spans.pending, vec![domain]);
    }
    #[test]
    fn support_pair_proves_disjoint_planes_and_retains_touch() {
        let domain = [[[0., 1.]; 2]; 2];
        let separated = inspect(domain, 1, 2, |side, r, _| {
            Ok((Some([r[0], r[1], [side as f64; 2]]), 1))
        })
        .unwrap();
        assert!(separated.complete_separated());
        assert_eq!(separated.separated, vec![domain]);
        let touch = inspect(domain, 1, 2, |_, r, _| Ok((Some([r[0], r[1], [0.; 2]]), 1))).unwrap();
        assert!(!touch.complete_separated());
        assert_eq!(touch.pending.iter().copied().map(volume).sum::<f64>(), 1.);
    }
}
