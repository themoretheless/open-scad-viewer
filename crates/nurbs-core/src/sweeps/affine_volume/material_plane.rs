//! Exact original affine-plane chart; no fitted replacement surface is evidence.
use super::*;
use crate::{curve::Curve, numerics::compensated::two_sum};
#[derive(Clone, Debug)]
pub struct MaterialPlaneReport {
    pub axes: [usize; 2],
    pub constant_axis: usize,
    pub constant: f64,
    /// Determinant of projected ORIGINAL normalized UV columns, outward enclosed.
    pub determinant: [f64; 2],
    pub bounds: [[f64; 2]; 2],
    pub orientation_parameter: [f64; 3],
}
pub struct MaterialPlaneChart {
    carrier: Surface,
    report: MaterialPlaneReport,
}
impl MaterialPlaneChart {
    pub fn carrier(&self) -> &Surface {
        &self.carrier
    }
    pub fn report(&self) -> &MaterialPlaneReport {
        &self.report
    }
}
pub struct MaterialPlaneInspection {
    pub chart: Option<MaterialPlaneChart>,
    pub work: u64,
    pub exhausted: bool,
}
fn sub_exact(a: f64, b: f64) -> Option<f64> {
    let (s, e) = two_sum(a, -b);
    (s.is_finite() && e == 0.).then_some(s)
}
/// Conditional material containment: the owner MUST first prove valid bounded
/// original UV trims and exact original world-curve/carrier binding. This proves
/// the original carrier's affine homeomorphism onto a coordinate plane, plus an
/// enclosing rectangle of the supplied positive-weight world curve control hull.
pub fn inspect_material_plane(
    carrier: &Surface,
    profiles: &[Vec<Curve>],
    endpoint: &Curve,
    max_work: u64,
) -> Result<MaterialPlaneInspection> {
    check(
        max_work <= 1000000,
        "Invalid material-plane proof work limit",
    )?;
    carrier.validate()?;
    endpoint.validate()?;
    for c in profiles.iter().flatten() {
        c.validate()?;
    }
    let mut result = MaterialPlaneInspection {
        chart: None,
        work: 0,
        exhausted: false,
    };
    macro_rules! charge {
        ($n:expr) => {
            if max_work - result.work < $n {
                result.exhausted = true;
                return Ok(result);
            }
            result.work += $n;
        };
    }
    if !carrier
        .control_points
        .iter()
        .flatten()
        .all(|p| p.len() == 3)
        || carrier.degree_u != 1
        || carrier.degree_v != 1
        || carrier.control_points.len() != 2
        || carrier.control_points[0].len() != 2
        || carrier.periodic_u
        || carrier.periodic_v
        || endpoint.periodic
        || endpoint.control_points[0].len() != 2
        || endpoint.knots[..=endpoint.degree]
            .iter()
            .any(|v| *v != endpoint.domain()[0])
    {
        return Ok(result);
    }
    let poles = [
        &carrier.control_points[0][0],
        &carrier.control_points[1][0],
        &carrier.control_points[0][1],
        &carrier.control_points[1][1],
    ];
    for w in carrier.weights.iter().flatten() {
        charge!(1);
        if *w != carrier.weights[0][0] {
            return Ok(result);
        }
    }
    let mut constant_axis = None;
    for k in 0..3 {
        let mut equal = true;
        for p in &poles {
            charge!(1);
            equal &= p[k] == poles[0][k];
        }
        if equal {
            constant_axis = Some(k);
            break;
        }
    }
    let Some(k) = constant_axis else {
        return Ok(result);
    };
    let axes = (0..3).filter(|&j| j != k).collect::<Vec<_>>();
    let axes = [axes[0], axes[1]];
    let mut e = [[0.; 3]; 2];
    for j in 0..3 {
        charge!(4);
        let (Some(a), Some(b), Some(c)) = (
            sub_exact(poles[1][j], poles[0][j]),
            sub_exact(poles[2][j], poles[0][j]),
            sub_exact(poles[3][j], poles[1][j]),
        ) else {
            return Ok(result);
        };
        if c != b {
            return Ok(result);
        }
        e[0][j] = a;
        e[1][j] = b;
    }
    // Keep the established UV rectangle path for axis-aligned parameter charts.
    if !(e[0][axes[0]] != 0. && e[1][axes[0]] != 0. || e[0][axes[1]] != 0. && e[1][axes[1]] != 0.) {
        return Ok(result);
    }
    charge!(8);
    let determinant = I::point(e[0][axes[0]])
        .mul(I::point(e[1][axes[1]]))?
        .sub(I::point(e[0][axes[1]]).mul(I::point(e[1][axes[0]]))?)?;
    if determinant.lo <= 0. && determinant.hi >= 0. {
        return Ok(result);
    }
    let domain = [
        [carrier.knots_u[1], carrier.knots_u[2]],
        [carrier.knots_v[1], carrier.knots_v[2]],
    ];
    let mut point = [0.5; 3];
    for j in 0..2 {
        charge!(8);
        let v = endpoint.control_points[0][j];
        if v < domain[j][0] || v > domain[j][1] {
            return Ok(result);
        }
        let normalized = I::point(v)
            .sub(I::point(domain[j][0]))?
            .div(I::point(domain[j][1]).sub(I::point(domain[j][0]))?)?
            .intersect(0., 1.)?;
        point[j] = (normalized.lo + (normalized.hi - normalized.lo) * 0.5).clamp(0., 1.);
    }
    let mut bounds = [[f64::INFINITY, f64::NEG_INFINITY]; 2];
    for c in profiles.iter().flatten() {
        for p in &c.control_points {
            charge!(5);
            if p.len() != 3 || p[k] != poles[0][k] {
                return Ok(result);
            }
            for j in 0..2 {
                bounds[j][0] = bounds[j][0].min(p[axes[j]]);
                bounds[j][1] = bounds[j][1].max(p[axes[j]]);
            }
        }
    }
    // The rounded UV orientation proposal itself is independently enclosed in the
    // same chart rectangle; never infer an orientation sign outside the proof set.
    for j in 0..2 {
        charge!(6);
        let axis = axes[j];
        let value = I::point(poles[0][axis])
            .add(I::point(e[0][axis]).mul(I::point(point[0]))?)?
            .add(I::point(e[1][axis]).mul(I::point(point[1]))?)?;
        bounds[j][0] = bounds[j][0].min(value.lo);
        bounds[j][1] = bounds[j][1].max(value.hi);
        if bounds[j][0] >= bounds[j][1] {
            return Ok(result);
        }
    }
    let control_points = bounds[0]
        .iter()
        .map(|&x| {
            bounds[1]
                .iter()
                .map(|&y| {
                    let mut p = vec![0.; 3];
                    p[axes[0]] = x;
                    p[axes[1]] = y;
                    p[k] = poles[0][k];
                    p
                })
                .collect()
        })
        .collect();
    let derived = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![bounds[0][0], bounds[0][0], bounds[0][1], bounds[0][1]],
        knots_v: vec![bounds[1][0], bounds[1][0], bounds[1][1], bounds[1][1]],
        control_points,
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    derived.validate()?;
    result.chart = Some(MaterialPlaneChart {
        carrier: derived,
        report: MaterialPlaneReport {
            axes,
            constant_axis: k,
            constant: poles[0][k],
            determinant: [determinant.lo, determinant.hi],
            bounds,
            orientation_parameter: point,
        },
    });
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn inputs() -> (Surface, Vec<Vec<Curve>>, Curve) {
        let s = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![-4., -4., 10., 10.],
            knots_v: vec![-4., -4., 4., 4.],
            control_points: vec![
                vec![vec![0., -4., 0.], vec![-8., 4., 0.]],
                vec![vec![14., -4., 0.], vec![6., 4., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let xy = [[1., -2.], [5., -2.], [5., 2.], [1., 2.], [1., -2.]];
        let world = xy
            .windows(2)
            .map(|p| Curve {
                degree: 1,
                knots: vec![0., 0., 1., 1.],
                control_points: p.iter().map(|p| vec![p[0], p[1], 0.]).collect(),
                weights: vec![1.; 2],
                periodic: false,
            })
            .collect();
        let uv = Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![vec![-1., -2.], vec![3., -2.]],
            weights: vec![1.; 2],
            periodic: false,
        };
        (s, vec![world], uv)
    }
    #[test]
    fn exact_original_plane_chart_refuses_missing_work_and_nonaffine_coefficients() {
        let (s, p, uv) = inputs();
        let full = inspect_material_plane(&s, &p, &uv, 1000000).unwrap();
        let c = full.chart.as_ref().unwrap();
        assert_eq!(c.report().axes, [0, 1]);
        assert!(
            c.report().determinant[0] > 0.
                && c.report().bounds[0][0] > 0.875
                && c.report().bounds[0][0] <= 1.
        );
        for budget in [0, full.work - 1] {
            let r = inspect_material_plane(&s, &p, &uv, budget).unwrap();
            assert!(r.chart.is_none() && r.exhausted && r.work <= budget);
        }
        let mut q = s.clone();
        q.control_points[1][1][0] = q.control_points[1][1][0].next_up();
        assert!(
            inspect_material_plane(&q, &p, &uv, 1000000)
                .unwrap()
                .chart
                .is_none()
        );
        q = s.clone();
        q.weights[1][1] = 2.;
        assert!(
            inspect_material_plane(&q, &p, &uv, 1000000)
                .unwrap()
                .chart
                .is_none()
        );
        q = s.clone();
        q.control_points[1][1][2] = 0.125;
        assert!(
            inspect_material_plane(&q, &p, &uv, 1000000)
                .unwrap()
                .chart
                .is_none()
        );
    }
}
