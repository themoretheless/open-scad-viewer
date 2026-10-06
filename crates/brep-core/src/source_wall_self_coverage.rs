//! Original Body ownership for continuous same-face wall lower thresholds.
//! Certificates cover self pairs only; distinct face pairs remain separate.
use crate::{material_wall_coverage, source_volume::Body};
use nurbs_core::{Error, Result, surface_self_chord_coverage as adaptive};
pub struct Limits {
    pub plane_controls: usize,
    pub cells: usize,
    pub spans: usize,
    pub cells_per_face: usize,
    pub spans_per_face: usize,
}
pub struct FaceCertificate<'a> {
    body: &'a Body,
    face: usize,
    minimum_mm: f64,
    max_sine_squared: f64,
    adaptive: Option<adaptive::Certificate<'a>>,
}
impl<'a> FaceCertificate<'a> {
    pub fn body(&self) -> &'a Body {
        self.body
    }
    pub fn face(&self) -> usize {
        self.face
    }
    pub fn minimum_mm(&self) -> f64 {
        self.minimum_mm
    }
    pub fn max_sine_squared(&self) -> f64 {
        self.max_sine_squared
    }
    pub fn adaptive(&self) -> Option<&adaptive::Certificate<'a>> {
        self.adaptive.as_ref()
    }
}
pub struct Face {
    pub face: usize,
    pub qualified: bool,
    pub reason: &'static str,
    pub cells: usize,
    pub spans: usize,
    pub pending: usize,
    pub uncertain: Option<[[[f64; 2]; 2]; 2]>,
}
pub struct Report<'a> {
    pub faces: Vec<Face>,
    pub plane_controls: usize,
    pub cells: usize,
    pub spans: usize,
    certificates: Vec<FaceCertificate<'a>>,
    expected_faces: usize,
}
impl<'a> Report<'a> {
    pub fn into_certificates(self) -> Vec<FaceCertificate<'a>> {
        self.certificates
    }
    pub fn certificates(&self) -> &[FaceCertificate<'a>] {
        &self.certificates
    }
    pub fn all_self_pairs_qualified(&self) -> bool {
        self.certificates.len() == self.expected_faces
    }
}
/// Every original face receives a result. Per-face work is capped independently
/// while all counted work also respects global limits. Qualified face facts are
/// private, freshly reconstructed and bound to this immutable source Body.
pub fn qualify<'a>(
    body: &'a Body,
    minimum_mm: f64,
    max_sine_squared: f64,
    limits: Limits,
) -> Result<Report<'a>> {
    if !minimum_mm.is_finite()
        || minimum_mm <= 0.
        || !max_sine_squared.is_finite()
        || !(0. ..1.).contains(&max_sine_squared)
        || !(1..=1000000).contains(&limits.plane_controls)
        || [
            limits.cells,
            limits.spans,
            limits.cells_per_face,
            limits.spans_per_face,
        ]
        .iter()
        .any(|n| !(1..=100000).contains(n))
    {
        return Err(Error::new(
            "BREP_SOURCE_SELF_WALL_INPUT",
            "Choose positive self-wall threshold and bounded original work",
        ));
    }
    let regions = body.geometry().shell().regions().unwrap();
    let mut out = Report {
        faces: Vec::with_capacity(regions.len()),
        plane_controls: 0,
        cells: 0,
        spans: 0,
        certificates: Vec::new(),
        expected_faces: regions.len(),
    };
    for (face, region) in regions.iter().enumerate() {
        let surface = region.loops()[0][0].surface();
        let (plane, used) =
            material_wall_coverage::plane(surface, limits.plane_controls - out.plane_controls);
        out.plane_controls += used;
        if plane.is_some() {
            out.certificates.push(FaceCertificate {
                body,
                face,
                minimum_mm,
                max_sine_squared,
                adaptive: None,
            });
            out.faces.push(Face {
                face,
                qualified: true,
                reason: "source-self-wall-planar-excluded",
                cells: 0,
                spans: 0,
                pending: 0,
                uncertain: None,
            });
            continue;
        }
        let domain = [
            [
                surface.knots_u[surface.degree_u],
                surface.knots_u[surface.control_points.len()],
            ],
            [
                surface.knots_v[surface.degree_v],
                surface.knots_v[surface.control_points[0].len()],
            ],
        ];
        let cells = limits.cells_per_face.min(limits.cells - out.cells);
        let spans = limits.spans_per_face.min(limits.spans - out.spans);
        if cells == 0 || spans == 0 {
            out.faces.push(Face {
                face,
                qualified: false,
                reason: "source-self-wall-total-work-limit",
                cells: 0,
                spans: 0,
                pending: 1,
                uncertain: Some([domain, domain]),
            });
            continue;
        }
        let r = adaptive::qualify(
            surface,
            minimum_mm,
            max_sine_squared,
            adaptive::Limits { cells, spans },
        )?;
        out.cells += r.cells;
        out.spans += r.spans;
        let qualified = r.certificate.is_some();
        let reason = if qualified {
            "source-self-wall-qualified"
        } else if r.reason == "self-coverage-resolution-limit" {
            "source-self-wall-resolution-limit"
        } else {
            "source-self-wall-work-limit"
        };
        out.faces.push(Face {
            face,
            qualified,
            reason,
            cells: r.cells,
            spans: r.spans,
            pending: r.pending,
            uncertain: r.uncertain,
        });
        if let Some(c) = r.certificate {
            out.certificates.push(FaceCertificate {
                body,
                face,
                minimum_mm,
                max_sine_squared,
                adaptive: Some(c),
            });
        }
    }
    Ok(out)
}
