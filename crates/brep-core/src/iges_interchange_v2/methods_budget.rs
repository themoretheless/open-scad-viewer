//! iges interchange v2 operations: budget.
use super::*;
impl<'a> CurveDispatcher<'a> {
pub(super) fn budget(&mut self, composite: usize, amount: usize) -> Result<()> {
        self.work = self.work.checked_add(amount).ok_or_else(|| {
            composite_error(
                "BREP_IGES_V2_COMPOSITE_BUDGET",
                format!("IGES composite DE {composite} work budget overflow"),
            )
        })?;
        if self.work > MAX_COMPOSITE_WORK {
            return Err(composite_error(
                "BREP_IGES_V2_COMPOSITE_BUDGET",
                format!("IGES composite DE {composite} work budget exceeded"),
            ));
        }
        Ok(())
    }
pub(super) fn base_curve(&mut self, composite: usize, de: usize, linear: bool) -> Result<Curve> {
        let entity = self.entities.get(&de).ok_or_else(|| {
            composite_error(
                "BREP_IGES_V2_COMPOSITE_DANGLING",
                format!("IGES composite DE {composite} has missing segment DE {de}"),
            )
        })?;
        let curve = match entity.directory.ty {
            100 => {
                return Err(composite_error(
                    "BREP_IGES_V2_COMPOSITE_UNSUPPORTED",
                    format!("IGES circular arc DE {de} is unsupported in curve dispatch"),
                ));
            }
            110 => {
                let values = &entity.values;
                if values.len() != 7 {
                    return Err(composite_error(
                        "BREP_IGES_V2_COMPOSITE_DANGLING",
                        format!("Malformed IGES line DE {de} in composite DE {composite}"),
                    ));
                }
                let mut points = Vec::with_capacity(2);
                for endpoint in 0..2 {
                    let at = 1 + 3 * endpoint;
                    let mut point = vec![f(values, at, "line X")?, f(values, at + 1, "line Y")?];
                    if self.dim == 3 {
                        point.push(f(values, at + 2, "line Z")?);
                    }
                    for coordinate in &mut point {
                        *coordinate *= self.scale;
                    }
                    points.push(point);
                }
                Curve::from_polyline(points)?
            }
            126 => curve126(self.entities, de, self.dim, self.scale)?,
            102 => {
                return Err(composite_error(
                    "BREP_IGES_V2_COMPOSITE_CYCLE",
                    format!("Unexpected nested dispatch at IGES composite DE {de}"),
                ));
            }
            _ => {
                return Err(composite_error(
                    "BREP_IGES_V2_COMPOSITE_DANGLING",
                    format!(
                        "IGES composite DE {composite} segment DE {de} has non-curve type {}",
                        entity.directory.ty
                    ),
                ));
            }
        };
        if !linear {
            return Ok(curve);
        }
        let uniform_weights = curve
            .weights
            .first()
            .is_some_and(|first| curve.weights.iter().all(|weight| weight == first));
        if curve.degree != 1 || curve.periodic || !uniform_weights {
            return Err(composite_error(
                "BREP_IGES_V2_COMPOSITE_UNSUPPORTED",
                format!(
                    "IGES composite DE {composite} segment DE {de} requires nonlinear or nonuniform-rational join"
                ),
            ));
        }
        if curve.control_points.len() > MAX_COMPOSITE_CONTROLS {
            return Err(composite_error(
                "BREP_IGES_V2_COMPOSITE_BUDGET",
                format!("IGES composite DE {composite} control budget exceeded at DE {de}"),
            ));
        }
        Ok(Curve::from_polyline(curve.control_points.clone())?)
    }
pub(super) fn endpoints(curve: &Curve) -> Result<(Vec<f64>, Vec<f64>)> {
        let domain = curve.domain();
        Ok((
            curve.evaluate(domain[0])?.point,
            curve.evaluate(domain[1])?.point,
        ))
    }
pub(super) fn distance(a: &[f64], b: &[f64]) -> Result<f64> {
        if a.len() != b.len() {
            return Err(composite_error(
                "BREP_IGES_V2_COMPOSITE_UNSUPPORTED",
                "IGES composite segments have inconsistent dimensions",
            ));
        }
        let distance = a
            .iter()
            .zip(b)
            .fold(0., |sum, (x, y)| sum + (x - y) * (x - y))
            .sqrt();
        if !distance.is_finite() {
            return Err(composite_error(
                "BREP_IGES_V2_COMPOSITE_UNSUPPORTED",
                "Non-finite IGES composite endpoint distance",
            ));
        }
        Ok(distance)
    }
pub(super) fn join_segments(
        composite: usize,
        segments: &[CompositeSegment],
        tolerance_mm: f64,
    ) -> Result<Curve> {
        let Some(first) = segments.first() else {
            return Err(composite_error(
                "BREP_IGES_V2_COMPOSITE_DANGLING",
                format!("IGES composite DE {composite} has no segments"),
            ));
        };
        let mut joined = Curve::from_polyline(first.curve.control_points.clone())?;
        let mut previous = first.de;
        for (index, segment) in segments.iter().enumerate().skip(1) {
            let (mut current_start, mut current_end) = Self::endpoints(&joined)?;
            let (segment_start, segment_end) = Self::endpoints(&segment.curve)?;
            let mut end_start = Self::distance(&current_end, &segment_start)?;
            let mut end_end = Self::distance(&current_end, &segment_end)?;
            let mut start_start = Self::distance(&current_start, &segment_start)?;
            let mut start_end = Self::distance(&current_start, &segment_end)?;
            let mut forward =
                usize::from(end_start <= tolerance_mm) + usize::from(end_end <= tolerance_mm);
            let mut backward =
                usize::from(start_start <= tolerance_mm) + usize::from(start_end <= tolerance_mm);
            if index == 1 && forward == 0 && backward > 0 {
                joined = joined.reverse()?;
                (current_start, current_end) = Self::endpoints(&joined)?;
                end_start = Self::distance(&current_end, &segment_start)?;
                end_end = Self::distance(&current_end, &segment_end)?;
                start_start = Self::distance(&current_start, &segment_start)?;
                start_end = Self::distance(&current_start, &segment_end)?;
                forward =
                    usize::from(end_start <= tolerance_mm) + usize::from(end_end <= tolerance_mm);
                backward = usize::from(start_start <= tolerance_mm)
                    + usize::from(start_end <= tolerance_mm);
            }
            if forward == 0 {
                if backward > 0 {
                    return Err(composite_error(
                        "BREP_IGES_V2_COMPOSITE_AMBIGUOUS",
                        format!(
                            "IGES composite DE {composite} requires reordering at segments DE {previous} and DE {}",
                            segment.de
                        ),
                    ));
                }
                let gap = end_start.min(end_end).min(start_start).min(start_end);
                return Err(composite_error(
                    "BREP_IGES_V2_COMPOSITE_GAP",
                    format!(
                        "IGES composite DE {composite}, segments DE {previous} -> DE {}, gap_mm={gap:.17e}, tolerance_mm={tolerance_mm:.17e}",
                        segment.de
                    ),
                ));
            }
            if forward > 1 || backward > 0 {
                return Err(composite_error(
                    "BREP_IGES_V2_COMPOSITE_AMBIGUOUS",
                    format!(
                        "IGES composite DE {composite} has ambiguous orientation at segments DE {previous} and DE {}",
                        segment.de
                    ),
                ));
            }
            let candidate = if end_end <= tolerance_mm {
                segment.curve.reverse()?
            } else {
                segment.curve.clone()
            };
            let (candidate_start, _) = Self::endpoints(&candidate)?;
            let snap = current_end
                .iter()
                .zip(&candidate_start)
                .map(|(a, b)| (a + b) * 0.5)
                .collect::<Vec<_>>();
            let mut points = joined.control_points.clone();
            let mut candidate_points = candidate.control_points.clone();
            let (Some(joined_end), Some(candidate_end)) =
                (points.last_mut(), candidate_points.first_mut())
            else {
                return Err(composite_error(
                    "BREP_IGES_V2_COMPOSITE_DANGLING",
                    format!("IGES composite DE {composite} has an empty segment"),
                ));
            };
            *joined_end = snap.clone();
            *candidate_end = snap;
            if points.len() + candidate_points.len() - 1 > MAX_COMPOSITE_CONTROLS {
                return Err(composite_error(
                    "BREP_IGES_V2_COMPOSITE_BUDGET",
                    format!("IGES composite DE {composite} control budget exceeded"),
                ));
            }
            points.extend(candidate_points.into_iter().skip(1));
            joined = Curve::from_polyline(points)?;
            previous = segment.de;
        }
        joined.validate()?;
        Ok(joined)
    }
pub(super) fn composite(&mut self, root: usize) -> Result<Curve> {
        let mut stack = vec![CompositeVisit::Enter(root, 0)];
        let mut active = BTreeSet::new();
        let mut memo = BTreeMap::<usize, Curve>::new();
        let mut segments = Vec::<CompositeSegment>::new();
        let mut result = None;
        while let Some(visit) = stack.pop() {
            self.budget(root, 1)?;
            match visit {
                CompositeVisit::Enter(de, depth) => {
                    if active.contains(&de) {
                        return Err(composite_error(
                            "BREP_IGES_V2_COMPOSITE_CYCLE",
                            format!("IGES composite cycle at DE {de}"),
                        ));
                    }
                    if let Some(curve) = memo.get(&de) {
                        if segments.len() >= MAX_COMPOSITE_SEGMENTS {
                            return Err(composite_error(
                                "BREP_IGES_V2_COMPOSITE_BUDGET",
                                format!("IGES composite DE {root} segment budget exceeded"),
                            ));
                        }
                        segments.push(CompositeSegment {
                            de,
                            curve: curve.clone(),
                        });
                        continue;
                    }
                    let entity = self.entities.get(&de).ok_or_else(|| {
                        composite_error(
                            "BREP_IGES_V2_COMPOSITE_DANGLING",
                            format!("Missing IGES composite segment DE {de}"),
                        )
                    })?;
                    if entity.directory.ty != 102 {
                        let curve = self.base_curve(root, de, true)?;
                        if segments.len() >= MAX_COMPOSITE_SEGMENTS {
                            return Err(composite_error(
                                "BREP_IGES_V2_COMPOSITE_BUDGET",
                                format!("IGES composite DE {root} segment budget exceeded"),
                            ));
                        }
                        segments.push(CompositeSegment { de, curve });
                        continue;
                    }
                    if depth >= MAX_COMPOSITE_DEPTH {
                        return Err(composite_error(
                            "BREP_IGES_V2_COMPOSITE_BUDGET",
                            format!("IGES composite DE {root} depth budget exceeded at DE {de}"),
                        ));
                    }
                    let refs = entity_references(de, entity)?;
                    if refs.is_empty() {
                        return Err(composite_error(
                            "BREP_IGES_V2_COMPOSITE_DANGLING",
                            format!("IGES composite DE {de} has no segments"),
                        ));
                    }
                    active.insert(de);
                    stack.push(CompositeVisit::Exit(de, segments.len()));
                    for reference in refs.iter().rev() {
                        self.budget(root, 1)?;
                        stack.push(CompositeVisit::Enter(reference.target, depth + 1));
                    }
                }
                CompositeVisit::Exit(de, start) => {
                    active.remove(&de);
                    let curve = Self::join_segments(de, &segments[start..], self.tolerance_mm)?;
                    memo.insert(de, curve.clone());
                    if de == root {
                        result = Some(curve);
                    }
                }
            }
        }
        result.ok_or_else(|| {
            composite_error(
                "BREP_IGES_V2_COMPOSITE_DANGLING",
                format!("IGES composite DE {root} produced no curve"),
            )
        })
    }
pub(super) fn dispatch(mut self, de: usize) -> Result<Curve> {
        let is_composite = self
            .entities
            .get(&de)
            .ok_or_else(|| refuse(format!("Missing IGES curve DE {de}")))?
            .directory
            .ty
            == 102;
        if is_composite {
            self.composite(de)
        } else {
            self.base_curve(de, de, false)
        }
    }
}
