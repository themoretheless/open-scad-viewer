//! Dequotient ORIGINAL vertex uses into their canonical UV chart copies.
//! This does not invent auxiliary corners/cuts or publish local face patches.
use super::*;
use crate::profile_certificate::algebraic_intersections::{
    decode_polynomial, decode_ratio, polynomial,
};
use num_bigint::BigInt;
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone, Copy, Debug)]
pub struct PeriodicSourceChartCopyLimits {
    pub max_copies: usize,
    pub max_source_uses: usize,
    pub max_root_checks: usize,
}
impl Default for PeriodicSourceChartCopyLimits {
    fn default() -> Self {
        Self {
            max_copies: 65536,
            max_source_uses: 100000,
            max_root_checks: 1000000,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicSourceChartCopy {
    pub source_vertex: usize,
    pub period_shift: [String; 2],
    /// Indices into this ACTUAL original vertex chart's source uses.
    pub uses: Vec<usize>,
    pub parameter: OriginalSourceParameter,
    /// Canonical PIECE coordinates, not the parent's common intrinsic chart.
    pub uv: [[ExactPolynomial; 2]; 2],
    /// Exact lower/upper native shore membership, including wrapped shores.
    pub bounds: [Option<usize>; 2],
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicSourceChartWorldPoint {
    pub parameter: OriginalSourceParameter,
    pub world: [[ExactPolynomial; 2]; 3],
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Data {
    copies: Vec<OriginalPeriodicSourceChartCopy>,
    source_uses: usize,
    root_checks: usize,
}
#[derive(Clone, Debug)]
pub struct OwnedPeriodicSourceChartCopies {
    owner: Arc<OwnedPeriodicSupportIncidence>,
    parameter_limits: ParameterOrderLimits,
    limits: PeriodicSourceChartCopyLimits,
    data: Data,
}
fn coordinate(p: &[Q]) -> Result<ExactPolynomial> {
    let mut end = p.len();
    while end > 0 && p[end - 1].is_zero() {
        end -= 1;
    }
    let p = &p[..end];
    if p.len() > 513
        || p.iter()
            .any(|q| q.numer().bits() > 16384 || q.denom().bits() > 16384)
    {
        return Err(refused(
            "Original canonical chart coordinate allowance exceeded",
        ));
    }
    Ok(polynomial(&p.to_vec()))
}
fn sign(
    p: &Vec<Q>,
    f: &Vec<Q>,
    root: &[Q; 2],
    n: &mut usize,
    cap: usize,
    refinements: usize,
) -> Result<i8> {
    *n = n
        .checked_add(1)
        .filter(|n| *n <= cap)
        .ok_or_else(|| refused("Original chart copy root-check allowance exhausted"))?;
    arithmetic::event_coordinates::sign_at(p, f, root, refinements)
}
const MAX_ALLOWANCE: usize = 1000000;
const MAX_REFINEMENTS: usize = 512;
const MAX_RESULTANT_SIZE: usize = 128;
/// The native chart and the shared root-check counter of one derivation.
struct Copier {
    native: [[Q; 2]; 2],
    periods: [Q; 2],
    wrapped: [bool; 2],
    max_root_checks: usize,
    refinements: usize,
    root_checks: usize,
}
impl Copier {
    fn sign(&mut self, p: &Vec<Q>, f: &Vec<Q>, root: &[Q; 2]) -> Result<i8> {
        sign(
            p,
            f,
            root,
            &mut self.root_checks,
            self.max_root_checks,
            self.refinements,
        )
    }
    /// Canonical coordinate of one chart copy on one axis: the use's position
    /// minus its whole-period translation, with its exact membership in the
    /// lower or upper native bound.
    fn coordinate(
        &mut self,
        u: &OriginalPeriodicVertexChartUse,
        shift: &str,
        axis: usize,
        f: &Vec<Q>,
        root: &[Q; 2],
    ) -> Result<([ExactPolynomial; 2], Option<usize>)> {
        let shift: BigInt = shift
            .parse()
            .map_err(|_| refused("Invalid actual chart copy period shift"))?;
        let offset = decode_ratio(&u.translation[axis])?;
        if offset != Q::from_integer(shift.clone()) * &self.periods[axis]
            || (!self.wrapped[axis] && !shift.is_zero())
        {
            return Err(refused(
                "Original chart copy transport is not its actual whole period",
            ));
        }
        let mut n = decode_polynomial(&u.position[axis][0])?;
        let d = decode_polynomial(&u.position[axis][1])?;
        n.resize(n.len().max(d.len()), Q::zero());
        for (i, q) in d.iter().enumerate() {
            n[i] -= &offset * q;
        }
        // Exact subtraction may cancel the entire numerator. Preserve
        // the codec's canonical zero/trailing-coefficient convention
        // and coefficient ceiling before any derived point is issued.
        let encoded = [coordinate(&n)?, coordinate(&d)?];
        let ds = self.sign(&d, f, root)?;
        if ds == 0 {
            return Err(refused(
                "Original chart copy coordinate denominator vanishes",
            ));
        }
        let mut sides = [0i8, 0i8];
        for end in 0..2 {
            let mut delta = n.clone();
            delta.resize(delta.len().max(d.len()), Q::zero());
            for (i, q) in d.iter().enumerate() {
                delta[i] -= &self.native[axis][end] * q;
            }
            sides[end] = ds * self.sign(&delta, f, root)?;
        }
        if sides[0] < 0 || sides[1] > 0 || (sides[0] == 0 && sides[1] == 0) {
            return Err(refused(
                "Original canonical chart copy escaped actual native bounds",
            ));
        }
        let bound = if sides[0] == 0 {
            Some(0)
        } else if sides[1] == 0 {
            Some(1)
        } else {
            None
        };
        Ok((encoded, bound))
    }
    /// One canonical copy of a vertex: every use with the same period shift.
    /// Parent transport proves every use in this group has the SAME
    /// common point and SAME whole-period offset, even in distinct root
    /// fields. Subtracting that offset preserves exact copy identity.
    fn copy(
        &mut self,
        v: &OriginalPeriodicVertexChart,
        period_shift: [String; 2],
        uses: Vec<usize>,
    ) -> Result<OriginalPeriodicSourceChartCopy> {
        let u = &v.uses[uses[0]];
        let f = decode_polynomial(&u.parameter.primitive)?;
        let root = [
            decode_ratio(&u.parameter.interval[0])?,
            decode_ratio(&u.parameter.interval[1])?,
        ];
        let (first, first_bound) = self.coordinate(u, &period_shift[0], 0, &f, &root)?;
        let (second, second_bound) = self.coordinate(u, &period_shift[1], 1, &f, &root)?;
        Ok(OriginalPeriodicSourceChartCopy {
            source_vertex: v.vertex,
            period_shift,
            uses,
            parameter: u.parameter.clone(),
            uv: [first, second],
            bounds: [first_bound, second_bound],
        })
    }
}
fn derive(o: &OwnedPeriodicSourceChartCopies) -> Result<Data> {
    let _replay = super::periodic_replay::scope();
    if [
        o.limits.max_copies,
        o.limits.max_source_uses,
        o.limits.max_root_checks,
    ]
    .iter()
    .any(|n| !(1..=MAX_ALLOWANCE).contains(n))
        || o.parameter_limits.max_refinements > MAX_REFINEMENTS
        || !(1..=MAX_RESULTANT_SIZE).contains(&o.parameter_limits.max_resultant_size)
    {
        return Err(refused("Invalid original chart copy allowance"));
    }
    super::periodic_replay::verify_arc(&o.owner, |owner| owner.reverify_source())?;
    let traces = o.owner.owner();
    let regions = traces.owner();
    let lifts = regions.owner().owner();
    let cycles = lifts.owner();
    let charts = cycles.rotation().owner().owner();
    let support = cycles.owner().rule().support();
    let cover = support.augmented().surface().pieces();
    let mut native = [[Q::zero(), Q::zero()], [Q::zero(), Q::zero()]];
    for (exact, bounds) in native.iter_mut().zip(cover.chart()) {
        for (q, v) in exact.iter_mut().zip(bounds) {
            *q =
                Q::from_float(v).ok_or_else(|| refused("Invalid original chart copy allowance"))?;
        }
    }
    let periods = native.clone().map(|d| d[1].clone() - &d[0]);
    let source_uses = charts.charts().iter().try_fold(0usize, |n, v| {
        n.checked_add(v.uses.len())
            .ok_or_else(|| refused("Original chart use census overflow"))
    })?;
    if source_uses > o.limits.max_source_uses || charts.charts().len() > o.limits.max_copies {
        return Err(refused(
            "Complete original chart copy census exceeds allowance",
        ));
    }
    let mut copier = Copier {
        native,
        periods,
        wrapped: cover.wrapped(),
        max_root_checks: o.limits.max_root_checks,
        refinements: o.parameter_limits.max_refinements,
        root_checks: 0,
    };
    let mut copies = Vec::new();
    for (vertex, v) in charts.charts().iter().enumerate() {
        if v.vertex != vertex {
            return Err(refused(
                "Original chart copies lost physical vertex identity",
            ));
        }
        let mut groups = BTreeMap::<[String; 2], Vec<usize>>::new();
        for (i, u) in v.uses.iter().enumerate() {
            groups.entry(u.period_shift.clone()).or_default().push(i);
        }
        for (period_shift, uses) in groups {
            if copies.len() >= o.limits.max_copies {
                return Err(refused("Original chart copy count exceeds allowance"));
            }
            copies.push(copier.copy(v, period_shift, uses)?);
        }
    }
    Ok(Data {
        copies,
        source_uses,
        root_checks: copier.root_checks,
    })
}
impl OwnedPeriodicSourceChartCopies {
    pub fn owner(&self) -> &Arc<OwnedPeriodicSupportIncidence> {
        &self.owner
    }
    pub fn copies(&self) -> &[OriginalPeriodicSourceChartCopy] {
        &self.data.copies
    }
    pub fn source_uses(&self) -> usize {
        self.data.source_uses
    }
    pub fn root_checks(&self) -> usize {
        self.data.root_checks
    }
    pub fn reverify_source(&self) -> Result<()> {
        if derive(self)? != self.data {
            return Err(refused(
                "Original chart copy source, root, UV or shore membership changed",
            ));
        }
        Ok(())
    }
    pub fn world_point(
        &self,
        copy: usize,
        max_work: usize,
    ) -> Result<OriginalPeriodicSourceChartWorldPoint> {
        self.reverify_source()?;
        let c = self
            .data
            .copies
            .get(copy)
            .ok_or_else(|| refused("Unknown original chart copy"))?;
        let traces = self.owner.owner();
        let regions = traces.owner();
        let lifts = regions.owner().owner();
        let cycles = lifts.owner();
        let surface = cycles
            .owner()
            .rule()
            .support()
            .augmented()
            .surface()
            .surface();
        let f = decode_polynomial(&c.parameter.primitive)?;
        let root = [
            decode_ratio(&c.parameter.interval[0])?,
            decode_ratio(&c.parameter.interval[1])?,
        ];
        let uv = [
            [
                decode_polynomial(&c.uv[0][0])?,
                decode_polynomial(&c.uv[0][1])?,
            ],
            [
                decode_polynomial(&c.uv[1][0])?,
                decode_polynomial(&c.uv[1][1])?,
            ],
        ];
        let world = arithmetic::surface_expression::evaluate(
            surface,
            &f,
            &root,
            &uv,
            self.parameter_limits.max_refinements,
            max_work,
        )?;
        Ok(OriginalPeriodicSourceChartWorldPoint {
            parameter: c.parameter.clone(),
            world: world.map(|v| v.map(|p| polynomial(&p))),
        })
    }
}
pub fn own_periodic_source_chart_copies(
    owner: Arc<OwnedPeriodicSupportIncidence>,
    parameter_limits: ParameterOrderLimits,
    limits: PeriodicSourceChartCopyLimits,
) -> Result<OwnedPeriodicSourceChartCopies> {
    let mut o = OwnedPeriodicSourceChartCopies {
        owner,
        parameter_limits,
        limits,
        data: Data {
            copies: Vec::new(),
            source_uses: 0,
            root_checks: 0,
        },
    };
    o.data = derive(&o)?;
    Ok(o)
}

#[cfg(test)]
mod tests {
    use super::super::periodic_support_regions::tests as fixtures;
    use super::*;
    fn incidence(regions: Arc<OwnedPeriodicSupportRegions>) -> Arc<OwnedPeriodicSupportIncidence> {
        let traces = Arc::new(
            own_periodic_support_traces(regions, ParameterOrderLimits::default(), 1000).unwrap(),
        );
        Arc::new(own_periodic_support_incidence(traces, 1000).unwrap())
    }
    #[test]
    fn original_chart_copies_keep_actual_plane_corners_and_reject_cached_transport_changes() {
        let signatures = fixtures::signatures(
            super::super::periodic_support_material::tests::full_plane(true),
        );
        let source = incidence(Arc::new(
            own_periodic_support_regions(signatures, &[], 100000).unwrap(),
        ));
        let limits = PeriodicSourceChartCopyLimits::default();
        assert!(
            own_periodic_source_chart_copies(
                source.clone(),
                ParameterOrderLimits::default(),
                PeriodicSourceChartCopyLimits {
                    max_copies: 1,
                    ..limits
                }
            )
            .is_err()
        );
        assert!(
            own_periodic_source_chart_copies(
                source.clone(),
                ParameterOrderLimits::default(),
                PeriodicSourceChartCopyLimits {
                    max_source_uses: 1,
                    ..limits
                }
            )
            .is_err()
        );
        assert!(
            own_periodic_source_chart_copies(
                source.clone(),
                ParameterOrderLimits::default(),
                PeriodicSourceChartCopyLimits {
                    max_root_checks: 1,
                    ..limits
                }
            )
            .is_err()
        );
        let o = own_periodic_source_chart_copies(source, ParameterOrderLimits::default(), limits)
            .unwrap();
        assert_eq!(
            (o.copies().len(), o.source_uses(), o.root_checks()),
            (4, 8, 24)
        );
        assert!(
            o.copies()
                .iter()
                .all(|c| c.period_shift == ["0", "0"] && c.bounds.iter().all(Option::is_some))
        );
        let corners = o
            .copies()
            .iter()
            .map(|c| c.bounds.map(Option::unwrap))
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(corners.len(), 4);
        let p = o.world_point(0, 100000).unwrap();
        assert_eq!(p.parameter, o.copies()[0].parameter);
        let f = decode_polynomial(&p.parameter.primitive).unwrap();
        let root = [
            decode_ratio(&p.parameter.interval[0]).unwrap(),
            decode_ratio(&p.parameter.interval[1]).unwrap(),
        ];
        for axis in 0..3 {
            let n = decode_polynomial(&p.world[axis][0]).unwrap();
            let d = decode_polynomial(&p.world[axis][1]).unwrap();
            let expected = if axis == 2 {
                Q::zero()
            } else {
                Q::from_integer(o.copies()[0].bounds[axis].unwrap().into())
            };
            assert_eq!(
                arithmetic::parameter_order::compare_rational(
                    arithmetic::parameter_order::RationalParameterRef::new(&f, [&n, &d], &root),
                    arithmetic::parameter_order::RationalParameterRef::new(
                        &vec![Q::zero(), Q::one()],
                        [&vec![expected], &vec![Q::one()]],
                        &[Q::zero(), Q::zero()],
                    ),
                    128,
                    128,
                )
                .unwrap(),
                std::cmp::Ordering::Equal
            );
        }
        let mut changed = o.clone();
        changed.data.copies[0].period_shift[0] = "1".into();
        assert!(changed.reverify_source().is_err());
        let mut changed = o.clone();
        changed.data.copies[0].uv[0][0] = vec![["999".into(), "1".into()]];
        assert!(changed.reverify_source().is_err());
        let mut changed = o.clone();
        changed.data.copies[0].bounds[0] = None;
        assert!(changed.reverify_source().is_err());
        let mut changed = o.clone();
        changed.data.copies[0].uses.clear();
        assert!(changed.reverify_source().is_err());
    }
    #[test]
    fn original_chart_copies_separate_both_actual_periodic_shores_without_splitting_world_vertices()
    {
        let signatures = Arc::new(
            own_periodic_support_signatures(
                fixtures::cylinder_bands(),
                ParameterOrderLimits::default(),
                10000,
            )
            .unwrap(),
        );
        let search = construct_periodic_support_regions(
            signatures,
            ParameterOrderLimits::default(),
            PeriodicRegionSearchLimits::default(),
        )
        .unwrap();
        assert_eq!(search.stop(), None);
        let o = own_periodic_source_chart_copies(
            incidence(Arc::new(search.regions().unwrap().clone())),
            ParameterOrderLimits::default(),
            PeriodicSourceChartCopyLimits::default(),
        )
        .unwrap();
        assert_eq!(o.copies().len(), 12);
        assert_eq!(
            o.copies()
                .iter()
                .filter(|c| c.bounds.iter().all(Option::is_some))
                .count(),
            4
        );
        let mut groups = BTreeMap::<usize, Vec<usize>>::new();
        for (i, c) in o.copies().iter().enumerate() {
            assert!(c.bounds[0].is_some());
            groups.entry(c.source_vertex).or_default().push(i);
        }
        assert_eq!(groups.len(), 6);
        assert!(groups.values().all(|v| v.len() == 2));
        for v in groups.values() {
            assert_ne!(o.copies()[v[0]].bounds[0], o.copies()[v[1]].bounds[0]);
        }
        let v = groups.values().next().unwrap();
        let a = o.world_point(v[0], 100000).unwrap();
        let b = o.world_point(v[1], 100000).unwrap();
        let f = decode_polynomial(&a.parameter.primitive).unwrap();
        let g = decode_polynomial(&b.parameter.primitive).unwrap();
        let ar = [
            decode_ratio(&a.parameter.interval[0]).unwrap(),
            decode_ratio(&a.parameter.interval[1]).unwrap(),
        ];
        let br = [
            decode_ratio(&b.parameter.interval[0]).unwrap(),
            decode_ratio(&b.parameter.interval[1]).unwrap(),
        ];
        for axis in 0..3 {
            let an = decode_polynomial(&a.world[axis][0]).unwrap();
            let ad = decode_polynomial(&a.world[axis][1]).unwrap();
            let bn = decode_polynomial(&b.world[axis][0]).unwrap();
            let bd = decode_polynomial(&b.world[axis][1]).unwrap();
            assert_eq!(
                arithmetic::parameter_order::compare_rational(
                    arithmetic::parameter_order::RationalParameterRef::new(&f, [&an, &ad], &ar),
                    arithmetic::parameter_order::RationalParameterRef::new(&g, [&bn, &bd], &br),
                    128,
                    128,
                )
                .unwrap(),
                std::cmp::Ordering::Equal
            );
        }
    }
}
