//! Owned ordering/identity of actual shared ORIGINAL source parameters.
use super::super::super::algebraic_intersections::{decode_polynomial, decode_ratio};
use super::*;
use std::{cmp::Ordering, sync::Arc};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EventParameterRef {
    pub node: usize,
    pub source: usize,
}
#[derive(Clone, Copy, Debug)]
pub struct ParameterOrderLimits {
    /// Rounds: at most one bisection of each original primitive root per round.
    /// Complete original census replay has its own existing allowances.
    pub max_refinements: usize,
    /// Maximum Sylvester matrix size for each individual value eliminant.
    pub max_resultant_size: usize,
}
impl Default for ParameterOrderLimits {
    fn default() -> Self {
        Self {
            max_refinements: 192,
            max_resultant_size: 64,
        }
    }
}
#[derive(Clone, Debug)]
pub struct QualifiedParameterOrder {
    owners: [Arc<CurveEventNodes>; 2],
    refs: [EventParameterRef; 2],
    limits: ParameterOrderLimits,
    order: Ordering,
}
fn decide(
    owners: &[Arc<CurveEventNodes>; 2],
    r: [EventParameterRef; 2],
    l: ParameterOrderLimits,
) -> Result<Ordering> {
    if l.max_refinements > 512
        || !(1..=128).contains(&l.max_resultant_size)
        || r.iter()
            .zip(owners)
            .any(|(r, o)| r.source > 1 || r.node >= o.nodes.len())
    {
        return Err(refused(
            "Invalid original event parameter reference or comparison allowance",
        ));
    }
    owners[0].reverify_source()?;
    owners[1].reverify_source()?;
    if owners[0].sources[r[0].source] != owners[1].sources[r[1].source] {
        return Err(refused(
            "Parameter ordering requires the same actual original source definition",
        ));
    }
    let a = &owners[0].nodes[r[0].node].exact;
    let b = &owners[1].nodes[r[1].node].exact;
    let interval = |i: &[ExactRatio; 2]| -> Result<[Q; 2]> {
        Ok([decode_ratio(&i[0])?, decode_ratio(&i[1])?])
    };
    arithmetic::parameter_order::compare(
        &decode_polynomial(&a.primitive)?,
        &decode_polynomial(&a.parameters[r[0].source])?,
        &interval(&a.isolating_interval)?,
        &decode_polynomial(&b.primitive)?,
        &decode_polynomial(&b.parameters[r[1].source])?,
        &interval(&b.isolating_interval)?,
        l.max_refinements,
        l.max_resultant_size,
    )
}
impl QualifiedParameterOrder {
    pub fn ordering(&self) -> Ordering {
        self.order
    }
    pub fn owners(&self) -> &[Arc<CurveEventNodes>; 2] {
        &self.owners
    }
    pub fn references(&self) -> [EventParameterRef; 2] {
        self.refs
    }
    pub fn reverify_source(&self) -> Result<()> {
        if self.order != decide(&self.owners, self.refs, self.limits)? {
            return Err(refused("Original parameter identity/order receipt changed"));
        }
        Ok(())
    }
}
/// Exact ordering on one shared actual curve. Each algebraic value stays bound
/// to its full original event census. Equality requires a common exact value
/// polynomial and a unique common selected root; overlapping intervals are
/// insufficient. Different source definitions refuse rather than granting a
/// caller assertion of source identity. Failure never becomes Equal.
pub fn compare_event_parameters(
    first: Arc<CurveEventNodes>,
    a: EventParameterRef,
    second: Arc<CurveEventNodes>,
    b: EventParameterRef,
    limits: ParameterOrderLimits,
) -> Result<QualifiedParameterOrder> {
    let owners = [first, second];
    let refs = [a, b];
    let order = decide(&owners, refs, limits)?;
    Ok(QualifiedParameterOrder {
        owners,
        refs,
        limits,
        order,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parameter_order_receipt_replays_actual_sources_bindings_and_decision() {
        let a = Curve::from_polyline(vec![vec![0., 0.], vec![1., 1.]]).unwrap();
        let b = Curve::from_polyline(vec![vec![1., 1.], vec![2., 0.]]).unwrap();
        let c = certify_span_intersections(&a, &b, false, 1e-12).unwrap();
        let nodes = Arc::new(own_curve_event_nodes(&a, &b, &c).unwrap());
        let r = EventParameterRef { node: 0, source: 0 };
        let owner =
            compare_event_parameters(nodes.clone(), r, nodes, r, ParameterOrderLimits::default())
                .unwrap();
        owner.reverify_source().unwrap();
        let mut bad = owner.clone();
        bad.order = Ordering::Less;
        assert!(bad.reverify_source().is_err());
        let mut bad = owner.clone();
        bad.refs[0].source = 1;
        assert!(bad.reverify_source().is_err());
        let mut bad = owner.clone();
        bad.refs[0].node = 99;
        assert!(bad.reverify_source().is_err());
        let mut bad = owner.clone();
        bad.limits.max_refinements = 513;
        assert!(bad.reverify_source().is_err());
        let mut bad = owner;
        Arc::make_mut(&mut bad.owners[0]).nodes[0].exact.parameters[0].clear();
        assert!(bad.reverify_source().is_err());
    }
}
