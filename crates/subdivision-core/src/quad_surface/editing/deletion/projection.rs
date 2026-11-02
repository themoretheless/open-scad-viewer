//! Bounded exact projection tests over original XYZ coordinates plus a chosen
//! projection direction. Never relabel rounded projected coordinates as input.
use super::*;
use cad_predicates::{
    self as p, AuthoredScalar, LeafRef, Outcome, PredicateContext, Sign, SourceArena,
    ToleranceContext,
};
pub(super) struct Projector {
    arena: SourceArena,
    tolerance: ToleranceContext,
    normal: [LeafRef; 3],
    points: Vec<[LeafRef; 3]>,
    direction: [f64; 3],
    boundary_count: usize,
    sense: i8,
    cache: BTreeMap<[usize; 3], i8>,
    between: BTreeMap<[usize; 3], bool>,
    pub used: u64,
    max: u64,
}
impl Projector {
    pub fn new(
        points: &[[f64; 3]],
        boundary_count: usize,
        normal: [f64; 3],
        max: u64,
    ) -> Result<Self> {
        if normal.iter().any(|v| !v.is_finite()) || normal.iter().all(|v| *v == 0.) {
            return Err(error("Choose a finite nonzero quad projection"));
        }
        let arena = SourceArena::authored(
            "quad-deletion-original-boundary",
            0,
            points
                .iter()
                .flatten()
                .chain(normal.iter())
                .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
                .collect(),
        )
        .map_err(|e| error(e.to_string()))?;
        let refs = (0..points.len())
            .map(|i| {
                Ok([
                    arena.leaf(3 * i)?,
                    arena.leaf(3 * i + 1)?,
                    arena.leaf(3 * i + 2)?,
                ])
            })
            .collect::<std::result::Result<Vec<_>, p::InputError>>()
            .map_err(|e| error(e.to_string()))?;
        let direction = normal;
        let k = 3 * points.len();
        let normal = [
            arena.leaf(k).map_err(|e| error(e.to_string()))?,
            arena.leaf(k + 1).map_err(|e| error(e.to_string()))?,
            arena.leaf(k + 2).map_err(|e| error(e.to_string()))?,
        ];
        Ok(Self {
            arena,
            tolerance: ToleranceContext::default_valid(),
            normal,
            points: refs,
            direction,
            boundary_count,
            sense: 1,
            cache: BTreeMap::new(),
            between: BTreeMap::new(),
            used: 0,
            max,
        })
    }
    fn decide(&mut self, kind: u8, ids: &[usize]) -> Result<i8> {
        let mut context = PredicateContext::new(
            &self.arena,
            &self.tolerance,
            p::Limits {
                max_work: self.max.saturating_sub(self.used).min(p::MAX_WORK),
                ..p::Limits::default()
            },
            None,
        );
        let r = match kind {
            0 => p::projected_orient3d(
                &mut context,
                self.points[ids[0]],
                self.points[ids[1]],
                self.points[ids[2]],
                self.normal,
            ),
            1 => p::projected_direction_dot3d(
                &mut context,
                self.points[ids[1]],
                self.points[ids[0]],
                self.points[ids[1]],
                self.points[ids[2]],
                self.normal,
            ),
            _ => p::projected_polygon_orientation(
                &mut context,
                &self.points[..self.boundary_count],
                self.normal,
            ),
        };
        self.used += context.work_used();
        match r.map_err(|e| error(e.to_string()))?.outcome {
            Outcome::Sign(Sign::Positive) => Ok(1),
            Outcome::Sign(Sign::Negative) => Ok(-1),
            Outcome::Sign(Sign::Zero) => Ok(0),
            Outcome::Indeterminate(r) => Err(error(format!(
                "Original projected predicate remains unproven: {r:?}"
            ))),
        }
    }
    pub fn boundary_count(&self) -> usize {
        self.boundary_count
    }
    pub fn direction(&self) -> [f64; 3] {
        self.direction.map(|v| v * self.sense as f64)
    }
    pub fn orient(&mut self, a: usize, b: usize, c: usize) -> Result<i8> {
        if a == b || a == c || b == c {
            return Ok(0);
        }
        let mut ids = [a, b, c];
        let mut parity = 1;
        for i in 0..3 {
            for j in i + 1..3 {
                if ids[i] > ids[j] {
                    ids.swap(i, j);
                    parity = -parity;
                }
            }
        }
        let s = if let Some(s) = self.cache.get(&ids) {
            *s
        } else {
            let s = self.decide(0, &ids)?;
            self.cache.insert(ids, s);
            s
        };
        Ok(s * parity * self.sense)
    }
    pub fn between(&mut self, a: usize, x: usize, b: usize) -> Result<bool> {
        let ids = [a.min(b), x, a.max(b)];
        if let Some(r) = self.between.get(&ids) {
            return Ok(*r);
        }
        let r = self.decide(1, &ids)? <= 0;
        self.between.insert(ids, r);
        Ok(r)
    }
    pub fn intersects(&mut self, a: usize, b: usize, c: usize, d: usize) -> Result<bool> {
        let q = [
            self.orient(a, b, c)?,
            self.orient(a, b, d)?,
            self.orient(c, d, a)?,
            self.orient(c, d, b)?,
        ];
        if q[0] * q[1] < 0 && q[2] * q[3] < 0 {
            return Ok(true);
        }
        Ok(q[0] == 0 && self.between(a, c, b)?
            || q[1] == 0 && self.between(a, d, b)?
            || q[2] == 0 && self.between(c, a, d)?
            || q[3] == 0 && self.between(c, b, d)?)
    }
    pub fn boundary(&mut self) -> Result<()> {
        let area = self.decide(2, &[])?;
        if area == 0 {
            return Err(error("Original hole projection has zero area"));
        }
        self.sense = area;
        let n = self.boundary_count;
        for i in 0..n {
            let j = (i + 1) % n;
            let mut nonzero = false;
            for k in 0..n {
                nonzero |= self.orient(i, j, k)? != 0;
            }
            if !nonzero {
                return Err(error("Original boundary has a collapsed projected edge"));
            }
            let previous = (i + n - 1) % n;
            if self.orient(previous, i, j)? == 0 && !self.between(previous, i, j)? {
                return Err(error("Projected boundary doubles back"));
            }
            for k in i + 1..n {
                let l = (k + 1) % n;
                if i != l && j != k && self.intersects(i, j, k, l)? {
                    return Err(error("Original projected hole boundary is not simple"));
                }
            }
        }
        Ok(())
    }
    fn cone(&mut self, i: usize, j: usize) -> Result<bool> {
        let n = self.boundary_count;
        let next = (i + 1) % n;
        let previous = (i + n - 1) % n;
        if self.orient(i, next, previous)? >= 0 {
            Ok(self.orient(i, j, previous)? > 0 && self.orient(j, i, next)? > 0)
        } else {
            Ok(!(self.orient(i, j, next)? >= 0 && self.orient(j, i, previous)? >= 0))
        }
    }
    pub fn diagonal(&mut self, i: usize, j: usize) -> Result<bool> {
        let n = self.boundary_count;
        if i == j {
            return Ok(false);
        }
        if i < n && j < n && ((i + 1) % n == j || (j + 1) % n == i) {
            return Ok(true);
        }
        if (i < n && !self.cone(i, j)?) || (j < n && !self.cone(j, i)?) {
            return Ok(false);
        }
        for k in 0..self.points.len() {
            if k != i && k != j && self.orient(i, j, k)? == 0 && self.between(i, k, j)? {
                return Ok(false);
            }
            if k >= n {
                continue;
            }
            let l = (k + 1) % n;
            if k != i && k != j && l != i && l != j && self.intersects(i, j, k, l)? {
                return Ok(false);
            }
        }
        Ok(true)
    }
    pub fn clear_edge(&mut self, a: usize, b: usize, count: usize) -> Result<bool> {
        let mut nonzero = false;
        for x in 0..count {
            if x == a || x == b {
                continue;
            }
            let side = self.orient(a, b, x)?;
            nonzero |= side != 0;
            if side == 0 && self.between(a, x, b)? {
                return Ok(false);
            }
        }
        Ok(nonzero)
    }
    pub fn quad(&mut self, q: [usize; 4]) -> Result<bool> {
        for [a, b, c, d] in [q, [q[1], q[2], q[3], q[0]]] {
            let x = self.orient(a, b, c)?;
            let y = self.orient(a, c, d)?;
            if x >= 0 && y >= 0 && (x > 0 || y > 0) {
                return Ok(true);
            }
        }
        Ok(false)
    }
}
