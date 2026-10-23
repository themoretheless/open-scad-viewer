//! Exact source restrictions of a privately qualified canonical world edge.
//! Root definitions and affine maps remain authoritative; bounds are for display.
use crate::{source_shared_edge::SharedEdge, source_shared_edge_restore};
use nurbs_core::{
    Error, Result,
    interval_eval::{self, Interval as I},
};
use value_codec::Value;
#[derive(Clone)]
pub struct Restriction {
    edge: SharedEdge,
}
impl Restriction {
    pub fn from_edge(edge: &SharedEdge) -> Self {
        Self { edge: edge.clone() }
    }
    pub fn edge(&self) -> &SharedEdge {
        &self.edge
    }
    /// Persistence contains original carriers, roots and mapping proposals.
    /// It contains no rounded parameter, vertex or cached admission authority.
    pub fn definition(&self) -> Value {
        self.edge.definition()
    }
    /// Both directed source uses are transported into increasing canonical
    /// parameter order. Their intersection encloses the proven common ends.
    pub fn parameter_bounds(&self) -> Result<[[f64; 2]; 2]> {
        let target = self.edge.world().domain();
        let mut bounds = [[[0.; 2]; 2]; 2];
        for (slot, fragment) in self.edge.uses().iter().enumerate() {
            let source = fragment.curve().domain();
            let flip = self.edge.reversed()[slot] ^ fragment.reversed();
            let range = self.edge.ranges().map(|r| r[slot]).unwrap_or(if flip {
                [[1., 1.], [0., 1.]]
            } else {
                [[0., 1.], [1., 1.]]
            });
            let first = I::point(range[0][0]).div(I::point(range[0][1]))?;
            let last = I::point(range[1][0]).div(I::point(range[1][1]))?;
            for (end, r) in fragment.parameter_bounds().iter().enumerate() {
                let q = I::new(r[0], r[1])?
                    .sub(I::point(source[0]))?
                    .div(I::point(source[1]).sub(I::point(source[0]))?)?
                    .intersect(0., 1.)?;
                let t = I::point(target[0])
                    .add(
                        first
                            .add(q.mul(last.sub(first)?)?)?
                            .intersect(0., 1.)?
                            .mul(I::point(target[1]).sub(I::point(target[0]))?)?,
                    )?
                    .intersect(target[0], target[1])?;
                bounds[slot][if self.edge.reversed()[slot] {
                    1 - end
                } else {
                    end
                }] = [t.lo, t.hi];
            }
        }
        let common = std::array::from_fn(|i| {
            [
                bounds[0][i][0].max(bounds[1][i][0]),
                bounds[0][i][1].min(bounds[1][i][1]),
            ]
        });
        if common.iter().any(|b| b[0] > b[1]) {
            return Err(Error::new(
                "BREP_SOURCE_RESTRICTION_BOUNDS",
                "Qualified endpoint enclosures are disjoint",
            ));
        }
        Ok(common)
    }
    /// Display samples only: endpoint root enclosures remain authoritative.
    /// Each segment box encloses the original carrier over the sampled interval;
    /// the polyline itself is not a geometric admission or chord-error certificate.
    pub fn display_segments(
        &self,
        segments: usize,
    ) -> Result<Vec<([f64; 2], [[f64; 3]; 2], [[f64; 2]; 3])>> {
        if !(1..=4096).contains(&segments) {
            return Err(Error::new(
                "BREP_SOURCE_DISPLAY_WORK",
                "Choose 1..4096 display segments",
            ));
        }
        let ends = self.parameter_bounds()?;
        let start = ends[0][0];
        let stop = ends[1][1];
        if start >= stop {
            return Err(Error::new(
                "BREP_SOURCE_DISPLAY_DOMAIN",
                "Display restriction needs an ordered interval",
            ));
        }
        let curve = self.edge.world();
        let mut output = Vec::with_capacity(segments);
        for i in 0..segments {
            let t = |j: usize| {
                if j == segments {
                    stop
                } else {
                    start + (stop - start) * (j as f64 / segments as f64)
                }
            };
            let range = [t(i), t(i + 1)];
            let evaluated = [curve.evaluate(range[0])?, curve.evaluate(range[1])?];
            let enclosure = interval_eval::evaluate_interval(curve, I::new(range[0], range[1])?)?;
            if enclosure.len() != 3 || evaluated.iter().any(|p| p.point.len() != 3) {
                return Err(Error::new(
                    "BREP_SOURCE_RESTRICTION_DIMENSION",
                    "Canonical carrier must be 3D",
                ));
            }
            output.push((
                range,
                evaluated.map(|p| std::array::from_fn(|a| p.point[a])),
                std::array::from_fn(|a| [enclosure[a].lo, enclosure[a].hi]),
            ));
        }
        Ok(output)
    }
    /// Enclose each canonical end, without choosing a point in its root box.
    /// Work counts all original nonempty knot spans, independently per end.
    pub fn endpoint_boxes(&self, max_spans: usize) -> Result<[[[f64; 2]; 3]; 2]> {
        let c = self.edge.world();
        let spans = (c.degree..c.control_points.len())
            .filter(|&i| c.knots[i] < c.knots[i + 1])
            .count();
        if max_spans == 0 || max_spans > 100000 || spans > max_spans {
            return Err(Error::new(
                "BREP_SOURCE_RESTRICTION_WORK",
                "Bound canonical endpoint enclosure work",
            ));
        }
        let bounds = self.parameter_bounds()?;
        let mut boxes = [[[0.; 2]; 3]; 2];
        for end in 0..2 {
            let p = interval_eval::evaluate_interval(c, I::new(bounds[end][0], bounds[end][1])?)?;
            if p.len() != 3 {
                return Err(Error::new(
                    "BREP_SOURCE_RESTRICTION_DIMENSION",
                    "Canonical carrier must be 3D",
                ));
            }
            for axis in 0..3 {
                boxes[end][axis] = [p[axis].lo, p[axis].hi];
            }
        }
        Ok(boxes)
    }
}
/// Restoration reruns the original cross-face identity and endpoint gates.
pub fn restore(
    value: Value,
    limits: source_shared_edge_restore::Limits,
) -> Result<Option<Restriction>> {
    Ok(source_shared_edge_restore::restore(value, limits)?
        .edge
        .map(|edge| Restriction { edge }))
}
#[cfg(test)]
pub(crate) fn assert_replay(edge: &SharedEdge) {
    let restriction = Restriction::from_edge(edge);
    let bytes = value_codec::to_string(&restriction.definition()).unwrap();
    let recovered = restore(
        value_codec::from_str(&bytes).unwrap(),
        source_shared_edge_restore::Limits {
            mapping_cells_per_use: 10000,
            exact_work: 100_000_000,
            driver_cells: 10000,
        },
    )
    .unwrap()
    .unwrap();
    assert_eq!(restriction.definition(), recovered.definition());
    assert_eq!(
        restriction.parameter_bounds().unwrap(),
        recovered.parameter_bounds().unwrap()
    );
    assert_eq!(
        restriction.endpoint_boxes(10000).unwrap(),
        recovered.endpoint_boxes(10000).unwrap()
    );
    assert!(restriction.endpoint_boxes(0).is_err());
    assert!(restriction.display_segments(0).is_err());
    assert!(restriction.display_segments(4097).is_err());
    let display = restriction.display_segments(16).unwrap();
    assert_eq!(display.len(), 16);
    assert_eq!(display, recovered.display_segments(16).unwrap());
    assert_eq!(
        display[0].0[0],
        restriction.parameter_bounds().unwrap()[0][0]
    );
    assert_eq!(
        display[15].0[1],
        restriction.parameter_bounds().unwrap()[1][1]
    );
    for pair in display.windows(2) {
        assert_eq!(pair[0].0[1], pair[1].0[0]);
        assert_eq!(pair[0].1[1], pair[1].1[0]);
    }
    for (range, _, bounds) in &display {
        for i in 0..=8 {
            let t = range[0] + (range[1] - range[0]) * i as f64 / 8.;
            let p = restriction.edge.world().evaluate(t).unwrap().point;
            for axis in 0..3 {
                assert!(bounds[axis][0] <= p[axis] && p[axis] <= bounds[axis][1]);
            }
        }
    }
}
