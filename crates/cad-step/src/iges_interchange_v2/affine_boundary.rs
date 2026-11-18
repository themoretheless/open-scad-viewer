use super::*;
use nurbs_core::editing::exact_projective::{PcurveEdgeAgreement, compare_pcurve_edge};
use nurbs_core::surface_injectivity::{
    certify_affine_coordinate_pair, certify_affine_parameterization,
};

pub(super) fn recover(
    de: usize,
    surface: &Surface,
    edge: &Curve,
    work: &mut usize,
) -> Result<Curve> {
    let last_u = surface.control_points.len() - 1;
    let last_v = surface.control_points[0].len() - 1;
    graph_work(work, (last_u + 1) * (last_v + 1))?;
    let origin = &surface.control_points[0][0];
    let u: [f64; 3] = std::array::from_fn(|i| surface.control_points[last_u][0][i] - origin[i]);
    let v: [f64; 3] = std::array::from_fn(|i| surface.control_points[0][last_v][i] - origin[i]);
    let determinant = |[i, j]: [usize; 2]| u[i] * v[j] - u[j] * v[i];
    let [i, j] = if certify_affine_parameterization(surface).is_some() {
        [[0, 1], [1, 2], [2, 0]]
            .into_iter()
            .max_by(|a, b| determinant(*a).abs().total_cmp(&determinant(*b).abs()))
            .unwrap()
    } else if let Some(pair) = certify_affine_coordinate_pair(surface) {
        pair
    } else {
        let limit = MAX_GRAPH_WORK.saturating_sub(*work).min(1_000_000);
        let (pair, used) =
            nurbs_core::editing::exact_surface_derivative::rational_affine_coordinate_pair(
                surface,
                limit as u64,
            )?;
        graph_work(work, used as usize)?;
        pair.ok_or_else(|| {
            trimmed_unsupported(
                de,
                "model-only boundary needs an invertible affine coordinate pair",
            )
        })?
    };
    let det = determinant([i, j]);
    let [u0, u1] = [
        surface.knots_u[surface.degree_u],
        surface.knots_u[last_u + 1],
    ];
    let [v0, v1] = [
        surface.knots_v[surface.degree_v],
        surface.knots_v[last_v + 1],
    ];
    if !det.is_finite() || det == 0. {
        return Err(trimmed_unsupported(de, "singular affine boundary chart"));
    }
    let mut uv = edge.clone();
    for point in &mut uv.control_points {
        let x = point[i] - origin[i];
        let y = point[j] - origin[j];
        let a = (x * v[j] - y * v[i]) / det;
        let b = (u[i] * y - u[j] * x) / det;
        *point = vec![u0 + a * (u1 - u0), v0 + b * (v1 - v0)];
    }
    let limit = MAX_GRAPH_WORK.saturating_sub(*work).min(1_000_000);
    let proof = compare_pcurve_edge(surface, &uv, edge, limit as u64)?;
    graph_work(work, proof.work_used as usize)?;
    if proof.outcome != PcurveEdgeAgreement::Equal {
        return Err(trimmed_unsupported(
            de,
            "model-only boundary has no certified UV image",
        ));
    }
    Ok(uv)
}
