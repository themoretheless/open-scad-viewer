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
impl Strip {
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
        let panel = &self.topology.faces[0].surface;
        let mut regions = Vec::new();
        for side in 0..2 {
            let mut chain = std::mem::take(&mut contacts[side]);
            require(
                !chain.is_empty() && chain.len() <= 4096,
                "Missing or oversized original contact chain",
            )?;
            chain.sort_by(|a, b| a.contact().uv_points()[0][0].cmp(&b.contact().uv_points()[0][0]));
            let source = panel.sources()[side];
            let n = source.degree_u;
            let low = Q::from_float(source.knots_u[n]).unwrap();
            let high = Q::from_float(source.knots_u[source.control_points.len()]).unwrap();
            let x0 = chain[0].contact().uv_points()[0][0].clone();
            let last = chain.last().unwrap().contact().uv_points().last().unwrap();
            let x1 = last[0].clone();
            require(
                low < x0 && x0 < x1 && x1 < high,
                "Retained graph notch must be strictly inside original U domain",
            )?;
            let mut removed = Q::zero();
            for contact in &chain {
                require(
                    contact.is_current(self) && contact.contact().source() == source,
                    "Source contact changed before trim-region construction",
                )?;
                let uv = contact.contact().uv_points();
                require(
                    uv.iter().all(|p| p[1] > Q::zero() && p[1] < Q::one()),
                    "Contact must stay strictly between original horizontal chart edges",
                )?;
                let span = checked(&uv.last().unwrap()[0] - &uv[0][0])?;
                require(span > Q::zero(), "Nonmonotone source contact clock")?;
                let sum = uv.iter().try_fold(Q::zero(), |a, p| checked(a + &p[1]))?;
                removed = checked(removed + span * sum / Q::from_integer(uv.len().into()))?;
            }
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
            for end in 0..2 {
                let mut boundary = std::mem::take(&mut ends[side][end]);
                require(!boundary.is_empty(), "Missing source end chart uses")?;
                boundary.sort_by(|a, b| {
                    a.contact().root_parameters()[0].cmp(&b.contact().root_parameters()[0])
                });
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
                if end == 0 {
                    for c in chain.drain(..) {
                        uses.push((RetainedUse::Contact(c), false));
                    }
                }
            }
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
            for i in 0..uses.len() {
                let (a, ar) = &uses[i];
                let (b, br) = &uses[(i + 1) % uses.len()];
                require(
                    finish(a, *ar) == start(b, *br),
                    "Exact original trim loop has a UV or 3D gap",
                )?;
            }
            regions.push(RetainedRegion {
                source: source.clone(),
                side,
                panel: panel.clone(),
                uses,
                area,
            });
        }
        let [a, b] = regions
            .try_into()
            .map_err(|_| Error::new("BREP_EXACT_GRAPH_STRIP", "Expected two source regions"))?;
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
