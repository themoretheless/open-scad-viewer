//! Independent outward flux integration over the actual retained face domains.
//! Unknown trim cells retain a complete [0,1] mask; no sampled area is omitted.
use super::*;
use nurbs_core::{interval_eval::Interval as I, trim_domain::Location};
#[derive(Clone, Copy, Debug)]
pub struct VolumeLimits {
    pub axis: usize,
    pub origin: f64,
    pub absolute_error: f64,
    pub cells: usize,
    pub spans: usize,
    pub domain_cells: usize,
    pub algebra: nurbs_core::ss_intersection::ExactSsAlgebraLimits,
}
pub struct VolumeReport {
    pub signed_bounds: Option<[f64; 2]>,
    pub positive_volume: Option<[f64; 2]>,
    pub cells: usize,
    pub spans: usize,
    pub domain_cells: usize,
    pub algebra_work: usize,
    pub reason: &'static str,
}
struct Node {
    face: usize,
    domain: [[f64; 2]; 2],
    bound: I,
    parent: Option<usize>,
    children: Option<[usize; 2]>,
}
#[derive(Clone, Copy)]
struct Priority {
    width: f64,
    index: usize,
}
impl PartialEq for Priority {
    fn eq(&self, other: &Self) -> bool {
        self.width.total_cmp(&other.width).is_eq() && self.index == other.index
    }
}
impl Eq for Priority {}
impl PartialOrd for Priority {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Priority {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.width
            .total_cmp(&other.width)
            .then(self.index.cmp(&other.index))
    }
}
fn evaluate(
    solid: &QualifiedClosedSolid,
    domains: &[crate::face_domain::FaceDomain],
    face: usize,
    d: [[f64; 2]; 2],
    reverse: bool,
    limits: VolumeLimits,
    out: &mut VolumeReport,
) -> Result<Option<I>> {
    if out.cells == limits.cells || out.spans == limits.spans {
        return Ok(None);
    }
    out.cells += 1;
    let mut algebra = limits.algebra;
    algebra.max_work = algebra.max_work.saturating_sub(out.algebra_work);
    let exact = if algebra.max_work > 0 {
        let r = nurbs_core::surface_flux::polynomial_bound(
            &solid.model.faces[face].surface,
            d,
            limits.axis,
            limits.origin,
            (limits.spans - out.spans).min(100000),
            algebra,
        )?;
        out.algebra_work += r.work;
        out.spans += r.spans;
        r.integral
    } else {
        None
    };
    if let Some(exact) = exact {
        if domains[face].contains_closed_rectangle(d) {
            let bound = I::new(exact[0], exact[1])?;
            return Ok(Some(if reverse {
                I::new(-bound.hi, -bound.lo)?
            } else {
                bound
            }));
        }
    }
    if out.spans == limits.spans {
        return Ok(None);
    }
    let flux = nurbs_core::surface_flux::bound(
        &solid.model.faces[face].surface,
        d,
        limits.axis,
        limits.origin,
        (limits.spans - out.spans).min(100000),
    )?;
    out.spans += flux.spans;
    let Some(flux) = flux.bound else {
        return Ok(None);
    };
    let area = I::point(d[0][1])
        .sub(I::point(d[0][0]))?
        .mul(I::point(d[1][1]).sub(I::point(d[1][0]))?)?;
    let mut bound = I::new(flux.density[0].min(0.), flux.density[1].max(0.))?.mul(area)?;
    if domains[face].contains_closed_rectangle(d) {
        let integral = exact.unwrap_or(flux.integral);
        bound = I::new(integral[0], integral[1])?;
    } else if out.domain_cells < limits.domain_cells {
        let placement =
            domains[face].classify(d, (limits.domain_cells - out.domain_cells).min(100000))?;
        out.domain_cells += placement.cells;
        if placement.location == Location::Inside {
            let integral = exact.unwrap_or(flux.integral);
            bound = I::new(integral[0], integral[1])?;
        }
        if placement.location == Location::Outside {
            bound = I::point(0.);
        }
    }
    Ok(Some(if reverse {
        I::new(-bound.hi, -bound.lo)?
    } else {
        bound
    }))
}
impl QualifiedClosedSolid {
    pub fn volume_bounds(&self, limits: VolumeLimits) -> Result<VolumeReport> {
        if limits.axis >= 3
            || !limits.origin.is_finite()
            || !limits.absolute_error.is_finite()
            || limits.absolute_error <= 0.
            || !(1..=1000000).contains(&limits.cells)
            || !(1..=1000000).contains(&limits.spans)
            || limits.domain_cells > 8000000
            || !(1..=10000000).contains(&limits.algebra.max_work)
            || !(1..=1000000).contains(&limits.algebra.max_terms)
            || !(64..=131072).contains(&limits.algebra.max_coefficient_bits)
        {
            return Err(Error::new(
                "BREP_INVALID_INPUT",
                "Volume flux needs bounded work, finite origin and positive error",
            ));
        }
        let mut out = VolumeReport {
            signed_bounds: None,
            positive_volume: None,
            cells: 0,
            spans: 0,
            domain_cells: 0,
            reason: "initial-volume-work-unproved",
            algebra_work: 0,
        };
        let domains = (0..self.model.faces.len())
            .map(|f| crate::face_domain::FaceDomain::new(&self.model, f, self.policy.tolerance_uv))
            .collect::<Result<Vec<_>>>()?;
        let mut reverse = vec![None; self.model.faces.len()];
        for use_ in self.model.shells.iter().flat_map(|s| &s.faces) {
            if reverse[use_.face].replace(use_.reversed).is_some() {
                return Err(Error::new(
                    "BREP_VOLUME_UNPROVED",
                    "Volume face has multiple material uses",
                ));
            }
        }
        let mut nodes = Vec::new();
        let mut roots = Vec::new();
        let mut queue = std::collections::BinaryHeap::new();
        for f in 0..self.model.faces.len() {
            let s = &self.model.faces[f].surface;
            let d = [
                [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
                [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
            ];
            let sign = reverse[f].ok_or_else(|| {
                Error::new(
                    "BREP_VOLUME_UNPROVED",
                    "Volume face is outside material shells",
                )
            })?;
            let Some(bound) = evaluate(self, &domains, f, d, sign, limits, &mut out)? else {
                return Ok(out);
            };
            let index = nodes.len();
            roots.push(index);
            queue.push(Priority {
                width: bound.hi - bound.lo,
                index,
            });
            nodes.push(Node {
                face: f,
                domain: d,
                bound,
                parent: None,
                children: None,
            });
        }
        loop {
            let mut total = I::point(0.);
            for &index in &roots {
                total = total.add(nodes[index].bound)?;
            }
            out.signed_bounds = Some([total.lo, total.hi]);
            if I::point(total.hi).sub(I::point(total.lo))?.hi <= limits.absolute_error {
                if total.lo > 0. {
                    out.positive_volume = Some([total.lo, total.hi]);
                    out.reason = "qualified-original-retained-volume";
                } else {
                    out.reason = "positive-material-volume-unproved";
                }
                return Ok(out);
            }
            if out.cells + 2 > limits.cells || out.spans == limits.spans {
                out.reason = "volume-refinement-work-unproved";
                return Ok(out);
            }
            let index = queue.pop().unwrap().index;
            let old = &nodes[index];
            let widths = old.domain.map(|d| d[1] - d[0]);
            let axis = usize::from(widths[1] > widths[0]);
            let middle = old.domain[axis][0] * 0.5 + old.domain[axis][1] * 0.5;
            if !(old.domain[axis][0] < middle && middle < old.domain[axis][1]) {
                out.reason = "volume-split-precision-unproved";
                return Ok(out);
            }
            let face = old.face;
            let mut a = old.domain;
            let mut b = a;
            a[axis][1] = middle;
            b[axis][0] = middle;
            let Some(ba) = evaluate(
                self,
                &domains,
                face,
                a,
                reverse[face].unwrap(),
                limits,
                &mut out,
            )?
            else {
                out.reason = "volume-child-work-unproved";
                return Ok(out);
            };
            let Some(bb) = evaluate(
                self,
                &domains,
                face,
                b,
                reverse[face].unwrap(),
                limits,
                &mut out,
            )?
            else {
                out.reason = "volume-child-work-unproved";
                return Ok(out);
            };
            // Atomic refinement keeps the previously complete parent bound if
            // either child lacks proof. Here both children cover the parent.
            let left = nodes.len();
            let right = left + 1;
            nodes.push(Node {
                face,
                domain: a,
                bound: ba,
                parent: Some(index),
                children: None,
            });
            nodes.push(Node {
                face,
                domain: b,
                bound: bb,
                parent: Some(index),
                children: None,
            });
            queue.push(Priority {
                width: ba.hi - ba.lo,
                index: left,
            });
            queue.push(Priority {
                width: bb.hi - bb.lo,
                index: right,
            });
            nodes[index].children = Some([left, right]);
            let mut update = Some(index);
            while let Some(i) = update {
                let [a, b] = nodes[i].children.unwrap();
                nodes[i].bound = nodes[a].bound.add(nodes[b].bound)?;
                update = nodes[i].parent;
            }
        }
    }
}
