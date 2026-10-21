//! Owned original monotone carrier on one exact chart shore.
use super::*;
use std::sync::Arc;
#[derive(Clone, Debug)]
pub struct OwnedOriginalShoreCarrier {
    chart: Arc<OwnedOriginalChartVertices>,
    carrier: usize,
    axis: usize,
    shore: usize,
    limits: ParameterOrderLimits,
    cells: Vec<usize>,
    direction: i8,
}
fn derive(
    chart: &OwnedOriginalChartVertices,
    carrier: usize,
    axis: usize,
    shore: usize,
    limits: ParameterOrderLimits,
) -> Result<(Vec<usize>, i8)> {
    chart.reverify_source()?;
    let owner = chart.graph().owner();
    if axis > 1 || shore > 1 || !chart.wrapped()[axis] || carrier >= owner.carriers().len() {
        return Err(refused("Invalid original periodic shore carrier reference"));
    }
    let c = &owner.carriers()[carrier];
    let bounds = [
        automatic_boundaries::bound(owner, c.source, &c.cuts[0])?,
        automatic_boundaries::bound(owner, c.source, &c.cuts[1])?,
    ];
    let cells = cells(&owner.atlas().sources()[c.source])?
        .into_iter()
        .map(|cell| (cell.net, cell.domain))
        .collect::<Vec<_>>();
    arithmetic::shore_carrier::prove(
        &cells,
        &bounds,
        axis,
        &Q::from_float(chart.chart()[axis][shore]).unwrap(),
        limits.max_refinements,
        limits.max_resultant_size,
    )
}
impl OwnedOriginalShoreCarrier {
    pub fn chart(&self) -> &Arc<OwnedOriginalChartVertices> {
        &self.chart
    }
    pub fn carrier(&self) -> usize {
        self.carrier
    }
    pub fn axis(&self) -> usize {
        self.axis
    }
    pub fn shore(&self) -> usize {
        self.shore
    }
    pub fn cells(&self) -> &[usize] {
        &self.cells
    }
    pub fn direction(&self) -> i8 {
        self.direction
    }
    /// Quotient vertex plus other-axis upper-shore provenance reconstructs
    /// exact transverse endpoint identity, including doubly wrapped corners.
    pub fn endpoint_keys(&self) -> [(usize, bool); 2] {
        self.chart.graph().ends()[self.carrier].map(|v| {
            (
                self.chart.vertex_groups()[v],
                self.chart.upper_shifts()[v][1 - self.axis],
            )
        })
    }
    pub fn reverify_source(&self) -> Result<()> {
        let (cells, direction) = derive(
            &self.chart,
            self.carrier,
            self.axis,
            self.shore,
            self.limits,
        )?;
        if cells != self.cells || direction != self.direction {
            return Err(refused(
                "Original shore carrier source census or direction changed",
            ));
        }
        Ok(())
    }
}
/// Complete actual source-cell identity and derivative-root census prove a
/// monotone original carrier on one wrapped chart shore. Endpoint critical
/// parameters remain owned cuts. This does not pair opposite shores or admit
/// physical surface agreement merely from matching endpoint keys.
pub fn own_original_shore_carrier(
    chart: Arc<OwnedOriginalChartVertices>,
    carrier: usize,
    axis: usize,
    shore: usize,
    limits: ParameterOrderLimits,
) -> Result<OwnedOriginalShoreCarrier> {
    let (cells, direction) = derive(&chart, carrier, axis, shore, limits)?;
    Ok(OwnedOriginalShoreCarrier {
        chart,
        carrier,
        axis,
        shore,
        limits,
        cells,
        direction,
    })
}
#[derive(Clone, Debug)]
pub struct OwnedOriginalShorePair {
    first: OwnedOriginalShoreCarrier,
    second: OwnedOriginalShoreCarrier,
    reversed: bool,
}
fn pair(first: &OwnedOriginalShoreCarrier, second: &OwnedOriginalShoreCarrier) -> Result<bool> {
    first.reverify_source()?;
    second.reverify_source()?;
    if !Arc::ptr_eq(first.chart(), second.chart())
        || first.carrier() == second.carrier()
        || first.axis() != second.axis()
        || first.shore() == second.shore()
    {
        return Err(refused(
            "Original shore pair does not share one actual opposite-shore context",
        ));
    }
    let a = first.endpoint_keys();
    let b = second.endpoint_keys();
    let reversed = if a == b {
        false
    } else if a == [b[1], b[0]] {
        true
    } else {
        return Err(refused(
            "Original opposite shore carriers have different complete transverse images",
        ));
    };
    if first.direction()
        != if reversed {
            -second.direction()
        } else {
            second.direction()
        }
    {
        return Err(refused(
            "Original shore pair direction conflicts with exact endpoint ownership",
        ));
    }
    Ok(reversed)
}
impl OwnedOriginalShorePair {
    pub fn first(&self) -> &OwnedOriginalShoreCarrier {
        &self.first
    }
    pub fn second(&self) -> &OwnedOriginalShoreCarrier {
        &self.second
    }
    pub fn reversed(&self) -> bool {
        self.reversed
    }
    pub fn reverify_source(&self) -> Result<()> {
        if pair(&self.first, &self.second)? != self.reversed {
            return Err(refused(
                "Original opposite-shore image correspondence changed",
            ));
        }
        Ok(())
    }
}
/// Equal exact transverse endpoints PLUS complete original monotone-image
/// proofs establish full interval correspondence under different parameter
/// clocks. Other-axis shore provenance distinguishes torus corner aliases.
/// This proves the UV shore image pair, not 3D periodic surface agreement.
pub fn own_original_shore_pair(
    first: OwnedOriginalShoreCarrier,
    second: OwnedOriginalShoreCarrier,
) -> Result<OwnedOriginalShorePair> {
    let reversed = pair(&first, &second)?;
    Ok(OwnedOriginalShorePair {
        first,
        second,
        reversed,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shore_receipt_replay_rejects_cell_direction_and_source_binding_changes() {
        let source = Curve::from_polyline(vec![vec![0., 0.], vec![0., 1.]]).unwrap();
        let atlas = Arc::new(
            assemble_original_contact_atlas(&[source], 1e-12, Default::default()).unwrap(),
        );
        let graph = Arc::new(
            own_original_carrier_graph(Arc::new(own_original_carriers(atlas).unwrap())).unwrap(),
        );
        let chart = Arc::new(
            own_original_chart_vertices(
                graph,
                [[0., 1.], [0., 1.]],
                [true, false],
                Default::default(),
            )
            .unwrap(),
        );
        let s = own_original_shore_carrier(chart, 0, 0, 0, Default::default()).unwrap();
        s.reverify_source().unwrap();
        let mut bad = s.clone();
        bad.cells.clear();
        assert!(bad.reverify_source().is_err());
        let mut bad = s.clone();
        bad.direction = -1;
        assert!(bad.reverify_source().is_err());
        let mut bad = s.clone();
        bad.shore = 1;
        assert!(bad.reverify_source().is_err());
        let mut bad = s;
        bad.limits.max_refinements = 513;
        assert!(bad.reverify_source().is_err());
    }
}
#[cfg(test)]
mod pair_tests {
    use super::*;
    #[test]
    fn original_shore_pair_replays_direction_and_actual_pair_references() {
        let curves = [
            Curve::from_polyline(vec![vec![0., 0.], vec![0., 1.]]).unwrap(),
            Curve::from_polyline(vec![vec![1., 0.], vec![1., 1.]]).unwrap(),
        ];
        let atlas =
            Arc::new(assemble_original_contact_atlas(&curves, 1e-12, Default::default()).unwrap());
        let graph = Arc::new(
            own_original_carrier_graph(Arc::new(own_original_carriers(atlas).unwrap())).unwrap(),
        );
        let chart = Arc::new(
            own_original_chart_vertices(
                graph,
                [[0., 1.], [0., 1.]],
                [true, false],
                Default::default(),
            )
            .unwrap(),
        );
        let a = own_original_shore_carrier(chart.clone(), 0, 0, 0, Default::default()).unwrap();
        let b = own_original_shore_carrier(chart, 1, 0, 1, Default::default()).unwrap();
        let p = own_original_shore_pair(a, b).unwrap();
        p.reverify_source().unwrap();
        let mut bad = p.clone();
        bad.reversed = !bad.reversed;
        assert!(bad.reverify_source().is_err());
        let mut bad = p;
        bad.second.carrier = 0;
        assert!(bad.reverify_source().is_err());
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OriginalShoreMembership {
    Entire,
    Absent,
    Mixed,
}
#[derive(Clone, Debug)]
pub struct OwnedOriginalShoreMembership {
    chart: Arc<OwnedOriginalChartVertices>,
    carrier: usize,
    axis: usize,
    shore: usize,
    refinements: usize,
    membership: OriginalShoreMembership,
    active: Vec<(usize, bool)>,
}
fn membership(
    chart: &OwnedOriginalChartVertices,
    carrier: usize,
    axis: usize,
    shore: usize,
    refinements: usize,
) -> Result<(OriginalShoreMembership, Vec<(usize, bool)>)> {
    chart.reverify_source()?;
    let owner = chart.graph().owner();
    if axis > 1 || shore > 1 || !chart.wrapped()[axis] || carrier >= owner.carriers().len() {
        return Err(refused("Invalid original shore membership reference"));
    }
    let c = &owner.carriers()[carrier];
    let bounds = [
        automatic_boundaries::bound(owner, c.source, &c.cuts[0])?,
        automatic_boundaries::bound(owner, c.source, &c.cuts[1])?,
    ];
    let cells = cells(&owner.atlas().sources()[c.source])?
        .into_iter()
        .map(|cell| (cell.net, cell.domain))
        .collect::<Vec<_>>();
    let (class, active) = arithmetic::shore_carrier::classify(
        &cells,
        &bounds,
        axis,
        &Q::from_float(chart.chart()[axis][shore]).unwrap(),
        refinements,
    )?;
    use arithmetic::shore_carrier::Membership;
    Ok((
        match class {
            Membership::Entire => OriginalShoreMembership::Entire,
            Membership::Absent => OriginalShoreMembership::Absent,
            Membership::Mixed => OriginalShoreMembership::Mixed,
        },
        active,
    ))
}
impl OwnedOriginalShoreMembership {
    pub fn membership(&self) -> OriginalShoreMembership {
        self.membership
    }
    pub fn active_cells(&self) -> &[(usize, bool)] {
        &self.active
    }
    pub fn reverify_source(&self) -> Result<()> {
        let (membership, active) = membership(
            &self.chart,
            self.carrier,
            self.axis,
            self.shore,
            self.refinements,
        )?;
        if membership != self.membership || active != self.active {
            return Err(refused(
                "Original shore membership or coefficient identity census changed",
            ));
        }
        Ok(())
    }
}
/// Absent means every proved active cell has a nonzero shore-identity
/// polynomial, so it contains no open shore interval. Isolated shore contacts
/// may still exist. Mixed needs owned subdivision; proof errors never mean Absent.
pub fn classify_original_shore_carrier(
    chart: Arc<OwnedOriginalChartVertices>,
    carrier: usize,
    axis: usize,
    shore: usize,
    refinements: usize,
) -> Result<OwnedOriginalShoreMembership> {
    let (membership, active) = membership(&chart, carrier, axis, shore, refinements)?;
    Ok(OwnedOriginalShoreMembership {
        chart,
        carrier,
        axis,
        shore,
        refinements,
        membership,
        active,
    })
}
#[cfg(test)]
mod membership_tests {
    use super::*;
    #[test]
    fn shore_membership_replay_rejects_negative_claim_and_cell_census_changes() {
        let curve = Curve::from_polyline(vec![vec![0., 0.], vec![0., 1.]]).unwrap();
        let atlas =
            Arc::new(assemble_original_contact_atlas(&[curve], 1e-12, Default::default()).unwrap());
        let graph = Arc::new(
            own_original_carrier_graph(Arc::new(own_original_carriers(atlas).unwrap())).unwrap(),
        );
        let chart = Arc::new(
            own_original_chart_vertices(
                graph,
                [[0., 1.], [0., 1.]],
                [true, false],
                Default::default(),
            )
            .unwrap(),
        );
        let m = classify_original_shore_carrier(chart, 0, 0, 0, 192).unwrap();
        m.reverify_source().unwrap();
        let mut bad = m.clone();
        bad.membership = OriginalShoreMembership::Absent;
        assert!(bad.reverify_source().is_err());
        let mut bad = m.clone();
        bad.active[0].1 = false;
        assert!(bad.reverify_source().is_err());
        let mut bad = m;
        bad.refinements = 513;
        assert!(bad.reverify_source().is_err());
    }
}
