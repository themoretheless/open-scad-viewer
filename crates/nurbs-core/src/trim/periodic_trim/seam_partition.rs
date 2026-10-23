//! Seam partition: preserves the original arithmetic and proof budgets.
use super::*;

/// End-to-end conservative split for exact isolated seam events. Non-point
/// intervals are retained unless an exact original linear residual proves a
/// binary64 parameter; sampled near-zero residuals never qualify a root.
/// Point events include endpoint/tangent events; simultaneous U/V events are
/// deduplicated as cut parameters while their separate proofs remain in search.
/// Search and deviation have separately explicit budgets (each <=100000).
pub fn split_verified(
    source: &Curve,
    chart: [[f64; 2]; 2],
    parameter_tolerance: f64,
    uv_tolerance: f64,
    max_pieces: usize,
    max_seams: usize,
    search_work: usize,
    admission_work: usize,
) -> Result<SplitReport> {
    check(
        uv_tolerance.is_finite()
            && uv_tolerance > 0.
            && (1..=256).contains(&max_pieces)
            && (1..=100000).contains(&admission_work),
        "Invalid verified split tolerance or budget",
    )?;
    let search = inspect_seams(source, chart, parameter_tolerance, max_seams, search_work)?;
    let mut result = SplitReport {
        status: SplitStatus::SearchIncomplete,
        search,
        resolved_events: Vec::new(),
        approximated_events: Vec::new(),
        parameter_attempts: 0,
        admission: None,
        connectivity: None,
        partition: None,
    };
    if result.search.status != SearchStatus::Complete {
        return Ok(result);
    }
    // Error-free TwoSum and FMA product residuals certify each authored
    // period and materialized seam, within a range excluding underflow.
    let bounded = |x: f64| x == 0. || (x.abs() >= 2f64.powi(-500) && x.abs() <= 2f64.powi(500));
    let exact_sum = |a: f64, b: f64, sum: f64| {
        let virtual_b = sum - a;
        let residual = (a - (sum - virtual_b)) + (b - virtual_b);
        sum.is_finite() && residual == 0.
    };
    let exact_lattice = chart.iter().all(|r| {
        bounded(r[0])
            && bounded(r[1])
            && bounded(r[1] - r[0])
            && exact_sum(r[1], -r[0], r[1] - r[0])
    }) && result.search.seams.iter().all(|seam| {
        let r = chart[seam.axis];
        let period = r[1] - r[0];
        let k = seam.tile_boundary as f64;
        let product = k * period;
        k.mul_add(period, -product) == 0. && exact_sum(r[0], product, seam.coordinate)
    });
    if !exact_lattice {
        result.status = SplitStatus::LatticeRoundingUnresolved;
        return Ok(result);
    }
    let [a, b] = source.domain();
    let mut cuts = Vec::new();
    let mut spatial = source.clone();
    for p in &mut spatial.control_points {
        p.push(0.);
    }
    for seam in &result.search.seams {
        for event in seam.roots.root_events() {
            let bounds = event.parameter_bounds;
            let t = if bounds[0] == bounds[1] {
                bounds[0]
            } else {
                // Try a bounded set of binary64 proposals, but admit one only
                // through the exact original homogeneous residual predicate.
                let proposals = [bounds[0] * 0.5 + bounds[1] * 0.5, bounds[0], bounds[1]];
                let mut normal = [0.; 3];
                normal[seam.axis] = 1.;
                let span = seam.roots.cells[event.cells[0]].span;
                let mut exact = None;
                for candidate in proposals {
                    result.parameter_attempts += 1;
                    if bounds[0] <= candidate
                        && candidate <= bounds[1]
                        && source.knots[span] <= candidate
                        && candidate <= source.knots[span + 1]
                        && (crate::curve_plane::exact_linear_parameter(
                            &spatial,
                            span,
                            candidate,
                            normal,
                            seam.coordinate,
                        ) || crate::exact_curve_segments::coordinate_incidence(
                            source,
                            span,
                            candidate,
                            seam.axis,
                            seam.coordinate,
                        ) == Some(true))
                    {
                        exact = Some(candidate);
                        break;
                    }
                }
                let Some(parameter) = exact else {
                    result.status = SplitStatus::IntervalRootNeedsConstruction;
                    return Ok(result);
                };
                result.resolved_events.push(ExactSeamEvent {
                    axis: seam.axis,
                    coordinate: seam.coordinate,
                    original_bounds: bounds,
                    parameter,
                });
                parameter
            };
            if a < t && t < b {
                cuts.push(t);
            }
        }
    }
    cuts.sort_by(f64::total_cmp);
    cuts.dedup();
    if cuts.len() >= max_pieces {
        result.status = SplitStatus::PieceLimit;
        return Ok(result);
    }
    let zero_tiles = vec![[0, 0]; cuts.len() + 1];
    let preliminary = split_candidates(source, chart, &cuts, &zero_tiles, max_pieces)?;
    let mut tiles = Vec::new();
    for piece in &preliminary.pieces {
        let mut tile = [0i32; 2];
        for axis in 0..2 {
            let lo = piece
                .curve
                .control_points
                .iter()
                .map(|p| p[axis])
                .fold(f64::INFINITY, f64::min);
            let k = result
                .search
                .seams
                .iter()
                .filter(|seam| seam.axis == axis && seam.coordinate <= lo)
                .map(|seam| seam.tile_boundary)
                .max();
            let Some(k) = k else {
                result.status = SplitStatus::TileContainmentUnresolved;
                return Ok(result);
            };
            tile[axis] = k;
        }
        tiles.push(tile);
    }
    let partition = split_candidates(source, chart, &cuts, &tiles, max_pieces)?;
    if !partition.candidate_hulls_contained {
        result.status = SplitStatus::TileContainmentUnresolved;
        return Ok(result);
    }
    let connectivity = inspect_connectivity(&partition, chart, None, max_pieces)?;
    let connected = connectivity.status == LinkStatus::Exact;
    result.connectivity = Some(connectivity);
    if !connected {
        result.status = SplitStatus::ConnectivityUnresolved;
        return Ok(result);
    }
    let admission = verify_partition(source, &partition, uv_tolerance, admission_work)?;
    let accepted = admission.status == AdmissionStatus::WithinTolerance;
    result.admission = Some(admission);
    if accepted {
        result.status = SplitStatus::VerifiedUvPartition;
        result.partition = Some(partition);
    } else {
        result.status = SplitStatus::DeviationUnresolved;
    }
    Ok(result)
}

/// Optional tolerance-qualified alternative to split_verified. A proposed cut
/// inside a non-point isolated root interval is snapped geometrically to its
/// seam; the original parameter is NOT claimed to be a root. All modified
/// controls then undergo continuous UV admission and exact lifted connectivity.
/// Overlapping non-point U/V events remain unresolved instead of guessed aliases.
/// This certifies candidate geometry only; it does not transfer region topology.
pub fn split_approximated(
    source: &Curve,
    chart: [[f64; 2]; 2],
    parameter_tolerance: f64,
    uv_tolerance: f64,
    max_pieces: usize,
    max_seams: usize,
    search_work: usize,
    admission_work: usize,
) -> Result<SplitReport> {
    let mut result = split_verified(
        source,
        chart,
        parameter_tolerance,
        uv_tolerance,
        max_pieces,
        max_seams,
        search_work,
        admission_work,
    )?;
    if result.status != SplitStatus::IntervalRootNeedsConstruction {
        return Ok(result);
    }
    result.resolved_events.clear();
    let [a, b] = source.domain();
    let mut events = Vec::new();
    for seam in &result.search.seams {
        for event in seam.roots.root_events() {
            let bounds = event.parameter_bounds;
            let t = if bounds[0] == bounds[1] {
                bounds[0]
            } else {
                bounds[0] * 0.5 + bounds[1] * 0.5
            };
            if bounds[0] == bounds[1] && (t == a || t == b) {
                continue;
            }
            if !(a < t && t < b) {
                result.status = SplitStatus::EventOrderUnresolved;
                return Ok(result);
            }
            events.push(ApproximatedEvent {
                axis: seam.axis,
                coordinate: seam.coordinate,
                parameter_bounds: bounds,
                parameter: t,
            });
        }
    }
    events.sort_by(|x, y| {
        x.parameter_bounds[0]
            .total_cmp(&y.parameter_bounds[0])
            .then(x.parameter_bounds[1].total_cmp(&y.parameter_bounds[1]))
    });
    for pair in events.windows(2) {
        if pair[0].parameter_bounds[1] >= pair[1].parameter_bounds[0] {
            let same_point = pair[0].parameter_bounds[0] == pair[0].parameter_bounds[1]
                && pair[0].parameter_bounds == pair[1].parameter_bounds;
            if !same_point {
                result.status = SplitStatus::EventOrderUnresolved;
                return Ok(result);
            }
        }
    }
    let mut cuts: Vec<_> = events.iter().map(|e| e.parameter).collect();
    cuts.dedup();
    if cuts.len() >= max_pieces {
        result.status = SplitStatus::PieceLimit;
        return Ok(result);
    }
    let mut partition = split_candidates(
        source,
        chart,
        &cuts,
        &vec![[0, 0]; cuts.len() + 1],
        max_pieces,
    )?;
    for (index, &t) in cuts.iter().enumerate() {
        let mut point = source.evaluate(t)?.point;
        for event in events.iter().filter(|e| e.parameter == t) {
            point[event.axis] = event.coordinate;
        }
        let last = partition.pieces[index].curve.control_points.len() - 1;
        partition.pieces[index].curve.control_points[last] = point.clone();
        partition.pieces[index + 1].curve.control_points[0] = point;
    }
    for piece in &mut partition.pieces {
        let mut tile = [0i32; 2];
        for axis in 0..2 {
            let lo = piece
                .curve
                .control_points
                .iter()
                .map(|p| p[axis])
                .fold(f64::INFINITY, f64::min);
            let k = result
                .search
                .seams
                .iter()
                .filter(|s| s.axis == axis && s.coordinate <= lo)
                .map(|s| s.tile_boundary)
                .max();
            let Some(k) = k else {
                result.status = SplitStatus::TileContainmentUnresolved;
                return Ok(result);
            };
            tile[axis] = k;
        }
        piece.tile = tile;
        piece.translation =
            std::array::from_fn(|axis| tile[axis] as f64 * (chart[axis][1] - chart[axis][0]));
        for p in &mut piece.curve.control_points {
            for axis in 0..2 {
                p[axis] -= piece.translation[axis];
            }
        }
        piece.curve.validate()?;
    }
    partition.candidate_hulls_contained = partition.pieces.iter().all(|piece| {
        piece
            .curve
            .control_points
            .iter()
            .all(|p| (0..2).all(|axis| chart[axis][0] <= p[axis] && p[axis] <= chart[axis][1]))
    });
    result.approximated_events = events
        .into_iter()
        .filter(|e| e.parameter_bounds[0] != e.parameter_bounds[1])
        .collect();
    if !partition.candidate_hulls_contained {
        result.status = SplitStatus::TileContainmentUnresolved;
        return Ok(result);
    }
    let connectivity = inspect_connectivity(&partition, chart, None, max_pieces)?;
    let connected = connectivity.status == LinkStatus::Exact;
    result.connectivity = Some(connectivity);
    if !connected {
        result.status = SplitStatus::ConnectivityUnresolved;
        return Ok(result);
    }
    let admission = verify_partition(source, &partition, uv_tolerance, admission_work)?;
    let admitted = admission.status == AdmissionStatus::WithinTolerance;
    result.admission = Some(admission);
    if admitted {
        result.partition = Some(partition);
        result.status = SplitStatus::VerifiedUvPartition;
    } else {
        result.status = SplitStatus::DeviationUnresolved;
    }
    Ok(result)
}
