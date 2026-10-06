//! Fresh material chord admission on immutable embedded original source Bodies.
//! A chord is local evidence and never certifies minimum whole-wall thickness.
use crate::{material_segment::SegmentReport, source_material_segment, source_volume::Body};
use nurbs_core::{
    Error, Result, normal_alignment,
    surface_distance::{enclosure_distance, rectangle_bounds},
};
pub struct Limits {
    pub cells: usize,
    pub domain_cells: usize,
    pub normal_spans: usize,
    pub max_sine_squared: f64,
}
pub struct Certificate<'a> {
    body: &'a Body,
    faces: [usize; 2],
    length_mm: [f64; 2],
}
impl<'a> Certificate<'a> {
    pub fn body(&self) -> &'a Body {
        self.body
    }
    pub fn faces(&self) -> [usize; 2] {
        self.faces
    }
    pub fn length_mm(&self) -> [f64; 2] {
        self.length_mm
    }
}
pub struct Report<'a> {
    pub certificate: Option<Certificate<'a>>,
    pub boundary: SegmentReport,
    pub normals: [Option<normal_alignment::Report>; 2],
    pub reason: &'static str,
}
/// Recompute all evidence. A strict exterior source-chart hull seed and exactly
/// two isolated transverse roots on an embedded closed Body prove material
/// between the roots. Both original endpoint rectangles must align with line.
pub fn qualify<'a>(
    body: &'a Body,
    origin: [f64; 3],
    direction: [f64; 3],
    tolerance_uv: f64,
    limits: Limits,
) -> Result<Report<'a>> {
    qualify_impl(body, origin, direction, tolerance_uv, limits, None)
}
/// Complete outside-seeded crossing parity permits multiple material intervals
/// separated by cavities. Only an entering/exiting pair in the requested groups
/// can supply a certificate; every crossing remains in the fresh boundary audit.
pub fn qualify_between<'a>(
    body: &'a Body,
    groups: [&[usize]; 2],
    origin: [f64; 3],
    direction: [f64; 3],
    tolerance_uv: f64,
    limits: Limits,
) -> Result<Report<'a>> {
    let count = body.geometry().shell().faces().len();
    if groups.iter().any(|g| {
        g.is_empty()
            || g.iter()
                .enumerate()
                .any(|(i, f)| *f >= count || g[..i].contains(f))
    }) || groups[0].iter().any(|f| groups[1].contains(f))
    {
        return Err(Error::new(
            "BREP_SOURCE_CHORD_GROUPS",
            "Choose disjoint owned original face groups",
        ));
    }
    qualify_impl(body, origin, direction, tolerance_uv, limits, Some(groups))
}
fn qualify_impl<'a>(
    body: &'a Body,
    origin: [f64; 3],
    direction: [f64; 3],
    tolerance_uv: f64,
    limits: Limits,
    groups: Option<[&[usize]; 2]>,
) -> Result<Report<'a>> {
    if !(1..=100000).contains(&limits.normal_spans)
        || !limits.max_sine_squared.is_finite()
        || !(0. ..1.).contains(&limits.max_sine_squared)
    {
        return Err(Error::new(
            "BREP_SOURCE_CHORD_LIMITS",
            "Choose bounded normal work and squared sine tolerance in [0,1)",
        ));
    }
    let boundary = source_material_segment::inspect_boundary(
        body,
        origin,
        direction,
        tolerance_uv,
        limits.cells,
        limits.domain_cells,
    )?;
    let mut out = Report {
        certificate: None,
        boundary,
        normals: [None, None],
        reason: "seed-unresolved",
    };
    let regions = body.geometry().shell().regions().unwrap();
    let mut hull = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
    for region in regions {
        let s = region.loops()[0][0].surface();
        let uv = [
            [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
            [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
        ];
        let b = rectangle_bounds(s, uv)?;
        for k in 0..3 {
            hull[k][0] = hull[k][0].min(b[k][0]);
            hull[k][1] = hull[k][1].max(b[k][1]);
        }
    }
    if !(0..3).any(|k| origin[k] < hull[k][0] || origin[k] > hull[k][1]) {
        return Ok(out);
    }
    if !out.boundary.unresolved.is_empty() {
        out.reason = "boundary-unresolved";
        return Ok(out);
    }
    if out.boundary.contacts.len() < 2
        || out.boundary.contacts.len() % 2 != 0
        || (groups.is_none() && out.boundary.contacts.len() != 2)
    {
        out.reason = "requires-two-crossings";
        return Ok(out);
    }
    out.boundary
        .contacts
        .sort_by(|a, b| a.parameter[0].total_cmp(&b.parameter[0]));
    if out
        .boundary
        .contacts
        .windows(2)
        .any(|w| w[0].parameter[1] >= w[1].parameter[0])
    {
        out.reason = "overlapping-root-intervals";
        return Ok(out);
    }
    let pair = if let Some(g) = groups {
        (0..out.boundary.contacts.len()).step_by(2).find(|&i| {
            let f = [
                out.boundary.contacts[i].face,
                out.boundary.contacts[i + 1].face,
            ];
            g[0].contains(&f[0]) && g[1].contains(&f[1])
                || g[0].contains(&f[1]) && g[1].contains(&f[0])
        })
    } else {
        Some(0)
    };
    let Some(pair) = pair else {
        out.reason = "material-pair-outside-groups";
        return Ok(out);
    };
    let a = &out.boundary.contacts[pair];
    let b = &out.boundary.contacts[pair + 1];
    let faces = [a.face, b.face];
    let mut points = Vec::new();
    let mut spans = 0;
    for i in 0..2 {
        let c = &out.boundary.contacts[pair + i];
        let s = regions[c.face].loops()[0][0].surface();
        points.push(rectangle_bounds(s, c.uv)?);
        if spans == limits.normal_spans {
            out.reason = "normal-work-limit";
            return Ok(out);
        }
        let r = normal_alignment::inspect(
            s,
            c.uv,
            direction,
            limits.max_sine_squared,
            limits.normal_spans - spans,
        )?;
        spans += r.spans;
        let aligned = r.aligned;
        out.normals[i] = Some(r);
        if aligned != Some(true) {
            out.reason = "normal-alignment-unproven";
            return Ok(out);
        }
    }
    let (lo, hi) = enclosure_distance(&points[0], &points[1])?;
    out.certificate = Some(Certificate {
        body,
        faces,
        length_mm: [lo, hi],
    });
    out.reason = "source-material-normal-chord-qualified";
    Ok(out)
}
