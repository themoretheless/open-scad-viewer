use super::*;
use nurbs_core::editing::exact_projective::{PcurveEdgeAgreement, compare_pcurve_edge};
use nurbs_core::surface_injectivity::certify_affine_parameterization;

pub(super) fn recover(
    de: usize,
    surface: &Surface,
    edge: &Curve,
    work: &mut usize,
) -> Result<Curve> {
    let chart = certify_affine_parameterization(surface).ok_or_else(|| {
        trimmed_unsupported(de, "model-only boundary needs a certified affine chart")
    })?;
    let origin = &chart.control_points[0][0];
    let u: [f64; 3] = std::array::from_fn(|i| chart.control_points[1][0][i] - origin[i]);
    let v: [f64; 3] = std::array::from_fn(|i| chart.control_points[0][1][i] - origin[i]);
    let (i, j, det) = [(0, 1), (1, 2), (2, 0)]
        .map(|(i, j)| (i, j, u[i] * v[j] - u[j] * v[i]))
        .into_iter()
        .max_by(|a, b| a.2.abs().total_cmp(&b.2.abs()))
        .unwrap();
    if !det.is_finite() || det == 0. {
        return Err(trimmed_unsupported(de, "singular affine boundary chart"));
    }
    let mut uv = edge.clone();
    for point in &mut uv.control_points {
        let x = point[i] - origin[i];
        let y = point[j] - origin[j];
        let a = (x * v[j] - y * v[i]) / det;
        let b = (u[i] * y - u[j] * x) / det;
        *point = vec![
            chart.knots_u[1] + a * (chart.knots_u[2] - chart.knots_u[1]),
            chart.knots_v[1] + b * (chart.knots_v[2] - chart.knots_v[1]),
        ];
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
