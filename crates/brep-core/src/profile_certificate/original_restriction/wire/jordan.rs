//! Complete self/pair census for exact rational closed wires. Only authored
//! adjacent endpoint contacts are allowed. Nesting and winding are separate.
use super::*;
use crate::profile_certificate::arithmetic::{self, algebraic_events as engine};
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub max_pairs: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self { max_pairs: 32768 }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    Monotone,
    DisjointHull,
    TouchingHulls,
    ImplicitEndpoint,
    Algebraic,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pair {
    pub first: [usize; 3],
    pub second: [usize; 3],
    pub method: Method,
}
#[derive(Clone, Debug)]
pub struct JordanWires {
    sources: Vec<Wire>,
    limits: Limits,
    pairs: Vec<Pair>,
}
struct Cell<'a> {
    id: [usize; 3],
    ordinal: usize,
    count: usize,
    net: &'a [[Q; 3]],
}
fn xy(h: &[Q; 3]) -> [Q; 2] {
    [&h[0] / &h[2], &h[1] / &h[2]]
}
fn bounds(net: &[[Q; 3]]) -> [[Q; 2]; 2] {
    let p = xy(&net[0]);
    let mut out = [p.clone(), p];
    for h in net {
        let p = xy(h);
        for a in 0..2 {
            out[0][a] = out[0][a].clone().min(p[a].clone());
            out[1][a] = out[1][a].clone().max(p[a].clone());
        }
    }
    out
}
fn allowed(a: &Cell, b: &Cell) -> Vec<[usize; 2]> {
    let mut out = Vec::new();
    if a.id[0] == b.id[0] {
        if a.ordinal + 1 == b.ordinal {
            out.push([1, 0]);
        }
        if a.ordinal == 0 && b.ordinal + 1 == a.count {
            out.push([0, 1]);
        }
    }
    out
}
fn endpoints(net: &[[Q; 3]]) -> [[Q; 2]; 2] {
    [xy(&net[0]), xy(net.last().unwrap())]
}
fn endpoint_census(a: &Cell, b: &Cell, allowed: &[[usize; 2]]) -> Result<()> {
    let x = endpoints(a.net);
    let y = endpoints(b.net);
    for i in 0..2 {
        for j in 0..2 {
            if x[i] == y[j] && !allowed.contains(&[i, j]) {
                return Err(refused("Exact profile has an unauthored endpoint contact"));
            }
        }
    }
    Ok(())
}
fn hull(a: &Cell, b: &Cell) -> Option<Method> {
    let x = bounds(a.net);
    let y = bounds(b.net);
    for axis in 0..2 {
        if x[1][axis] < y[0][axis] || y[1][axis] < x[0][axis] {
            return Some(Method::DisjointHull);
        }
        // Positive Bernstein weights imply STRICT interior separation when
        // each hull has positive extent and they only touch on this axis.
        if x[0][axis] < x[1][axis]
            && y[0][axis] < y[1][axis]
            && (x[1][axis] == y[0][axis] || y[1][axis] == x[0][axis])
        {
            return Some(Method::TouchingHulls);
        }
    }
    None
}
fn endpoint_parameter(p: &[Q; 2]) -> Option<usize> {
    if p[0] == p[1] {
        if p[0].is_zero() {
            return Some(0);
        }
        if p[0].is_one() {
            return Some(1);
        }
    }
    None
}
fn algebraic(a: &Cell, b: &Cell, self_pair: bool, allowed: &[[usize; 2]]) -> Result<()> {
    let width = Q::new(1.into(), 1_000_000_000_000i64.into());
    let (projection, factors) = engine::produce(a.net, b.net, self_pair, &width)?;
    let events = engine::verify(a.net, b.net, self_pair, &width, projection, &factors)?;
    for event in events {
        let contact = endpoint_parameter(&event.first)
            .zip(endpoint_parameter(&event.second))
            .map(|(i, j)| [i, j]);
        if contact.as_ref().is_none_or(|p| !allowed.contains(p)) {
            return Err(refused(
                "Exact profile has an interior intersection or unauthored contact",
            ));
        }
    }
    Ok(())
}
fn derive(sources: &[Wire], limits: Limits) -> Result<Vec<Pair>> {
    let n = sources
        .iter()
        .flat_map(|w| &w.pieces)
        .map(Vec::len)
        .sum::<usize>();
    if sources.is_empty()
        || sources.len() > 64
        || n == 0
        || n > 254
        || limits.max_pairs > 32768
        || n * (n + 1) / 2 > limits.max_pairs
    {
        return Err(refused("Exact Jordan pair census exceeds its budget"));
    }
    for wire in sources {
        wire.reverify()?;
    }
    let mut cells = Vec::new();
    for (wi, wire) in sources.iter().enumerate() {
        let count = wire.pieces.iter().map(Vec::len).sum();
        let mut ordinal = 0;
        for (pi, piece) in wire.pieces.iter().enumerate() {
            for (si, span) in piece.iter().enumerate() {
                smooth::regular(&span.net)?;
                cells.push(Cell {
                    id: [wi, pi, si],
                    ordinal,
                    count,
                    net: &span.net,
                });
                ordinal += 1;
            }
        }
    }
    let mut pairs = Vec::new();
    for i in 0..cells.len() {
        for j in i..cells.len() {
            let a = &cells[i];
            let b = &cells[j];
            let method = if i == j {
                if (0..2).any(|axis| matches!(arithmetic::monotone(a.net, axis), Ok(true))) {
                    Method::Monotone
                } else {
                    let contacts = if a.count == 1 { vec![[0, 1]] } else { vec![] };
                    algebraic(a, b, true, &contacts)?;
                    Method::Algebraic
                }
            } else {
                let contacts = allowed(a, b);
                endpoint_census(a, b, &contacts)?;
                if let Some(method) = hull(a, b) {
                    method
                } else if matches!(
                    arithmetic::endpoint_only(a.net, b.net, !contacts.is_empty()),
                    Ok(true)
                ) {
                    Method::ImplicitEndpoint
                } else {
                    algebraic(a, b, false, &contacts)?;
                    Method::Algebraic
                }
            };
            pairs.push(Pair {
                first: a.id,
                second: b.id,
                method,
            });
        }
    }
    Ok(pairs)
}
pub fn qualify(sources: &[Wire], limits: Limits) -> Result<JordanWires> {
    let pairs = derive(sources, limits)?;
    Ok(JordanWires {
        sources: sources.to_vec(),
        limits,
        pairs,
    })
}
impl JordanWires {
    pub fn sources(&self) -> &[Wire] {
        &self.sources
    }
    pub fn pairs(&self) -> &[Pair] {
        &self.pairs
    }
    pub fn reverify(&self) -> Result<()> {
        if derive(&self.sources, self.limits)? != self.pairs {
            return Err(refused("Exact Jordan source census changed"));
        }
        Ok(())
    }
}
