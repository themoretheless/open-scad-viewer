//! Exact original source views on every crossed periodic chart tile.
use super::super::super::super::algebraic_intersections::{decode_polynomial, decode_ratio};
use super::*;
use num_bigint::BigInt;
use std::cmp::Ordering;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicEvent {
    pub source: usize,
    pub cell: usize,
    pub axis: usize,
    pub level: String,
    pub parameter: OriginalSourceParameter,
    pub position: [[ExactPolynomial; 2]; 2],
    pub order: usize,
    pub sides: [Option<i8>; 2],
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicShoreCell {
    pub source: usize,
    pub cell: usize,
    pub axis: usize,
    pub level: String,
    pub domain: [ExactRatio; 2],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OriginalPeriodicCutOrigin {
    Knot { boundary: usize },
    Event { event: usize },
    Coincident { cell: usize, endpoint: usize },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicCut {
    pub value: OriginalSourceParameter,
    pub origins: Vec<OriginalPeriodicCutOrigin>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicPiece {
    pub source: usize,
    pub cell: usize,
    pub cuts: [usize; 2],
    pub tiles: [Option<String>; 2],
    pub translation: [ExactRatio; 2],
    pub on_shore: [bool; 2],
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicTransition {
    pub source: usize,
    pub cut: usize,
    pub pieces: [usize; 2],
    pub period_delta: [String; 2],
    pub translation_delta: [ExactRatio; 2],
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicPoint {
    pub parameter: OriginalSourceParameter,
    pub coordinates: [[ExactPolynomial; 2]; 2],
}
#[derive(Clone, Debug)]
pub struct OwnedOriginalPeriodicPieces {
    sources: Vec<Curve>,
    chart: [[f64; 2]; 2],
    wrapped: [bool; 2],
    width: f64,
    limits: ParameterOrderLimits,
    max_levels: usize,
    events: Vec<OriginalPeriodicEvent>,
    coincident: Vec<OriginalPeriodicShoreCell>,
    cuts: Vec<Vec<OriginalPeriodicCut>>,
    pieces: Vec<OriginalPeriodicPiece>,
    transitions: Vec<OriginalPeriodicTransition>,
}
type Data = (
    Vec<OriginalPeriodicEvent>,
    Vec<OriginalPeriodicShoreCell>,
    Vec<Vec<OriginalPeriodicCut>>,
    Vec<OriginalPeriodicPiece>,
    Vec<OriginalPeriodicTransition>,
);
fn endpoint(
    originals: &[Vec<Cell>],
    cuts: &[Vec<OriginalPeriodicCut>],
    piece: &OriginalPeriodicPiece,
    side: usize,
    refinements: usize,
) -> Result<OriginalPeriodicPoint> {
    if side > 1 {
        return Err(refused("Invalid original periodic piece endpoint"));
    }
    let value = &cuts[piece.source][piece.cuts[side]].value;
    let f = decode_polynomial(&value.primitive)?;
    let parameter = decode_polynomial(&value.parameter)?;
    let root = [
        decode_ratio(&value.interval[0])?,
        decode_ratio(&value.interval[1])?,
    ];
    let cell = &originals[piece.source][piece.cell];
    let offset = [
        decode_ratio(&piece.translation[0])?,
        decode_ratio(&piece.translation[1])?,
    ];
    let coordinates = arithmetic::shore_events::translated_point(
        &cell.net,
        &cell.domain,
        &parameter,
        &f,
        &root,
        &offset,
        refinements,
    )?;
    Ok(OriginalPeriodicPoint {
        parameter: value.clone(),
        coordinates: coordinates.map(|c| c.map(|p| polynomial(&p))),
    })
}
// Size and work allowances of one derivation. `work` is a single counter
// shared by the lattice census, the cut ordering and the cell coverage, so
// each work limit bounds the running total, not its own phase.
const MAX_SOURCES: usize = 32;
const MAX_LEVELS: usize = 4096;
const MAX_REFINEMENTS: usize = 512;
const MAX_RESULTANT_SIZE: usize = 128;
const LATTICE_CENSUS_WORK: usize = 16384;
const MAX_EVENTS: usize = 16384;
const MAX_COINCIDENT_CELLS: usize = 8192;
const MAX_RAW_CUTS: usize = 32768;
const CUT_ORDERING_WORK: usize = 131072;
const CELL_COVERAGE_WORK: usize = 262144;
type RawCut = (OriginalSourceParameter, OriginalPeriodicCutOrigin);
/// Exact chart corner and period on each axis.
struct Lattice {
    chart: [[Q; 2]; 2],
    periods: [Q; 2],
}
fn validate(o: &OwnedOriginalPeriodicPieces) -> Result<()> {
    if o.sources.len() > MAX_SOURCES
        || !o.width.is_finite()
        || !(1e-18..=1e-3).contains(&o.width)
        || o.max_levels == 0
        || o.max_levels > MAX_LEVELS
        || o.limits.max_refinements > MAX_REFINEMENTS
        || !(1..=MAX_RESULTANT_SIZE).contains(&o.limits.max_resultant_size)
        || o.chart
            .iter()
            .any(|c| c.iter().any(|v| !v.is_finite()) || c[0] >= c[1])
    {
        return Err(refused(
            "Invalid original periodic source domain or proof budget",
        ));
    }
    Ok(())
}
/// One source cell on one wrapped axis.
#[derive(Clone, Copy)]
struct CellAxis<'a> {
    source: usize,
    cell: usize,
    axis: usize,
    c: &'a Cell,
}
/// Lattice events, coincident shore cells and the unordered cut proposals of
/// every source.
struct Census {
    events: Vec<OriginalPeriodicEvent>,
    coincident: Vec<OriginalPeriodicShoreCell>,
    raw: Vec<Vec<RawCut>>,
}
impl Census {
    fn collect(
        o: &OwnedOriginalPeriodicPieces,
        lattice: &Lattice,
        originals: &[Vec<Cell>],
        work: &mut usize,
    ) -> Result<Self> {
        let mut out = Self {
            events: Vec::new(),
            coincident: Vec::new(),
            raw: vec![Vec::new(); o.sources.len()],
        };
        for (source, cs) in originals.iter().enumerate() {
            let knots = cs
                .iter()
                .map(|c| &c.domain[0])
                .chain(cs.last().map(|c| &c.domain[1]));
            for (boundary, q) in knots.enumerate() {
                out.raw[source].push((
                    chart_intervals::rational(q),
                    OriginalPeriodicCutOrigin::Knot { boundary },
                ));
            }
            for (cell, c) in cs.iter().enumerate() {
                for axis in (0..2).filter(|&axis| o.wrapped[axis]) {
                    let at = CellAxis {
                        source,
                        cell,
                        axis,
                        c,
                    };
                    out.cross_shores(o, lattice, at, work)?;
                }
            }
        }
        Ok(out)
    }
    /// Records every lattice shore this cell can reach on this axis.
    fn cross_shores(
        &mut self,
        o: &OwnedOriginalPeriodicPieces,
        lattice: &Lattice,
        at: CellAxis<'_>,
        work: &mut usize,
    ) -> Result<()> {
        let CellAxis { axis, c, .. } = at;
        let (origin, period) = (&lattice.chart[axis][0], &lattice.periods[axis]);
        // Positive homogeneous weights prove the full actual cell lies in
        // this EXACT control hull. It proposes all possible lattice levels.
        let hull = c.net.iter().map(|p| &p[axis] / &p[2]).collect::<Vec<_>>();
        let low = ((hull.iter().min().unwrap() - origin) / period)
            .ceil()
            .to_integer();
        let high = ((hull.iter().max().unwrap() - origin) / period)
            .floor()
            .to_integer();
        if high < low {
            return Ok(());
        }
        if &high - &low + BigInt::from(1) > BigInt::from(o.max_levels) {
            return Err(refused(
                "Original periodic cell crosses too many candidate lattice shores",
            ));
        }
        let width = Q::from_float(o.width).unwrap() / (&c.domain[1] - &c.domain[0]).max(Q::one());
        let mut level = low;
        while level <= high {
            *work += 1;
            if *work > LATTICE_CENSUS_WORK {
                return Err(refused(
                    "Original periodic lattice census allowance exceeded",
                ));
            }
            let shore = origin + Q::from_integer(level.clone()) * period;
            let produced = arithmetic::shore_events::produce(
                &c.net,
                &c.domain,
                axis,
                &shore,
                &width,
                o.limits.max_refinements,
            )?;
            match produced {
                None => self.record_coincident(at, &level),
                Some(roots) => self.record_events(at, &level, roots),
            }
            if self.events.len() > MAX_EVENTS || self.coincident.len() > MAX_COINCIDENT_CELLS {
                return Err(refused("Original periodic event census allowance exceeded"));
            }
            level += 1;
        }
        Ok(())
    }
    /// The whole cell lies on the shore: both its ends become cuts.
    fn record_coincident(&mut self, at: CellAxis<'_>, level: &BigInt) {
        let CellAxis {
            source,
            cell,
            axis,
            c,
        } = at;
        let index = self.coincident.len();
        self.coincident.push(OriginalPeriodicShoreCell {
            source,
            cell,
            axis,
            level: level.to_string(),
            domain: c.domain.clone().map(|q| ratio(&q)),
        });
        for endpoint in 0..2 {
            self.raw[source].push((
                chart_intervals::rational(&c.domain[endpoint]),
                OriginalPeriodicCutOrigin::Coincident {
                    cell: index,
                    endpoint,
                },
            ));
        }
    }
    fn record_events(
        &mut self,
        at: CellAxis<'_>,
        level: &BigInt,
        roots: Vec<arithmetic::shore_events::ShoreRoot>,
    ) {
        let CellAxis {
            source, cell, axis, ..
        } = at;
        for r in roots {
            let parameter = OriginalSourceParameter {
                primitive: polynomial(&r.primitive),
                interval: r.root.map(|q| ratio(&q)),
                parameter: polynomial(&r.parameter),
            };
            self.raw[source].push((
                parameter.clone(),
                OriginalPeriodicCutOrigin::Event {
                    event: self.events.len(),
                },
            ));
            self.events.push(OriginalPeriodicEvent {
                source,
                cell,
                axis,
                level: level.to_string(),
                parameter,
                position: r.position.map(|c| c.map(|p| polynomial(&p))),
                order: r.order,
                sides: r.sides,
            });
        }
    }
}
/// Sorts one source's cut proposals by exact parameter order and merges the
/// origins of equal parameters into one cut.
fn order_cuts(
    input: Vec<RawCut>,
    limits: ParameterOrderLimits,
    work: &mut usize,
) -> Result<Vec<OriginalPeriodicCut>> {
    let mut ordered: Vec<OriginalPeriodicCut> = Vec::new();
    for (value, origin) in input {
        let mut pos = 0;
        let mut joined = false;
        while pos < ordered.len() {
            *work += 1;
            if *work > CUT_ORDERING_WORK {
                return Err(refused("Original periodic cut ordering allowance exceeded"));
            }
            match source_carriers::compare(&ordered[pos].value, &value, limits)? {
                Ordering::Equal => {
                    ordered[pos].origins.push(origin);
                    joined = true;
                    break;
                }
                Ordering::Greater => break,
                Ordering::Less => pos += 1,
            }
        }
        if !joined {
            ordered.insert(
                pos,
                OriginalPeriodicCut {
                    value,
                    origins: vec![origin],
                },
            );
        }
    }
    Ok(ordered)
}
/// The first and last cuts must be exactly the ends of the source domain.
fn require_complete_domain(
    source_cuts: &[OriginalPeriodicCut],
    cs: &[Cell],
    limits: ParameterOrderLimits,
) -> Result<()> {
    if source_cuts.len() < 2
        || source_carriers::compare(
            &source_cuts[0].value,
            &chart_intervals::rational(&cs[0].domain[0]),
            limits,
        )? != Ordering::Equal
        || source_carriers::compare(
            &source_cuts.last().unwrap().value,
            &chart_intervals::rational(&cs.last().unwrap().domain[1]),
            limits,
        )? != Ordering::Equal
    {
        return Err(refused(
            "Original periodic pieces do not cover the complete source domain",
        ));
    }
    Ok(())
}
/// The single source cell that contains the interval between two adjacent cuts.
fn owning_cell(
    cs: &[Cell],
    [lower, upper]: [&OriginalSourceParameter; 2],
    limits: ParameterOrderLimits,
    work: &mut usize,
) -> Result<usize> {
    let mut found = Vec::new();
    for (cell, c) in cs.iter().enumerate() {
        *work += 1;
        if *work > CELL_COVERAGE_WORK {
            return Err(refused(
                "Original periodic source cell coverage allowance exceeded",
            ));
        }
        if source_carriers::compare(lower, &chart_intervals::rational(&c.domain[0]), limits)?
            != Ordering::Less
            && source_carriers::compare(upper, &chart_intervals::rational(&c.domain[1]), limits)?
                != Ordering::Greater
        {
            found.push(cell);
        }
    }
    match found[..] {
        [cell] => Ok(cell),
        _ => Err(refused(
            "Original periodic interval does not own one source cell",
        )),
    }
}
/// Tile index, exact translation back to the base tile and shore incidence of
/// one piece on each wrapped axis.
struct Placement {
    tiles: [Option<String>; 2],
    translation: [Q; 2],
    on_shore: [bool; 2],
}
fn place(
    o: &OwnedOriginalPeriodicPieces,
    lattice: &Lattice,
    coincident: &[OriginalPeriodicShoreCell],
    [source, cell]: [usize; 2],
    point: &[Q; 2],
) -> Result<Placement> {
    let mut out = Placement {
        tiles: [None, None],
        translation: [Q::zero(), Q::zero()],
        on_shore: [false, false],
    };
    for axis in (0..2).filter(|&axis| o.wrapped[axis]) {
        let relative = (&point[axis] - &lattice.chart[axis][0]) / &lattice.periods[axis];
        let tile = relative.floor().to_integer();
        out.on_shore[axis] = relative == Q::from_integer(tile.clone());
        if out.on_shore[axis]
            && !coincident.iter().any(|c| {
                c.source == source
                    && c.cell == cell
                    && c.axis == axis
                    && c.level == tile.to_string()
            })
        {
            return Err(refused(
                "Original periodic piece contains an unowned lattice root",
            ));
        }
        out.translation[axis] = -Q::from_integer(tile.clone()) * &lattice.periods[axis];
        out.tiles[axis] = Some(tile.to_string());
    }
    Ok(out)
}
fn tile_index(piece: &OriginalPeriodicPiece, axis: usize) -> Result<BigInt> {
    piece.tiles[axis]
        .as_ref()
        .unwrap()
        .parse::<BigInt>()
        .map_err(|_| refused("Invalid periodic tile integer"))
}
/// Period and translation deltas across the cut shared by two consecutive
/// pieces, after proving the source endpoint is exactly continuous there.
fn seam(
    o: &OwnedOriginalPeriodicPieces,
    originals: &[Vec<Cell>],
    cuts: &[Vec<OriginalPeriodicCut>],
    [first, piece]: [&OriginalPeriodicPiece; 2],
) -> Result<([BigInt; 2], [Q; 2])> {
    let mut delta = [BigInt::from(0), BigInt::from(0)];
    let shift = [
        decode_ratio(&piece.translation[0])? - decode_ratio(&first.translation[0])?,
        decode_ratio(&piece.translation[1])? - decode_ratio(&first.translation[1])?,
    ];
    for axis in (0..2).filter(|&axis| o.wrapped[axis]) {
        delta[axis] = tile_index(piece, axis)? - tile_index(first, axis)?;
        if delta[axis] < BigInt::from(-1) || delta[axis] > BigInt::from(1) {
            return Err(refused(
                "Original periodic transition skips an unowned lattice shore",
            ));
        }
    }
    // Both one-sided endpoint evaluations use the SAME owned cut
    // primitive; verify exact C0 after accounting for translations.
    let a = endpoint(originals, cuts, first, 1, o.limits.max_refinements)?;
    let b = endpoint(originals, cuts, piece, 0, o.limits.max_refinements)?;
    let f = decode_polynomial(&a.parameter.primitive)?;
    let root = [
        decode_ratio(&a.parameter.interval[0])?,
        decode_ratio(&a.parameter.interval[1])?,
    ];
    for axis in 0..2 {
        let an = decode_polynomial(&a.coordinates[axis][0])?;
        let ad = decode_polynomial(&a.coordinates[axis][1])?;
        let bn = decode_polynomial(&b.coordinates[axis][0])?;
        let bd = decode_polynomial(&b.coordinates[axis][1])?;
        if !arithmetic::event_coordinates::translated_equal(
            &an,
            &ad,
            &bn,
            &bd,
            &shift[axis],
            &f,
            &root,
        )? {
            return Err(refused(
                "Original periodic transition does not preserve the actual source endpoint",
            ));
        }
    }
    Ok((delta, shift))
}
fn derive(o: &OwnedOriginalPeriodicPieces) -> Result<Data> {
    validate(o)?;
    let chart = o.chart.map(|a| a.map(|q| Q::from_float(q).unwrap()));
    let periods = chart.each_ref().map(|a| &a[1] - &a[0]);
    let lattice = Lattice { chart, periods };
    let originals = o.sources.iter().map(cells).collect::<Result<Vec<_>>>()?;
    let mut work = 0usize;
    let Census {
        events,
        coincident,
        raw,
    } = Census::collect(o, &lattice, &originals, &mut work)?;
    if raw.iter().map(Vec::len).sum::<usize>() > MAX_RAW_CUTS {
        return Err(refused("Original periodic cut allowance exceeded"));
    }
    let cuts = raw
        .into_iter()
        .map(|input| order_cuts(input, o.limits, &mut work))
        .collect::<Result<Vec<_>>>()?;
    let mut pieces: Vec<OriginalPeriodicPiece> = Vec::new();
    let mut transitions = Vec::new();
    for (source, source_cuts) in cuts.iter().enumerate() {
        let cs = &originals[source];
        require_complete_domain(source_cuts, cs, o.limits)?;
        let mut previous: Option<usize> = None;
        for cut in 0..source_cuts.len() - 1 {
            let interval = [&source_cuts[cut].value, &source_cuts[cut + 1].value];
            let cell = owning_cell(cs, interval, o.limits, &mut work)?;
            let c = &cs[cell];
            let witness =
                chart_intervals::interior(interval[0], interval[1], o.limits.max_refinements)?;
            let local = (&witness - &c.domain[0]) / (&c.domain[1] - &c.domain[0]);
            let point = arithmetic::shore_events::point(&c.net, &local)?;
            let Placement {
                tiles,
                translation,
                on_shore,
            } = place(o, &lattice, &coincident, [source, cell], &point)?;
            let piece = OriginalPeriodicPiece {
                source,
                cell,
                cuts: [cut, cut + 1],
                tiles,
                translation: translation.map(|q| ratio(&q)),
                on_shore,
            };
            let index = pieces.len();
            if let Some(before) = previous {
                let (delta, shift) = seam(o, &originals, &cuts, [&pieces[before], &piece])?;
                transitions.push(OriginalPeriodicTransition {
                    source,
                    cut,
                    pieces: [before, index],
                    period_delta: delta.map(|n| n.to_string()),
                    translation_delta: shift.map(|q| ratio(&q)),
                });
            }
            pieces.push(piece);
            previous = Some(index);
        }
    }
    Ok((events, coincident, cuts, pieces, transitions))
}
impl OwnedOriginalPeriodicPieces {
    pub fn sources(&self) -> &[Curve] {
        &self.sources
    }
    pub fn chart(&self) -> [[f64; 2]; 2] {
        self.chart
    }
    pub fn wrapped(&self) -> [bool; 2] {
        self.wrapped
    }
    pub fn events(&self) -> &[OriginalPeriodicEvent] {
        &self.events
    }
    pub fn coincident_cells(&self) -> &[OriginalPeriodicShoreCell] {
        &self.coincident
    }
    pub fn cuts(&self) -> &[Vec<OriginalPeriodicCut>] {
        &self.cuts
    }
    /// Geometry is the ORIGINAL source curve restricted to exact natural cuts,
    /// plus this exact rational translation. No shifted f64 control net exists.
    pub fn pieces(&self) -> &[OriginalPeriodicPiece] {
        &self.pieces
    }
    pub fn transitions(&self) -> &[OriginalPeriodicTransition] {
        &self.transitions
    }
    pub fn piece_endpoint(&self, piece: usize, side: usize) -> Result<OriginalPeriodicPoint> {
        self.reverify_source()?;
        self.endpoint_verified(piece, side)
    }
    // Caller must replay this owner (or its supporting-surface owner) first.
    pub(super) fn endpoint_verified(
        &self,
        piece: usize,
        side: usize,
    ) -> Result<OriginalPeriodicPoint> {
        let p = self
            .pieces
            .get(piece)
            .ok_or_else(|| refused("Unknown original periodic piece"))?;
        let originals = self
            .sources
            .iter()
            .map(cells)
            .collect::<Result<Vec<_>>>()?;
        endpoint(
            &originals,
            &self.cuts,
            p,
            side,
            self.limits.max_refinements,
        )
    }
    pub fn reverify_source(&self) -> Result<()> {
        let (e, c, k, p, t) = derive(self)?;
        if e != self.events
            || c != self.coincident
            || k != self.cuts
            || p != self.pieces
            || t != self.transitions
        {
            return Err(refused(
                "Original periodic source views or seam transitions changed",
            ));
        }
        Ok(())
    }
}
pub fn own_original_periodic_pieces(
    sources: &[Curve],
    chart: [[f64; 2]; 2],
    wrapped: [bool; 2],
    width: f64,
    limits: ParameterOrderLimits,
    max_levels: usize,
) -> Result<OwnedOriginalPeriodicPieces> {
    let mut out = OwnedOriginalPeriodicPieces {
        sources: sources.to_vec(),
        chart,
        wrapped,
        width,
        limits,
        max_levels,
        events: Vec::new(),
        coincident: Vec::new(),
        cuts: Vec::new(),
        pieces: Vec::new(),
        transitions: Vec::new(),
    };
    (
        out.events,
        out.coincident,
        out.cuts,
        out.pieces,
        out.transitions,
    ) = derive(&out)?;
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replay_rejects_missing_lattice_events_translation_and_transition_changes() {
        let a = Curve::from_polyline(vec![vec![2.25, 0.], vec![4.25, 1.]]).unwrap();
        let x = own_original_periodic_pieces(
            &[a],
            [[0., 1.], [-2., 2.]],
            [true, false],
            1e-12,
            ParameterOrderLimits::default(),
            32,
        )
        .unwrap();
        x.reverify_source().unwrap();
        let mut bad = x.clone();
        bad.events.pop();
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.pieces[0].translation[0] = ["0".into(), "1".into()];
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.transitions[0].period_delta[0] = "2".into();
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.cuts[0][1].origins.clear();
        assert!(bad.reverify_source().is_err());
    }
}
