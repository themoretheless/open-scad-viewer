//! Complete material-side census on exact rational Wire geometry. Both inputs
//! already own simple oriented Jordan regions. All isolated intersections and
//! common-image branch critical values cut every selected source parameter.
//! One fully certified transverse witness per open atom decides both windings.
//! Every nonempty open difference component has an open boundary atom; finite
//! critical vertices cannot hide an open material component. No graph or f64
//! surrogate is substituted for the actual rational boundary.
use super::*;
use crate::profile_certificate::arithmetic as a;
use a::{branch_neighbourhood::ParameterValue, carrier_sides, ray_winding};
mod projection;
pub use projection::{Partition, Root};
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub max_pair_projections: usize,
    pub max_atoms: usize,
    pub max_ray_cells: usize,
    pub refinements: usize,
    pub radius_halvings: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_pair_projections: 32768,
            max_atoms: 32768,
            max_ray_cells: 32768,
            refinements: 192,
            radius_halvings: 32,
        }
    }
}
/// Actual material-census work, independent of the supplied allowances.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Work {
    pub pair_projections: usize,
    pub atoms: usize,
    pub ray_cells: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sector {
    pub selected: usize,
    pub parameter: ExactRatio,
    pub side: usize,
    pub point: [ExactRatio; 2],
    pub source_winding: i64,
    pub result_winding: i64,
}
#[derive(Clone, Debug)]
pub struct Relation {
    source: nesting::Nesting,
    result: nesting::Nesting,
    limits: Limits,
    partitions: Vec<Partition>,
    sectors: Vec<Sector>,
    pair_projections: usize,
    atoms: usize,
    ray_cells: usize,
}
#[derive(Clone, Debug)]
pub struct NoAddition {
    relation: Relation,
}
fn rational(value: Q) -> ParameterValue {
    ParameterValue {
        primitive: vec![Q::zero(), Q::one()],
        interval: [Q::zero(), Q::zero()],
        value: if value.is_zero() {
            Vec::new()
        } else {
            vec![value]
        },
    }
}
fn centers(
    nets: &[Vec<[Q; 3]>],
    selected: usize,
    t: &Q,
) -> Result<Vec<carrier_sides::CenterInterval>> {
    let n = &nets[selected];
    let h: [Q; 3] = std::array::from_fn(|k| a::at(&a::power(n, k), t));
    let point = [&h[0] / &h[2], &h[1] / &h[2]];
    let mut centers = Vec::new();
    for (cell, net) in nets.iter().enumerate() {
        let w = a::power(net, 2);
        let equations: [Vec<Q>; 2] = std::array::from_fn(|k| {
            a::sub(
                &a::power(net, k),
                &w.iter().map(|v| v * &point[k]).collect::<Vec<_>>(),
            )
        });
        let g = a::gcd(&equations[0], &equations[1])?;
        if g.is_empty() {
            return Err(refused("Constant exact material boundary cell"));
        }
        if a::at(&g, &Q::zero()).is_zero() || a::at(&g, &Q::one()).is_zero() {
            return Err(refused("Material atom hits an uncut boundary endpoint"));
        }
        let count = a::algebraic_events::count(&g, &Q::zero(), &Q::one())?;
        if count > 1 {
            return Err(refused(
                "Material center has multiple source-cell preimages",
            ));
        }
        if count == 1 {
            centers.push(carrier_sides::CenterInterval {
                cell,
                bounds: [rational(Q::zero()), rational(Q::one())],
            });
        }
    }
    Ok(centers)
}
fn probes(nets: &[Vec<[Q; 3]>], selected: usize, t: &Q, limits: Limits) -> Result<[[Q; 2]; 2]> {
    let centers = centers(nets, selected, t)?;
    let mut radius = carrier_sides::radius::suggest(nets, selected, t, limits.refinements)
        .unwrap_or_else(|_| Q::one());
    for step in 0..=limits.radius_halvings {
        if let Ok(probes) = carrier_sides::probes_with_centers(
            carrier_sides::TransverseSegment {
                nets,
                selected,
                t,
                radius: &radius,
                refinements: limits.refinements,
            },
            &centers,
            64,
        ) {
            return Ok(probes);
        }
        if step < limits.radius_halvings {
            radius /= Q::from_integer(2.into());
        }
    }
    Err(refused(
        "Exact rational material transverse census exhausted",
    ))
}
fn winding(nets: &[Vec<[Q; 3]>], point: &[Q; 2], work: &mut usize, limits: Limits) -> Result<i64> {
    let mut sum = 0i64;
    for net in nets {
        if *work >= limits.max_ray_cells {
            return Err(refused("Exact material winding budget exhausted"));
        }
        *work += 1;
        let witness = ray_winding::produce(net, point)?;
        sum = sum
            .checked_add(ray_winding::verify(net, point, &witness)?)
            .ok_or_else(|| refused("Exact material winding overflow"))?;
    }
    Ok(sum)
}
fn nets(region: &nesting::Nesting) -> Vec<Vec<[Q; 3]>> {
    region
        .jordan()
        .sources()
        .iter()
        .flat_map(|w| &w.pieces)
        .flatten()
        .map(|s| s.net.clone())
        .collect()
}
pub fn inspect(
    source: &nesting::Nesting,
    result: &nesting::Nesting,
    limits: Limits,
) -> Result<Relation> {
    source.reverify()?;
    result.reverify()?;
    let old = nets(source);
    let new = nets(result);
    let combined = old.iter().chain(&new).cloned().collect::<Vec<_>>();
    if combined.len() > 254
        || limits.max_pair_projections > 32768
        || limits.max_atoms > 32768
        || limits.max_ray_cells > 32768
        || limits.refinements > 512
        || limits.radius_halvings > 512
    {
        return Err(refused("Invalid exact material census or allowances"));
    }
    let mut pairs = 0;
    let mut atoms = 0;
    let mut ray_cells = 0;
    let mut partitions = Vec::new();
    let mut sectors = Vec::new();
    for selected in 0..combined.len() {
        let other = if selected < old.len() { &new } else { &old };
        let mut terms = Vec::new();
        for net in other {
            if pairs >= limits.max_pair_projections {
                return Err(refused("Exact material pair projection budget exhausted"));
            }
            pairs += 1;
            terms.push(projection::pair(&combined[selected], net)?);
        }
        let (partition, samples) = projection::partition(selected, &terms, limits)?;
        for t in samples {
            if atoms >= limits.max_atoms {
                return Err(refused("Exact material atom budget exhausted"));
            }
            atoms += 1;
            for (side, point) in probes(&combined, selected, &t, limits)?
                .into_iter()
                .enumerate()
            {
                sectors.push(Sector {
                    selected,
                    parameter: ratio(&t),
                    side,
                    point: point.each_ref().map(ratio),
                    source_winding: winding(&old, &point, &mut ray_cells, limits)?,
                    result_winding: winding(&new, &point, &mut ray_cells, limits)?,
                });
            }
        }
        partitions.push(partition);
    }
    Ok(Relation {
        source: source.clone(),
        result: result.clone(),
        limits,
        partitions,
        sectors,
        pair_projections: pairs,
        atoms,
        ray_cells,
    })
}
impl Relation {
    pub fn work(&self) -> Work {
        Work {
            pair_projections: self.pair_projections,
            atoms: self.atoms,
            ray_cells: self.ray_cells,
        }
    }
    pub fn limits(&self) -> Limits {
        self.limits
    }
    pub fn source(&self) -> &nesting::Nesting {
        &self.source
    }
    pub fn result(&self) -> &nesting::Nesting {
        &self.result
    }
    pub fn partitions(&self) -> &[Partition] {
        &self.partitions
    }
    pub fn sectors(&self) -> &[Sector] {
        &self.sectors
    }
    pub fn ray_cells(&self) -> usize {
        self.ray_cells
    }
    pub fn added(&self) -> impl Iterator<Item = &Sector> {
        self.sectors
            .iter()
            .filter(|s| s.source_winding == 0 && s.result_winding != 0)
    }
    pub fn removed(&self) -> impl Iterator<Item = &Sector> {
        self.sectors
            .iter()
            .filter(|s| s.source_winding != 0 && s.result_winding == 0)
    }
    pub fn result_is_subset(&self) -> bool {
        self.added().next().is_none()
    }
    pub fn source_is_subset(&self) -> bool {
        self.removed().next().is_none()
    }
    pub fn require_no_addition(self) -> Result<NoAddition> {
        if !self.result_is_subset() {
            return Err(refused("Exact rational result contains added material"));
        }
        Ok(NoAddition { relation: self })
    }
    pub fn reverify(&self) -> Result<()> {
        let expected = inspect(&self.source, &self.result, self.limits)?;
        if expected.partitions != self.partitions
            || expected.sectors != self.sectors
            || expected.work() != self.work()
        {
            return Err(refused("Exact rational material census replay changed"));
        }
        Ok(())
    }
}
impl NoAddition {
    pub fn relation(&self) -> &Relation {
        &self.relation
    }
}
