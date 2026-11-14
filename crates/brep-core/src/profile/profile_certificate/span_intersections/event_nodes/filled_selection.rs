//! Shared selection of filled material boundaries from weighted side windings.
use super::*;
/// Refusal messages of one caller, so each owner keeps its own wording.
pub(super) struct Messages {
    pub product_overflow: &'static str,
    pub total_overflow: &'static str,
    pub sectors: &'static str,
    pub missing_twin: &'static str,
    pub no_outgoing: &'static str,
    pub permutation: &'static str,
    pub missing_successor: &'static str,
    pub orbit: &'static str,
}
/// Winding of one side weighted by the wire coefficients.
pub(super) fn weighted(values: &[i64], coefficients: &[i32], m: &Messages) -> Result<i64> {
    values
        .iter()
        .zip(coefficients)
        .try_fold(0i64, |sum, (&w, &c)| {
            sum.checked_add(
                w.checked_mul(i64::from(c))
                    .ok_or_else(|| refused(m.product_overflow))?,
            )
            .ok_or_else(|| refused(m.total_overflow))
        })
}
/// A halfedge graph with weighted windings on both sides of every halfedge.
pub(super) struct Halfedges<'a, Side, Arrive> {
    pub count: usize,
    /// Rotation of halfedges around every vertex.
    pub rotations: &'a [Vec<usize>],
    /// Weighted winding on the left and on the right of a halfedge.
    pub side: Side,
    /// Arrival vertex of a halfedge and its twin halfedge.
    pub arrive: Arrive,
    pub messages: &'a Messages,
}
/// Retained halfedges, their boundary successors and the boundary orbits.
pub(super) type Selection = (Vec<usize>, Vec<Option<usize>>, Vec<Vec<usize>>);
impl<Side, Arrive> Halfedges<'_, Side, Arrive>
where
    Side: Fn(usize) -> [i64; 2],
    Arrive: Fn(usize) -> (usize, usize),
{
    /// Adjacent halfedges around a vertex agree on the sector between them.
    fn verify_sectors(&self) -> Result<()> {
        for r in self.rotations {
            for i in 0..r.len() {
                if (self.side)(r[i])[0] != (self.side)(r[(i + 1) % r.len()])[1] {
                    return Err(refused(self.messages.sectors));
                }
            }
        }
        Ok(())
    }
    /// The next retained halfedge clockwise after the twin at the arrival.
    fn successor(&self, h: usize, selected: &[bool]) -> Result<usize> {
        let (target, twin) = (self.arrive)(h);
        let r = &self.rotations[target];
        let at = r
            .iter()
            .position(|&v| v == twin)
            .ok_or_else(|| refused(self.messages.missing_twin))?;
        (1..=r.len())
            .map(|step| r[(at + r.len() - step) % r.len()])
            .find(|&v| selected[v])
            .ok_or_else(|| refused(self.messages.no_outgoing))
    }
    /// Orbits of the successor permutation over the retained halfedges.
    fn orbits(&self, retained: &[usize], successor: &[Option<usize>]) -> Result<Vec<Vec<usize>>> {
        let mut visited = vec![false; self.count];
        let mut boundaries = Vec::new();
        for &seed in retained {
            if visited[seed] {
                continue;
            }
            let mut h = seed;
            let mut boundary = Vec::new();
            while !visited[h] {
                visited[h] = true;
                boundary.push(h);
                h = successor[h].ok_or_else(|| refused(self.messages.missing_successor))?;
            }
            if h != seed {
                return Err(refused(self.messages.orbit));
            }
            boundaries.push(boundary);
        }
        Ok(boundaries)
    }
    /// Halfedges with material on the left only, linked into closed boundaries.
    pub(super) fn select(&self) -> Result<Selection> {
        self.verify_sectors()?;
        let retained = (0..self.count)
            .filter(|&h| {
                let [left, right] = (self.side)(h);
                left != 0 && right == 0
            })
            .collect::<Vec<_>>();
        let mut selected = vec![false; self.count];
        for &h in &retained {
            selected[h] = true;
        }
        let mut successor = vec![None; self.count];
        let mut indegree = vec![0usize; self.count];
        for &h in &retained {
            let next = self.successor(h, &selected)?;
            successor[h] = Some(next);
            indegree[next] += 1;
        }
        if (0..self.count).any(|h| indegree[h] != usize::from(selected[h])) {
            return Err(refused(self.messages.permutation));
        }
        let boundaries = self.orbits(&retained, &successor)?;
        Ok((retained, successor, boundaries))
    }
}
