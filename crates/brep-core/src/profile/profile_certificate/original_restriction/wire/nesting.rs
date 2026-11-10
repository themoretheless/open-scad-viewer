//! Exact orientation and containment of already certified disjoint Jordan
//! wires. Current cap layout is one CCW outer loop and nonnested CW holes.
use super::*;
use crate::profile_certificate::arithmetic::{carrier_sides, ray_winding};
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub max_ray_cells: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_ray_cells: 32768,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Orientation {
    pub probes: [[ExactRatio; 2]; 2],
    pub windings: [i64; 2],
}
#[derive(Clone, Debug)]
pub struct Nesting {
    jordan: jordan::JordanWires,
    limits: Limits,
    orientations: Vec<Orientation>,
    containment: Vec<Vec<Option<i64>>>,
    ray_cells: usize,
}
fn winding(nets: &[Vec<[Q; 3]>], point: &[Q; 2], work: &mut usize, limits: Limits) -> Result<i64> {
    let mut total = 0i64;
    for net in nets {
        if *work >= limits.max_ray_cells {
            return Err(refused("Exact nesting ray-cell budget exhausted"));
        }
        *work += 1;
        let witness = ray_winding::produce(net, point)?;
        total = total
            .checked_add(ray_winding::verify(net, point, &witness)?)
            .ok_or_else(|| refused("Exact nesting winding overflow"))?;
    }
    Ok(total)
}
fn orientation(nets: &[Vec<[Q; 3]>], work: &mut usize, limits: Limits) -> Result<Orientation> {
    let t = Q::new(1.into(), 2.into());
    let mut radius = carrier_sides::radius::suggest(nets, 0, &t, 192).unwrap_or_else(|_| Q::one());
    let mut probes = None;
    for step in 0..=32 {
        if let Ok(p) = carrier_sides::probes(nets, 0, &t, &radius, 192) {
            probes = Some(p);
            break;
        }
        if step < 32 {
            radius /= Q::from_integer(2.into());
        }
    }
    let probes =
        probes.ok_or_else(|| refused("Exact nesting transverse orientation remains unproved"))?;
    let windings = [
        winding(nets, &probes[0], work, limits)?,
        winding(nets, &probes[1], work, limits)?,
    ];
    if windings != [1, 0] && windings != [0, -1] {
        return Err(refused(
            "Exact Jordan orientation probes have inconsistent winding",
        ));
    }
    Ok(Orientation {
        probes: probes.map(|p| p.each_ref().map(ratio)),
        windings,
    })
}
type Data = (Vec<Orientation>, Vec<Vec<Option<i64>>>, usize);
fn derive(jordan: &jordan::JordanWires, limits: Limits) -> Result<Data> {
    if limits.max_ray_cells > 32768 {
        return Err(refused("Invalid exact nesting ray budget"));
    }
    jordan.reverify()?;
    let nets = jordan
        .sources()
        .iter()
        .map(|w| {
            w.pieces
                .iter()
                .flatten()
                .map(|s| s.net.clone())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let mut work = 0;
    let mut orientations = Vec::new();
    for (i, net) in nets.iter().enumerate() {
        let o = orientation(net, &mut work, limits)?;
        if o.windings != if i == 0 { [1, 0] } else { [0, -1] } {
            return Err(refused("Exact cap needs CCW outer and CW hole orientation"));
        }
        orientations.push(o);
    }
    let mut containment = vec![vec![None; nets.len()]; nets.len()];
    for (i, net) in nets.iter().enumerate() {
        let h = &net[0][0];
        let point = [&h[0] / &h[2], &h[1] / &h[2]];
        for (j, other) in nets.iter().enumerate() {
            if i == j {
                continue;
            }
            let value = winding(other, &point, &mut work, limits)?;
            containment[i][j] = Some(value);
            let expected = if i > 0 && j == 0 { 1 } else { 0 };
            if value != expected {
                return Err(refused(
                    "Exact cap hole is outside, contains the outer, or nests another hole",
                ));
            }
        }
    }
    Ok((orientations, containment, work))
}
pub fn qualify(jordan: &jordan::JordanWires, limits: Limits) -> Result<Nesting> {
    let (orientations, containment, ray_cells) = derive(jordan, limits)?;
    Ok(Nesting {
        jordan: jordan.clone(),
        limits,
        orientations,
        containment,
        ray_cells,
    })
}
impl Nesting {
    pub fn jordan(&self) -> &jordan::JordanWires {
        &self.jordan
    }
    pub fn orientations(&self) -> &[Orientation] {
        &self.orientations
    }
    /// Entry [i][j] is the winding of loop j at an exact point on loop i.
    /// Diagonal entries are intentionally absent (the point is on that loop).
    pub fn containment(&self) -> &[Vec<Option<i64>>] {
        &self.containment
    }
    pub fn ray_cells(&self) -> usize {
        self.ray_cells
    }
    pub fn reverify(&self) -> Result<()> {
        let (o, c, w) = derive(&self.jordan, self.limits)?;
        if o != self.orientations || c != self.containment || w != self.ray_cells {
            return Err(refused("Exact cap nesting replay changed"));
        }
        Ok(())
    }
}
