//! A full transition's only plane preimage is its owned natural rail.
use super::*;
use nurbs_core::interval_eval::Interval as I;
type Rectangle = [[f64; 2]; 2];
fn plane_normal(plane: [[f64; 3]; 3]) -> Result<[I; 3]> {
    // An exactly constant coordinate gives an exact plane equation. Avoid
    // introducing cross-product roundoff in its mathematically zero terms.
    for axis in 0..3 {
        if plane[0][axis] == plane[1][axis] && plane[0][axis] == plane[2][axis] {
            return Ok(std::array::from_fn(|k| {
                I::point(if k == axis { 1. } else { 0. })
            }));
        }
    }
    let a = std::array::from_fn::<_, 3, _>(|k| I::point(plane[1][k]).sub(I::point(plane[0][k])));
    let b = std::array::from_fn::<_, 3, _>(|k| I::point(plane[2][k]).sub(I::point(plane[0][k])));
    let a = a.into_iter().collect::<Result<Vec<_>>>()?;
    let b = b.into_iter().collect::<Result<Vec<_>>>()?;
    Ok([
        a[1].mul(b[2])?.sub(a[2].mul(b[1])?)?,
        a[2].mul(b[0])?.sub(a[0].mul(b[2])?)?,
        a[0].mul(b[1])?.sub(a[1].mul(b[0])?)?,
    ])
}
// The caller proves zero position at anchor and continuity on the path.
// Every derivative query covers a whole interval; no sampled quadrature.
fn integrated_position<F>(
    anchor: f64,
    target: [f64; 2],
    high: bool,
    budget: usize,
    mut derivative: F,
) -> Result<(Option<I>, usize)>
where
    F: FnMut([f64; 2], usize) -> Result<(Option<I>, usize)>,
{
    let near = target[usize::from(high)];
    let mut sum = I::point(0.);
    let mut spent = 0;
    let mut previous = anchor;
    if near != anchor {
        for step in 1..=8 {
            let next = if step == 8 {
                near
            } else {
                anchor + (near - anchor) * (step as f64 / 8.)
            };
            if next == previous {
                continue;
            }
            if spent == budget {
                return Ok((None, spent));
            }
            let (bound, used) =
                derivative([previous.min(next), previous.max(next)], budget - spent)?;
            spent += used;
            let Some(bound) = bound else {
                return Ok((None, spent));
            };
            sum = sum.add(bound.mul(I::point(next).sub(I::point(previous))?)?)?;
            previous = next;
        }
    }
    if target[0] != target[1] {
        if spent == budget {
            return Ok((None, spent));
        }
        let (bound, used) = derivative(target, budget - spent)?;
        spent += used;
        let Some(bound) = bound else {
            return Ok((None, spent));
        };
        sum = sum.add(bound.mul(I::new(target[0], target[1])?.sub(I::point(near))?)?)?;
    }
    Ok((Some(sum), spent))
}
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
    // Process each refinement level before descending further. A difficult
    // anchor must not consume the whole budget while coarse siblings wait.
    let mut stack = std::collections::VecDeque::from([domain]);
    while let Some(rect) = stack.pop_front() {
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
        stack.push_back(left);
        stack.push_back(right);
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
        #[cfg(test)]
        if rail == 1 && max_work > 0 {
            if let Some(trace) = crate::source_ruled_projection_contact::exact_trace::certify(
                neighbor,
                [0, 1],
                1,
                max_controls,
            )? {
                assert!(std::ptr::eq(trace.source(), neighbor));
                assert_eq!(trace.axes(), [0, 1]);
                assert_eq!(trace.linear_axis(), 1);
                let monotone=crate::source_ruled_projection_contact::exact_trace::certify_monotone_coordinate(
                    &trace,1,max_controls)?.expect("whole ruled neighbor trace is strictly monotone in Y");
                assert!(std::ptr::eq(monotone.trace(),&trace));
                assert!(monotone.increasing() && monotone.coordinate()==1);
                assert_eq!(monotone.parameter_domain(),trace.curve().domain());
                let section0=profile.section(root.axis,root.parameters[0])?;
                let section1=profile.section(root.axis,root.parameters[1])?;
                let bridge=nurbs_core::normal_alignment::ProfileSectionBridge::new(section0,section1,
                    [root.parameters[1]-root.parameters[0];2])?;
                if let Some(bridge)=bridge.constrain_original_rails(max_controls)?.0 {
                    if let Some(bounds)=bridge.enclose_with_transverse_correction(domain[0],domain[1],max_spans)?.0 {
                        let candidates=monotone.coordinate_preimage(bounds[1],128)?;
                        assert!(candidates.cells<=128);
                        assert!(std::ptr::eq(candidates.owner(),&monotone));
                        assert_eq!(candidates.target(),bounds[1]);
                        let owned=self.assembly.patch.rails.face.boundary.rooted[rail];
                        let neighbor_side=1-owned.points[0].source_side();
                        let root_parameters=owned.points.each_ref().map(|p|
                            p.uv_bounds()[neighbor_side][1-trace.linear_axis()]);
                        let exterior_candidates=candidates.exterior_domains(root_parameters)?;
                        eprintln!("transition neighbor rootParameters={root_parameters:?} exteriorCandidates={exterior_candidates:?}");
                        eprintln!("transition neighbor full Y target={:?} candidateParameters={:?} cells={}",
                            bounds[1],candidates.domains,candidates.cells);
                    }
                }
                eprintln!("transition curved neighbor monotoneY range={:?} domain={:?}",
                    monotone.coordinate_range(),monotone.parameter_domain());
                eprintln!(
                    "transition curved neighbor exactTrace=true curveDegree={} curveDomain={:?}",
                    trace.curve().degree,
                    trace.curve().domain()
                );
            }
            eprintln!("transition curved neighbor face={} degrees={:?} knots={:?} controls={:?} weights={:?} sourceFreeAxis={} sourceControls={:?}",
                index, [neighbor.degree_u,neighbor.degree_v], [&neighbor.knots_u,&neighbor.knots_v],
                neighbor.control_points,neighbor.weights,free_axis,source.control_points);
        }
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
        let normal = plane_normal(plane)?;
        let result = cover(
            domain,
            rail == 1,
            max_cells,
            max_spans,
            |rect, derivative, budget| {
                let (bounds, mut spent) = if derivative {
                    if out.anchor_order == Some(2) {
                        constrained.second_free_derivatives_with_transverse_correction(
                            rect[0], rect[1], budget,
                        )?
                    } else {
                        let (jet, spent) = constrained
                            .derivatives_with_transverse_correction(rect[0], rect[1], budget)?;
                        (jet.map(|j| j[0]), spent)
                    }
                } else {
                    constrained.enclose_with_transverse_correction(rect[0], rect[1], budget)?
                };
                let Some(bounds) = bounds else {
                    return Ok((None, spent));
                };
                let mut value = I::point(0.);
                for k in 0..3 {
                    if normal[k].lo == 0. && normal[k].hi == 0. {
                        continue;
                    }
                    let coordinate = I::new(bounds[k][0], bounds[k][1])?;
                    let coordinate = if derivative {
                        coordinate
                    } else {
                        coordinate.sub(I::point(plane[0][k]))?
                    };
                    value = value.add(normal[k].mul(coordinate)?)?;
                }
                if !derivative
                    && value.lo <= 0.
                    && value.hi >= 0.
                    && out.anchor_order == Some(2)
                    && spent < budget
                {
                    // Position subtraction can remain broad away from a
                    // tangent rail. Twice integrate a strict second normal
                    // derivative over the entire path from that exact rail.
                    let path = if rail == 1 {
                        [rect[0][0], domain[0][1]]
                    } else {
                        [domain[0][0], rect[0][1]]
                    };
                    let (second, used) = constrained
                        .second_free_derivatives_with_transverse_correction(
                            path,
                            rect[1],
                            budget - spent,
                        )?;
                    spent += used;
                    if let Some(second) = second {
                        let mut curvature = I::point(0.);
                        for k in 0..3 {
                            if normal[k].lo == 0. && normal[k].hi == 0. {
                                continue;
                            }
                            curvature = curvature
                                .add(normal[k].mul(I::new(second[k][0], second[k][1])?)?)?;
                        }
                        if curvature.lo > 0. || curvature.hi < 0. {
                            // The cover only consumes strict nonzero status;
                            // this is a derivative bound, not a position box.
                            return Ok((Some([curvature.lo, curvature.hi]), spent));
                        }
                    }
                }
                if !derivative && value.lo <= 0. && value.hi >= 0. && spent < budget {
                    let (integral, used) = integrated_position(
                        domain[0][usize::from(rail == 1)],
                        rect[0],
                        rail == 1,
                        budget - spent,
                        |free, remaining| {
                            let (jet, used) = constrained
                                .derivatives_with_transverse_correction(free, rect[1], remaining)?;
                            let Some(jet) = jet else {
                                return Ok((None, used));
                            };
                            let mut dot = I::point(0.);
                            for k in 0..3 {
                                if normal[k].lo == 0. && normal[k].hi == 0. {
                                    continue;
                                }
                                dot =
                                    dot.add(normal[k].mul(I::new(jet[0][k][0], jet[0][k][1])?)?)?;
                            }
                            Ok((Some(dot), used))
                        },
                    )?;
                    spent += used;
                    if let Some(integral) = integral {
                        value = value.intersect(integral.lo, integral.hi)?;
                    }
                }
                Ok((Some([value.lo, value.hi]), spent))
            },
        )?;
        out.cells = result.cells;
        out.spans = result.spans;
        out.pending = result.pending;
        out.reason = "expression-transition-plane-cover-pending";
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
    fn plane_cover_integral_preserves_zeros_orientation_and_budget() {
        for high in [false, true] {
            let anchor = if high { 1. } else { 0. };
            let (value, spent) = integrated_position(anchor, [0.4, 0.6], high, 9, |r, _| {
                let t = I::new(r[0], r[1])?;
                let d = if high {
                    I::point(-2.).mul(I::point(1.).sub(t)?)?
                } else {
                    I::point(2.).mul(t)?
                };
                Ok((Some(d), 1))
            })
            .unwrap();
            let value = value.unwrap();
            assert!(value.lo > 0. && value.lo <= 0.16 && value.hi >= 0.36);
            assert_eq!(spent, 9);
        }
        let (zero, _) = integrated_position(0., [0.5; 2], false, 8, |r, _| {
            let t = I::new(r[0], r[1])?;
            let d = I::point(2.).mul(t)?.sub(I::point(6.).mul(t.mul(t)?)?)?;
            Ok((Some(d), 1))
        })
        .unwrap();
        let zero = zero.unwrap();
        assert!(zero.lo <= 0. && zero.hi >= 0.);
        for budget in 0..9 {
            let exhausted = integrated_position(0., [0.4, 0.6], false, budget, |_, remaining| {
                assert!(remaining > 0);
                Ok((Some(I::point(1.)), 1))
            })
            .unwrap();
            assert!(exhausted.0.is_none());
            assert_eq!(exhausted.1, budget);
        }
        for missing in 0..9 {
            let mut query = 0;
            let partial = integrated_position(0., [0.4, 0.6], false, 9, |_, _| {
                let bound = if query == missing {
                    None
                } else {
                    Some(I::point(1.))
                };
                query += 1;
                Ok((bound, 1))
            })
            .unwrap();
            assert!(partial.0.is_none());
            assert_eq!(partial.1, missing + 1);
        }
    }
    #[test]
    fn plane_cover_axis_equation_has_exact_zero_components() {
        for axis in 0..3 {
            let mut plane = [[0.; 3]; 3];
            for p in &mut plane {
                p[axis] = 1000000.;
            }
            plane[1][(axis + 1) % 3] = 3.;
            plane[2][(axis + 2) % 3] = 7.;
            let normal = plane_normal(plane).unwrap();
            for k in 0..3 {
                let expected = if k == axis { 1. } else { 0. };
                assert_eq!([normal[k].lo, normal[k].hi], [expected; 2]);
            }
        }
    }
    #[test]
    fn plane_cover_budget_reaches_all_coarse_siblings() {
        let domain = [[0., 1.]; 2];
        let mut visited = Vec::new();
        let report = cover(domain, false, 7, 100, |rect, _, _| {
            visited.push(rect);
            Ok((None, 1))
        })
        .unwrap();
        assert_eq!(visited.len(), 7);
        for x in [[0., 0.5], [0.5, 1.]] {
            for y in [[0., 0.5], [0.5, 1.]] {
                assert!(visited.contains(&[x, y]));
            }
        }
        assert!(report.accepted.is_empty());
        assert_eq!(report.pending.len(), 8);
        assert_eq!(report.pending.iter().copied().map(area).sum::<f64>(), 1.);
        let exhausted = cover(domain, false, 100, 3, |_, _, remaining| {
            assert!(remaining > 0);
            Ok((None, 1))
        })
        .unwrap();
        assert_eq!(exhausted.spans, 3);
        assert_eq!(exhausted.cells, 3);
        assert!(exhausted.accepted.is_empty());
        assert_eq!(exhausted.pending.iter().copied().map(area).sum::<f64>(), 1.);
    }
    #[test]
    fn second_order_plane_cover_refuses_an_extra_interior_zero() {
        let domain = [[0., 1.]; 2];
        let result = cover(domain, false, 127, 127, |rect, second, _| {
            let t = I::new(rect[0][0], rect[0][1])?;
            let value = if second {
                I::point(2.).sub(I::point(12.).mul(t)?)?
            } else {
                t.mul(t)?.mul(I::point(1.).sub(I::point(2.).mul(t)?)?)?
            };
            if !second && value.lo <= 0. && value.hi >= 0. {
                let path = I::new(0., rect[0][1])?;
                let curvature = I::point(2.).sub(I::point(12.).mul(path)?)?;
                if curvature.lo > 0. || curvature.hi < 0. {
                    return Ok((Some([curvature.lo, curvature.hi]), 1));
                }
            }
            Ok((Some([value.lo, value.hi]), 1))
        })
        .unwrap();
        assert!(!result.pending.is_empty());
        assert!(result
            .pending
            .iter()
            .any(|r| r[0][0] <= 0.5 && r[0][1] >= 0.5));
        let covered = result
            .pending
            .iter()
            .chain(&result.accepted)
            .copied()
            .map(area)
            .sum::<f64>();
        assert!((covered - 1.).abs() < 1e-12);
        let quadratic = cover(domain, false, 1, 1, |_, second, _| {
            assert!(second);
            Ok((Some([2.; 2]), 1))
        })
        .unwrap();
        assert!(quadratic.pending.is_empty());
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
