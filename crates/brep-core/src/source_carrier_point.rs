//! Transport a certified original UV root through one privately admitted shared
//! carrier. Enclosures never replace the root expression by a fixed parameter.
use crate::{source_contact_point::SourcePoint, source_shared_edge::SharedEdge};
use nurbs_core::{Error, Result, interval_eval::Interval as I};
#[derive(Clone)]
pub struct Point {
    edge: SharedEdge,
    root: SourcePoint,
    source_side: usize,
    canonical: [f64; 2],
    parameters: [[f64; 2]; 2],
    uv: [[[f64; 2]; 2]; 2],
    world: [[f64; 2]; 3],
}
impl Point {
    pub fn edge(&self) -> &SharedEdge {
        &self.edge
    }
    pub fn root(&self) -> &SourcePoint {
        &self.root
    }
    pub fn source_side(&self) -> usize {
        self.source_side
    }
    pub fn canonical_bounds(&self) -> [f64; 2] {
        self.canonical
    }
    pub fn parameter_bounds(&self) -> [[f64; 2]; 2] {
        self.parameters
    }
    pub fn uv_bounds(&self) -> [[[f64; 2]; 2]; 2] {
        self.uv
    }
    pub fn world_bounds(&self) -> [[f64; 2]; 3] {
        self.world
    }
    /// Original definitions only. Cached coordinates and flags grant no authority.
    pub fn definition(&self) -> value_codec::Value {
        value_codec::json!({"version":1,"kind":"original-carrier-root","edge":self.edge.definition(),
            "root":self.root.definition(),"sourceSide":self.source_side})
    }
}
/// Exact restriction of an original incident use. The transported root stays
/// attached to its admitted carrier; parameter bounds are only enclosures.
/// This view does not admit a new shared edge or sewn body.
pub struct Restriction<'a> {
    point: &'a Point,
    side: usize,
    half: usize,
}
impl Point {
    /// Partition either incident use in its original traversal direction.
    /// Both returned restrictions share the same root expression.
    pub fn retained_parts(&self, side: usize) -> Result<[Restriction<'_>; 2]> {
        if side > 1 {
            return Err(Error::new(
                "BREP_CARRIER_ROOT",
                "Choose an original incident use",
            ));
        }
        Ok(std::array::from_fn(|half| Restriction {
            point: self,
            side,
            half,
        }))
    }
}
impl Restriction<'_> {
    pub fn source(&self) -> &crate::source_boundary_fragment::Fragment {
        &self.point.edge.uses()[self.side]
    }
    pub fn root(&self) -> &Point {
        self.point
    }
    pub fn side(&self) -> usize {
        self.side
    }
    pub fn parameter_bounds(&self) -> [[f64; 2]; 2] {
        let ends = self.source().parameter_bounds();
        let cut = self.point.parameters[self.side];
        if self.half == 0 {
            [ends[0], cut]
        } else {
            [cut, ends[1]]
        }
    }
    /// Source identity and traversal, never overlapping coordinate boxes.
    pub fn joins(&self, next: &Restriction<'_>) -> bool {
        self.half == 0
            && next.half == 1
            && self.side == next.side
            && self.point.definition() == next.point.definition()
    }
    pub fn definition(&self) -> value_codec::Value {
        value_codec::json!({"version":1,"kind":"carrier-root-restriction",
            "point":self.point.definition(),"side":self.side,"half":self.half})
    }
    /// Conservative full-source enclosure on the restricted parameter interval.
    /// This is positional evidence only; it cannot authorize endpoint welding.
    pub fn enclose(&self, max_spans: usize) -> Result<RestrictionEnclosure> {
        if max_spans > 100000 {
            return Err(Error::new(
                "BREP_CARRIER_ROOT",
                "Bound restriction span work",
            ));
        }
        let ends = self.parameter_bounds();
        let t = I::new(ends[0][0].min(ends[1][0]), ends[0][1].max(ends[1][1]))?;
        let curve = self.source().curve();
        let spans = (curve.degree..curve.control_points.len())
            .filter(|&i| {
                curve.knots[i] < curve.knots[i + 1]
                    && curve.knots[i] <= t.hi
                    && curve.knots[i + 1] >= t.lo
            })
            .count();
        if spans > max_spans {
            return Ok(RestrictionEnclosure {
                uv: None,
                spans: 0,
                complete: false,
            });
        }
        let uv = nurbs_core::interval_eval::evaluate_interval(curve, t)?;
        Ok(RestrictionEnclosure {
            uv: Some(std::array::from_fn(|k| [uv[k].lo, uv[k].hi])),
            spans,
            complete: true,
        })
    }
}
pub struct RestrictionEnclosure {
    pub uv: Option<[[f64; 2]; 2]>,
    pub spans: usize,
    pub complete: bool,
}
/// Original parent correspondence proves a transported endpoint identity only
/// under unchanged world geometry, incident source definitions and affine maps.
/// Fresh whole-source composition remains the caller's responsibility.
pub(crate) fn same_mapped_endpoint(
    world: &nurbs_core::curve::Curve,
    fragments: [&crate::source_boundary_fragment::Fragment; 2],
    ranges: [[[f64; 2]; 2]; 2],
    ends: [&crate::source_boundary_fragment::Endpoint; 2],
) -> bool {
    use crate::source_boundary_fragment::{Endpoint, Role};
    let (carrier, own, other) = match ends {
        [Endpoint::Carrier { point, side }, _] => (point.as_ref(), *side, 1),
        [_, Endpoint::Carrier { point, side }] => (point.as_ref(), *side, 0),
        _ => return false,
    };
    if carrier.edge.world() != world {
        return false;
    }
    let own_index = 1 - other;
    let other_side = match ends[other] {
        Endpoint::Carrier { point, side } if point.definition() == carrier.definition() => *side,
        Endpoint::Crossing {
            point,
            role: Role::Boundary,
        } if point.definition() == carrier.root.definition() => carrier.source_side,
        _ => return false,
    };
    if own == other_side {
        return false;
    }
    let sides = if own_index == 0 {
        [own, other_side]
    } else {
        [other_side, own]
    };
    let original_ranges = crate::source_seam_tangency::ranges(&carrier.edge);
    (0..2).all(|i| {
        let source = &carrier.edge.uses()[sides[i]];
        source.surface() == fragments[i].surface()
            && source.curve() == fragments[i].curve()
            && ranges[i] == original_ranges[sides[i]]
    })
}
pub struct Report {
    pub point: Option<Point>,
    pub spans: usize,
    pub reason: &'static str,
}
/// Full original carrier maps are required. Actual retained ends must strictly
/// contain the new root on both incident uses; enclosing an old end is refused.
pub fn qualify(
    edge: &SharedEdge,
    root: &SourcePoint,
    side: usize,
    max_spans: usize,
) -> Result<Report> {
    if side > 1 || max_spans > 100000 {
        return Err(Error::new(
            "BREP_CARRIER_ROOT",
            "Choose an original side and bounded carrier work",
        ));
    }
    let mut out = Report {
        point: None,
        spans: 0,
        reason: "carrier-root-source-unproven",
    };
    if edge.uses().iter().any(|u| {
        u.endpoints()
            .iter()
            .any(|e| matches!(e, crate::source_boundary_fragment::Endpoint::Carrier { .. }))
    }) {
        out.reason = "carrier-root-nested-parent-refused";
        return Ok(out);
    }
    let source = &edge.uses()[side];
    if !edge.maps_complete_canonical_source()
        || root.surface() != source.surface()
        || root.boundary() != source.curve()
    {
        return Ok(out);
    }
    let r = root.root_parameters()[0];
    let canonical =
        crate::source_seam_tangency::map(edge, side, I::new(r[0], r[1])?)?.intersect(0., 1.)?;
    let mut parameters = [[0.; 2]; 2];
    for current in 0..2 {
        let t = crate::source_seam_tangency::inverse(edge, current, canonical)?;
        let ends = edge.uses()[current].parameter_bounds();
        if t.lo <= ends[0][1].min(ends[1][1]) || t.hi >= ends[0][0].max(ends[1][0]) {
            out.reason = "carrier-root-retained-order-unproven";
            return Ok(out);
        }
        parameters[current] = [t.lo, t.hi];
    }
    let d = edge.world().domain();
    let world_parameter = I::point(d[0])
        .add(canonical.mul(I::point(d[1]).sub(I::point(d[0]))?)?)?
        .intersect(d[0], d[1])?;
    let queries = [
        (
            edge.uses()[0].curve(),
            I::new(parameters[0][0], parameters[0][1])?,
        ),
        (
            edge.uses()[1].curve(),
            I::new(parameters[1][0], parameters[1][1])?,
        ),
        (edge.world(), world_parameter),
    ];
    let mut values = Vec::new();
    for (curve, t) in queries {
        let spent = (curve.degree..curve.control_points.len())
            .filter(|&i| {
                curve.knots[i] < curve.knots[i + 1]
                    && curve.knots[i] <= t.hi
                    && curve.knots[i + 1] >= t.lo
            })
            .count();
        if spent > max_spans - out.spans {
            out.reason = "carrier-root-work-limit";
            return Ok(out);
        }
        out.spans += spent;
        values.push(nurbs_core::interval_eval::evaluate_interval(curve, t)?);
    }
    out.point = Some(Point {
        edge: edge.clone(),
        root: root.clone(),
        source_side: side,
        canonical: [canonical.lo, canonical.hi],
        parameters,
        uv: std::array::from_fn(|s| std::array::from_fn(|k| [values[s][k].lo, values[s][k].hi])),
        world: std::array::from_fn(|k| [values[2][k].lo, values[2][k].hi]),
    });
    out.reason = "carrier-root-transport-qualified";
    Ok(out)
}
pub struct RestoreLimits {
    pub edge: crate::source_shared_edge_restore::Limits,
    pub root_mapping_cells: usize,
    pub spans: usize,
}
pub fn restore(value: value_codec::Value, limits: RestoreLimits) -> Result<Report> {
    if value["version"].as_u64() != Some(1)
        || value["kind"].as_str() != Some("original-carrier-root")
    {
        return Err(Error::new(
            "BREP_CARRIER_ROOT",
            "Unsupported original carrier root definition",
        ));
    }
    // Inspect direct parent endpoint kinds before any recursive restoration.
    let uses = value["edge"]["uses"]
        .as_array()
        .filter(|u| u.len() == 2)
        .ok_or_else(|| Error::new("BREP_CARRIER_ROOT", "Require two original parent uses"))?;
    for original in uses {
        let ends = original["endpoints"]
            .as_array()
            .filter(|e| e.len() == 2)
            .ok_or_else(|| Error::new("BREP_CARRIER_ROOT", "Require original parent endpoints"))?;
        if ends
            .iter()
            .any(|e| !matches!(e["kind"].as_str(), Some("parameter" | "crossing")))
        {
            return Err(Error::new(
                "BREP_CARRIER_ROOT",
                "Nested carrier parents are unsupported",
            ));
        }
    }
    let side = value["sourceSide"]
        .as_u64()
        .filter(|s| *s <= 1)
        .ok_or_else(|| Error::new("BREP_CARRIER_ROOT", "Require an original source side"))?
        as usize;
    let edge = crate::source_shared_edge_restore::restore(value["edge"].clone(), limits.edge)?;
    let Some(edge) = edge.edge else {
        return Ok(Report {
            point: None,
            spans: 0,
            reason: edge.reason,
        });
    };
    let root =
        crate::source_contact_point::restore(value["root"].clone(), limits.root_mapping_cells)?;
    let Some(root) = root.point else {
        return Ok(Report {
            point: None,
            spans: 0,
            reason: root.reason,
        });
    };
    qualify(&edge, &root, side, limits.spans)
}
