//! One original graph support, independent of an intersection partner.
use super::*;
#[derive(Clone, Debug)]
pub struct QualifiedGraphSupport {
    source: Surface,
    limits: QuadraticGraphSSLimits,
    xy: [f64; 4],
    domain: [[f64; 2]; 2],
    height: Poly,
    operations: usize,
}
pub fn qualify_graph_support(
    source: &Surface,
    limits: QuadraticGraphSSLimits,
) -> Result<QualifiedGraphSupport> {
    let mut work = Work::new(limits.max_operations)?;
    let (xy, domain, height) = graph(source, &mut work)?;
    Ok(QualifiedGraphSupport {
        source: source.clone(),
        limits,
        xy,
        domain,
        height,
        operations: work.used,
    })
}
impl QualifiedGraphSupport {
    pub fn source(&self) -> &Surface {
        &self.source
    }
    /// x=x0+dx*s, y=y0+dy*t for normalized ORIGINAL native parameters.
    pub fn xy_frame(&self) -> [f64; 4] {
        self.xy
    }
    pub fn domain(&self) -> [[f64; 2]; 2] {
        self.domain
    }
    pub fn height_coefficients(&self) -> &[[f64; 3]; 3] {
        &self.height
    }
    pub fn operations(&self) -> usize {
        self.operations
    }
    pub fn reverify(&self) -> Result<()> {
        let fresh = qualify_graph_support(&self.source, self.limits)?;
        check(
            self.xy == fresh.xy
                && self.domain == fresh.domain
                && self.height == fresh.height
                && self.operations == fresh.operations,
            "Original graph support changed",
        )
    }
    /// Strict complete-surface separation from a world-Z floor by the positive
    /// rational control hull. This sufficient test refuses inconclusive hulls.
    /// Fresh source replay and all comparisons share the supplied work bound.
    pub fn prove_above_floor(&self, floor: f64, max_operations: usize) -> Result<usize> {
        check(floor.is_finite(), "Need a finite original floor height")?;
        let mut work = Work::new(max_operations)?;
        for _ in 0..self.operations {
            work.tick()?;
        }
        self.reverify()?;
        for row in &self.source.control_points {
            for p in row {
                work.tick()?;
                check(
                    p[2] > floor,
                    "Original positive-weight roof hull does not strictly clear the floor",
                )?;
            }
        }
        Ok(work.used)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn patch() -> Surface {
        Surface {
            degree_u: 2,
            degree_v: 2,
            knots_u: vec![2., 2., 2., 6., 6., 6.],
            knots_v: vec![-3., -3., -3., 1., 1., 1.],
            control_points: (0..3)
                .map(|i| {
                    (0..3)
                        .map(|j| {
                            vec![
                                i as f64 / 2.,
                                j as f64 / 2.,
                                2. + [0.25, -0.25, 0.25][i] + [0.25, -0.25, 0.25][j],
                            ]
                        })
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 3]; 3],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn original_graph_support_replays_native_clocks_and_whole_positive_hull() {
        let surface = patch();
        let before = surface.clone();
        let proof = qualify_graph_support(&surface, Default::default()).unwrap();
        assert_eq!(proof.xy_frame(), [0., 1., 0., 1.]);
        assert_eq!(proof.domain(), [[2., 6.], [-3., 1.]]);
        let work = proof.prove_above_floor(0., 4096).unwrap();
        assert!(proof.prove_above_floor(0., work - 1).is_err());
        assert_eq!(proof.prove_above_floor(0., work).unwrap(), work);
        assert!(proof.prove_above_floor(1.5, 4096).is_err());
        assert!(proof.prove_above_floor(f64::NAN, 4096).is_err());
        proof.reverify().unwrap();
        assert_eq!(surface, before);
        let mut bad = surface.clone();
        bad.control_points[1][1][0] = 0.5f64.next_up();
        assert!(qualify_graph_support(&bad, Default::default()).is_err());
        assert!(
            qualify_graph_support(&surface, QuadraticGraphSSLimits { max_operations: 1 }).is_err()
        );
    }
    #[test]
    fn varying_original_weights_are_checked_before_hull_authority() {
        let x = [0., 0.0625, 0.5, 0.9375, 1.];
        let square = [0., 0., 1., 26.25, 30.];
        let surface = Surface {
            degree_u: 4,
            degree_v: 2,
            knots_u: vec![0., 0., 0., 0., 0., 1., 1., 1., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: (0..5)
                .map(|i| {
                    (0..3)
                        .map(|j| {
                            vec![
                                x[i],
                                j as f64 / 2.,
                                2. + square[i] / 64. + [0.25, -0.25, 0.25][j],
                            ]
                        })
                        .collect()
                })
                .collect(),
            weights: [1., 4., 5., 4., 1.]
                .into_iter()
                .map(|w| vec![w; 3])
                .collect(),
            periodic_u: false,
            periodic_v: false,
        };
        let proof = qualify_graph_support(
            &surface,
            QuadraticGraphSSLimits {
                max_operations: 4096,
            },
        )
        .unwrap();
        proof.prove_above_floor(0., 4096).unwrap();
        let mut bad = surface.clone();
        bad.weights[2][1] = bad.weights[2][1].next_up();
        assert!(
            qualify_graph_support(
                &bad,
                QuadraticGraphSSLimits {
                    max_operations: 4096
                }
            )
            .is_err()
        );
    }
}
