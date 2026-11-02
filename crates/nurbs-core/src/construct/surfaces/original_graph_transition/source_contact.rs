//! Source-chart pcurves with fresh exact polynomial lift identity. This does
//! not create a trimmed source face or grant solid admission.
use super::*;
use std::sync::Arc;
struct Root {
    source: Surface,
    side: usize,
    world: Vec<[Q; 3]>,
    uv: Vec<[Q; 2]>,
}
#[derive(Clone)]
pub struct SourceContact {
    root: Arc<Root>,
    range: [Q; 2],
    world: Vec<[Q; 3]>,
    uv: Vec<[Q; 2]>,
}
impl SourceContact {
    pub fn source(&self) -> &Surface {
        &self.root.source
    }
    pub fn side(&self) -> usize {
        self.root.side
    }
    pub fn root_parameters(&self) -> &[Q; 2] {
        &self.range
    }
    pub fn points(&self) -> &[[Q; 3]] {
        &self.world
    }
    pub fn uv_points(&self) -> &[[Q; 2]] {
        &self.uv
    }
    /// Local restriction is mapped to the immutable ORIGINAL contact clock.
    /// Both the original root restriction and actual source lift are rechecked.
    pub fn restricted(&self, local: [Q; 2], b: &mut Budget) -> Result<Self> {
        let lo = b.keep(local[0].clone())?;
        let hi = b.keep(local[1].clone())?;
        check(
            lo >= Q::zero() && lo < hi && hi <= Q::one(),
            "Invalid source contact restriction",
        )?;
        let span = b.keep(&self.range[1] - &self.range[0])?;
        let range = [
            b.keep(&self.range[0] + &span * lo)?,
            b.keep(&self.range[0] + span * hi)?,
        ];
        from_root(self.root.clone(), range, b)
    }
}
fn from_root(root: Arc<Root>, range: [Q; 2], b: &mut Budget) -> Result<SourceContact> {
    let world = BoundaryDefinition {
        points: root.world.clone(),
        uv: [[Q::zero(), Q::zero()], [Q::one(), Q::zero()]],
    }
    .restricted(range.clone(), b)?
    .points;
    let mut columns = Vec::new();
    for k in 0..2 {
        columns.push(restrict(
            &root.uv.iter().map(|p| p[k].clone()).collect::<Vec<_>>(),
            &range[0],
            &range[1],
            b,
        )?);
    }
    let uv = (0..root.uv.len())
        .map(|i| [columns[0][i].clone(), columns[1][i].clone()])
        .collect();
    let out = SourceContact {
        root,
        range,
        world,
        uv,
    };
    verify(&out, b)?;
    Ok(out)
}
fn verify(c: &SourceContact, b: &mut Budget) -> Result<()> {
    let s = &c.root.source;
    let n = s.degree_u;
    let m = c.uv.len() - 1;
    check(
        c.world.len() == n + m + 1,
        "Source lift degree does not match actual edge",
    )?;
    let a = c.uv[0][0].clone();
    let end = c.uv.last().unwrap()[0].clone();
    let span = b.keep(&end - &a)?;
    check(
        span > Q::zero(),
        "Source contact must have an increasing physical U clock",
    )?;
    for (i, p) in c.uv.iter().enumerate() {
        let u = b.keep(&a + &span * ratio(i as u64, m as u64))?;
        check(
            p[0] == u && p[1] >= Q::zero() && p[1] <= Q::one(),
            "Source contact left its original graph chart",
        )?;
    }
    let low = b.authored(s.knots_u[n])?;
    let high = b.authored(s.knots_u[s.control_points.len()])?;
    let width = b.keep(high - &low)?;
    let lo = b.keep((&a - &low) / &width)?;
    let hi = b.keep((&end - &low) / width)?;
    check(
        lo >= Q::zero() && lo < hi && hi <= Q::one(),
        "Source contact lies outside original U domain",
    )?;
    let q = s
        .control_points
        .iter()
        .map(|r| b.authored(r[0][2]))
        .collect::<Result<Vec<_>>>()?;
    let h = s
        .control_points
        .iter()
        .map(|r| {
            let a = b.authored(r[0][2])?;
            let z = b.authored(r[1][2])?;
            b.keep(a - z)
        })
        .collect::<Result<Vec<_>>>()?;
    let q = restrict(&q, &lo, &hi, b)?;
    let h = restrict(&h, &lo, &hi, b)?;
    let v = c.uv.iter().map(|p| p[1].clone()).collect::<Vec<_>>();
    let hw = product(&h, &v, b)?;
    let q = elevate(&q, n + m, b)?;
    let x = elevate(
        &c.uv.iter().map(|p| p[0].clone()).collect::<Vec<_>>(),
        n + m,
        b,
    )?;
    let y = elevate(&v, n + m, b)?;
    for i in 0..c.world.len() {
        let expected = [
            x[i].clone(),
            if c.root.side == 0 {
                -&y[i]
            } else {
                y[i].clone()
            },
            b.keep(&q[i] - &hw[i])?,
        ];
        check(
            c.world[i] == expected,
            "Exact original source lift differs from canonical contact edge",
        )?;
    }
    Ok(())
}
impl Patch {
    /// The immutable Patch constructor already owns the original graph-class
    /// preconditions. This freshly composes its ACTUAL stored source and UV net.
    pub fn source_contact(&self, side: usize, b: &mut Budget) -> Result<SourceContact> {
        check(side < 2, "Choose an original source side")?;
        let boundary = self.boundary_definition(
            if side == 0 {
                crate::surface_join::Boundary::VMin
            } else {
                crate::surface_join::Boundary::VMax
            },
            b,
        )?;
        let domain = self.width.domain();
        let a = b.authored(domain[0])?;
        let end = b.authored(domain[1])?;
        let m = self.width.degree;
        let mut uv = Vec::new();
        for i in 0..=m {
            let u = b.keep(&a + (&end - &a) * ratio(i as u64, m as u64))?;
            uv.push([u, b.authored(self.width.control_points[i][0])?]);
        }
        let root = Arc::new(Root {
            source: self.sources[side].clone(),
            side,
            world: boundary.points,
            uv,
        });
        from_root(root, [Q::zero(), Q::one()], b)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn changing_an_actual_source_pcurve_is_not_a_cached_contact_certificate() {
        let sources: [Surface;2] = std::array::from_fn(|side| Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: [0., 0., 1.]
                .into_iter()
                .enumerate()
                .map(|(i, z)| {
                    vec![
                        vec![i as f64 / 2., 0., z],
                        vec![i as f64 / 2., if side == 0 { -1. } else { 1. }, z - 0.25],
                    ]
                })
                .collect(),
            weights: vec![vec![1.; 2]; 3],
            periodic_u: false,
            periodic_v: false,
        });
        let width = Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: vec![
                vec![0.125, 0., 0.],
                vec![0.25, 0., 0.],
                vec![0.375, 0., 0.],
                vec![0.25, 0., 0.],
            ],
            weights: vec![1.; 4],
            periodic: false,
        };
        let mut b = Budget::new(100_000).unwrap();
        let patch = build([&sources[0], &sources[1]], &width, &mut b).unwrap();
        for side in 0..2 {
            let original = patch.source_contact(side, &mut b).unwrap();
            assert_eq!(original.source(), &sources[side]);
            let mut sub = original
                .restricted([ratio(1, 3), ratio(2, 3)], &mut b)
                .unwrap();
            assert_eq!(sub.root_parameters(), &[ratio(1, 3), ratio(2, 3)]);
            let t = ratio(1, 3);
            let w = ratio(1, 8) + ratio(3, 8) * &t - ratio(1, 4) * t.pow(3);
            assert_eq!(sub.uv_points()[0], [t.clone(), w.clone()]);
            assert_eq!(
                sub.points()[0],
                [
                    t.clone(),
                    if side == 0 { -&w } else { w.clone() },
                    &t * &t - ratio(1, 4) * w
                ]
            );
            sub.uv[1][1] += ratio(1, 1 << 20);
            assert!(verify(&sub, &mut b).is_err());
            assert!(
                original
                    .restricted([ratio(1, 3), ratio(2, 3)], &mut Budget::new(0).unwrap())
                    .is_err()
            );
        }
    }
}
