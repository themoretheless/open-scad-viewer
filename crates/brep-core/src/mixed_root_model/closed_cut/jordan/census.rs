//! Complete original-root coverage bound to distinct admitted components.
use super::*;
use std::sync::Arc;
#[derive(Clone, Copy, Debug)]
pub struct InteriorCensusAnchor {
    pub component: usize,
    pub edge: usize,
    /// Free clock of the corresponding coverage fiber, not a fitted point.
    pub parameter: f64,
    pub phase: [i32; 4],
}
#[derive(Clone, Copy)]
pub struct InteriorCensusLimits {
    pub separation_queries: usize,
    /// Explicit per-query allowance, shared query count bounds total work.
    pub geometry: ss::OriginalSsCurveLimits,
}
impl Default for InteriorCensusLimits {
    fn default() -> Self {
        Self {
            separation_queries: 4096,
            geometry: Default::default(),
        }
    }
}
pub struct QualifiedInteriorSsCensus {
    loops: Vec<Arc<QualifiedInteriorSsLoop>>,
    coverage: ss::QualifiedQuotientCoverage,
    anchors: Vec<InteriorCensusAnchor>,
    limits: InteriorCensusLimits,
    separation_queries: usize,
}
impl QualifiedInteriorSsCensus {
    pub fn components(&self) -> &[Arc<QualifiedInteriorSsLoop>] {
        &self.loops
    }
    pub fn coverage(&self) -> &ss::QualifiedQuotientCoverage {
        &self.coverage
    }
    pub fn anchors(&self) -> &[InteriorCensusAnchor] {
        &self.anchors
    }
    pub fn separation_queries(&self) -> usize {
        self.separation_queries
    }
    pub fn reverify(&self) -> Result<()> {
        for component in &self.loops {
            component.reverify()?;
        }
        self.coverage.reverify()?;
        let fresh = qualify_interior_ss_census(
            self.loops.clone(),
            self.coverage.clone(),
            &self.anchors,
            self.limits,
        )?;
        if fresh.separation_queries != self.separation_queries {
            return Err(unproved("SS component separation work changed"));
        }
        Ok(())
    }
}
fn separate(
    a: &QualifiedInteriorSsLoop,
    b: &QualifiedInteriorSsLoop,
    limits: InteriorCensusLimits,
    used: &mut usize,
) -> Result<()> {
    if *used == limits.separation_queries {
        return Err(unproved("SS component separation budget exhausted"));
    }
    *used += 1;
    let point = a.cuts[0].wire.edges()[0]
        .start()
        .point()
        .enclose(limits.geometry)?
        .uv;
    for edge in b.cuts[0].wire.edges() {
        let d = edge.parameter_bounds();
        let natural = edge.curve().parameter_domain();
        let range = [
            clock(&d[0], true)?.max(natural[0]),
            clock(&d[1], false)?.min(natural[1]),
        ];
        let mut pending = vec![(range, 0usize)];
        while let Some((range, depth)) = pending.pop() {
            if *used == limits.separation_queries {
                return Err(unproved("SS component separation budget exhausted"));
            }
            *used += 1;
            let uv = edge.curve().enclose(range, limits.geometry)?.uv;
            if (0..4).any(|k| point[k][1] < uv[k][0] || uv[k][1] < point[k][0]) {
                continue;
            }
            let middle = range[0] * 0.5 + range[1] * 0.5;
            if depth == 32 || !(range[0] < middle && middle < range[1]) {
                return Err(unproved(
                    "SS loops are duplicated or distinct components remain unproved",
                ));
            }
            pending.push(([range[0], middle], depth + 1));
            pending.push(([middle, range[1]], depth + 1));
        }
    }
    Ok(())
}
/// All roots in the complete original product are assigned to admitted Jordan
/// components. Connected graph fibers with actual root anchors cannot leave
/// their relatively open/closed component. Pairwise separation of one exact
/// selector from the entire other loop proves the component images distinct.
/// This does not assemble Boolean material or admit a mixed closed solid.
pub fn qualify_interior_ss_census(
    loops: Vec<Arc<QualifiedInteriorSsLoop>>,
    coverage: ss::QualifiedQuotientCoverage,
    anchors: &[InteriorCensusAnchor],
    limits: InteriorCensusLimits,
) -> Result<QualifiedInteriorSsCensus> {
    if !(1..=64).contains(&loops.len())
        || !coverage.covers_entire_surfaces()
        || anchors.len() != coverage.fibers().len()
        || !(1..=1000000).contains(&limits.separation_queries)
    {
        return Err(unproved(
            "SS census requires complete coverage and one anchor per source graph",
        ));
    }
    let source = coverage.fibers()[0].source();
    for component in &loops {
        let s = component.cuts[0].wire.edges()[0].curve().source();
        if s.first() != source.first() || s.second() != source.second() {
            return Err(unproved(
                "SS census component has foreign original supports",
            ));
        }
    }
    let mut used = 0;
    for i in 0..loops.len() {
        for j in i + 1..loops.len() {
            separate(&loops[i], &loops[j], limits, &mut used)?;
        }
    }
    let mut owned = vec![false; loops.len()];
    for (fiber, anchor) in coverage.fibers().iter().zip(anchors) {
        let component = loops
            .get(anchor.component)
            .ok_or_else(|| unproved("SS census anchor component is absent"))?;
        let edge = component.cuts[0]
            .wire
            .edges()
            .get(anchor.edge)
            .ok_or_else(|| unproved("SS census anchor edge is absent"))?;
        let curve = Arc::new(ss::OriginalSsRootCurve::from_quotient(fiber.clone()));
        let point = Arc::new(ss::OriginalSsRootPoint::new(curve, anchor.parameter)?);
        let port =
            ss::bind_original_ss_port(point, edge.curve().clone(), anchor.phase, limits.geometry)?;
        let p = port.parameter_bounds();
        let (lo, hi) = if edge.forward() {
            (
                edge.start().parameter_bounds(),
                edge.end().parameter_bounds(),
            )
        } else {
            (
                edge.end().parameter_bounds(),
                edge.start().parameter_bounds(),
            )
        };
        if !(lo[1] < p[0] && p[1] < hi[0]) {
            return Err(unproved(
                "SS census anchor is not strictly on the retained root edge",
            ));
        }
        owned[anchor.component] = true;
    }
    if owned.iter().any(|x| !*x) {
        return Err(unproved("SS census has an unowned supplied component"));
    }
    Ok(QualifiedInteriorSsCensus {
        loops,
        coverage,
        anchors: anchors.to_vec(),
        limits,
        separation_queries: used,
    })
}
