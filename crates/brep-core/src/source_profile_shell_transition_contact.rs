//! A full transition's only plane preimage is its owned natural rail.
use super::*;
use nurbs_core::interval_eval::Interval as I;
type Rectangle = [[f64; 2]; 2];
struct Cover {
    accepted: Vec<Rectangle>,
    pending: Vec<Rectangle>,
    cells: usize,
    spans: usize,
}
fn cover<F>(
    domain: Rectangle,
    high: bool,
    max_cells: usize,
    max_spans: usize,
    mut bound: F,
) -> Result<Cover>
where
    F: FnMut(Rectangle, bool, usize) -> Result<(Option<[f64; 2]>, usize)>,
{
    let mut out = Cover {
        accepted: Vec::new(),
        pending: Vec::new(),
        cells: 0,
        spans: 0,
    };
    let anchor = domain[0][usize::from(high)];
    let mut stack = vec![domain];
    while let Some(rect) = stack.pop() {
        if out.cells == max_cells {
            out.pending.push(rect);
            out.pending.extend(stack);
            break;
        }
        out.cells += 1;
        let anchored = rect[0][usize::from(high)] == anchor;
        let (value, spent) = bound(rect, anchored, max_spans - out.spans)?;
        out.spans += spent;
        if value.is_some_and(|v| v[0] > 0. || v[1] < 0.) {
            out.accepted.push(rect);
            continue;
        }
        let mut choice = None;
        let mut width = -1.;
        for axis in 0..2 {
            let r = rect[axis];
            let middle = r[0] + (r[1] - r[0]) / 2.;
            let scale = domain[axis][1] - domain[axis][0];
            if middle > r[0] && middle < r[1] && scale > 0. && (r[1] - r[0]) / scale > width {
                choice = Some((axis, middle));
                width = (r[1] - r[0]) / scale;
            }
        }
        if out.spans == max_spans {
            out.pending.push(rect);
            out.pending.extend(stack);
            break;
        }
        let Some((axis, middle)) = choice else {
            out.pending.push(rect);
            continue;
        };
        let mut left = rect;
        let mut right = rect;
        left[axis][1] = middle;
        right[axis][0] = middle;
        stack.push(right);
        stack.push(left);
    }
    Ok(out)
}
pub struct TransitionPlaneContact<'x> {
    owners: [&'x ShellFace; 2],
    rail: usize,
    plane: [[f64; 3]; 3],
    accepted: Vec<Rectangle>,
}
impl TransitionPlaneContact<'_> {
    pub fn owners(&self) -> [&ShellFace; 2] {
        self.owners
    }
    pub fn rail(&self) -> usize {
        self.rail
    }
    pub fn plane(&self) -> [[f64; 3]; 3] {
        self.plane
    }
    pub fn accepted(&self) -> &[Rectangle] {
        &self.accepted
    }
}
pub struct TransitionPlaneReport<'x> {
    pub anchor_order: Option<usize>,
    pub certificate: Option<TransitionPlaneContact<'x>>,
    pub pending: Vec<Rectangle>,
    pub cells: usize,
    pub spans: usize,
    pub exact_work: u64,
    pub reason: &'static str,
}
impl ExpressionShell<'_, '_, '_, '_, '_, '_, '_, '_> {
    /// Requires an owned transition rail and its original planar neighbor.
    /// The exact corrected bridge is evaluated afresh; no old contact report
    /// or adjacency flag admits its plane preimage.
    pub fn qualify_transition_plane_contact(
        &self,
        rail: usize,
        max_work: u64,
        max_controls: usize,
        max_cells: usize,
        max_spans: usize,
    ) -> Result<TransitionPlaneReport<'_>> {
        if rail > 1
            || max_work > 100000000
            || max_controls > 1000000
            || max_cells > 100000
            || max_spans > 100000
        {
            return Err(Error::new(
                "BREP_PROFILE_CONTACTS",
                "Bound transition plane contact work",
            ));
        }
        let pair = self
            .pairs
            .iter()
            .find(|p| p.kind == ShellPairKind::Rail(rail))
            .unwrap();
        let owners = pair.uses.map(|a| &self.faces[a.face]);
        let mut out = TransitionPlaneReport {
            certificate: None,
            anchor_order: None,
            pending: Vec::new(),
            cells: 0,
            spans: 0,
            exact_work: 0,
            reason: "expression-transition-neighbor-planarity-unproven",
        };
        let [ShellFaceKind::Transition, ShellFaceKind::Original(index)] = owners.map(|f| f.kind)
        else {
            return Ok(out);
        };
        let root = self.assembly.patch.rails.face.boundary.rooted[0];
        let profile = root.restricted.proposal.profile(root.face).unwrap();
        let source = profile.source();
        let free_axis = 1 - root.axis;
        let domain = [
            profile.section(root.axis, root.parameters[0])?.range(),
            [0., 1.],
        ];
        out.pending.push(domain);
        if max_work == 0 {
            out.reason = "expression-transition-plane-work-limit";
            return Ok(out);
        }
        let neighbor =
            root.restricted.proposal.body.geometry().shell().faces()[index][0].edges()[0].surface();
        let Some(plane) =
            crate::source_allowed_contact::plane(neighbor, &mut out.exact_work, max_work)?
        else {
            return Ok(out);
        };
        // Positive profile weights preserve exact coplanarity of the natural
        // rail's original control points, including its entire source image.
        let boundary = if rail == 1 {
            if free_axis == 0 {
                source.control_points.len() - 1
            } else {
                source.control_points[0].len() - 1
            }
        } else {
            0
        };
        for (u, row) in source.control_points.iter().enumerate() {
            for (v, p) in row.iter().enumerate() {
                if [u, v][free_axis] == boundary
                    && crate::source_allowed_contact::orient(
                        &[plane[0], plane[1], plane[2], [p[0], p[1], p[2]]],
                        None,
                        &mut out.exact_work,
                        max_work,
                    )? != Some(cad_predicates::Sign::Zero)
                {
                    out.reason = "expression-transition-rail-plane-unproven";
                    return Ok(out);
                }
            }
        }
        out.anchor_order = Some(1);
        let count = if free_axis == 0 {
            source.control_points.len()
        } else {
            source.control_points[0].len()
        };
        let knots = if free_axis == 0 {
            &source.knots_u
        } else {
            &source.knots_v
        };
        let degree = if free_axis == 0 {
            source.degree_u
        } else {
            source.degree_v
        };
        let endpoint = domain[0][rail];
        let clamped = if rail == 0 {
            knots[..=degree].iter().all(|k| *k == endpoint)
        } else {
            knots[count..].iter().all(|k| *k == endpoint)
        };
        if count >= 2 && degree >= 1 && clamped {
            let adjacent = if rail == 0 { 1 } else { count - 2 };
            let mut tangent = true;
            for (u, row) in source.control_points.iter().enumerate() {
                for (v, p) in row.iter().enumerate() {
                    if [u, v][free_axis] == adjacent
                        && crate::source_allowed_contact::orient(
                            &[plane[0], plane[1], plane[2], [p[0], p[1], p[2]]],
                            None,
                            &mut out.exact_work,
                            max_work,
                        )? != Some(cad_predicates::Sign::Zero)
                    {
                        tangent = false;
                    }
                }
            }
            if tangent {
                out.anchor_order = Some(2);
            }
        }
        let first = profile.section(root.axis, root.parameters[0])?;
        let last = profile.section(root.axis, root.parameters[1])?;
        let width = root.parameters[1] - root.parameters[0];
        let bridge =
            nurbs_core::normal_alignment::ProfileSectionBridge::new(first, last, [width; 2])?;
        let (constrained, _) = bridge.constrain_original_rails(max_controls)?;
        let Some(constrained) = constrained else {
            out.reason = "expression-transition-plane-recipe-unproven";
            return Ok(out);
        };
        let a =
            std::array::from_fn::<_, 3, _>(|k| I::point(plane[1][k]).sub(I::point(plane[0][k])));
        let b =
            std::array::from_fn::<_, 3, _>(|k| I::point(plane[2][k]).sub(I::point(plane[0][k])));
        let a = a.into_iter().collect::<Result<Vec<_>>>()?;
        let b = b.into_iter().collect::<Result<Vec<_>>>()?;
        let normal = [
            a[1].mul(b[2])?.sub(a[2].mul(b[1])?)?,
            a[2].mul(b[0])?.sub(a[0].mul(b[2])?)?,
            a[0].mul(b[1])?.sub(a[1].mul(b[0])?)?,
        ];
        let result = cover(
            domain,
            rail == 1,
            max_cells,
            max_spans,
            |rect, derivative, budget| {
                let (bounds, spent) = if derivative {
                    let (jet, spent) = constrained
                        .derivatives_with_transverse_correction(rect[0], rect[1], budget)?;
                    (jet.map(|j| j[0]), spent)
                } else {
                    constrained.enclose_with_transverse_correction(rect[0], rect[1], budget)?
                };
                let Some(bounds) = bounds else {
                    return Ok((None, spent));
                };
                let mut value = I::point(0.);
                for k in 0..3 {
                    let coordinate = I::new(bounds[k][0], bounds[k][1])?;
                    let coordinate = if derivative {
                        coordinate
                    } else {
                        coordinate.sub(I::point(plane[0][k]))?
                    };
                    value = value.add(normal[k].mul(coordinate)?)?;
                }
                Ok((Some([value.lo, value.hi]), spent))
            },
        )?;
        out.cells = result.cells;
        out.spans = result.spans;
        out.pending = result.pending;
        out.reason = if out.anchor_order == Some(2) {
            "expression-transition-plane-second-jet-required"
        } else {
            "expression-transition-plane-cover-pending"
        };
        if out.pending.is_empty() {
            out.certificate = Some(TransitionPlaneContact {
                owners,
                rail,
                plane,
                accepted: result.accepted,
            });
            out.reason = "expression-transition-owned-plane-contact-qualified";
        }
        Ok(out)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn area(r: Rectangle) -> f64 {
        (r[0][1] - r[0][0]) * (r[1][1] - r[1][0])
    }
    #[test]
    fn transition_plane_cover_keeps_extra_zero_and_budget_remainder() {
        let domain = [[0., 1.]; 2];
        let r = cover(domain, false, 127, 127, |rect, derivative, _| {
            let t = I::new(rect[0][0], rect[0][1])?;
            let value = if derivative {
                I::point(1.).sub(I::point(4.).mul(t)?)?
            } else {
                t.mul(I::point(1.).sub(I::point(2.).mul(t)?)?)?
            };
            Ok((Some([value.lo, value.hi]), 1))
        })
        .unwrap();
        assert!(!r.pending.is_empty());
        let covered = r
            .pending
            .iter()
            .chain(&r.accepted)
            .copied()
            .map(area)
            .sum::<f64>();
        assert!((covered - 1.).abs() < 1e-12);
        assert!(r.pending.iter().any(|r| r[0][0] <= 0.5 && r[0][1] >= 0.5));
        let no = cover(domain, false, 0, 0, |_, _, _| panic!("no work")).unwrap();
        assert_eq!(no.pending, vec![domain]);
        let plane = cover(domain, false, 1, 1, |_, derivative, _| {
            assert!(derivative);
            Ok((Some([1.; 2]), 1))
        })
        .unwrap();
        assert!(plane.pending.is_empty());
        assert_eq!(plane.accepted, vec![domain]);
    }
}
