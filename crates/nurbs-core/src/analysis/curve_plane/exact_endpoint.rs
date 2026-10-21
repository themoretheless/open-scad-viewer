//! Shared-budget exact original-source endpoint incidence fallback.
use crate::curve::Curve;
use cad_predicates::{
    AuthoredScalar, Limits, OriginalPlaneRootsOutcome, Outcome, PredicateContext, Reason, Sign,
    SourceArena, ToleranceContext, rational_bspline_plane_incidence, rational_bspline_plane_roots,
    rational_quadratic_plane_discriminant,
};
use std::collections::BTreeMap;
pub(super) struct ExactEndpoints<'a> {
    c: &'a Curve,
    normal: [f64; 3],
    offset: f64,
    max_work: u64,
    pub(super) work: u64,
    pub(super) exhausted: bool,
    cache: BTreeMap<(usize, u64), Option<bool>>,
    quadratic_cache: BTreeMap<usize, Option<Sign>>,
    roots_cache: BTreeMap<(usize, u64), Option<OriginalPlaneRootsOutcome>>,
}
impl<'a> ExactEndpoints<'a> {
    pub(super) fn new(c: &'a Curve, normal: [f64; 3], offset: f64, max_work: u64) -> Self {
        Self {
            c,
            normal,
            offset,
            max_work,
            work: 0,
            exhausted: false,
            cache: BTreeMap::new(),
            quadratic_cache: BTreeMap::new(),
            roots_cache: BTreeMap::new(),
        }
    }
    pub(super) fn incidence(&mut self, span: usize, t: f64) -> Option<bool> {
        let key = (span, if t == 0. { 0 } else { t.to_bits() });
        if let Some(v) = self.cache.get(&key) {
            return *v;
        }
        if self.exhausted || self.work == self.max_work {
            self.exhausted = true;
            return None;
        }
        let c = self.c;
        let mut values: Vec<_> = c
            .control_points
            .iter()
            .zip(&c.weights)
            .flat_map(|(p, w)| [p[0], p[1], p[2], *w])
            .collect();
        let knot_start = values.len();
        values.extend(&c.knots);
        let plane_start = values.len();
        values.extend(self.normal);
        values.extend([self.offset, t]);
        let arena = SourceArena::authored(
            "nurbs-core/curve-plane/original-query",
            1,
            values
                .into_iter()
                .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
                .collect(),
        )
        .ok()?;
        let controls: Vec<_> = (0..c.control_points.len())
            .map(|i| std::array::from_fn(|j| arena.leaf(i * 4 + j).unwrap()))
            .collect();
        let knots: Vec<_> = (0..c.knots.len())
            .map(|i| arena.leaf(knot_start + i).unwrap())
            .collect();
        let plane = std::array::from_fn(|i| arena.leaf(plane_start + i).unwrap());
        let parameter = arena.leaf(plane_start + 4).unwrap();
        let tolerance = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(
            &arena,
            &tolerance,
            Limits {
                max_work: self.max_work - self.work,
                ..Limits::default()
            },
            None,
        );
        let decision = rational_bspline_plane_incidence(
            &mut ctx, &controls, &knots, c.degree, span, parameter, plane,
        )
        .ok()?;
        let result = self.record(decision).map(|sign| sign == Sign::Zero);
        self.cache.insert(key, result);
        result
    }
    fn record(&mut self, decision: cad_predicates::Decision) -> Option<Sign> {
        self.work += decision.work_used;
        match decision.outcome {
            Outcome::Sign(sign) => Some(sign),
            Outcome::Indeterminate(Reason::ResourceLimit) => {
                self.exhausted = true;
                None
            }
            Outcome::Indeterminate(_) => None,
        }
    }
    pub(super) fn discriminant(&mut self, span: usize) -> Option<Sign> {
        if let Some(v) = self.quadratic_cache.get(&span) {
            return *v;
        }
        if self.exhausted || self.work == self.max_work {
            self.exhausted = true;
            return None;
        }
        let c = self.c;
        let mut values = Vec::new();
        for i in span - 2..=span {
            values.extend([
                c.control_points[i][0],
                c.control_points[i][1],
                c.control_points[i][2],
                c.weights[i],
            ]);
        }
        values.extend(self.normal);
        values.push(self.offset);
        let arena = SourceArena::authored(
            "nurbs-core/curve-plane/quadratic-source",
            1,
            values
                .into_iter()
                .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
                .collect(),
        )
        .ok()?;
        let controls =
            std::array::from_fn(|i| std::array::from_fn(|j| arena.leaf(i * 4 + j).unwrap()));
        let plane = std::array::from_fn(|i| arena.leaf(12 + i).unwrap());
        let tolerance = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(
            &arena,
            &tolerance,
            Limits {
                max_work: self.max_work - self.work,
                ..Limits::default()
            },
            None,
        );
        let decision = rational_quadratic_plane_discriminant(&mut ctx, controls, plane).ok()?;
        let result = self.record(decision);
        self.quadratic_cache.insert(span, result);
        result
    }
    pub(super) fn roots(
        &mut self,
        span: usize,
        tolerance: f64,
    ) -> Option<OriginalPlaneRootsOutcome> {
        let key = (span, tolerance.to_bits());
        if let Some(v) = self.roots_cache.get(&key) {
            return v.clone();
        }
        if self.exhausted || self.work == self.max_work {
            self.exhausted = true;
            return None;
        }
        let c = self.c;
        let mut values: Vec<_> = c
            .control_points
            .iter()
            .zip(&c.weights)
            .flat_map(|(p, w)| [p[0], p[1], p[2], *w])
            .collect();
        let knot_start = values.len();
        values.extend(&c.knots);
        let plane_start = values.len();
        values.extend(self.normal);
        values.push(self.offset);
        let arena = SourceArena::authored(
            "nurbs-core/curve-plane/original-polynomial",
            1,
            values
                .into_iter()
                .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
                .collect(),
        )
        .ok()?;
        let controls: Vec<_> = (0..c.control_points.len())
            .map(|i| std::array::from_fn(|j| arena.leaf(i * 4 + j).unwrap()))
            .collect();
        let knots: Vec<_> = (0..c.knots.len())
            .map(|i| arena.leaf(knot_start + i).unwrap())
            .collect();
        let plane = std::array::from_fn(|i| arena.leaf(plane_start + i).unwrap());
        let tol = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(
            &arena,
            &tol,
            Limits {
                max_work: self.max_work - self.work,
                ..Limits::default()
            },
            None,
        );
        let decision = rational_bspline_plane_roots(
            &mut ctx, &controls, &knots, c.degree, span, plane, tolerance,
        )
        .ok()?;
        self.work += decision.work_used;
        let result = match decision.outcome {
            OriginalPlaneRootsOutcome::Indeterminate(Reason::ResourceLimit) => {
                self.exhausted = true;
                None
            }
            OriginalPlaneRootsOutcome::Indeterminate(_) => None,
            v => Some(v),
        };
        self.roots_cache.insert(key, result.clone());
        result
    }
}
