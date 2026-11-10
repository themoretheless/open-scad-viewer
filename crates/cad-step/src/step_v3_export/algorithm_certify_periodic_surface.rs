//! step v3 export algorithm: certify_periodic_surface.
use super::*;
pub fn certify_periodic_surface(surface: &Surface) -> Result<Option<StepRegularityEvidence>> {
    let nu = surface.control_points.len();
    let nv = surface.control_points.first().map_or(0, Vec::len);
    if nu != 9
        || surface.degree_u != 2
        || !matches!((nv, surface.degree_v), (2, 1) | (5, 2) | (9, 2))
    {
        return Ok(None);
    }
    if surface
        .weights
        .iter()
        .flatten()
        .any(|weight| !weight.is_finite() || *weight <= 0.)
    {
        return Err(refuse(
            "STEP /8 rational denominator is not positive on the whole parameter domain",
        ));
    }
    if surface.control_points[0] != surface.control_points[nu - 1]
        || surface.weights[0] != surface.weights[nu - 1]
    {
        return Err(refuse(
            "STEP /8 lifted U seam lacks exact homogeneous control-net identity",
        ));
    }
    let du = [
        surface.knots_u[surface.degree_u],
        surface.knots_u[surface.knots_u.len() - surface.degree_u - 1],
    ];
    let dv = [
        surface.knots_v[surface.degree_v],
        surface.knots_v[surface.knots_v.len() - surface.degree_v - 1],
    ];
    let denominator_lower_bound = surface
        .weights
        .iter()
        .flatten()
        .copied()
        .fold(f64::INFINITY, f64::min);
    let radius_at = |column: usize| {
        surface.control_points[0][column]
            .iter()
            .zip(&surface.control_points[4][column])
            .map(|(left, right)| (left - right) * (left - right))
            .sum::<f64>()
            .sqrt()
            / 2.
    };
    let radii = if nv == 2 {
        vec![radius_at(0), radius_at(1)]
    } else {
        (0..nv).step_by(2).map(radius_at).collect::<Vec<_>>()
    };
    let positive_min = radii
        .iter()
        .copied()
        .filter(|radius| *radius > 1e-12)
        .fold(f64::INFINITY, f64::min);
    let positive_max = radii.iter().copied().fold(0., f64::max);
    let (
        carrier,
        collapsed_boundaries,
        regular_open_domain,
        jacobian_lower_bound,
        lifted_v,
        identity,
    ) = match nv {
        2 => {
            let collapsed = if constant_boundary(surface, 0) {
                vec!["v-min"]
            } else if constant_boundary(surface, 1) {
                vec!["v-max"]
            } else {
                vec![]
            };
            let carrier = if collapsed.is_empty() && ((radii[0] - radii[1]).abs() <= 1e-12) {
                "cylinder"
            } else {
                "cone"
            };
            (
                carrier,
                collapsed,
                !radii.iter().all(|radius| *radius > 1e-12),
                if radii.iter().all(|radius| *radius > 1e-12) {
                    positive_min
                } else {
                    0.
                },
                0.,
                "positive rational quadratic circle × affine generator; J=(affine radius)·J0",
            )
        }
        5 => {
            if !constant_boundary(surface, 0) || !constant_boundary(surface, nv - 1) {
                return Err(refuse(
                    "STEP /8 sphere requires exact collapsed V boundary control nets",
                ));
            }
            (
                "sphere",
                vec!["v-min", "v-max"],
                true,
                0.,
                0.,
                "positive rational circle × semicircle; J=R² cos(v)·J0 with only endpoint zeros",
            )
        }
        9 => {
            if surface
                .control_points
                .iter()
                .map(|row| &row[0])
                .ne(surface.control_points.iter().map(|row| &row[nv - 1]))
                || surface
                    .weights
                    .iter()
                    .map(|row| row[0])
                    .ne(surface.weights.iter().map(|row| row[nv - 1]))
            {
                return Err(refuse(
                    "STEP /8 lifted V seam lacks exact homogeneous control-net identity",
                ));
            }
            if !(positive_min > 1e-12 && positive_max > positive_min) {
                return Err(refuse(
                    "STEP /8 torus requires exact positive major-minus-minor radius",
                ));
            }
            (
                "torus",
                vec![],
                false,
                positive_min,
                dv[1] - dv[0],
                "positive rational circle product; J=r(R+r cos(v))·J0 and R-r>0",
            )
        }
        _ => unreachable!(),
    };
    Ok(Some(StepRegularityEvidence {
        carrier,
        parameter_u: du,
        parameter_v: dv,
        lifted_periods: [du[1] - du[0], lifted_v],
        denominator_lower_bound,
        jacobian_lower_bound,
        collapsed_boundaries,
        regular_open_domain,
        identity,
    }))
}
