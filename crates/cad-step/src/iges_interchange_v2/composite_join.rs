use super::*;

impl CurveDispatcher<'_> {
    pub(super) fn join_segments(
        &mut self,
        composite: usize,
        segments: &[CompositeSegment],
        tolerance_mm: f64,
    ) -> Result<Curve> {
        let linear = segments.iter().all(|segment| {
            let c = &segment.curve;
            c.degree == 1 && !c.periodic && c.weights.iter().all(|w| *w == c.weights[0])
        });
        // Retain the historical polyline parameterization only on the all-linear
        // path. Rational verification needs each original knot vector and weight.
        let polylines;
        let segments = if linear {
            polylines = segments
                .iter()
                .map(|s| {
                    Ok(CompositeSegment {
                        de: s.de,
                        curve: Curve::from_polyline(s.curve.control_points.clone())?,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            &polylines[..]
        } else {
            segments
        };
        let Some(first) = segments.first() else {
            return Err(composite_error(
                "BREP_IGES_V2_COMPOSITE_DANGLING",
                format!("IGES composite DE {composite} has no segments"),
            ));
        };
        let mut directions = vec![false; segments.len()];
        let (mut current_start, mut current_end) = Self::endpoints(&first.curve)?;
        let mut previous = first.de;
        for (index, segment) in segments.iter().enumerate().skip(1) {
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
                directions[0] = true;
                std::mem::swap(&mut current_start, &mut current_end);
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
            directions[index] = end_end <= tolerance_mm;
            current_end = if directions[index] {
                segment_start
            } else {
                segment_end
            };
            previous = segment.de;
        }
        if !linear {
            use nurbs_core::curve_partition_agreement::concatenate::{Input, concatenate};
            let inputs: Vec<_> = segments
                .iter()
                .zip(directions)
                .map(|(s, reversed)| Input {
                    curve: &s.curve,
                    reversed,
                })
                .collect();
            let proof = concatenate(&inputs, false, tolerance_mm, MAX_COMPOSITE_WORK - self.work)
                .map_err(|e| {
                composite_error(
                    "BREP_IGES_V2_COMPOSITE_UNSUPPORTED",
                    format!("IGES composite DE {composite}: {}", e.message),
                )
            })?;
            self.budget(composite, proof.visited())?;
            return Ok(proof.curve().clone());
        }
        let mut joined = if directions[0] {
            first.curve.reverse()?
        } else {
            first.curve.clone()
        };
        for (segment, reversed) in segments.iter().zip(directions).skip(1) {
            let (_, current_end) = Self::endpoints(&joined)?;
            let candidate = if reversed {
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
        }
        joined.validate()?;
        Ok(joined)
    }
}
