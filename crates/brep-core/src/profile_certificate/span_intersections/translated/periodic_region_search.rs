//! Bounded positive connection search. Exhaustion never proves separation.
use super::periodic_material_clearance::ClearanceView;
use super::*;
use num_traits::ToPrimitive;
use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;
#[derive(Clone, Copy, Debug)]
pub struct PeriodicRegionSearchLimits {
    pub max_level: usize,
    pub max_period_radius: usize,
    pub max_grid_nodes: usize,
    pub max_work: usize,
    pub max_region_work: usize,
}
impl Default for PeriodicRegionSearchLimits {
    fn default() -> Self {
        Self {
            max_level: 6,
            max_period_radius: 2,
            max_grid_nodes: 65536,
            max_work: 100000,
            max_region_work: 1000000,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PeriodicRegionSearchStop {
    WorkBudget,
    GridNodeBudget,
    ResolutionLimit,
    PathPointBudget,
    RegionProofBudget,
}
#[derive(Clone, Debug)]
pub struct PeriodicRegionSearchReport {
    owner: Arc<OwnedPeriodicSupportSignatures>,
    paths: Vec<Arc<OwnedPeriodicSupportPath>>,
    regions: Option<OwnedPeriodicSupportRegions>,
    stop: Option<PeriodicRegionSearchStop>,
    charged_work: usize,
    visited_nodes: usize,
    grids: usize,
}
impl PeriodicRegionSearchReport {
    pub fn owner(&self) -> &Arc<OwnedPeriodicSupportSignatures> {
        &self.owner
    }
    pub fn paths(&self) -> &[Arc<OwnedPeriodicSupportPath>] {
        &self.paths
    }
    pub fn regions(&self) -> Option<&OwnedPeriodicSupportRegions> {
        self.regions.as_ref()
    }
    pub fn stop(&self) -> Option<PeriodicRegionSearchStop> {
        self.stop
    }
    pub fn charged_work(&self) -> usize {
        self.charged_work
    }
    pub fn visited_nodes(&self) -> usize {
        self.visited_nodes
    }
    pub fn grids(&self) -> usize {
        self.grids
    }
}
struct Budget {
    charged: usize,
    visited: usize,
    grids: usize,
    cap: usize,
}
impl Budget {
    fn charge(&mut self, n: usize) -> bool {
        if n > self.cap - self.charged {
            false
        } else {
            self.charged += n;
            true
        }
    }
    fn edge(&mut self, v: &ClearanceView<'_>, a: &[Q; 2], b: &[Q; 2]) -> Result<Option<bool>> {
        if !self.charge(v.cost(a, b)?) {
            return Ok(None);
        }
        Ok(Some(v.clear(a, b)?))
    }
}
enum Attempt {
    Found(Arc<OwnedPeriodicSupportPath>),
    Stop(PeriodicRegionSearchStop),
    NoRoute,
}
fn compress(points: Vec<[Q; 2]>) -> Vec<[Q; 2]> {
    let mut out: Vec<[Q; 2]> = Vec::new();
    for p in points {
        if out.last() == Some(&p) {
            continue;
        }
        if out.len() >= 2 {
            let a = &out[out.len() - 2];
            let b = out.last().unwrap();
            let x = [&b[0] - &a[0], &b[1] - &a[1]];
            let y = [&p[0] - &b[0], &p[1] - &b[1]];
            if (&x[0] * &y[1] - &x[1] * &y[0]).is_zero()
                && &x[0] * &y[0] + &x[1] * &y[1] >= Q::zero()
            {
                *out.last_mut().unwrap() = p;
                continue;
            }
        }
        out.push(p);
    }
    if out.len() == 1 {
        out.push(out[0].clone());
    }
    out
}
fn certify(
    v: &ClearanceView<'_>,
    at: [[usize; 2]; 2],
    points: Vec<[Q; 2]>,
    b: &mut Budget,
    already_clear: bool,
) -> Result<Attempt> {
    let points = compress(points);
    if points.len() > 4096 {
        return Ok(Attempt::Stop(PeriodicRegionSearchStop::PathPointBudget));
    }
    let mut cost = 0usize;
    for w in points.windows(2) {
        cost = cost
            .checked_add(v.cost(&w[0], &w[1])?)
            .ok_or_else(|| refused("Constructed material path work overflow"))?;
    }
    if !b.charge(cost) {
        return Ok(Attempt::Stop(PeriodicRegionSearchStop::WorkBudget));
    }
    let encoded = points
        .iter()
        .map(|p| p.clone().map(|q| ratio(&q)))
        .collect::<Vec<_>>();
    match super::periodic_support_path::construct_from_view(v, at, &encoded, 1000000)? {
        Some(p) => Ok(Attempt::Found(Arc::new(p))),
        None if !already_clear => Ok(Attempt::NoRoute),
        None => Err(refused(
            "Constructed grid connection lost its exact original clearance",
        )),
    }
}
struct Grid {
    low: [i64; 2],
    high: [i64; 2],
    height: usize,
    chart: [[Q; 2]; 2],
    step: [Q; 2],
}
impl Grid {
    fn new(v: &ClearanceView<'_>, level: usize, r: usize, cap: usize) -> Option<(Self, usize)> {
        let d = 1i64.checked_shl(level as u32)?;
        let r = r as i64;
        let low = std::array::from_fn(|i| if v.wrapped()[i] { -r * d } else { 1 });
        let high = std::array::from_fn(|i| if v.wrapped()[i] { (r + 1) * d } else { d - 1 });
        let width = usize::try_from(high[0] - low[0] + 1).ok()?;
        let height = usize::try_from(high[1] - low[1] + 1).ok()?;
        let n = width.checked_mul(height)?;
        if n > cap {
            return None;
        }
        Some((
            Self {
                low,
                high,
                height,
                chart: v.chart().clone(),
                step: v.periods().clone().map(|p| p / Q::from_integer(d.into())),
            },
            n,
        ))
    }
    fn id(&self, ij: [i64; 2]) -> Option<usize> {
        if (0..2).any(|a| ij[a] < self.low[a] || ij[a] > self.high[a]) {
            None
        } else {
            Some((ij[0] - self.low[0]) as usize * self.height + (ij[1] - self.low[1]) as usize)
        }
    }
    fn indices(&self, id: usize) -> [i64; 2] {
        [
            self.low[0] + (id / self.height) as i64,
            self.low[1] + (id % self.height) as i64,
        ]
    }
    fn point(&self, id: usize) -> [Q; 2] {
        let ij = self.indices(id);
        std::array::from_fn(|i| &self.chart[i][0] + &self.step[i] * Q::from_integer(ij[i].into()))
    }
    fn corners(&self, p: &[Q; 2]) -> Result<Vec<usize>> {
        let mut lo = [0i64; 2];
        for i in 0..2 {
            lo[i] = ((&p[i] - &self.chart[i][0]) / &self.step[i])
                .floor()
                .to_integer()
                .to_i64()
                .ok_or_else(|| refused("Material grid endpoint index overflow"))?;
        }
        let mut ids = Vec::new();
        for u in [lo[0], lo[0] + 1] {
            for v in [lo[1], lo[1] + 1] {
                if let Some(i) = self.id([u, v]) {
                    ids.push(i);
                }
            }
        }
        Ok(ids)
    }
}
fn grid_search(
    v: &ClearanceView<'_>,
    at: [[usize; 2]; 2],
    a: &[Q; 2],
    target: &[Q; 2],
    g: &Grid,
    n: usize,
    r: usize,
    b: &mut Budget,
) -> Result<Attempt> {
    let mut goals = BTreeMap::<usize, Vec<[Q; 2]>>::new();
    let ru = if v.wrapped()[0] { r as i64 } else { 0 };
    let rv = if v.wrapped()[1] { r as i64 } else { 0 };
    for u in -ru..=ru {
        for w in -rv..=rv {
            let q = [
                &target[0] + &v.periods()[0] * Q::from_integer(u.into()),
                &target[1] + &v.periods()[1] * Q::from_integer(w.into()),
            ];
            for i in g.corners(&q)? {
                goals.entry(i).or_default().push(q.clone());
            }
        }
    }
    let mut parent = vec![None; n];
    let mut queue = VecDeque::new();
    for id in g.corners(a)? {
        let p = g.point(id);
        match b.edge(v, a, &p)? {
            None => return Ok(Attempt::Stop(PeriodicRegionSearchStop::WorkBudget)),
            Some(false) => continue,
            Some(true) => {}
        }
        if parent[id].is_none() {
            parent[id] = Some(id);
            queue.push_back(id);
            b.visited += 1;
        }
    }
    while let Some(id) = queue.pop_front() {
        let p = g.point(id);
        if let Some(targets) = goals.get(&id) {
            for q in targets {
                match b.edge(v, &p, q)? {
                    None => return Ok(Attempt::Stop(PeriodicRegionSearchStop::WorkBudget)),
                    Some(false) => continue,
                    Some(true) => {}
                }
                let mut route = vec![p.clone()];
                let mut cur = id;
                while parent[cur] != Some(cur) {
                    cur = parent[cur]
                        .ok_or_else(|| refused("Material grid route lost a reachable parent"))?;
                    route.push(g.point(cur));
                }
                route.reverse();
                route.insert(0, a.clone());
                route.push(q.clone());
                return certify(v, at, route, b, true);
            }
        }
        let ij = g.indices(id);
        for next in [
            [ij[0] - 1, ij[1]],
            [ij[0] + 1, ij[1]],
            [ij[0], ij[1] - 1],
            [ij[0], ij[1] + 1],
        ] {
            let Some(j) = g.id(next) else { continue };
            if parent[j].is_some() {
                continue;
            }
            let q = g.point(j);
            match b.edge(v, &p, &q)? {
                None => return Ok(Attempt::Stop(PeriodicRegionSearchStop::WorkBudget)),
                Some(false) => continue,
                Some(true) => {}
            }
            parent[j] = Some(id);
            queue.push_back(j);
            b.visited += 1;
        }
    }
    Ok(Attempt::NoRoute)
}
fn connect(
    v: &ClearanceView<'_>,
    at: [[usize; 2]; 2],
    l: PeriodicRegionSearchLimits,
    b: &mut Budget,
) -> Result<Attempt> {
    let a = v.sample(at[0])?;
    let target = v.sample(at[1])?;
    match certify(v, at, vec![a.clone(), target.clone()], b, false)? {
        Attempt::NoRoute => {}
        result => return Ok(result),
    }
    let radius = if v.wrapped().iter().any(|x| *x) {
        l.max_period_radius
    } else {
        0
    };
    let mut skipped = false;
    // Fair diagonal enumeration prevents fine local grids starving coarse wider covers.
    for wave in 1..=l.max_level + radius {
        for level in 1..=l.max_level {
            if level > wave {
                continue;
            }
            let r = wave - level;
            if r > radius {
                continue;
            }
            let Some((g, n)) = Grid::new(v, level, r, l.max_grid_nodes) else {
                skipped = true;
                continue;
            };
            b.grids += 1;
            match grid_search(v, at, &a, &target, &g, n, r, b)? {
                Attempt::NoRoute => {}
                result => return Ok(result),
            }
        }
    }
    Ok(Attempt::Stop(if skipped {
        PeriodicRegionSearchStop::GridNodeBudget
    } else {
        PeriodicRegionSearchStop::ResolutionLimit
    }))
}
/// Successful reports contain the ordinary fully replayable region owner.
/// Incomplete reports retain real paths but never declare any class disconnected.
pub fn construct_periodic_support_regions(
    owner: Arc<OwnedPeriodicSupportSignatures>,
    parameter_limits: ParameterOrderLimits,
    limits: PeriodicRegionSearchLimits,
) -> Result<PeriodicRegionSearchReport> {
    if parameter_limits.max_refinements > 512
        || !(1..=128).contains(&parameter_limits.max_resultant_size)
        || !(1..=16).contains(&limits.max_level)
        || limits.max_period_radius > 16
        || !(1..=1000000).contains(&limits.max_grid_nodes)
        || !(1..=1000000).contains(&limits.max_work)
        || !(1..=1000000).contains(&limits.max_region_work)
    {
        return Err(refused("Invalid automatic material connection limits"));
    }
    owner.reverify_source()?;
    let mut groups = BTreeMap::<Vec<String>, Vec<usize>>::new();
    for (i, p) in owner.probes().iter().enumerate() {
        groups.entry(p.signature.clone()).or_default().push(i);
    }
    let mut b = Budget {
        charged: 0,
        visited: 0,
        grids: 0,
        cap: limits.max_work,
    };
    let mut paths = Vec::new();
    let mut stop = None;
    if groups.values().any(|w| w.len() > 1) {
        let view = ClearanceView::new(owner.owner(), parameter_limits)?;
        'groups: for group in groups.values() {
            for &j in &group[1..] {
                let at = [
                    owner.probes()[group[0]].endpoint,
                    owner.probes()[j].endpoint,
                ];
                match connect(&view, at, limits, &mut b)? {
                    Attempt::Found(p) => paths.push(p),
                    Attempt::Stop(s) => {
                        stop = Some(s);
                        break 'groups;
                    }
                    Attempt::NoRoute => {
                        return Err(refused(
                            "Automatic region search returned an unclassified route state",
                        ));
                    }
                }
            }
        }
    }
    let mut regions = None;
    if stop.is_none() {
        let needed = paths.iter().try_fold(
            owner
                .work()
                .checked_add(owner.probes().len())
                .and_then(|v| v.checked_add(paths.len()))
                .ok_or_else(|| refused("Material region proof work overflow"))?,
            |n, p| {
                n.checked_add(p.work())
                    .ok_or_else(|| refused("Material region proof work overflow"))
            },
        )?;
        if needed > limits.max_region_work {
            stop = Some(PeriodicRegionSearchStop::RegionProofBudget)
        } else {
            regions = Some(own_periodic_support_regions(
                owner.clone(),
                &paths,
                limits.max_region_work,
            )?);
        }
    }
    Ok(PeriodicRegionSearchReport {
        owner,
        paths,
        regions,
        stop,
        charged_work: b.charged,
        visited_nodes: b.visited,
        grids: b.grids,
    })
}

#[cfg(test)]
mod tests;
