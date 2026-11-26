use super::*;
use nurbs_core::surface::Surface;

/// Build the source authority from authored UV contours, a support chart and
/// an explicit winding value at one interior UV point.
pub fn sew_periodic_source_face(
    surface: &Surface,
    curves: &[Curve],
    contours: &[Vec<OriginalPeriodicSourceUse>],
    wrapped: [bool; 2],
    base_uv: [f64; 2],
    base_winding: i64,
    probe_radius: f64,
    max_work: usize,
    limits: PeriodicSewnFaceLimits,
) -> Result<OwnedPeriodicSewnFaces> {
    let _replay = super::periodic_replay::scope();
    surface.validate()?;
    if !wrapped.iter().any(|&axis| axis)
        || !probe_radius.is_finite()
        || probe_radius <= 0.
        || !base_uv.iter().all(|x| x.is_finite())
        || !(1..=MAX_ALLOWANCE).contains(&max_work)
    {
        return Err(refused(
            "Invalid periodic chart, winding anchor, probe radius or work limit",
        ));
    }
    let base_uv = base_uv.map(|x| ratio(&Q::from_float(x).unwrap()));
    let probe_radius = ratio(&Q::from_float(probe_radius).unwrap());
    let chart = [
        [
            surface.knots_u[surface.degree_u],
            surface.knots_u[surface.control_points.len()],
        ],
        [
            surface.knots_v[surface.degree_v],
            surface.knots_v[surface.control_points[0].len()],
        ],
    ];
    let parameter_limits = ParameterOrderLimits::default();
    let width = 1e-12;
    let pieces = Arc::new(own_original_periodic_pieces(
        curves,
        chart,
        wrapped,
        width,
        parameter_limits,
        64,
    )?);
    let bound = Arc::new(bind_original_periodic_pieces_to_surface(
        pieces,
        surface,
        parameter_limits,
    )?);
    let original = Arc::new(own_original_periodic_wires(bound, contours)?);
    let support = Arc::new(own_periodic_support_boundaries(
        original,
        width,
        parameter_limits,
        64,
        max_work,
    )?);
    let wires = support.augmented().clone();
    let pieces = wires.surface().pieces().clone();
    let count = pieces.pieces().len();
    let pairs = count
        .checked_mul(count + 1)
        .ok_or_else(|| refused("Periodic pair count overflow"))?
        / 2;
    let copies = 3usize.pow(wrapped.iter().filter(|&&x| x).count() as u32);
    let pair_work = (pairs * copies).max(1);
    if pair_work > max_work {
        return Err(refused(
            "Periodic pair census exceeds the authored work limit",
        ));
    }
    let embedding = Arc::new(own_original_periodic_embedding(
        wires,
        limits.max_regularity_cells,
        limits.atlas,
        limits.pairs,
    )?);
    let cuts = Arc::new(own_periodic_arrangement_cuts(
        pieces,
        width,
        parameter_limits,
        pairs.max(1),
    )?);
    let seams = Arc::new(own_periodic_seam_contacts(
        embedding.clone(),
        width,
        parameter_limits,
        (pairs * (copies - 1)).max(1),
    )?);
    let combined = Arc::new(own_periodic_quotient_arrangement_cuts(
        cuts,
        seams,
        parameter_limits,
        pair_work,
    )?);
    let aligned = Arc::new(own_periodic_aligned_intervals(
        combined,
        parameter_limits,
        max_work,
    )?);
    let graph = Arc::new(own_periodic_quotient_graph(aligned, parameter_limits)?);
    let charts = Arc::new(own_periodic_vertex_charts(graph, 192, max_work)?);
    let jumps = Arc::new(own_periodic_wire_jumps(charts, max_work)?);
    let base = Arc::new(own_periodic_relative_winding(
        jumps.clone(),
        &[base_uv.clone(), base_uv.clone()],
        parameter_limits,
        max_work,
    )?);
    let rule = Arc::new(own_periodic_support_material_rule(
        support.clone(),
        base,
        &[OriginalPeriodicOperand {
            wire_weights: vec![1; contours.len()],
            base_winding: base_winding.to_string(),
            fill: OriginalPeriodicFillRule::NonZero,
        }],
        &[OriginalPeriodicBooleanOp::Operand(0)],
    )?);
    let edge_count = jumps.owner().owner().edges().len();
    if edge_count > limits.max_edges {
        return Err(refused(
            "Periodic material side count exceeds the authored edge limit",
        ));
    }
    let mut sides = Vec::with_capacity(edge_count);
    for edge in 0..edge_count {
        let probes = Arc::new(own_periodic_support_side_probes(
            support.clone(),
            jumps.clone(),
            edge,
            probe_radius.clone(),
            parameter_limits,
        )?);
        let mut samples = [None, None];
        for (side, sample) in samples.iter_mut().enumerate() {
            if !probes.exterior()[side] {
                let winding = Arc::new(own_periodic_relative_winding(
                    jumps.clone(),
                    &[base_uv.clone(), probes.queries()[side].clone()],
                    parameter_limits,
                    max_work,
                )?);
                *sample = Some(Arc::new(own_periodic_material_sample(
                    rule.material().clone(),
                    winding,
                )?));
            }
        }
        sides.push(Arc::new(own_periodic_support_sides(
            probes,
            rule.clone(),
            samples,
        )?));
    }
    let ports = Arc::new(own_periodic_ports(jumps.owner().clone(), 192)?);
    let rotation = Arc::new(own_periodic_rotation(
        ports,
        ContactPortOrderLimits::default(),
    )?);
    let selection = Arc::new(own_periodic_support_selection(rule, &sides)?);
    let cycles = Arc::new(own_periodic_support_cycles(
        selection,
        rotation,
        192,
        limits.max_edges,
    )?);
    let lifts = Arc::new(own_periodic_support_lifts(cycles, 192, limits.max_edges)?);
    let signatures = Arc::new(own_periodic_support_signatures(
        lifts,
        parameter_limits,
        max_work,
    )?);
    let regions = construct_periodic_support_regions(
        signatures,
        parameter_limits,
        PeriodicRegionSearchLimits {
            max_work,
            max_region_work: max_work,
            ..Default::default()
        },
    )?;
    let regions = Arc::new(
        regions
            .regions()
            .cloned()
            .ok_or_else(|| refused("Could not resolve the periodic material regions"))?,
    );
    let traces = Arc::new(own_periodic_support_traces(
        regions,
        parameter_limits,
        max_work,
    )?);
    let incidence = Arc::new(own_periodic_support_incidence(traces, max_work)?);
    let copies = Arc::new(own_periodic_source_chart_copies(
        incidence,
        parameter_limits,
        Default::default(),
    )?);
    let corners = Arc::new(own_periodic_native_chart_corners(copies, max_work)?);
    let shores = Arc::new(own_periodic_native_chart_shores(
        corners,
        parameter_limits,
        Default::default(),
    )?);
    let samples = Arc::new(own_periodic_shore_material_samples(
        shores,
        parameter_limits,
        Default::default(),
    )?);
    let intervals = Arc::new(own_periodic_shore_material_intervals(
        samples,
        parameter_limits,
        Default::default(),
    )?);
    let winding =
        construct_periodic_chart_winding(intervals, Default::default(), Default::default())?;
    let loops = Arc::new(own_periodic_chart_boundary_loops(
        winding.owner().clone(),
        Default::default(),
    )?);
    let winding = Arc::new(own_periodic_chart_loop_winding(loops, Default::default())?);
    let patches = Arc::new(own_periodic_chart_fan_patches(winding, Default::default())?);
    own_with_embedding(patches, embedding, limits)
}
