//! Transport adapter. All intersection, loop and finite-face work is native.
use crate::*;
use value_codec::Serialize;

pub(in crate::dispatch) fn closed_solid_policy(tolerance_uv: f64) -> brep_core::closed_solid::AdmissionPolicy {
    use brep_core::{boundary_embedding, closed_solid, face_contacts, volume_validity};
    closed_solid::AdmissionPolicy {
        tolerance_uv,
        max_linear_cells: 100_000,
        geometry: volume_validity::Limits {
            boundary: boundary_embedding::Limits {
                exact_work: 1_000_000,
                trim_pairs: 1000,
                trim_cells: 10_000,
                trim_domain_cells: 100_000,
                spans: 8192,
                contacts: face_contacts::Limits {
                    pairs: 1000,
                    cells: 100_000,
                    domain_cells: 1_000_000,
                    cells_per_pair: 1024,
                    domain_cells_per_pair: 10_000,
                },
            },
            nesting_pairs: 100,
            nesting_cells: 10_000,
            nesting_domain_cells: 100_000,
            orientation_cells: 10_000,
            orientation_domain_cells: 100_000,
            orientation_spans: 8192,
        },
    }
}

/// Execute the solid-level Boolean and retain its replayable CAD source.
pub(super) fn assemble_boolean(v: Value, network: bool) -> Result<Value> {
    use brep_core::{closed_solid, mixed_root_model};
    use closed_solid::BooleanMaterialOp;

    let keys = ["op", "operation", "a", "b", "toleranceMm", "toleranceUv", "volumeErrorMm3"];
    if !v.as_object().is_some_and(|object| {
        keys.iter().all(|key| object.contains_key(*key))
            && object.len() == keys.len() + usize::from(object.contains_key("volumeAxis"))
    }) {
        return Err(input("Closed SS Boolean request fields do not match schema"));
    }
    let operation = match v["operation"].as_str() {
        Some("union") => BooleanMaterialOp::Union,
        Some("intersection") => BooleanMaterialOp::Intersection,
        Some("difference") => BooleanMaterialOp::Difference,
        Some("xor") => BooleanMaterialOp::Xor,
        _ => return Err(input("Unknown closed SS Boolean operation")),
    };
    let tolerance_mm: f64 = field(&v, "toleranceMm")?;
    let tolerance_uv: f64 = field(&v, "toleranceUv")?;
    let volume_error: f64 = field(&v, "volumeErrorMm3")?;
    let volume_axis = v.get("volumeAxis").map(|value| {
        match value.as_u64() {
            Some(axis @ 0..=2) => Ok(axis as usize),
            _ => Err(input("Closed SS Boolean volumeAxis must be 0, 1, or 2")),
        }
    }).transpose()?;
    if [tolerance_mm, tolerance_uv, volume_error].iter().any(|x| !x.is_finite() || *x <= 0.) {
        return Err(input("Closed SS Boolean tolerances must be finite and positive"));
    }
    let policy = closed_solid_policy(tolerance_uv);
    let admit = |key| -> Result<closed_solid::QualifiedClosedSolid> {
        closed_solid::qualify(field(&v, key)?, policy)?
            .into_solid().ok_or_else(|| input("Closed SS Boolean operand is not a qualified solid"))
    };
    let a = admit("a")?;
    let b = admit("b")?;
    let mut limits = mixed_root_model::ClosedSsBooleanLimits::default();
    limits.volume.absolute_error = volume_error;
    if let Some(axis) = volume_axis {
        limits.volume.axis = axis;
    }
    if network {
        limits.cells_per_support = 1024;
    } else {
        limits.max_pair_attempts = 4;
    }
    limits.census.discovery.max_charts = 192;
    limits.census.discovery.max_repairs = 128;
    limits.census.discovery.seed_steps = 32;
    limits.census.discovery.proposal_rectangles = 250_000;
    limits.census.discovery.coverage.max_cells = 65_536;
    limits.census.discovery.coverage.max_rectangles = 100_000;
    limits.census.discovery.coverage.max_depth = 32;
    limits.census.discovery.components.max_bridges = 191;
    limits.census.cycle.max_steps = 256;
    limits.census.cycle.max_candidates = 8192;
    limits.census.cycle.max_start_attempts = 64;
    limits.census.coverage.max_cells = 65_536;
    limits.census.coverage.max_rectangles = 1_000_000;
    limits.census.coverage.max_depth = 48;
    for cut in &mut limits.census.cuts.cuts {
        cut.tolerance_uv = tolerance_uv;
    }
    let directions = [[0., 0., 1.], [1., 0.317, 0.173], [0.239, 1., 0.419]];
    use mixed_root_model::QualifiedInteriorSsCensus;
    let solids = [&a, &b];
    let (censuses, body, volume_bounds) = if network {
        QualifiedInteriorSsCensus::assemble_closed_ss_network_boolean_between(
            solids, operation, &directions, tolerance_mm, limits,
        )?
    } else {
        let (census, body, volume) = QualifiedInteriorSsCensus::assemble_closed_ss_boolean_between(
            solids, operation, &directions, tolerance_mm, limits,
        )?;
        (vec![census], body, volume)
    };
    let topology = body.topology();
    let mut report = json!({
        "source": v,
        "branches": censuses.iter().map(|census| census.components().len()).sum::<usize>(),
        "bodies": topology.bodies.len(),
        "faces": topology.faces.len(),
        "edges": topology.edges.len(),
        "boundaryEdges": topology.boundary_edges().len(),
        "volumeBoundsMm3": volume_bounds
    });
    if network {
        report["facePairs"] = json!(censuses.iter().map(|census| json!({
            "faces": census.face_indices(),
            "coverageDomain": census.coverage().domain(),
            "branches": census.components().len()
        })).collect::<Vec<_>>());
        report["topology"] = json!({
            "vertices": topology.vertices.len(),
            "edges": topology.edges.iter().map(|edge| json!({
                "vertices": edge.vertices,
                "degenerate": edge.degenerate,
                "root": matches!(&edge.curve, mixed_root_model::MixedEdge::Root(_))
            })).collect::<Vec<_>>(),
            "loops": topology.loops.iter().map(|wire| wire.coedges.iter().map(|coedge| json!({
                "edge": coedge.edge, "reversed": coedge.reversed
            })).collect::<Vec<_>>()).collect::<Vec<_>>(),
            "faces": topology.faces.iter().map(|face| json!({
                "outer": face.outer, "holes": face.holes
            })).collect::<Vec<_>>(),
            "shells": topology.shells.iter().map(|shell| {
                shell.faces.iter().map(|face| json!({
                    "face": face.face, "reversed": face.reversed
                })).collect::<Vec<_>>()
            }).collect::<Vec<_>>(),
            "bodies": topology.bodies.iter().map(|body| json!({
                "outerShell": body.outer_shell, "innerShells": body.inner_shells
            })).collect::<Vec<_>>()
        });
    } else {
        report["coverageDomain"] = json!(censuses[0].coverage().domain());
    }
    Ok(report)
}

pub(super) fn partition(v: Value) -> Result<Value> {
    let object = v.as_object().ok_or_else(|| {
        nurbs_core::Error::new("BREP_CLOSED_SS_INPUT", "Expected closed SS request")
    })?;
    let keys = [
        "op",
        "a",
        "b",
        "faceA",
        "faceB",
        "toleranceMm",
        "toleranceUv",
    ];
    if object.len() != keys.len() || keys.iter().any(|k| !object.contains_key(*k)) {
        return Err(nurbs_core::Error::new(
            "BREP_CLOSED_SS_INPUT",
            "Closed SS request fields do not match schema",
        ));
    }
    let a: brep_core::Model = field(&v, "a")?;
    let b: brep_core::Model = field(&v, "b")?;
    let fa: usize = field(&v, "faceA")?;
    let fb: usize = field(&v, "faceB")?;
    let tolerance_world: f64 = field(&v, "toleranceMm")?;
    let tolerance_uv: f64 = field(&v, "toleranceUv")?;
    let mut limits = brep_core::face_cut::ClosedSsLimits::default();
    limits.trace.tolerance_world = tolerance_world;
    limits.trace.tolerance_uv = tolerance_uv;
    let result = brep_core::face_cut::partition_closed_ss(&a, fa, &b, fb, limits)?;
    Ok(
        json!({"a":result.first.model.to_value(),"b":result.second.model.to_value(),
        "facesA":result.first.faces,"facesB":result.second.faces,
        "coverageDomain":result.coverage_domain,"originalSourceClosedCoverage":result.covers_entire_surfaces,"materialClassified":false,
        "segments":result.trace.segments().len(),"cells":result.trace.cells(),
        "worldErrorUpper":result.trace.world_error_upper(),"uvErrorUpper":result.trace.uv_error_upper()}),
    )
}
pub(super) fn network(v: Value) -> Result<Value> {
    let object = v.as_object().ok_or_else(|| {
        nurbs_core::Error::new("BREP_CLOSED_SS_INPUT", "Expected closed SS network request")
    })?;
    let required = [
        "op",
        "a",
        "b",
        "faceA",
        "faceB",
        "toleranceMm",
        "toleranceUv",
    ];
    if required.iter().any(|k| !object.contains_key(*k))
        || object.keys().any(|k| {
            !required.contains(&k.as_str()) && k != "maxRepairs" && k != "maxStartAttempts"
        })
    {
        return Err(nurbs_core::Error::new(
            "BREP_CLOSED_SS_INPUT",
            "Closed SS network request fields do not match schema",
        ));
    }
    let a: brep_core::Model = field(&v, "a")?;
    let b: brep_core::Model = field(&v, "b")?;
    let mut limits = brep_core::face_cut::ClosedSsLimits::default();
    limits.trace.tolerance_world = field(&v, "toleranceMm")?;
    limits.trace.tolerance_uv = field(&v, "toleranceUv")?;
    if object.contains_key("maxRepairs") {
        limits.branches.max_repairs = field(&v, "maxRepairs")?;
    }
    if let Some(value) = object.get("maxStartAttempts") {
        let attempts = value
            .as_u64()
            .filter(|attempts| (1..=128).contains(attempts))
            .ok_or_else(|| {
                nurbs_core::Error::new("BREP_CLOSED_SS_INPUT", "Invalid maxStartAttempts")
            })?;
        limits.cycle.max_start_attempts = attempts as usize;
    }
    let result = brep_core::face_cut::partition_all_closed_ss(
        &a,
        field(&v, "faceA")?,
        &b,
        field(&v, "faceB")?,
        limits,
    )?;
    let world = result
        .traces
        .iter()
        .map(|t| t.world_error_upper())
        .fold(0_f64, f64::max);
    let uv: [f64; 2] = std::array::from_fn(|k| {
        result
            .traces
            .iter()
            .map(|t| t.uv_error_upper()[k])
            .fold(0_f64, f64::max)
    });
    Ok(
        json!({"a":result.models[0].to_value(),"b":result.models[1].to_value(),"facesA":result.faces[0],"facesB":result.faces[1],
        "coverageDomain":result.coverage_domain,"originalSourceClosedCoverage":result.covers_entire_surfaces,"materialClassified":false,"branches":result.traces.len(),
        "segments":result.traces.iter().map(|t|t.segments().len()).sum::<usize>(),"placementAttempts":result.placement_attempts,
        "worldErrorUpper":world,"uvErrorUpper":uv}),
    )
}

/// Intersect two admitted source bodies and return the replayable native result.
pub(super) fn intersect_periodic_stock_band(v: Value) -> Result<Value> {
    use brep_core::{closed_solid, root_model::QualifiedRootShellPositiveFlux, source_box};

    let keys = ["op", "side", "stock", "toleranceUv"];
    if !v.as_object().is_some_and(|object| {
        object.len() == keys.len() && keys.iter().all(|key| object.contains_key(*key))
    }) {
        return Err(input("Periodic stock intersection request fields do not match schema"));
    }
    let tolerance_uv: f64 = field(&v, "toleranceUv")?;
    if !tolerance_uv.is_finite() || tolerance_uv <= 0. {
        return Err(input("Periodic stock intersection needs a positive finite UV tolerance"));
    }
    let model: brep_core::Model = field(&v, "side")?;
    let stock: brep_core::Model = field(&v, "stock")?;
    let policy = closed_solid_policy(tolerance_uv);
    let side = closed_solid::qualify(model, policy)?.into_solid()
        .ok_or_else(|| input("Periodic stock intersection needs a qualified side solid"))?;
    let budget = source_box::Limits {
        operations: 100000,
        body: brep_core::source_body_restore::Limits {
            shell: brep_core::source_shell_restore::Limits {
                exact_work: 1000000,
                driver_cells: 10000,
                regions: brep_core::source_region_restore::Limits {
                    region: brep_core::trimmed_face_recipe::Limits {
                        pairs: policy.geometry.boundary.trim_pairs,
                        region_cells: policy.geometry.boundary.trim_cells,
                        domain_cells: policy.geometry.boundary.trim_domain_cells,
                        agreement_cells: 10000,
                    },
                    search_cells: 10000,
                    point_checks: 10000,
                    mapping_cells: 10000,
                    driver_cells: 10000,
                    membership_cells: 10000,
                    controls: 10000,
                    winding_cells: 10000,
                    steps: 16,
                },
            },
            embedding: brep_core::source_shell_geometry::Limits {
                tolerance_uv, corners: 1000, spans: policy.geometry.boundary.spans,
                linear_cells: policy.max_linear_cells, pairs: policy.geometry.boundary.contacts,
                exact_work: policy.geometry.boundary.exact_work, driver_cells: 10000,
            },
            volume: brep_core::source_volume::Limits {
                axis: 2, origin: 0., absolute_error: 1e-6, tolerance_uv,
                cells: 10000, spans: 10000, domain_cells: 100000,
            },
        },
    };
    let stock = source_box::qualify(&stock, &budget)?;
    let body = QualifiedRootShellPositiveFlux::intersect_periodic_stock_band(
        &side, &stock, Default::default(), periodic_band_limits(2, 0.),
    )?;
    periodic_band_report(v, &body)
}

fn periodic_band_limits(axis: usize, origin: f64) -> brep_core::root_model::RootPeriodicBandLimits {
    use brep_core::root_model::{RootPeriodicBandLimits, RootShellFluxLimits};
    RootPeriodicBandLimits {
        max_chart_attempts: 64,
        max_orientation_probes: 128,
        max_chart_spans: 8192,
        max_contact_cells: 4096,
        flux: RootShellFluxLimits {
            axis, origin, max_cells: 2048, max_spans: 8192, max_depth: 16,
            winding: Default::default(),
        },
    }
}

/// Return a replayable original-source definition and its qualification summary.
/// The exact root topology can be reconstructed by executing the same request.
pub(super) fn qualify_periodic_band(v: Value) -> Result<Value> {
    use brep_core::root_model::QualifiedRootShellPositiveFlux;
    use nurbs_core::ss_intersection as ss;

    let keys = ["op", "side", "planes", "toleranceMm"];
    if !v.as_object().is_some_and(|object| {
        object.len() == keys.len() && keys.iter().all(|key| object.contains_key(*key))
    }) {
        return Err(input("Periodic band request fields do not match schema"));
    }
    let side: nurbs_core::surface::Surface = field(&v, "side")?;
    let [a, b]: [nurbs_core::surface::Surface; 2] = field(&v, "planes")?;
    let tolerance: f64 = field(&v, "toleranceMm")?;
    if !(tolerance.is_finite() && tolerance > 0.) {
        return Err(input(
            "Periodic band toleranceMm must be finite and positive",
        ));
    }
    let a_source = ss::FiberSources::new(&side, &a)?;
    let b_source = ss::FiberSources::new(&side, &b)?;
    let first = ss::qualify_periodic_plane_ss(&a_source, Default::default())?;
    let second = ss::qualify_periodic_plane_ss(&b_source, Default::default())?;
    let axis = first
        .support_proof()
        .axial_coordinate
        .ok_or_else(|| input("Periodic band needs a proved axial coordinate"))?;
    let body = QualifiedRootShellPositiveFlux::qualify_periodic_band(
        [&first, &second],
        tolerance,
        periodic_band_limits(axis, a.control_points[0][0][axis].midpoint(b.control_points[0][0][axis])),
    )?;
    periodic_band_report(v, &body)
}

fn periodic_band_report(v: Value, body: &brep_core::root_model::QualifiedRootShellPositiveFlux) -> Result<Value> {
    let model = body.to_body()?;
    let topology = model.topology();
    Ok(json!({
        "source": v,
        "bodies": topology.bodies.len(),
        "faces": topology.faces.len(),
        "edges": topology.edges.len(),
        "boundaryEdges": topology.boundary_edges().len(),
        "volumeBoundsMm3": body.report().signed_bounds,
    }))
}

/// Return exact original UV coordinates for a certified whole-support contact.
pub(super) fn qualify_isolated_contact(v: Value) -> Result<Value> {
    use nurbs_core::{ss_intersection as ss, surface::Surface};

    let keys = ["op", "a", "b"];
    if !v.as_object().is_some_and(|object| {
        object.len() == keys.len() && keys.iter().all(|key| object.contains_key(*key))
    }) {
        return Err(input(
            "Isolated SS contact request fields do not match schema",
        ));
    }
    let a: Surface = field(&v, "a")?;
    let b: Surface = field(&v, "b")?;
    let source = ss::FiberSources::new(&a, &b)?;
    if let Some(contact) = ss::discover_isolated_graph_contact(&source)? {
        let [au, av, bu, bv] = contact.native_parameters().map(|q| q.to_string());
        let spec = contact.exact_specification();
        return Ok(json!({
            "proof": "definite-powers",
            "uvA": [au, av],
            "uvB": [bu, bv],
            "centerA": spec.center.each_ref().map(ToString::to_string),
            "powers": spec.powers,
            "diagonalCoefficients": spec.coefficients.each_ref().map(ToString::to_string),
            "mixedCoefficient": contact.mixed_coefficient().to_string(),
            "coversEntireSurfaces": contact.covers_entire_surfaces(),
        }));
    }
    if let Some(contact) = ss::discover_convex_graph_contact(&source, 256)? {
        let [au, av, bu, bv] = contact.native_parameters().map(|q| q.to_string());
        return Ok(json!({
            "proof": "definite-hessian",
            "uvA": [au, av],
            "uvB": [bu, bv],
            "centerA": contact.center().each_ref().map(ToString::to_string),
            "coversEntireSurfaces": true,
        }));
    }
    Err(Error::new(
        "BREP_SS_ISOLATED_CONTACT_UNPROVED",
        "The source pair has no proved complete isolated graph contact",
    ))
}
