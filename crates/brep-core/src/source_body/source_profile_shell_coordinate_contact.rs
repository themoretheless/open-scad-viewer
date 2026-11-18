//! Whole transition exclusion against an unchanged constant-coordinate support.
use super::*;
type Rectangle = [[f64; 2]; 2];
pub struct TransitionCoordinateContact<'x> {
    owners: [&'x ShellFace; 2],
    axis: usize,
    value: f64,
    cover: Vec<(Rectangle, [f64; 2])>,
}
impl TransitionCoordinateContact<'_> {
    pub fn owners(&self) -> [&ShellFace; 2] {
        self.owners
    }
    pub fn axis(&self) -> usize {
        self.axis
    }
    pub fn value(&self) -> f64 {
        self.value
    }
    pub fn cover(&self) -> &[(Rectangle, [f64; 2])] {
        &self.cover
    }
}
pub struct TransitionCoordinateReport<'x> {
    pub certificate: Option<TransitionCoordinateContact<'x>>,
    pub pending: Vec<Rectangle>,
    pub cells: usize,
    pub spans: usize,
    pub reason: &'static str,
}
struct Cover {
    separated: Vec<(Rectangle, [f64; 2])>,
    pending: Vec<Rectangle>,
    cells: usize,
    spans: usize,
}
fn cover<F>(
    domain: Rectangle,
    axis: usize,
    value: f64,
    max_cells: usize,
    max_spans: usize,
    mut bounds: F,
) -> Result<Cover>
where
    F: FnMut(Rectangle, usize) -> Result<(Option<[[f64; 2]; 3]>, usize)>,
{
    let mut out = Cover {
        separated: vec![],
        pending: vec![],
        cells: 0,
        spans: 0,
    };
    let mut stack = vec![domain];
    while let Some(r) = stack.pop() {
        if out.cells == max_cells {
            out.pending.push(r);
            out.pending.extend(stack);
            break;
        }
        out.cells += 1;
        let (b, spent) = bounds(r, max_spans - out.spans)?;
        if spent > max_spans - out.spans {
            return Err(Error::new(
                "BREP_PROFILE_CONTACTS",
                "Coordinate enclosure exceeded shared span work",
            ));
        }
        out.spans += spent;
        let Some(b) = b else {
            out.pending.push(r);
            out.pending.extend(stack);
            break;
        };
        let z = b[axis];
        if !z[0].is_finite() || !z[1].is_finite() || z[0] > z[1] {
            return Err(Error::new(
                "BREP_PROFILE_CONTACTS",
                "Require finite ordered native coordinate bounds",
            ));
        }
        if z[1] < value || z[0] > value {
            out.separated.push((r, z));
            continue;
        }
        let mut choice = None;
        let mut width = -1.;
        for a in 0..2 {
            let m = r[a][0] + (r[a][1] - r[a][0]) / 2.;
            let w = (r[a][1] - r[a][0]) / (domain[a][1] - domain[a][0]);
            if m > r[a][0] && m < r[a][1] && w > width {
                choice = Some((a, m));
                width = w;
            }
        }
        let Some((a, m)) = choice else {
            out.pending.push(r);
            continue;
        };
        let mut lo = r;
        let mut hi = r;
        lo[a][1] = m;
        hi[a][0] = m;
        stack.push(hi);
        stack.push(lo);
    }
    Ok(out)
}
impl ExpressionShell<'_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn qualify_transition_coordinate_contact(
        &self,
        faces: [usize; 2],
        max_cells: usize,
        max_controls: usize,
        max_spans: usize,
    ) -> Result<TransitionCoordinateReport<'_>> {
        if faces[0] == faces[1]
            || faces.iter().any(|&i| i >= self.faces.len())
            || max_cells > 100000
            || max_controls > 1000000
            || max_spans > 100000
        {
            return Err(Error::new(
                "BREP_PROFILE_CONTACTS",
                "Bound transition coordinate work",
            ));
        }
        let mut out = TransitionCoordinateReport {
            certificate: None,
            pending: vec![],
            cells: 0,
            spans: 0,
            reason: "transition-coordinate-support-unproven",
        };
        let (original, transition, original_side) = match faces.map(|i| self.faces[i].kind()) {
            [ShellFaceKind::Original(a), ShellFaceKind::Transition] => (a, faces[1], 0),
            [ShellFaceKind::Transition, ShellFaceKind::Original(a)] => (a, faces[0], 1),
            _ => return Ok(out),
        };
        let root = self.assembly.patch.rails.face.boundary.rooted[0];
        let shell = root.restricted.proposal.body.geometry().shell();
        let regions = shell.regions().unwrap();
        let s = regions[original].loops()[0][0].surface();
        if self.faces[faces[original_side]]
            .loops
            .iter()
            .flatten()
            .any(|u| !matches!(u,ShellUse::Source(f) if f.surface()==s))
            || s.weights
                .iter()
                .flatten()
                .any(|w| !w.is_finite() || *w <= 0.)
        {
            return Ok(out);
        }
        let Some(axis) = (0..3).find(|&a| {
            s.control_points
                .iter()
                .flatten()
                .all(|p| p[a].is_finite() && p[a] == s.control_points[0][0][a])
        }) else {
            return Ok(out);
        };
        let value = s.control_points[0][0][axis];
        let domain = self.support_domain(transition)?;
        if max_cells == 0 || max_controls == 0 || max_spans == 0 {
            out.pending.push(domain);
            out.reason = "transition-coordinate-work-limit";
            return Ok(out);
        }
        let report = cover(domain, axis, value, max_cells, max_spans, |r, spans| {
            self.enclose_support(transition, r, max_controls, spans)
        })?;
        out.cells = report.cells;
        out.spans = report.spans;
        out.pending = report.pending;
        if out.pending.is_empty() {
            out.certificate = Some(TransitionCoordinateContact {
                owners: faces.map(|i| &self.faces[i]),
                axis,
                value,
                cover: report.separated,
            });
            out.reason = "transition-whole-coordinate-separated";
        } else {
            out.reason = "transition-coordinate-cover-unproven";
        }
        Ok(out)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn area(rs: impl Iterator<Item = Rectangle>) -> f64 {
        rs.map(|r| (r[0][1] - r[0][0]) * (r[1][1] - r[1][0])).sum()
    }
    #[test]
    fn coordinate_cover_keeps_crossing_touch_and_exhausted_frontier() {
        let d = [[0., 1.]; 2];
        let positive = cover(d, 2, 0., 1, 1, |_, _| Ok((Some([[1., 2.]; 3]), 1))).unwrap();
        assert!(positive.pending.is_empty());
        assert_eq!(area(positive.separated.iter().map(|x| x.0)), 1.);
        let crossing = cover(d, 2, 0., 31, 31, |r, _| {
            Ok((Some([[r[0][0] - 0.5, r[0][1] - 0.5]; 3]), 1))
        })
        .unwrap();
        assert!(!crossing.pending.is_empty());
        assert_eq!(
            area(crossing.pending.iter().copied()) + area(crossing.separated.iter().map(|x| x.0)),
            1.
        );
        let touch = cover(d, 2, 0., 3, 3, |_, _| Ok((Some([[0., 1.]; 3]), 1))).unwrap();
        assert!(!touch.pending.is_empty());
        let empty = cover(d, 2, 0., 0, 0, |_, _| {
            panic!("no remaining cells must make no query")
        })
        .unwrap();
        assert_eq!(empty.pending, vec![d]);
        assert!(cover(d, 2, 0., 1, 0, |_, _| Ok((None, 1))).is_err());
    }
}
