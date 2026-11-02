//! Exact source use of one end-rim half. G2 requires actual original h,h',h''
//! to vanish at the end; a merely close or tangent-looking end is refused.
use super::*;
pub struct SourceEndContact {
    source: Surface,
    side: usize,
    at_end: bool,
    range: [Q; 2],
    world: Vec<[Q; 3]>,
    uv: [[Q; 2]; 2],
}
impl SourceEndContact {
    pub fn source(&self) -> &Surface {
        &self.source
    }
    pub fn side(&self) -> usize {
        self.side
    }
    pub fn at_end(&self) -> bool {
        self.at_end
    }
    pub fn root_parameters(&self) -> &[Q; 2] {
        &self.range
    }
    pub fn points(&self) -> &[[Q; 3]] {
        &self.world
    }
    pub fn uv_points(&self) -> &[[Q; 2]; 2] {
        &self.uv
    }
}
fn diff(a: &[Q], span: &Q, b: &mut Budget) -> Result<Vec<Q>> {
    if a.len() == 1 {
        return Ok(vec![Q::zero()]);
    }
    a.windows(2)
        .map(|p| b.keep((&p[1] - &p[0]) * Q::from_integer((a.len() - 1).into()) / span))
        .collect()
}
fn at(a: &[Q], t: &Q, b: &mut Budget) -> Result<Q> {
    Ok(split(a, t, b)?.0.last().unwrap().clone())
}
impl Patch {
    /// Original normalized V range must remain on one side of the source crease.
    /// The opaque result binds full exact lift and original second-order end jets.
    pub fn source_end_contact(
        &self,
        at_end: bool,
        side: usize,
        range: [Q; 2],
        b: &mut Budget,
    ) -> Result<SourceEndContact> {
        check(side < 2, "Choose an original source side")?;
        let lo = b.keep(range[0].clone())?;
        let hi = b.keep(range[1].clone())?;
        let half = ratio(1, 2);
        check(
            lo >= Q::zero()
                && lo < hi
                && hi <= Q::one()
                && if side == 0 { hi <= half } else { lo >= half },
            "Split the end rim at the original source crease",
        )?;
        let definition = self
            .boundary_definition(
                if at_end {
                    crate::surface_join::Boundary::UMax
                } else {
                    crate::surface_join::Boundary::UMin
                },
                b,
            )?
            .restricted([lo.clone(), hi.clone()], b)?;
        let s = &self.sources[side];
        let x = b.authored(self.width.domain()[usize::from(at_end)])?;
        let n = s.degree_u;
        let u0 = b.authored(s.knots_u[n])?;
        let u1 = b.authored(s.knots_u[s.control_points.len()])?;
        let span = b.keep(u1 - &u0)?;
        let t = b.keep((&x - u0) / &span)?;
        check(
            t >= Q::zero() && t <= Q::one(),
            "End rim is outside the original source chart",
        )?;
        let h = s
            .control_points
            .iter()
            .map(|r| {
                let a = b.authored(r[0][2])?;
                let z = b.authored(r[1][2])?;
                b.keep(a - z)
            })
            .collect::<Result<Vec<_>>>()?;
        let dh = diff(&h, &span, b)?;
        let ddh = diff(&dh, &span, b)?;
        for jet in [&h, &dh, &ddh] {
            check(
                at(jet, &t, b)?.is_zero(),
                "Original source end second-order graph jets do not vanish",
            )?;
        }
        let q = s
            .control_points
            .iter()
            .map(|r| b.authored(r[0][2]))
            .collect::<Result<Vec<_>>>()?;
        let z = at(&q, &t, b)?;
        let end = definition.points.last().unwrap();
        let uv = [
            [
                x.clone(),
                if side == 0 {
                    -&definition.points[0][1]
                } else {
                    definition.points[0][1].clone()
                },
            ],
            [x.clone(), if side == 0 { -&end[1] } else { end[1].clone() }],
        ];
        check(
            uv.iter().all(|p| p[1] >= Q::zero() && p[1] <= Q::one()),
            "End source use left the original V chart",
        )?;
        let v = elevate(
            &[uv[0][1].clone(), uv[1][1].clone()],
            definition.points.len() - 1,
            b,
        )?;
        for (i, p) in definition.points.iter().enumerate() {
            let expected = [
                x.clone(),
                if side == 0 { -&v[i] } else { v[i].clone() },
                z.clone(),
            ];
            check(
                p == &expected,
                "Actual original source end lift differs from canonical edge",
            )?;
        }
        // In the proved regular XY charts, h=h'=h''=0 makes both complete
        // graph 2-jets equal to q(X), independently of width derivatives.
        Ok(SourceEndContact {
            source: s.clone(),
            side,
            at_end,
            range: [lo, hi],
            world: definition.points,
            uv,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_end_jets_and_whole_half_lifts_are_exact_or_refused() {
        let sources: [Surface; 2] = std::array::from_fn(|side| Surface {
            degree_u: 8,
            degree_v: 1,
            knots_u: [vec![0.; 9], vec![1.; 9]].concat(),
            knots_v: vec![0., 0., 1., 1.],
            control_points: (0..9)
                .map(|i| {
                    let x = i as f64 / 8.;
                    let h = if i == 4 { 1. / 1024. } else { 0. };
                    vec![
                        vec![x, 0., x],
                        vec![x, if side == 0 { -1. } else { 1. }, x - h],
                    ]
                })
                .collect(),
            weights: vec![vec![1.; 2]; 9],
            periodic_u: false,
            periodic_v: false,
        });
        let mut width = Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![vec![0.25, 0., 0.]; 2],
            weights: vec![1.; 2],
            periodic: false,
        };
        let mut budget = Budget::new(100_000).unwrap();
        let patch = build([&sources[0], &sources[1]], &width, &mut budget).unwrap();
        for end in [false, true] {
            for side in 0..2 {
                let range = if side == 0 {
                    [ratio(1, 3), ratio(1, 2)]
                } else {
                    [ratio(1, 2), ratio(2, 3)]
                };
                let contact = patch
                    .source_end_contact(end, side, range.clone(), &mut budget)
                    .unwrap();
                assert_eq!(contact.source(), &sources[side]);
                assert_eq!(contact.root_parameters(), &range);
                for (i, t) in [0, 4].into_iter().zip(range) {
                    let x = Q::from_integer(usize::from(end).into());
                    assert_eq!(
                        contact.points()[i],
                        [x.clone(), ratio(1, 4) * (ratio(2, 1) * t - Q::one()), x]
                    );
                }
            }
        }
        assert!(
            patch
                .source_end_contact(false, 0, [ratio(1, 3), ratio(2, 3)], &mut budget)
                .is_err()
        );
        assert!(
            patch
                .source_end_contact(
                    false,
                    0,
                    [ratio(0, 1), ratio(1, 2)],
                    &mut Budget::new(0).unwrap()
                )
                .is_err()
        );
        width.knots = vec![0.25, 0.25, 0.75, 0.75];
        let middle = build([&sources[0], &sources[1]], &width, &mut budget).unwrap();
        assert!(
            middle
                .source_end_contact(false, 0, [ratio(0, 1), ratio(1, 2)], &mut budget)
                .is_err()
        );
    }
}
