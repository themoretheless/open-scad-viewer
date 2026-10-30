//! Closed boundary subdivision with a compact C2 interior tile.
//! Original rim uses remain literal; no material or embedding admission.
use crate::{Model, face_domain::FaceDomain};
use nurbs_core::{Error, Result, curve::Curve, surface::Axis, trim_domain::Location};
#[derive(Clone, Copy)]
pub struct InteriorTileLimits {
    pub domain_cells: usize,
    pub agreement_cells: usize,
    pub seam_cells: usize,
    pub exact_work: u64,
    /// Optional correlated polynomial-graph proof in the fixed integer chart.
    /// This never substitutes sampled seams or relaxes unsupported cases.
    pub graph_coefficients: Option<usize>,
}
pub struct InteriorTileProposal {
    pub model: Model,
    pub tile_face: usize,
    pub shared_edges: [usize; 4],
    pub lifted_pole: [usize; 2],
    pub lifted_poles: Vec<[usize; 2]>,
    pub seams: Vec<nurbs_core::surface_g2_pcurves::Report>,
    pub domain_cells: usize,
    pub closure: crate::watertight::QualifiedReport,
    pub coefficient_work: usize,
}
fn check(ok: bool, message: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(Error::new("BREP_INTERIOR_TILE_UNPROVEN", message))
    }
}
/// Extract a rectangular child surface, retain three control layers at every
/// side and lift one interior pole. Add a hole to the original face and its
/// complementary tile using FOUR shared world edges and owned vertices.
/// Every complete boundary receives fresh original-support G2 inspection.
/// Topology closure and continuous edge lifts are mandatory before return.
/// The source is immutable. Material, embedding and thickness remain separate.
pub fn propose_interior_tile(
    source: &Model,
    face: usize,
    rectangle: [[f64; 2]; 2],
    displacement: [f64; 3],
    tolerance_uv: f64,
    tolerances: [f64; 3],
    limits: InteriorTileLimits,
) -> Result<InteriorTileProposal> {
    build_tile(
        source,
        face,
        rectangle,
        displacement,
        tolerance_uv,
        tolerances,
        limits,
        false,
    )
}
/// Distribute a lift over all interior poles while retaining three boundary
/// layers. Return a closed G2 tile only when its ENTIRE trimmed curvature
/// cover is Within the declared limit. No material or solid admission.
pub fn propose_curvature_limited_tile(
    source: &Model,
    face: usize,
    rectangle: [[f64; 2]; 2],
    displacement: [f64; 3],
    tolerance_uv: f64,
    tolerances: [f64; 3],
    limits: InteriorTileLimits,
    maximum_curvature: f64,
    curvature_limits: crate::face_curvature::Limits,
) -> Result<(InteriorTileProposal, crate::face_curvature::Report)> {
    // Reject invalid limits before doing any expensive seam construction.
    check(
        maximum_curvature.is_finite()
            && maximum_curvature > 0.
            && (1..=50000).contains(&curvature_limits.cells)
            && (1..=1000000).contains(&curvature_limits.domain_cells)
            && (1..=1000000).contains(&curvature_limits.shape_cells),
        "Invalid curvature maximum",
    )?;
    let out = build_tile(
        source,
        face,
        rectangle,
        displacement,
        tolerance_uv,
        tolerances,
        limits,
        true,
    )?;
    let curvature = crate::face_curvature::inspect(
        &out.model,
        out.tile_face,
        maximum_curvature,
        tolerance_uv,
        curvature_limits,
        &[],
    )?;
    check(
        curvature.decision == crate::face_curvature::Decision::Within,
        &format!(
            "Distributed tile curvature {:?}: {}",
            curvature.decision, curvature.reason
        ),
    )?;
    Ok((out, curvature))
}
fn build_tile(
    source: &Model,
    face: usize,
    rectangle: [[f64; 2]; 2],
    displacement: [f64; 3],
    tolerance_uv: f64,
    tolerances: [f64; 3],
    limits: InteriorTileLimits,
    distributed: bool,
) -> Result<InteriorTileProposal> {
    check(
        limits.domain_cells <= 1_000_000
            && limits.agreement_cells <= 100000
            && limits.seam_cells <= 100000
            && limits.exact_work <= 1_000_000
            && limits
                .graph_coefficients
                .is_none_or(|n| n as u64 <= 50_000_000_000)
            && tolerances.iter().all(|x| x.is_finite() && *x > 0.)
            && tolerances[0] <= source.tolerance_mm
            && tolerances[1] <= 2.
            && displacement.iter().all(|x| x.is_finite() && *x >= 0.)
            && displacement.iter().any(|x| *x > 0.),
        "Invalid tile tolerances, lift or work limits",
    )?;
    let domain = FaceDomain::new(source, face, tolerance_uv)?;
    let s = &source.faces[face].surface;
    check(
        !s.periodic_u
            && !s.periodic_v
            && (6..=12).contains(&s.degree_u)
            && (6..=12).contains(&s.degree_v)
            && s.control_points
                .len()
                .saturating_mul(s.control_points[0].len())
                <= 4096,
        "Tile construction requires a bounded nonperiodic degree 6..12 support",
    )?;
    check(limits.domain_cells > 0, "Tile ownership budget exhausted")?;
    let ownership = domain.classify(rectangle, limits.domain_cells.min(100000))?;
    check(
        ownership.location == Location::Inside,
        "Whole tile rectangle is not certified Inside",
    )?;
    let mut tile = s.trim([
        rectangle[0][0],
        rectangle[0][1],
        rectangle[1][0],
        rectangle[1][1],
    ])?;
    let counts = [tile.control_points.len(), tile.control_points[0].len()];
    check(
        counts[0] >= 7 && counts[1] >= 7 && counts[0] * counts[1] <= 4096,
        "Tile needs three retained control layers on each side",
    )?;
    for (knots, p, n) in [
        (&tile.knots_u, tile.degree_u, counts[0]),
        (&tile.knots_v, tile.degree_v, counts[1]),
    ] {
        let mut i = p + 1;
        while i < n {
            let mut j = i + 1;
            while j < n && knots[j] == knots[i] {
                j += 1;
            }
            check(j - i <= p - 2, "Tile has an internal knot below C2")?;
            i = j;
        }
    }
    let lifted_pole = counts.map(|n| n / 2);
    check(
        (0..2).all(|k| lifted_pole[k] >= 3 && lifted_pole[k] + 3 < counts[k]),
        "Lift would change a retained boundary jet layer",
    )?;
    let range = |axis: usize| {
        if distributed {
            3..counts[axis] - 3
        } else {
            lifted_pole[axis]..lifted_pole[axis] + 1
        }
    };
    let mut lifted_poles = Vec::new();
    for i in range(0) {
        for j in range(1) {
            lifted_poles.push([i, j]);
            let point = &mut tile.control_points[i][j];
            for k in 0..3 {
                let v = point[k] + displacement[k];
                check(v.is_finite() && v >= point[k], "Tile lift overflow")?;
                point[k] = v;
            }
        }
    }
    tile.validate()?;
    let [u, v] = rectangle;
    let uv = [[u[0], v[0]], [u[1], v[0]], [u[1], v[1]], [u[0], v[1]]];
    let mut curves = [
        s.iso(Axis::V, v[0])?.trim(u[0], u[1])?,
        s.iso(Axis::U, u[1])?.trim(v[0], v[1])?,
        s.iso(Axis::V, v[1])?.trim(u[0], u[1])?.reverse()?,
        s.iso(Axis::U, u[0])?.trim(v[0], v[1])?.reverse()?,
    ];
    // New curves bind by the explicit UV corner role, never by proximity.
    // Reverify their actual stored coefficients below after endpoint binding.
    let corners = uv.map(|p| s.evaluate(p[0], p[1]).map(|e| e.point));
    let corners = corners.into_iter().collect::<Result<Vec<_>>>()?;
    for i in 0..4 {
        curves[i].control_points[0] = corners[i].to_vec();
        *curves[i].control_points.last_mut().unwrap() = corners[(i + 1) % 4].to_vec();
    }
    let pcs = (0..4)
        .map(|i| Curve::from_polyline(vec![uv[i].to_vec(), uv[(i + 1) % 4].to_vec()]))
        .collect::<Result<Vec<_>>>()?;
    let mut cells = 0;
    let mut exact = 0;
    let mut coefficient_work = 0;
    let mut seams = Vec::new();
    for i in 0..4 {
        let seam = if let Some(max) = limits.graph_coefficients {
            let proof = nurbs_core::surface_g2_pcurves::inspect_owned_graph(
                &curves[i],
                [&pcs[i], &pcs[i]],
                [s, &tile],
                tolerances,
                limits.seam_cells.saturating_sub(cells),
                limits.exact_work.saturating_sub(exact),
                max.saturating_sub(coefficient_work),
            )?;
            coefficient_work += proof.coefficient_work;
            proof.owned.report
        } else {
            nurbs_core::surface_g2_pcurves::inspect(
                &curves[i],
                [&pcs[i], &pcs[i]],
                [s, &tile],
                [false, false],
                tolerances[0],
                tolerances[1],
                tolerances[2],
                limits.seam_cells.saturating_sub(cells),
                limits.exact_work.saturating_sub(exact),
            )?
        };
        cells += seam.geometric.cells;
        exact += seam.positions.exact_work;
        check(
            seam.positions.proven
                && seam.geometric.decision == nurbs_core::surface_join::Decision::WithinTolerance,
            &format!(
                "Tile side {i}: positions={}, geometric={:?}, branches={:?}",
                seam.positions.proven, seam.geometric.decision, seam.geometric.branches
            ),
        )?;
        seams.push(seam);
    }
    let boundaries = (0..4)
        .map(|i| crate::face_window::Boundary {
            curve: curves[i].clone(),
            source_pcurve: pcs[i].clone(),
            replacement_pcurve: pcs[i].clone(),
        })
        .collect::<Vec<_>>();
    let window = crate::face_window::replace_with_exact_limit(
        source,
        face,
        &tile,
        &boundaries,
        tolerance_uv,
        limits.agreement_cells,
        limits.exact_work.saturating_sub(exact),
    )?;
    let shared_edges: [usize; 4] = window.edges.as_slice().try_into().map_err(|_| {
        Error::new(
            "BREP_INTERIOR_TILE_UNPROVEN",
            "Window did not return four shared edges",
        )
    })?;
    let tile_face = window.replacement_face;
    let model = window.model;
    exact += window.exact_work;
    let closure = crate::watertight::inspect_with_exact(
        &model,
        limits.agreement_cells.saturating_sub(window.cells),
        limits.exact_work.saturating_sub(exact),
    )?;
    check(
        closure.closure.status == crate::watertight::Closure::WithinTolerance,
        "The composite tile boundary is not continuously closed",
    )?;
    Ok(InteriorTileProposal {
        model,
        tile_face,
        shared_edges,
        lifted_pole,
        lifted_poles,
        seams,
        domain_cells: ownership.cells,
        closure,
        coefficient_work,
    })
}
