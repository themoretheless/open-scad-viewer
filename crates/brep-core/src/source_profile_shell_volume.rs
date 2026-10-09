//! Adaptive outward flux integration of admitted expression geometry.
//! Reports enclosures and work limits; this does not construct a solid Body.
use super::*;
use nurbs_core::interval_eval::Interval as I;
use std::{cmp::Ordering, collections::BinaryHeap};

pub struct ExpressionVolumeLimits {
    pub axis: usize,
    pub origin: f64,
    pub absolute_error: f64,
    pub tolerance_uv: f64,
    pub controls_per_enclosure: usize,
    pub cells: usize,
    pub spans: usize,
    pub domain_cells: usize,
}
pub struct ExpressionVolumeReport {
    pub signed_bounds: Option<[f64; 2]>,
    pub complete: bool,
    pub cells: usize,
    pub spans: usize,
    pub domain_cells: usize,
    pub uncertain_face: Option<usize>,
    pub uncertain_domain: Option<[[f64; 2]; 2]>,
    pub reason: &'static str,
}
struct Node {
    face: usize,
    domain: [[f64; 2]; 2],
    bound: I,
    parent: Option<usize>,
    children: Option<[usize; 2]>,
}
struct Priority(usize, f64);
impl PartialEq for Priority { fn eq(&self, b: &Self) -> bool { self.cmp(b) == Ordering::Equal } }
impl Eq for Priority {}
impl PartialOrd for Priority { fn partial_cmp(&self, b: &Self) -> Option<Ordering> { Some(self.cmp(b)) } }
impl Ord for Priority { fn cmp(&self, b: &Self) -> Ordering { self.1.total_cmp(&b.1).then(self.0.cmp(&b.0)) } }

fn refine_sum(previous: I, left: I, right: I) -> Result<I> {
    let sum = left.add(right)?;
    // Both enclose the integral of the same complete material rectangle.
    // Retain the tighter old enclosure if child dependency bounds are wider.
    I::new(previous.lo.max(sum.lo), previous.hi.min(sum.hi))
}

impl EmbeddedExpressionGeometry<'_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn inspect_volume(&self, limits: ExpressionVolumeLimits) -> Result<ExpressionVolumeReport> {
        if limits.axis >= 3 || !limits.origin.is_finite()
            || !limits.absolute_error.is_finite() || limits.absolute_error <= 0.
            || !limits.tolerance_uv.is_finite() || limits.tolerance_uv <= 0.
            || limits.controls_per_enclosure > 1000000
            || limits.cells > 1000000 || limits.spans > 1000000 || limits.domain_cells > 8000000
        {
            return Err(Error::new("BREP_EXPRESSION_VOLUME", "Choose finite volume inputs and bounded work"));
        }
        let shell = self.shell();
        let mut out = ExpressionVolumeReport { signed_bounds: None, complete: false, cells: 0, spans: 0,
            domain_cells: 0, uncertain_face: None, uncertain_domain: None, reason: "expression-volume-initial-work-limit" };
        let evaluate = |face, domain, out: &mut ExpressionVolumeReport| -> Result<Option<I>> {
            out.uncertain_face = Some(face); out.uncertain_domain = Some(domain);
            if out.cells == limits.cells || out.spans == limits.spans { return Ok(None); }
            out.cells += 1;
            let report = shell.enclose_material_flux(face, domain, limits.axis, limits.origin,
                limits.tolerance_uv, limits.controls_per_enclosure,
                (limits.spans - out.spans).min(100000), (limits.domain_cells - out.domain_cells).min(100000))?;
            out.spans += report.spans; out.domain_cells += report.domain_cells;
            report.bound.map(|b| I::new(b[0], b[1])).transpose()
        };
        let mut nodes: Vec<Node> = Vec::new();
        let mut roots = Vec::new();
        let mut queue = BinaryHeap::new();
        for face in 0..shell.faces().len() {
            let domain = shell.support_domain(face)?;
            let Some(bound) = evaluate(face, domain, &mut out)? else { return Ok(out); };
            let index = nodes.len(); roots.push(index);
            queue.push(Priority(index, bound.hi - bound.lo));
            nodes.push(Node { face, domain, bound, parent: None, children: None });
        }
        loop {
            let mut total = I::point(0.);
            for &index in &roots { total = total.add(nodes[index].bound)?; }
            out.signed_bounds = Some([total.lo, total.hi]);
            if I::point(total.hi).sub(I::point(total.lo))?.hi <= limits.absolute_error {
                out.complete = true; out.uncertain_face = None; out.uncertain_domain = None;
                out.reason = if total.lo > 0. || total.hi < 0. { "expression-volume-sign-and-error-enclosed" } else { "expression-volume-sign-unresolved" };
                return Ok(out);
            }
            if let Some(next) = queue.peek() {
                out.uncertain_face = Some(nodes[next.0].face);
                out.uncertain_domain = Some(nodes[next.0].domain);
            }
            if out.cells.saturating_add(2) > limits.cells || out.spans == limits.spans {
                out.reason = "expression-volume-work-limit"; return Ok(out);
            }
            let Some(Priority(index, _)) = queue.pop() else { out.reason = "expression-volume-resolution-limit"; return Ok(out); };
            let node = &nodes[index]; let domain = node.domain; let face = node.face;
            let axis = usize::from(domain[1][1] - domain[1][0] > domain[0][1] - domain[0][0]);
            let midpoint = domain[axis][0] * 0.5 + domain[axis][1] * 0.5;
            if !(domain[axis][0] < midpoint && midpoint < domain[axis][1]) { continue; }
            let mut left = domain; let mut right = domain;
            left[axis][1] = midpoint; right[axis][0] = midpoint;
            let Some(a) = evaluate(face, left, &mut out)? else { out.reason = "expression-volume-work-limit"; return Ok(out); };
            let Some(b) = evaluate(face, right, &mut out)? else { out.reason = "expression-volume-work-limit"; return Ok(out); };
            let children = [nodes.len(), nodes.len() + 1];
            nodes[index].children = Some(children); nodes[index].bound = refine_sum(nodes[index].bound, a, b)?;
            for (domain, bound, child) in [(left, a, children[0]), (right, b, children[1])] {
                nodes.push(Node { face, domain, bound, parent: Some(index), children: None });
                queue.push(Priority(child, bound.hi - bound.lo));
            }
            let mut current = index;
            while let Some(parent) = nodes[current].parent {
                let [a, b] = nodes[parent].children.unwrap();
                nodes[parent].bound = refine_sum(nodes[parent].bound, nodes[a].bound, nodes[b].bound)?; current = parent;
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn subdivision_preserves_complete_old_bounds_and_refuses_contradiction() {
        let old = I::new(10., 20.).unwrap();
        let wide = refine_sum(old, I::new(-100., 100.).unwrap(), I::new(-100., 100.).unwrap()).unwrap();
        assert_eq!([wide.lo, wide.hi], [10., 20.]);
        let tight = refine_sum(old, I::new(5., 6.).unwrap(), I::new(6., 7.).unwrap()).unwrap();
        assert!(tight.lo <= 11. && tight.hi >= 13. && tight.lo > old.lo && tight.hi < old.hi);
        assert!(refine_sum(old, I::point(-5.), I::point(-5.)).is_err());
    }
}
