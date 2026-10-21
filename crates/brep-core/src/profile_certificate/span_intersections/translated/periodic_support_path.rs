//! Positive regularized-material connectivity in the universal support cover.
//! This owner proves only a supplied path, not complete region discovery.
use super::*;
use crate::profile_certificate::algebraic_intersections::decode_ratio;
use std::sync::Arc;
#[derive(Clone, Debug)]
pub struct OwnedPeriodicSupportPath {
    owner: Arc<OwnedPeriodicSupportLifts>,
    endpoints: [[usize; 2]; 2],
    path: Vec<[ExactRatio; 2]>,
    limits: ParameterOrderLimits,
    max_work: usize,
    work: usize,
    endpoint_period_shifts: [[String; 2]; 2],
}
type Data = (usize, [[String; 2]; 2]);
fn derive(o: &OwnedPeriodicSupportPath) -> Result<Data> {
    let _replay = super::periodic_replay::scope();
    if !(2..=4096).contains(&o.path.len())
        || !(1..=1000000).contains(&o.max_work)
        || o.limits.max_refinements > 512
        || !(1..=128).contains(&o.limits.max_resultant_size)
    {
        return Err(refused("Invalid complete material path allowance"));
    }
    let view = super::periodic_material_clearance::ClearanceView::new(&o.owner, o.limits)?;
    data_from_view(&view, o.endpoints, &o.path, o.max_work)?
        .ok_or_else(|| refused("Material path contacts a selected original boundary"))
}
fn data_from_view(
    view: &super::periodic_material_clearance::ClearanceView<'_>,
    endpoints: [[usize; 2]; 2],
    path: &[[ExactRatio; 2]],
    max_work: usize,
) -> Result<Option<Data>> {
    if !(2..=4096).contains(&path.len()) || !(1..=1000000).contains(&max_work) {
        return Err(refused("Invalid complete material path allowance"));
    }
    let path = path
        .iter()
        .map(|p| Ok([decode_ratio(&p[0])?, decode_ratio(&p[1])?]))
        .collect::<Result<Vec<_>>>()?;
    let shifts = view.bind(endpoints, &path)?;
    let mut work = 0usize;
    for s in path.windows(2) {
        work = work
            .checked_add(view.cost(&s[0], &s[1])?)
            .filter(|v| *v <= max_work)
            .ok_or_else(|| refused("Complete selected-boundary path census exceeds allowance"))?;
    }
    for s in path.windows(2) {
        if !view.clear(&s[0], &s[1])? {
            return Ok(None);
        }
    }
    Ok(Some((work, shifts)))
}
/// The view has already replayed and immutably borrows THIS original owner.
/// A successful path still replays its own entire source on public revalidation.
pub(in super::super) fn construct_from_view(
    view: &super::periodic_material_clearance::ClearanceView<'_>,
    endpoints: [[usize; 2]; 2],
    path: &[[ExactRatio; 2]],
    max_work: usize,
) -> Result<Option<OwnedPeriodicSupportPath>> {
    let Some((work, shifts)) = data_from_view(view, endpoints, path, max_work)? else {
        return Ok(None);
    };
    Ok(Some(OwnedPeriodicSupportPath {
        owner: view.owner().clone(),
        endpoints,
        path: path.to_vec(),
        limits: view.limits(),
        max_work,
        work,
        endpoint_period_shifts: shifts,
    }))
}
impl OwnedPeriodicSupportPath {
    pub fn owner(&self) -> &Arc<OwnedPeriodicSupportLifts> {
        &self.owner
    }
    pub fn endpoints(&self) -> &[[usize; 2]; 2] {
        &self.endpoints
    }
    pub fn path(&self) -> &[[ExactRatio; 2]] {
        &self.path
    }
    pub fn endpoint_period_shifts(&self) -> &[[String; 2]; 2] {
        &self.endpoint_period_shifts
    }
    pub fn work(&self) -> usize {
        self.work
    }
    pub fn reverify_source(&self) -> Result<()> {
        if derive(self)? != (self.work, self.endpoint_period_shifts.clone()) {
            return Err(refused("Material path source census changed"));
        }
        Ok(())
    }
}
pub fn own_periodic_support_path(
    owner: Arc<OwnedPeriodicSupportLifts>,
    endpoints: [[usize; 2]; 2],
    path: &[[ExactRatio; 2]],
    limits: ParameterOrderLimits,
    max_work: usize,
) -> Result<OwnedPeriodicSupportPath> {
    let mut o = OwnedPeriodicSupportPath {
        owner,
        endpoints,
        path: path.to_vec(),
        limits,
        max_work,
        work: 0,
        endpoint_period_shifts: [
            ["0".to_string(), "0".to_string()],
            ["0".to_string(), "0".to_string()],
        ],
    };
    (o.work, o.endpoint_period_shifts) = derive(&o)?;
    Ok(o)
}
#[cfg(test)]
pub(in crate::profile_certificate::span_intersections::translated) mod tests {
    use super::*;
    fn lifts(selection: Arc<OwnedPeriodicSupportSelection>) -> Arc<OwnedPeriodicSupportLifts> {
        let charts = selection.rule().material().base().owner().owner().clone();
        let ports = Arc::new(own_periodic_ports(charts, 192).unwrap());
        let rotation =
            Arc::new(own_periodic_rotation(ports, ContactPortOrderLimits::default()).unwrap());
        let cycles = Arc::new(own_periodic_support_cycles(selection, rotation, 192, 128).unwrap());
        Arc::new(own_periodic_support_lifts(cycles, 192, 128).unwrap())
    }
    fn point(owner: &OwnedPeriodicSupportLifts, at: [usize; 2]) -> [ExactRatio; 2] {
        owner.owner().owner().sides()[at[0]].probes().queries()[at[1]].clone()
    }
    fn choose(owner: &OwnedPeriodicSupportLifts, test: impl Fn(&[Q; 2]) -> bool) -> [usize; 2] {
        for (edge, row) in owner.owner().owner().sides().iter().enumerate() {
            for side in 0..2 {
                if row.selected()[side] {
                    let p = &row.probes().queries()[side];
                    let p = [decode_ratio(&p[0]).unwrap(), decode_ratio(&p[1]).unwrap()];
                    if test(&p) {
                        return [edge, side];
                    }
                }
            }
        }
        panic!("Fixture has no matching actual material-side sample")
    }
    #[test]
    fn actual_material_path_binds_samples_and_rejects_stale_endpoints_or_census() {
        let o = lifts(super::super::periodic_support_material::tests::full_plane(
            true,
        ));
        let left = choose(&o, |p| p[0] < Q::new(1.into(), 8.into()));
        let right = choose(&o, |p| p[0] > Q::new(7.into(), 8.into()));
        let path = [point(&o, left), point(&o, right)];
        let proof = own_periodic_support_path(
            o.clone(),
            [left, right],
            &path,
            ParameterOrderLimits::default(),
            128,
        )
        .unwrap();
        assert_eq!(proof.work(), 4);
        proof.reverify_source().unwrap();
        let mut bad = proof.clone();
        bad.work -= 1;
        assert!(bad.reverify_source().is_err());
        let mut bad = proof.clone();
        bad.endpoints.swap(0, 1);
        assert!(bad.reverify_source().is_err());
        assert!(
            own_periodic_support_path(
                o.clone(),
                [left, right],
                &path,
                ParameterOrderLimits::default(),
                3
            )
            .is_err()
        );
        let mut outside = path.to_vec();
        outside.insert(
            1,
            [["-1".into(), "1".into()], ["1".into(), "2".to_string()]],
        );
        assert!(
            own_periodic_support_path(
                o,
                [left, right],
                &outside,
                ParameterOrderLimits::default(),
                128
            )
            .is_err()
        );
    }
    #[test]
    fn actual_hole_requires_clear_connection_not_equal_boolean_labels() {
        let o = lifts(super::super::periodic_support_material::tests::with_hole());
        let quarter = Q::new(1.into(), 4.into());
        let three = Q::new(3.into(), 4.into());
        let left = choose(&o, |p| {
            p[0] < quarter && p[0] > Q::new(1.into(), 8.into()) && p[1] > quarter && p[1] < three
        });
        let right = choose(&o, |p| {
            p[0] > three && p[0] < Q::new(7.into(), 8.into()) && p[1] > quarter && p[1] < three
        });
        let a = point(&o, left);
        let b = point(&o, right);
        // Both probes are true, and total signed crossings cancel, but the direct
        // connection cuts through the excluded hole and is not material connectivity.
        assert!(
            own_periodic_support_path(
                o.clone(),
                [left, right],
                &[a.clone(), b.clone()],
                ParameterOrderLimits::default(),
                128
            )
            .is_err()
        );
        let y = ["7".into(), "8".into()];
        let path = [a.clone(), [a[0].clone(), y.clone()], [b[0].clone(), y], b];
        let proof = own_periodic_support_path(
            o.clone(),
            [left, right],
            &path,
            ParameterOrderLimits::default(),
            128,
        )
        .unwrap();
        assert_eq!(proof.work(), 24);
        proof.reverify_source().unwrap();
        let mut bad = proof;
        bad.path[1] = [["1".into(), "2".to_string()], ["1".into(), "2".to_string()]];
        assert!(bad.reverify_source().is_err());
    }
    pub(in crate::profile_certificate::span_intersections::translated) fn cylinder_with_hole()
    -> Arc<OwnedPeriodicSupportLifts> {
        let _replay = super::super::periodic_replay::scope();
        let rim = Curve::from_polyline(vec![
            vec![0., 0., 0.],
            vec![1., 0., 0.],
            vec![1., 1., 0.],
            vec![0., 1., 0.],
            vec![0., 0., 0.],
        ])
        .unwrap();
        let surface = nurbs_core::surface::extrude(&rim, [0., 0., 1.]).unwrap();
        let input = super::super::periodic_support_boundaries::tests::original(
            &surface,
            [true, false],
            &[Curve::from_polyline(vec![
                vec![0.25, 0.25],
                vec![0.75, 0.25],
                vec![0.75, 0.75],
                vec![0.25, 0.75],
                vec![0.25, 0.25],
            ])
            .unwrap()],
            &[vec![OriginalPeriodicSourceUse {
                source: 0,
                reversed: false,
            }]],
        );
        let support = Arc::new(
            own_periodic_support_boundaries(
                input,
                1e-12,
                ParameterOrderLimits::default(),
                64,
                100000,
            )
            .unwrap(),
        );
        let jumps =
            super::super::periodic_support_boundaries::tests::graph(support.augmented().clone());
        let a = [["1".into(), "1".into()], ["1".into(), "2".to_string()]];
        let base = Arc::new(
            own_periodic_relative_winding(
                jumps.clone(),
                &[a.clone(), a.clone()],
                ParameterOrderLimits::default(),
                1000,
            )
            .unwrap(),
        );
        let rule = Arc::new(
            own_periodic_support_material_rule(
                support.clone(),
                base,
                &[OriginalPeriodicOperand {
                    wire_weights: vec![1],
                    base_winding: "0".to_string(),
                    fill: OriginalPeriodicFillRule::NonZero,
                }],
                &[
                    OriginalPeriodicBooleanOp::Operand(0),
                    OriginalPeriodicBooleanOp::Complement(0),
                ],
            )
            .unwrap(),
        );
        let mut sides = Vec::new();
        for edge in 0..jumps.owner().owner().edges().len() {
            let p = Arc::new(
                own_periodic_support_side_probes(
                    support.clone(),
                    jumps.clone(),
                    edge,
                    ["1".into(), "1024".into()],
                    ParameterOrderLimits::default(),
                )
                .unwrap(),
            );
            let samples = std::array::from_fn(|s| {
                if p.exterior()[s] {
                    None
                } else {
                    Some(Arc::new(
                        own_periodic_material_sample(
                            rule.material().clone(),
                            Arc::new(
                                own_periodic_relative_winding(
                                    jumps.clone(),
                                    &[a.clone(), p.queries()[s].clone()],
                                    ParameterOrderLimits::default(),
                                    1000,
                                )
                                .unwrap(),
                            ),
                        )
                        .unwrap(),
                    ))
                }
            });
            sides.push(Arc::new(
                own_periodic_support_sides(p, rule.clone(), samples).unwrap(),
            ));
        }

        lifts(Arc::new(
            own_periodic_support_selection(rule, &sides).unwrap(),
        ))
    }
    #[test]
    fn actual_periodic_paths_bind_lifted_samples_and_census_distant_positive_and_negative_tiles() {
        let o = cylinder_with_hole();
        let cover = o
            .owner()
            .owner()
            .rule()
            .material()
            .base()
            .owner()
            .owner()
            .owner()
            .owner()
            .owner()
            .pieces();
        let period = Q::from_float(cover.chart()[0][1]).unwrap()
            - Q::from_float(cover.chart()[0][0]).unwrap();
        let low = choose(&o, |p| p[1] < Q::new(1.into(), 8.into()));
        let a = point(&o, low);
        let mut b = a.clone();
        b[0] = ratio(&(decode_ratio(&a[0]).unwrap() + &period * Q::from_integer(2.into())));
        let essential = own_periodic_support_path(
            o.clone(),
            [low, low],
            &[a.clone(), b],
            ParameterOrderLimits::default(),
            1000,
        )
        .unwrap();
        assert_eq!(
            essential.endpoint_period_shifts(),
            &[
                ["0".to_string(), "0".to_string()],
                ["2".to_string(), "0".to_string()]
            ]
        );
        assert_eq!(essential.work(), o.parts_count() * 3);
        let mut stale = essential.clone();
        stale.endpoint_period_shifts[1][0] = "1".into();
        assert!(stale.reverify_source().is_err());
        assert!(
            own_periodic_support_path(
                o.clone(),
                [low, low],
                essential.path(),
                ParameterOrderLimits::default(),
                essential.work() - 1
            )
            .is_err()
        );
        let quarter = Q::new(1.into(), 4.into());
        let three = Q::new(3.into(), 4.into());
        let left = choose(&o, |p| {
            p[0] < quarter && p[0] > Q::new(1.into(), 8.into()) && p[1] > quarter && p[1] < three
        });
        let right = choose(&o, |p| {
            p[0] > three && p[0] < Q::from_integer(1.into()) && p[1] > quarter && p[1] < three
        });
        for shift in [3i64, -3] {
            let lift = |at| {
                let mut p = point(&o, at);
                p[0] = ratio(
                    &(decode_ratio(&p[0]).unwrap() + &period * Q::from_integer(shift.into())),
                );
                p
            };
            let a = lift(left);
            let b = lift(right);
            assert!(
                own_periodic_support_path(
                    o.clone(),
                    [left, right],
                    &[a.clone(), b.clone()],
                    ParameterOrderLimits::default(),
                    1000
                )
                .is_err()
            );
            let y = ["7".into(), "8".into()];
            let path = [a.clone(), [a[0].clone(), y.clone()], [b[0].clone(), y], b];
            let routed = own_periodic_support_path(
                o.clone(),
                [left, right],
                &path,
                ParameterOrderLimits::default(),
                1000,
            )
            .unwrap();
            assert_eq!(
                routed.endpoint_period_shifts(),
                &[
                    [shift.to_string(), "0".to_string()],
                    [shift.to_string(), "0".to_string()]
                ]
            );
            assert_eq!(routed.work(), o.parts_count() * 3);
        }
        let mut fractional = a.clone();
        fractional[0] =
            ratio(&(decode_ratio(&a[0]).unwrap() + &period / Q::from_integer(2.into())));
        assert!(
            own_periodic_support_path(
                o,
                [low, low],
                &[a, fractional],
                ParameterOrderLimits::default(),
                1000
            )
            .is_err()
        );
    }
}
