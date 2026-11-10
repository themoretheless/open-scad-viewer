//! Original selected boundary expressions on their actual supporting surface.
//! These are exact compositions, not independently fitted spatial NURBS curves.
use super::*;
use crate::profile_certificate::algebraic_intersections::{
    decode_polynomial, decode_ratio, polynomial, ratio,
};
use num_bigint::BigInt;
use std::{cmp::Ordering, sync::Arc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicSurfaceTrace {
    pub component: usize,
    pub walk: usize,
    pub part: usize,
    /// Includes the original algebraic cut selectors and the continuous deck lift.
    pub original: OriginalPeriodicBoundaryPart,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicSurfaceTracePoint {
    pub uv: [ExactRatio; 2],
    pub world: [ExactRatio; 3],
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicSurfaceTraceEndpoint {
    /// The unchanged original primitive, isolating interval and native clock.
    pub parameter: OriginalSourceParameter,
    /// Continuous lifted UV fractions in this endpoint's OWN original root field.
    pub uv: [[ExactPolynomial; 2]; 2],
    /// Exact original supporting-surface values in that SAME selected field.
    pub world: [[ExactPolynomial; 2]; 3],
}
#[derive(Clone, Debug)]
pub struct OwnedPeriodicSupportTraces {
    owner: Arc<OwnedPeriodicSupportRegions>,
    limits: ParameterOrderLimits,
    max_traces: usize,
    traces: Vec<OriginalPeriodicSurfaceTrace>,
}
fn derive(o: &OwnedPeriodicSupportTraces) -> Result<Vec<OriginalPeriodicSurfaceTrace>> {
    let _replay = super::periodic_replay::scope();
    if !(1..=1000000).contains(&o.max_traces)
        || o.limits.max_refinements > 512
        || !(1..=128).contains(&o.limits.max_resultant_size)
    {
        return Err(refused("Invalid original surface trace allowance"));
    }
    super::periodic_replay::verify_arc(&o.owner, |owner| owner.reverify_source())?;
    let lifts = o.owner.owner().owner();
    if lifts.parts_count() > o.max_traces {
        return Err(refused("Original surface trace count exceeds allowance"));
    }
    let mut seen = vec![false; lifts.walks().len()];
    let mut traces = Vec::with_capacity(lifts.parts_count());
    for (component, region) in o.owner.components().iter().enumerate() {
        for &walk in &region.boundary_walks {
            if walk >= seen.len() || seen[walk] {
                return Err(refused(
                    "Surface traces repeat or rebind a component boundary",
                ));
            }
            seen[walk] = true;
            for (part, original) in lifts.walks()[walk].parts.iter().enumerate() {
                traces.push(OriginalPeriodicSurfaceTrace {
                    component,
                    walk,
                    part,
                    original: original.clone(),
                });
            }
        }
    }
    if seen.iter().any(|v| !v) || traces.len() != lifts.parts_count() {
        return Err(refused("Surface traces omit an original selected boundary"));
    }
    Ok(traces)
}
impl OwnedPeriodicSupportTraces {
    pub fn endpoint(
        &self,
        trace: usize,
        end: usize,
        max_work: usize,
    ) -> Result<OriginalPeriodicSurfaceTraceEndpoint> {
        if end > 1 || !(1..=1000000).contains(&max_work) {
            return Err(refused("Invalid original surface endpoint query"));
        }
        self.reverify_source()?;
        let p = &self
            .traces
            .get(trace)
            .ok_or_else(|| refused("Unknown original surface endpoint trace"))?
            .original;
        let parameter = &p.parameters[end];
        let f = decode_polynomial(&parameter.primitive)?;
        let root = [
            decode_ratio(&parameter.interval[0])?,
            decode_ratio(&parameter.interval[1])?,
        ];
        let support = self.owner.owner().owner().owner().owner().rule().support();
        let source = support.augmented().surface();
        let cover = source.pieces();
        let mut uv = [[Vec::new(), Vec::new()], [Vec::new(), Vec::new()]];
        for axis in 0..2 {
            let mut n = decode_polynomial(&p.endpoints[end][axis][0])?;
            let d = decode_polynomial(&p.endpoints[end][axis][1])?;
            let deck: BigInt = p.period_shift[axis]
                .parse()
                .map_err(|_| refused("Invalid actual endpoint deck shift"))?;
            if !cover.wrapped()[axis] && !deck.is_zero() {
                return Err(refused(
                    "Endpoint deck shift changes an unwrapped coordinate",
                ));
            }
            let chart = cover.chart()[axis];
            let period = Q::from_float(chart[1]).unwrap() - Q::from_float(chart[0]).unwrap();
            let offset = Q::from_integer(deck) * period;
            n.resize(n.len().max(d.len()), Q::zero());
            for (i, q) in d.iter().enumerate() {
                n[i] -= &offset * q;
            }
            // Only the proved history deck shift is removed. The canonical
            // original piece translation remains, including actual shore ends.
            uv[axis] = [n, d];
        }
        let world = arithmetic::surface_expression::evaluate(
            source.surface(),
            &f,
            &root,
            &uv,
            self.limits.max_refinements,
            max_work,
        )?;
        Ok(OriginalPeriodicSurfaceTraceEndpoint {
            parameter: parameter.clone(),
            uv: p.endpoints[end].clone(),
            world: world.map(|v| v.map(|p| polynomial(&p))),
        })
    }
    pub fn verify_endpoint(
        &self,
        trace: usize,
        end: usize,
        endpoint: &OriginalPeriodicSurfaceTraceEndpoint,
        max_work: usize,
    ) -> Result<()> {
        if self.endpoint(trace, end, max_work)? != *endpoint {
            return Err(refused(
                "Original surface endpoint coordinates or selected root changed",
            ));
        }
        Ok(())
    }
    pub fn owner(&self) -> &Arc<OwnedPeriodicSupportRegions> {
        &self.owner
    }
    pub fn traces(&self) -> &[OriginalPeriodicSurfaceTrace] {
        &self.traces
    }
    pub fn reverify_source(&self) -> Result<()> {
        if derive(self)? != self.traces {
            return Err(refused(
                "Original surface trace source, cuts or component changed",
            ));
        }
        Ok(())
    }
    /// A rational native-clock query inside the CLOSED actual selected cuts.
    /// Algebraic endpoints remain exact selectors in traces(); this query does
    /// not replace them by interval midpoints or publish a manifold face.
    pub fn point(
        &self,
        trace: usize,
        native: &ExactRatio,
    ) -> Result<OriginalPeriodicSurfaceTracePoint> {
        self.reverify_source()?;
        let p = &self
            .traces
            .get(trace)
            .ok_or_else(|| refused("Unknown original surface trace"))?
            .original;
        let t = decode_ratio(native)?;
        let mut order = Vec::new();
        for cut in &p.parameters {
            order.push(arithmetic::parameter_order::compare(
                &vec![Q::zero(), Q::one()],
                &vec![t.clone()],
                &[Q::zero(), Q::zero()],
                &decode_polynomial(&cut.primitive)?,
                &decode_polynomial(&cut.parameter)?,
                &[
                    decode_ratio(&cut.interval[0])?,
                    decode_ratio(&cut.interval[1])?,
                ],
                self.limits.max_refinements,
                self.limits.max_resultant_size,
            )?);
        }
        let inside = if p.direction > 0 {
            order[0] != Ordering::Less && order[1] != Ordering::Greater
        } else {
            order[0] != Ordering::Greater && order[1] != Ordering::Less
        };
        if !inside {
            return Err(refused(
                "Surface trace query escaped its original selected cuts",
            ));
        }
        let support = self.owner.owner().owner().owner().owner().rule().support();
        let original = support.augmented().surface().pieces();
        let source = original
            .sources()
            .get(p.source)
            .ok_or_else(|| refused("Surface trace lost original pcurve"))?;
        let source_cells = cells(source)?;
        let c = source_cells
            .get(p.cell)
            .ok_or_else(|| refused("Surface trace lost original source cell"))?;
        if t < c.domain[0] || t > c.domain[1] {
            return Err(refused(
                "Surface trace query escaped its original native cell",
            ));
        }
        let s = (&t - &c.domain[0]) / (&c.domain[1] - &c.domain[0]);
        let left = Q::one() - &s;
        let mut net = c.net.clone();
        while net.len() > 1 {
            net = net
                .windows(2)
                .map(|v| std::array::from_fn(|axis| &left * &v[0][axis] + &s * &v[1][axis]))
                .collect();
        }
        if net[0][2] <= Q::zero() {
            return Err(refused("Surface trace query denominator is not positive"));
        }
        let uv = [
            ratio(&(&net[0][0] / &net[0][2] + decode_ratio(&p.translation[0])?)),
            ratio(&(&net[0][1] / &net[0][2] + decode_ratio(&p.translation[1])?)),
        ];
        let world = support.world_uv_point(&uv)?;
        Ok(OriginalPeriodicSurfaceTracePoint { uv, world })
    }
}
pub fn own_periodic_support_traces(
    owner: Arc<OwnedPeriodicSupportRegions>,
    limits: ParameterOrderLimits,
    max_traces: usize,
) -> Result<OwnedPeriodicSupportTraces> {
    let mut o = OwnedPeriodicSupportTraces {
        owner,
        limits,
        max_traces,
        traces: Vec::new(),
    };
    o.traces = derive(&o)?;
    Ok(o)
}

#[cfg(test)]
mod tests {
    use super::super::periodic_support_regions::tests as fixtures;
    use super::*;
    #[test]
    fn original_surface_traces_keep_every_selected_native_curve_and_reject_rebinding() {
        let signatures = fixtures::signatures(
            super::super::periodic_support_material::tests::full_plane(true),
        );
        let regions = Arc::new(own_periodic_support_regions(signatures, &[], 100000).unwrap());
        assert!(
            own_periodic_support_traces(regions.clone(), ParameterOrderLimits::default(), 1)
                .is_err()
        );
        let o = own_periodic_support_traces(regions, ParameterOrderLimits::default(), 16).unwrap();
        assert_eq!(o.traces().len(), 4);
        let mut reversed = false;
        for (i, t) in o.traces().iter().enumerate() {
            assert_eq!(t.component, 0);
            reversed |= t.original.direction < 0;
            let p = o.point(i, &["1".into(), "2".into()]).unwrap();
            assert_eq!(
                p.world,
                [p.uv[0].clone(), p.uv[1].clone(), ["0".into(), "1".into()]]
            );
            for q in ["0", "1"] {
                o.point(i, &[q.into(), "1".into()]).unwrap();
            }
            assert!(o.point(i, &["2".into(), "1".into()]).is_err());
            for end in 0..2 {
                let endpoint = o.endpoint(i, end, 100000).unwrap();
                assert_eq!(endpoint.parameter, t.original.parameters[end]);
                assert_eq!(endpoint.uv, t.original.endpoints[end]);
                let f = decode_polynomial(&endpoint.parameter.primitive).unwrap();
                let root = [
                    decode_ratio(&endpoint.parameter.interval[0]).unwrap(),
                    decode_ratio(&endpoint.parameter.interval[1]).unwrap(),
                ];
                for axis in 0..2 {
                    assert!(
                        arithmetic::event_coordinates::translated_equal(
                            &decode_polynomial(&endpoint.uv[axis][0]).unwrap(),
                            &decode_polynomial(&endpoint.uv[axis][1]).unwrap(),
                            &decode_polynomial(&endpoint.world[axis][0]).unwrap(),
                            &decode_polynomial(&endpoint.world[axis][1]).unwrap(),
                            &Q::zero(),
                            &f,
                            &root,
                        )
                        .unwrap()
                    );
                }
                assert!(
                    arithmetic::event_coordinates::zero_at(
                        &decode_polynomial(&endpoint.world[2][0]).unwrap(),
                        &f,
                        &root
                    )
                    .unwrap()
                );
            }
        }
        assert!(reversed);
        assert!(o.endpoint(0, 2, 100000).is_err());
        assert!(o.endpoint(0, 0, 1).is_err());
        let mut endpoint = o.endpoint(0, 0, 100000).unwrap();
        o.verify_endpoint(0, 0, &endpoint, 100000).unwrap();
        endpoint.world[0][0] = vec![["999".into(), "1".into()]];
        assert!(o.verify_endpoint(0, 0, &endpoint, 100000).is_err());
        assert!(o.point(4, &["1".into(), "2".into()]).is_err());
        assert!(o.point(0, &["1".into(), "0".into()]).is_err());
        let mut changed = o.clone();
        changed.traces[0].component = 1;
        assert!(changed.reverify_source().is_err());
        let mut changed = o.clone();
        changed.traces[0].original.translation[0] = ["1".into(), "3".into()];
        assert!(changed.reverify_source().is_err());
        let mut changed = o.clone();
        changed.traces[0].original.parameters.swap(0, 1);
        assert!(changed.reverify_source().is_err());
    }
}
