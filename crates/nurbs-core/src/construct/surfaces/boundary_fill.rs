//! A G0 fan of rational patches filling an oriented boundary cycle.
use crate::{Result, check, curve::Curve, surface::Surface};

/// Edges run around a closed cycle. The supplied center need not be coplanar.
/// Each patch preserves one complete edge; shared spokes are rational lines.
/// This does not certify a simple boundary, injectivity or G1 across spokes.
pub fn fan(boundaries: &[Curve], center: [f64; 3]) -> Result<Vec<Surface>> {
    check(
        (3..=32).contains(&boundaries.len()),
        "Boundary fill needs 3..32 oriented edges",
    )?;
    check(
        center.iter().all(|x| x.is_finite()),
        "Boundary fill center must be finite",
    )?;
    let mut edges = Vec::with_capacity(boundaries.len());
    for c in boundaries {
        c.validate()?;
        check(
            c.control_points[0].len() == 3 && !c.periodic,
            "Boundary fill needs nonperiodic 3D edges",
        )?;
        let [a, b] = c.domain();
        edges.push(c.trim(a, b)?);
    }
    for i in 0..edges.len() {
        check(
            edges[i].control_points.last() == Some(&edges[(i + 1) % edges.len()].control_points[0]),
            "Boundary fill edges must form an exact oriented cycle",
        )?;
    }
    // Unit-weight spokes give identical Cartesian parameterization on both
    // neighboring patches despite independent boundary weight scales.
    let spokes: Vec<Curve> = edges
        .iter()
        .map(|c| crate::primitives::line(std::array::from_fn(|k| c.control_points[0][k]), center))
        .collect::<Result<_>>()?;
    (0..edges.len())
        .map(|i| {
            crate::triangular_patch::patch(&edges[i], &spokes[i], &spokes[(i + 1) % edges.len()])
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_four_rational_circle_arcs_and_their_radial_seams() {
        let nets = [
            [[1., 0., 0.], [1., 1., 0.], [0., 1., 0.]],
            [[0., 1., 0.], [-1., 1., 0.], [-1., 0., 0.]],
            [[-1., 0., 0.], [-1., -1., 0.], [0., -1., 0.]],
            [[0., -1., 0.], [1., -1., 0.], [1., 0., 0.]],
        ];
        let edges: Vec<Curve> = nets
            .into_iter()
            .map(|p| {
                crate::paths::bezier(
                    p.into_iter().map(|p| p.to_vec()).collect(),
                    Some(vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.]),
                )
                .unwrap()
            })
            .collect();
        let patches = fan(&edges, [0.; 3]).unwrap();
        for i in 0..4 {
            for t in [0., 0.13, 0.37, 0.83, 1.] {
                let p = patches[i].evaluate(t, 0.).unwrap().point;
                assert!((p[0] * p[0] + p[1] * p[1] - 1.).abs() < 1e-10);
                let a = patches[i].evaluate(1., t).unwrap().point;
                let b = patches[(i + 1) % 4].evaluate(0., t).unwrap().point;
                for k in 0..3 {
                    assert!((a[k] - b[k]).abs() < 1e-10);
                }
            }
        }
    }
    #[cfg(feature = "transport")]
    #[test]
    fn json_preserves_all_patches_instead_of_only_the_first() {
        let v=crate::transport::dispatch(value_codec::json!({"op":"surface_boundary_fill","boundaries":edges(),"center":[1.,1.,0.]})).unwrap();
        let patches: Vec<Surface> = value_codec::from_value(v).unwrap();
        assert_eq!(patches.len(), 5);
        for p in patches {
            assert_eq!(p.evaluate(0.37, 1.).unwrap().point, [1., 1., 0.]);
        }
    }
    fn edges() -> Vec<Curve> {
        let p = [
            [0., 0., 0.],
            [2., 0., 0.],
            [3., 1., 0.],
            [1., 3., 0.],
            [-1., 1., 0.],
        ];
        (0..p.len())
            .map(|i| crate::primitives::line(p[i], p[(i + 1) % p.len()]).unwrap())
            .collect()
    }
    #[test]
    fn retains_every_boundary_and_shared_spoke() {
        let mut e = edges();
        e[1].weights = vec![2., 3.];
        let center = [1., 1., 2.];
        let patches = fan(&e, center).unwrap();
        assert_eq!(patches.len(), 5);
        for i in 0..e.len() {
            for t in [0., 0.13, 0.37, 0.83, 1.] {
                let p = patches[i].evaluate(t, 0.).unwrap().point;
                let q = e[i].evaluate(t).unwrap().point;
                let a = patches[i].evaluate(1., t).unwrap().point;
                let b = patches[(i + 1) % e.len()].evaluate(0., t).unwrap().point;
                for k in 0..3 {
                    assert!((p[k] - q[k]).abs() < 1e-10);
                    assert!((a[k] - b[k]).abs() < 1e-10);
                    assert!(
                        (a[k]
                            - ((1. - t) * e[(i + 1) % e.len()].control_points[0][k]
                                + t * center[k]))
                            .abs()
                            < 1e-10
                    );
                }
            }
        }
    }
    #[test]
    fn matches_independent_affine_fan_and_rejects_open_cycles() {
        let e = edges();
        let center = [1., 1., 0.];
        let patches = fan(&e, center).unwrap();
        for i in 0..e.len() {
            for u in [0.13, 0.5, 0.87] {
                for v in [0.17, 0.5, 0.83] {
                    let p = patches[i].evaluate(u, v).unwrap().point;
                    for k in 0..3 {
                        let q = (1. - v)
                            * ((1. - u) * e[i].control_points[0][k]
                                + u * e[i].control_points[1][k])
                            + v * center[k];
                        assert!((p[k] - q).abs() < 1e-10);
                    }
                }
            }
        }
        assert!(fan(&e[..4], center).is_err());
        assert!(fan(&[], center).is_err());
        assert!(fan(&e, [f64::NAN, 0., 0.]).is_err());
    }
}
