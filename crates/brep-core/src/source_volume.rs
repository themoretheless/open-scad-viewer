//! Conservative source-region flux integration on admitted embedded geometry.
use crate::{source_contour_winding, source_shell_geometry::Geometry};
use nurbs_core::{interval_eval::Interval as I, trim_domain::Location, Error, Result};
use std::{cmp::Ordering, collections::BinaryHeap};
pub struct Limits {
    pub axis: usize,
    pub origin: f64,
    pub absolute_error: f64,
    pub tolerance_uv: f64,
    pub cells: usize,
    pub spans: usize,
    pub domain_cells: usize,
}
pub struct Body {
    geometry: Geometry,
    signed_volume: [f64; 2],
    volume: [f64; 2],
    reverse_orientation: bool,
}
impl Body {
    pub fn geometry(&self) -> &Geometry {
        &self.geometry
    }
    pub fn signed_volume(&self) -> [f64; 2] {
        self.signed_volume
    }
    pub fn volume(&self) -> [f64; 2] {
        self.volume
    }
    pub fn reverse_orientation(&self) -> bool {
        self.reverse_orientation
    }
}
pub struct Report {
    pub body: Option<Body>,
    pub signed_bounds: Option<[f64; 2]>,
    pub cells: usize,
    pub spans: usize,
    pub domain_cells: usize,
    pub reason: &'static str,
    pub uncertain_face: Option<usize>,
    pub uncertain_uv: Option<[[f64; 2]; 2]>,
}
struct Node {
    face: usize,
    rectangle: [[f64; 2]; 2],
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
    fn eq(&self, b: &Self) -> bool {
        self.width.total_cmp(&b.width) == Ordering::Equal && self.index == b.index
    }
}
impl Eq for Priority {}
impl PartialOrd for Priority {
    fn partial_cmp(&self, b: &Self) -> Option<Ordering> {
        Some(self.cmp(b))
    }
}
impl Ord for Priority {
    fn cmp(&self, b: &Self) -> Ordering {
        self.width
            .total_cmp(&b.width)
            .then(self.index.cmp(&b.index))
    }
}
fn evaluate(
    geometry: &Geometry,
    face: usize,
    rectangle: [[f64; 2]; 2],
    limits: &Limits,
    out: &mut Report,
) -> Result<Option<I>> {
    out.uncertain_face = Some(face);
    out.uncertain_uv = Some(rectangle);
    if out.cells == limits.cells || out.spans == limits.spans {
        return Ok(None);
    }
    out.cells += 1;
    let region = &geometry.shell().regions().unwrap()[face];
    let surface = region.loops()[0][0].surface();
    let flux = nurbs_core::surface_flux::bound(
        surface,
        rectangle,
        limits.axis,
        limits.origin,
        (limits.spans - out.spans).min(100000),
    )?;
    out.spans += flux.spans;
    let Some(flux) = flux.bound else {
        return Ok(None);
    };
    let area = I::point(rectangle[0][1])
        .sub(I::point(rectangle[0][0]))?
        .mul(I::point(rectangle[1][1]).sub(I::point(rectangle[1][0]))?)?;
    let unknown = I::new(flux.density[0].min(0.), flux.density[1].max(0.))?.mul(area)?;
    let mut result = if region.whole_chart_material() {
        I::new(flux.integral[0], flux.integral[1])?
    } else {
        unknown
    };
    // A tiny flux can safely remain unclassified: this complete mask envelope
    // still covers every possible material subset and contributes to the error.
    let natural_area = (surface.knots_u[surface.control_points.len()]
        - surface.knots_u[surface.degree_u])
        * (surface.knots_v[surface.control_points[0].len()] - surface.knots_v[surface.degree_v]);
    // Scale the threshold by the full face area, not this leaf's area. Otherwise
    // many small unclassified leaves can retain a large total uncertainty.
    if !region.whole_chart_material()
        && (flux.density[1].max(0.) - flux.density[0].min(0.)) * natural_area
            > limits.absolute_error / (8. * geometry.shell().faces().len() as f64)
        && out.domain_cells < limits.domain_cells
    {
        let classification = source_contour_winding::classify(
            region.loops(),
            rectangle,
            limits.tolerance_uv,
            (limits.domain_cells - out.domain_cells).min(100000),
        )?;
        out.domain_cells += classification.cells;
        if classification.location == Location::Outside {
            result = I::point(0.);
        } else if classification.location == Location::Inside
            && classification.winding == Some(region.chart_winding())
        {
            result = I::new(flux.integral[0], flux.integral[1])?;
        }
    }
    Ok(Some(match region.chart_winding() {
        1 => result,
        -1 => I::new(-result.hi, -result.lo)?,
        _ => {
            return Err(Error::new(
                "BREP_SOURCE_VOLUME_ORIENTATION",
                "Source chart orientation is unqualified",
            ))
        }
    }))
}
/// The divergence of F_axis=x_axis-origin is one. All retained holes and
/// root-valued trims use original winding classification. Unknown rectangles
/// keep a complete [0,1] material-mask enclosure; no area is silently omitted.
/// Only a requested-width interval excluding zero authorizes this native body.
pub fn qualify(geometry: Geometry, limits: Limits) -> Result<Report> {
    if limits.axis >= 3
        || !limits.origin.is_finite()
        || !limits.absolute_error.is_finite()
        || limits.absolute_error <= 0.
        || !limits.tolerance_uv.is_finite()
        || limits.tolerance_uv <= 0.
        || !(1..=1000000).contains(&limits.cells)
        || !(1..=1000000).contains(&limits.spans)
        || limits.domain_cells > 8000000
    {
        return Err(Error::new(
            "BREP_SOURCE_VOLUME",
            "Choose finite flux/error inputs and bounded source-volume work",
        ));
    }
    let mut out = Report {
        body: None,
        signed_bounds: None,
        cells: 0,
        spans: 0,
        domain_cells: 0,
        reason: "source-volume-initial-work-limit",
        uncertain_face: None,
        uncertain_uv: None,
    };
    let mut nodes = Vec::new();
    let mut roots = Vec::new();
    let mut queue = BinaryHeap::new();
    for face in 0..geometry.shell().faces().len() {
        let s = geometry.shell().faces()[face][0].edges()[0].surface();
        let rectangle = [
            [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
            [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
        ];
        let Some(bound) = evaluate(&geometry, face, rectangle, &limits, &mut out)? else {
            return Ok(out);
        };
        let index = nodes.len();
        roots.push(index);
        queue.push(Priority {
            index,
            width: bound.hi - bound.lo,
        });
        nodes.push(Node {
            face,
            rectangle,
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
            if total.lo > 0. || total.hi < 0. {
                let reverse = total.hi < 0.;
                let positive = if reverse {
                    I::new(-total.hi, -total.lo)?
                } else {
                    total
                };
                out.body = Some(Body {
                    geometry,
                    signed_volume: [total.lo, total.hi],
                    volume: [positive.lo, positive.hi],
                    reverse_orientation: reverse,
                });
                out.uncertain_face = None;
                out.uncertain_uv = None;
                out.reason = "source-volume-and-orientation-qualified";
                return Ok(out);
            }
            out.reason = "source-volume-sign-unresolved";
            return Ok(out);
        }
        if let Some(next) = queue.peek() {
            out.uncertain_face = Some(nodes[next.index].face);
            out.uncertain_uv = Some(nodes[next.index].rectangle);
        }
        if out.cells + 2 > limits.cells || out.spans == limits.spans {
            out.reason = "source-volume-work-limit";
            return Ok(out);
        }
        let Some(next) = queue.pop() else {
            out.reason = "source-volume-resolution-limit";
            return Ok(out);
        };
        let p = next.index;
        let rectangle = nodes[p].rectangle;
        let face = nodes[p].face;
        let axis =
            usize::from(rectangle[1][1] - rectangle[1][0] > rectangle[0][1] - rectangle[0][0]);
        let mid = rectangle[axis][0] * 0.5 + rectangle[axis][1] * 0.5;
        if !(rectangle[axis][0] < mid && mid < rectangle[axis][1]) {
            continue;
        }
        let mut a = rectangle;
        let mut b = rectangle;
        a[axis][1] = mid;
        b[axis][0] = mid;
        let Some(left) = evaluate(&geometry, face, a, &limits, &mut out)? else {
            out.reason = "source-volume-work-limit";
            return Ok(out);
        };
        let Some(right) = evaluate(&geometry, face, b, &limits, &mut out)? else {
            out.reason = "source-volume-work-limit";
            return Ok(out);
        };
        let children = [nodes.len(), nodes.len() + 1];
        nodes[p].children = Some(children);
        nodes[p].bound = left.add(right)?;
        for (rectangle, bound, index) in [(a, left, children[0]), (b, right, children[1])] {
            nodes.push(Node {
                face,
                rectangle,
                bound,
                parent: Some(p),
                children: None,
            });
            queue.push(Priority {
                index,
                width: bound.hi - bound.lo,
            });
        }
        let mut current = p;
        while let Some(parent) = nodes[current].parent {
            let [a, b] = nodes[parent].children.unwrap();
            nodes[parent].bound = nodes[a].bound.add(nodes[b].bound)?;
            current = parent;
        }
    }
}
