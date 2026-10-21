//! Source-owned exact point preimages, including algebraic clipping endpoints.
use super::super::super::super::algebraic_intersections::{decode_polynomial, decode_ratio};
use super::*;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPointPreimage {
    pub source_cell: usize,
    /// Maps this joint primitive back to the selected input point primitive.
    pub input_primitive: ExactPolynomial,
    /// Parameters: original input-pair source zero, then target curve.
    pub exact: AlgebraicEventPosition,
}
#[derive(Clone, Debug)]
pub struct OwnedOriginalPointPullback {
    point: Arc<OwnedOriginalPairCuts>,
    cut: usize,
    curve: Curve,
    width: f64,
    preimages: Vec<OriginalPointPreimage>,
}
fn derive(
    point: &OwnedOriginalPairCuts,
    cut: usize,
    curve: &Curve,
    width: f64,
) -> Result<Vec<OriginalPointPreimage>> {
    if !width.is_finite() || !(1e-18..=1e-3).contains(&width) {
        return Err(refused("Invalid original point pullback width"));
    }
    point.reverify_source()?;
    let node = &point
        .cuts()
        .get(cut)
        .ok_or_else(|| refused("Unknown original point cut"))?
        .node
        .exact;
    let f = decode_polynomial(&node.primitive)?;
    let root = [
        decode_ratio(&node.isolating_interval[0])?,
        decode_ratio(&node.isolating_interval[1])?,
    ];
    let position = [
        [
            decode_polynomial(&node.position[0][0])?,
            decode_polynomial(&node.position[0][1])?,
        ],
        [
            decode_polynomial(&node.position[1][0])?,
            decode_polynomial(&node.position[1][1])?,
        ],
    ];
    let first = decode_polynomial(&node.parameters[0])?;
    let original = cells(curve)?;
    let mut out = Vec::new();
    for (source_cell, cell) in original.iter().enumerate() {
        let w = Q::from_float(width).unwrap() / (&cell.domain[1] - &cell.domain[0]).max(Q::one());
        for p in
            arithmetic::algebraic_point_pullback::pullback(&f, &root, &position, &cell.net, &w)?
        {
            // Actual right-sided knot ownership, matching original pair events.
            if source_cell + 1 < original.len()
                && coordinates::equals_one(&p.local_parameter, &p.primitive, &p.interval)?
            {
                continue;
            }
            let left = coordinates::compose(&first, &p.source_primitive, &p.primitive)?;
            let mut right = p
                .local_parameter
                .iter()
                .map(|q| q * (&cell.domain[1] - &cell.domain[0]))
                .collect::<Vec<_>>();
            if right.is_empty() {
                right.push(cell.domain[0].clone());
            } else {
                right[0] += &cell.domain[0];
            }
            while right.last().is_some_and(|q| q.is_zero()) {
                right.pop();
            }
            out.push(OriginalPointPreimage {
                source_cell,
                input_primitive: polynomial(&p.source_primitive),
                exact: AlgebraicEventPosition {
                    primitive: polynomial(&p.primitive),
                    isolating_interval: p.interval.map(|q| ratio(&q)),
                    parameters: [polynomial(&left), polynomial(&right)],
                    position: p.position.map(|c| c.map(|p| polynomial(&p))),
                },
            });
        }
        if out.len() > 4096 {
            return Err(refused(
                "Original point pullback preimage allowance exceeded",
            ));
        }
    }
    Ok(out)
}
impl OwnedOriginalPointPullback {
    pub fn point(&self) -> &Arc<OwnedOriginalPairCuts> {
        &self.point
    }
    pub fn cut(&self) -> usize {
        self.cut
    }
    pub fn curve(&self) -> &Curve {
        &self.curve
    }
    pub fn preimages(&self) -> &[OriginalPointPreimage] {
        &self.preimages
    }
    pub fn reverify_source(&self) -> Result<()> {
        if self.preimages != derive(&self.point, self.cut, &self.curve, self.width)? {
            return Err(refused(
                "Original point pullback source or preimage census changed",
            ));
        }
        Ok(())
    }
}
pub fn own_original_point_pullback(
    point: Arc<OwnedOriginalPairCuts>,
    cut: usize,
    curve: &Curve,
    width: f64,
) -> Result<OwnedOriginalPointPullback> {
    let preimages = derive(&point, cut, curve, width)?;
    Ok(OwnedOriginalPointPullback {
        point,
        cut,
        curve: curve.clone(),
        width,
        preimages,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_replay_rejects_preimage_omission_and_changed_target_parameter() {
        let a = Curve::from_polyline(vec![vec![0., 0.], vec![2., 0.]]).unwrap();
        let b = Curve::from_polyline(vec![vec![1., -1.], vec![1., 1.]]).unwrap();
        let c = certify_span_intersections(&a, &b, false, 1e-12).unwrap();
        let point = Arc::new(
            own_original_pair_cuts(
                Arc::new(own_curve_event_nodes(&a, &b, &c).unwrap()),
                ParameterOrderLimits::default(),
            )
            .unwrap(),
        );
        let target = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.], vec![2., 0.]]).unwrap();
        let x = own_original_point_pullback(point, 0, &target, 1e-12).unwrap();
        x.reverify_source().unwrap();
        assert_eq!(x.preimages().len(), 1);
        assert_eq!(x.preimages()[0].source_cell, 1);
        let mut bad = x.clone();
        bad.preimages.clear();
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.preimages[0].exact.parameters[1].clear();
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.preimages[0].input_primitive.clear();
        assert!(bad.reverify_source().is_err());
    }
}
