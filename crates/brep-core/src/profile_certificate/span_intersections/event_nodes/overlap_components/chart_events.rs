//! Complete source-owned chart-shore events, including tangencies and identities.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalChartShoreEvent {
    pub source: usize,
    pub cell: usize,
    pub axis: usize,
    pub shore: usize,
    pub parameter: OriginalSourceParameter,
    pub position: [[ExactPolynomial; 2]; 2],
    pub order: usize,
    pub sides: [Option<i8>; 2],
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalChartShoreCell {
    pub source: usize,
    pub cell: usize,
    pub axis: usize,
    pub shore: usize,
    pub domain: [ExactRatio; 2],
}
#[derive(Clone, Debug)]
pub struct OwnedOriginalChartEvents {
    sources: Vec<Curve>,
    chart: [[f64; 2]; 2],
    wrapped: [bool; 2],
    width: f64,
    refinements: usize,
    events: Vec<OriginalChartShoreEvent>,
    coincident: Vec<OriginalChartShoreCell>,
}
fn derive(
    o: &OwnedOriginalChartEvents,
) -> Result<(Vec<OriginalChartShoreEvent>, Vec<OriginalChartShoreCell>)> {
    if o.sources.len() > 32
        || !o.width.is_finite()
        || !(1e-18..=1e-3).contains(&o.width)
        || o.refinements > 512
        || o.chart
            .iter()
            .any(|a| a.iter().any(|v| !v.is_finite()) || a[0] >= a[1])
    {
        return Err(refused(
            "Invalid original chart-shore source census or domain",
        ));
    }
    let mut events = Vec::new();
    let mut coincident = Vec::new();
    for (source, curve) in o.sources.iter().enumerate() {
        for (cell, c) in cells(curve)?.iter().enumerate() {
            let width =
                Q::from_float(o.width).unwrap() / (&c.domain[1] - &c.domain[0]).max(Q::one());
            for axis in 0..2 {
                if !o.wrapped[axis] {
                    continue;
                }
                for shore in 0..2 {
                    let value = Q::from_float(o.chart[axis][shore]).unwrap();
                    match arithmetic::shore_events::produce(
                        &c.net,
                        &c.domain,
                        axis,
                        &value,
                        &width,
                        o.refinements,
                    )? {
                        None => coincident.push(OriginalChartShoreCell {
                            source,
                            cell,
                            axis,
                            shore,
                            domain: c.domain.clone().map(|q| ratio(&q)),
                        }),
                        Some(roots) => {
                            for r in roots {
                                events.push(OriginalChartShoreEvent {
                                    source,
                                    cell,
                                    axis,
                                    shore,
                                    parameter: OriginalSourceParameter {
                                        primitive: polynomial(&r.primitive),
                                        interval: r.root.map(|q| ratio(&q)),
                                        parameter: polynomial(&r.parameter),
                                    },
                                    position: r.position.map(|c| c.map(|p| polynomial(&p))),
                                    order: r.order,
                                    sides: r.sides,
                                });
                            }
                        }
                    }
                    if events.len() > 16384 || coincident.len() > 8192 {
                        return Err(refused("Original chart-shore event allowance exceeded"));
                    }
                }
            }
        }
    }
    Ok((events, coincident))
}
impl OwnedOriginalChartEvents {
    pub fn sources(&self) -> &[Curve] {
        &self.sources
    }
    pub fn chart(&self) -> [[f64; 2]; 2] {
        self.chart
    }
    pub fn wrapped(&self) -> [bool; 2] {
        self.wrapped
    }
    /// Cell endpoint events retain both original one-sided provenance records.
    /// None means this actual cell has no branch on that side of the endpoint.
    pub fn events(&self) -> &[OriginalChartShoreEvent] {
        &self.events
    }
    pub fn coincident_cells(&self) -> &[OriginalChartShoreCell] {
        &self.coincident
    }
    pub fn reverify_source(&self) -> Result<()> {
        let (e, c) = derive(self)?;
        if e != self.events || c != self.coincident {
            return Err(refused(
                "Original chart-shore root census or identity cells changed",
            ));
        }
        Ok(())
    }
}
pub fn own_original_chart_events(
    sources: &[Curve],
    chart: [[f64; 2]; 2],
    wrapped: [bool; 2],
    width: f64,
    refinements: usize,
) -> Result<OwnedOriginalChartEvents> {
    let mut out = OwnedOriginalChartEvents {
        sources: sources.to_vec(),
        chart,
        wrapped,
        width,
        refinements,
        events: Vec::new(),
        coincident: Vec::new(),
    };
    (out.events, out.coincident) = derive(&out)?;
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replay_rejects_missing_shore_root_side_order_and_identity_cell() {
        let a = Curve::from_polyline(vec![vec![-1., 0.], vec![1., 1.]]).unwrap();
        let x = own_original_chart_events(&[a], [[0., 2.], [-2., 2.]], [true, false], 1e-12, 192)
            .unwrap();
        x.reverify_source().unwrap();
        let mut bad = x.clone();
        bad.events.clear();
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.events[0].sides = [Some(1), Some(1)];
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.events[0].order = 2;
        assert!(bad.reverify_source().is_err());
        let b = Curve::from_polyline(vec![vec![0., 0.], vec![0., 1.]]).unwrap();
        let x = own_original_chart_events(&[b], [[0., 2.], [-2., 2.]], [true, false], 1e-12, 192)
            .unwrap();
        let mut bad = x.clone();
        bad.coincident.clear();
        assert!(bad.reverify_source().is_err());
    }
}
