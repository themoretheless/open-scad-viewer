//! Trimmed original graph domains, owning exact curved source-chart loops.
//! This does not append them to the transition shell or admit a solid.
use super::*;
use nurbs_core::original_graph_transition::SourceLine;
#[derive(Clone)]
pub enum RetainedUse {
    Contact(SourceBinding),
    End(SourceEndBinding),
    Natural(SourceLine),
}
impl RetainedUse {
    pub fn uv_points(&self) -> &[[Q; 2]] {
        match self {
            Self::Contact(b) => b.contact().uv_points(),
            Self::End(b) => b.contact().uv_points(),
            Self::Natural(l) => l.uv_points(),
        }
    }
    pub fn points(&self) -> &[[Q; 3]] {
        match self {
            Self::Contact(b) => b.contact().points(),
            Self::End(b) => b.contact().points(),
            Self::Natural(l) => l.points(),
        }
    }
    pub fn edge(&self) -> Option<usize> {
        match self {
            Self::Contact(b) => Some(b.edge()),
            Self::End(b) => Some(b.edge()),
            Self::Natural(_) => None,
        }
    }
}
#[derive(Clone)]
pub struct RetainedRegion {
    source: nurbs_core::surface::Surface,
    side: usize,
    panel: Arc<Patch>,
    uses: Vec<(RetainedUse, bool)>,
    area: Q,
}
impl RetainedRegion {
    pub fn source(&self) -> &nurbs_core::surface::Surface {
        &self.source
    }
    pub fn side(&self) -> usize {
        self.side
    }
    pub fn uses(&self) -> &[(RetainedUse, bool)] {
        &self.uses
    }
    pub fn parameter_area(&self) -> &Q {
        &self.area
    }
    pub fn is_current(&self, strip: &Strip) -> bool {
        Arc::ptr_eq(&self.panel, &strip.topology.faces[0].surface)
            && self.uses.iter().all(|(u, _)| match u {
                RetainedUse::Contact(b) => b.is_current(strip),
                RetainedUse::End(b) => b.is_current(strip),
                RetainedUse::Natural(l) => l.source() == &self.source,
            })
    }
}
fn checked(q: Q) -> Result<Q> {
    require(
        q.denom().is_positive() && q.numer().bits() <= 8192 && q.denom().bits() <= 8192,
        "Source-region coefficient exceeds exact bit limits",
    )?;
    Ok(q)
}
fn start(u: &RetainedUse, reverse: bool) -> (&[Q; 2], &[Q; 3]) {
    let uv = u.uv_points();
    let xyz = u.points();
    (
        &uv[if reverse { uv.len() - 1 } else { 0 }],
        &xyz[if reverse { xyz.len() - 1 } else { 0 }],
    )
}
fn finish(u: &RetainedUse, reverse: bool) -> (&[Q; 2], &[Q; 3]) {
    start(u, !reverse)
}
/// Largest original contact chain admitted on one source side.
const MAX_CONTACT_CHAIN: usize = 4096;
fn exact_knot(x: f64) -> Result<Q> {
    let q = Q::from_float(x);
    require(q.is_some(), "Original U knot is not finite")?;
    Ok(q.unwrap_or_default())
}
/// Clamped U domain of an original source surface.
fn u_domain(source: &nurbs_core::surface::Surface) -> Result<[Q; 2]> {
    let low = exact_knot(source.knots_u[source.degree_u])?;
    let high = exact_knot(source.knots_u[source.control_points.len()])?;
    Ok([low, high])
}
/// First and last contact U clock of a non-empty, sorted chain.
fn chain_clock(chain: &[SourceBinding]) -> Result<[Q; 2]> {
    let first = chain.first().and_then(|c| c.contact().uv_points().first());
    let last = chain.last().and_then(|c| c.contact().uv_points().last());
    let (Some(first), Some(last)) = (first, last) else {
        return Err(Error::new(
            "BREP_EXACT_GRAPH_STRIP",
            "Missing or oversized original contact chain",
        ));
    };
    Ok([first[0].clone(), last[0].clone()])
}
/// Every consecutive pair of uses (cyclically) meets exactly in UV and 3D.
fn check_closed_loop(uses: &[(RetainedUse, bool)]) -> Result<()> {
    for (i, (a, ar)) in uses.iter().enumerate() {
        let (b, br) = &uses[(i + 1) % uses.len()];
        require(
            finish(a, *ar) == start(b, *br),
            "Exact original trim loop has a UV or 3D gap",
        )?;
    }
    Ok(())
}
impl Strip {
    /// Parameter area removed below the chain, after currency/strictness checks.
    fn removed_area(
        &self,
        chain: &[SourceBinding],
        source: &nurbs_core::surface::Surface,
    ) -> Result<Q> {
        let mut removed = Q::zero();
        for contact in chain {
            require(
                contact.is_current(self) && contact.contact().source() == source,
                "Source contact changed before trim-region construction",
            )?;
            let uv = contact.contact().uv_points();
            require(
                uv.iter().all(|p| p[1] > Q::zero() && p[1] < Q::one()),
                "Contact must stay strictly between original horizontal chart edges",
            )?;
            let (Some(first), Some(last)) = (uv.first(), uv.last()) else {
                return Err(Error::new(
                    "BREP_EXACT_GRAPH_STRIP",
                    "Nonmonotone source contact clock",
                ));
            };
            let span = checked(&last[0] - &first[0])?;
            require(span > Q::zero(), "Nonmonotone source contact clock")?;
            let sum = uv.iter().try_fold(Q::zero(), |a, p| checked(a + &p[1]))?;
            removed = checked(removed + span * sum / Q::from_integer(uv.len().into()))?;
        }
        Ok(removed)
    }
    /// Sorted end uses of one source end, with their traversal direction.
    fn push_end_uses(
        &self,
        side: usize,
        end: usize,
        mut boundary: Vec<SourceEndBinding>,
        source: &nurbs_core::surface::Surface,
        uses: &mut Vec<(RetainedUse, bool)>,
    ) -> Result<()> {
        require(!boundary.is_empty(), "Missing source end chart uses")?;
        boundary.sort_by(|a, b| a.contact().root_parameters()[0].cmp(&b.contact().root_parameters()[0]));
        let reverse = (side == 0 && end == 0) || (side == 1 && end == 1);
        if reverse {
            boundary.reverse();
        }
        for b in boundary {
            require(
                b.is_current(self) && b.contact().source() == source,
                "Source end changed before trim-region construction",
            )?;
            uses.push((RetainedUse::End(b), reverse));
        }
        Ok(())
    }
    /// Retained region of one source side from its contact chain and end uses.
    fn side_region(
        &self,
        side: usize,
        mut chain: Vec<SourceBinding>,
        ends: [Vec<SourceEndBinding>; 2],
        budget: &mut Budget,
    ) -> Result<RetainedRegion> {
        let panel = &self.topology.faces[0].surface;
        require(
            !chain.is_empty() && chain.len() <= MAX_CONTACT_CHAIN,
            "Missing or oversized original contact chain",
        )?;
        chain.sort_by(|a, b| a.contact().uv_points()[0][0].cmp(&b.contact().uv_points()[0][0]));
        let source = panel.sources()[side];
        let [low, high] = u_domain(source)?;
        let [x0, x1] = chain_clock(&chain)?;
        require(
            low < x0 && x0 < x1 && x1 < high,
            "Retained graph notch must be strictly inside original U domain",
        )?;
        let removed = self.removed_area(&chain, source)?;
        let area = checked(&high - &low - removed)?;
        require(
            area > Q::zero(),
            "Retained source region has no positive area",
        )?;
        let mut uses = Vec::new();
        let line = panel.source_line(
            side,
            [[low.clone(), Q::zero()], [x0.clone(), Q::zero()]],
            budget,
        )?;
        uses.push((RetainedUse::Natural(line), false));
        let [start_ends, finish_ends] = ends;
        self.push_end_uses(side, 0, start_ends, source, &mut uses)?;
        for c in chain.drain(..) {
            uses.push((RetainedUse::Contact(c), false));
        }
        self.push_end_uses(side, 1, finish_ends, source, &mut uses)?;
        for uv in [
            [[x1, Q::zero()], [high.clone(), Q::zero()]],
            [[high.clone(), Q::zero()], [high.clone(), Q::one()]],
            [[high, Q::one()], [low.clone(), Q::one()]],
            [[low.clone(), Q::one()], [low, Q::zero()]],
        ] {
            uses.push((
                RetainedUse::Natural(panel.source_line(side, uv, budget)?),
                false,
            ));
        }
        check_closed_loop(&uses)?;
        Ok(RetainedRegion {
            source: source.clone(),
            side,
            panel: panel.clone(),
            uses,
            area,
        })
    }
    /// A simple original-chart graph notch is proved by monotone U, strict
    /// 0<V<1 hulls, exact adjacency and the outer source rectangle. Arbitrary
    /// trim arrangements/holes remain outside this constructor's scope.
    pub fn retained_regions(&self, budget: &mut Budget) -> Result<[RetainedRegion; 2]> {
        let mut contacts: [Vec<SourceBinding>; 2] = std::array::from_fn(|_| Vec::new());
        for b in self.source_contacts(budget)? {
            let side = b.source_side();
            contacts[side].push(b);
        }
        let mut ends: [[Vec<SourceEndBinding>; 2]; 2] =
            std::array::from_fn(|_| std::array::from_fn(|_| Vec::new()));
        for b in self.source_end_contacts(budget)? {
            let side = b.contact().side();
            let end = usize::from(b.contact().at_end());
            ends[side][end].push(b);
        }
        let [c0, c1] = contacts;
        let [e0, e1] = ends;
        let a = self.side_region(0, c0, e0, budget)?;
        let b = self.side_region(1, c1, e1, budget)?;
        Ok([a, b])
    }
}

impl RetainedRegion {
    pub(super) fn split_natural_use(&self, index: usize, parameter: &Q, b: &mut Budget) -> Result<Self> {
        let (u, reverse) = self.uses.get(index).ok_or_else(|| Error::new("BREP_EXACT_GRAPH_ROOF", "Unknown retained use"))?;
        let RetainedUse::Natural(line) = u else {
            return Err(Error::new("BREP_EXACT_GRAPH_ROOF", "Only natural source edges can be split here"));
        };
        let uv = line.uv_points();
        let middle = std::array::from_fn(|k| &uv[0][k] + (&uv[1][k] - &uv[0][k]) * parameter);
        let left = RetainedUse::Natural(self.panel.source_line(self.side, [uv[0].clone(), middle.clone()], b)?);
        let right = RetainedUse::Natural(self.panel.source_line(self.side, [middle, uv[1].clone()], b)?);
        let mut result = self.clone();
        let replacement = if *reverse { vec![(right,true),(left,true)] } else { vec![(left,false),(right,false)] };
        result.uses.splice(index..=index,replacement);
        Ok(result)
    }
}
