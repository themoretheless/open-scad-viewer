//! Exact literal-row restrictions of Bezier-segmented retained profile axes.
//! Source curves/laws and their original derivative units remain untouched.
use super::*;
pub(super) fn patches(
    surface: &nurbs_core::surface::Surface,
) -> Result<Vec<nurbs_core::surface::Surface>> {
    let p = surface.degree_u;
    let n = surface.control_points.len();
    let k = &surface.knots_u;
    let [a, b] = [k[p], k[n]];
    if surface.periodic_u
        || p == 0
        || !a.is_finite()
        || !b.is_finite()
        || a >= b
        || !k[..=p].iter().all(|&t| t == a)
        || !k[n..].iter().all(|&t| t == b)
    {
        return Err(fail(
            "Retained profile Bezier domain normalization is unproved",
        ));
    }
    // At multiplicity >= degree the basis on each positive original span is
    // Bernstein on its literal degree+1 rows. Select entire XYZ/weight rows
    // across ALL station columns; independent per-column gauges would change
    // the tensor and are forbidden. No insertion, elevation or knot ratio is
    // rounded. General lower-multiplicity U restrictions need another proof.
    let mut i = p + 1;
    while i < n {
        let mut end = i + 1;
        while end < k.len() && k[end] == k[i] {
            end += 1;
        }
        if end - i < p {
            return Err(fail(
                "Retained profile exact tensor restriction is unproved",
            ));
        }
        i = end;
    }
    let mut out = Vec::new();
    for i in p..n {
        if k[i] == k[i + 1] {
            continue;
        }
        let mut result = surface.clone();
        result.control_points = surface.control_points[i - p..=i].to_vec();
        result.weights = surface.weights[i - p..=i].to_vec();
        result.knots_u = std::iter::repeat_n(0., p + 1)
            .chain(std::iter::repeat_n(1., p + 1))
            .collect();
        out.push(result);
    }
    Ok(out)
}
