//! Reusable, budgeted original anchor paths for one unchanged Body face pair.
//! Product direction proofs never establish complete pair or wall coverage.
use crate::source_volume::Body;
use nurbs_core::{Error, Result, rational_pole_differential as polar, surface_self_chord as chord};

pub struct PreparedPair<'a, E = crate::source_shared_edge::SharedEdge> {
    body: &'a Body<E>,
    faces: [usize; 2],
    anchors: [chord::SurfaceAnchorCertificate<'a>; 2],
    differential: [Option<polar::Certificate<'a>>; 2],
}
pub struct Preparation<'a, E = crate::source_shared_edge::SharedEdge> {
    pub prepared: Option<PreparedPair<'a, E>>,
    pub exact_work: u64,
}
pub struct ProductCertificate<'a, E = crate::source_shared_edge::SharedEdge> {
    body: &'a Body<E>,
    faces: [usize; 2],
    rectangles: [[[f64; 2]; 2]; 2],
    max_sine_squared: f64,
}
impl<'a, E: crate::source_seam_tangency::Carrier> ProductCertificate<'a, E> {
    pub fn body(&self) -> &'a Body<E> {
        self.body
    }
    pub fn faces(&self) -> [usize; 2] {
        self.faces
    }
    pub fn rectangles(&self) -> [[[f64; 2]; 2]; 2] {
        self.rectangles
    }
    pub fn max_sine_squared(&self) -> f64 {
        self.max_sine_squared
    }
}
/// UV and world inputs are proposals; exact composition on both original
/// surfaces must prove their common identity before preparing any paths.
pub fn prepare<'a, E: crate::source_seam_tangency::Carrier>(
    body: &'a Body<E>,
    faces: [usize; 2],
    uv: [[f64; 2]; 2],
    world: [f64; 3],
    max_work: u64,
) -> Result<Preparation<'a, E>> {
    let regions = body.geometry().shell().regions().ok_or_else(|| {
        Error::new(
            "BREP_SOURCE_WALL_ANCHOR",
            "Require original retained regions",
        )
    })?;
    if faces[0] == faces[1] || faces.iter().any(|&f| f >= regions.len()) || max_work > 100000000 {
        return Err(Error::new(
            "BREP_SOURCE_WALL_ANCHOR",
            "Choose distinct owned faces and bounded exact work",
        ));
    }
    let sources = faces.map(|f| regions[f].loops()[0][0].surface());
    let mut used = 0;
    let mut anchors = Vec::new();
    for side in 0..2 {
        let (anchor, work) =
            chord::qualify_surface_anchor(sources[side], uv[side], world, max_work - used)?;
        used += work;
        let Some(anchor) = anchor else {
            return Ok(Preparation {
                prepared: None,
                exact_work: used,
            });
        };
        anchors.push(anchor);
    }
    let mut differential = [None, None];
    for side in 0..2 {
        if used == max_work {
            break;
        }
        let s = sources[side];
        let low = s.knots_u[s.degree_u];
        let high = s.knots_u[s.control_points.len()];
        let end = if uv[side][0] == low {
            false
        } else if uv[side][0] == high {
            true
        } else {
            continue;
        };
        match polar::prepare(s, end, max_work - used) {
            Ok(r) => {
                used += r.work;
                differential[side] = r.certificate;
            }
            Err(e) if e.code == "NURBS_INVALID_INPUT" => {}
            Err(e) if e.code == "NURBS_NUMERIC_ERROR" => {
                used = max_work;
                break;
            }
            Err(e) => return Err(e),
        }
    }
    let anchors = anchors.try_into().ok().expect("two original anchors");
    Ok(Preparation {
        prepared: Some(PreparedPair {
            body,
            faces,
            anchors,
            differential,
        }),
        exact_work: used,
    })
}
/// Authored pcurve ends propose anchors; evaluated coincidence is only a
/// candidate filter. Exact source identities remain mandatory on both sides.
pub fn prepare_boundary_anchor<'a, E: crate::source_seam_tangency::Carrier>(
    body: &'a Body<E>,
    faces: [usize; 2],
    max_work: u64,
) -> Result<Preparation<'a, E>> {
    let regions = body.geometry().shell().regions().ok_or_else(|| {
        Error::new(
            "BREP_SOURCE_WALL_ANCHOR",
            "Require original retained regions",
        )
    })?;
    if faces[0] == faces[1] || faces.iter().any(|&f| f >= regions.len()) || max_work > 100000000 {
        return Err(Error::new(
            "BREP_SOURCE_WALL_ANCHOR",
            "Choose distinct owned faces and bounded exact work",
        ));
    }
    let points = faces.map(|f| {
        let mut points = Vec::new();
        for edge in regions[f].loops().iter().flatten() {
            for p in [
                edge.curve().control_points.first(),
                edge.curve().control_points.last(),
            ]
            .into_iter()
            .flatten()
            {
                if p.len() == 2 {
                    let uv = [p[0], p[1]];
                    if !points.contains(&uv) && points.len() < 16 {
                        points.push(uv);
                    }
                }
            }
        }
        points
    });
    let sources = faces.map(|f| regions[f].loops()[0][0].surface());
    let mut used = 0;
    for a in &points[0] {
        for b in &points[1] {
            if used == max_work {
                return Ok(Preparation {
                    prepared: None,
                    exact_work: used,
                });
            }
            let Ok(first) = sources[0].evaluate(a[0], a[1]) else {
                continue;
            };
            let Ok(second) = sources[1].evaluate(b[0], b[1]) else {
                continue;
            };
            if first.point != second.point || first.point.len() != 3 {
                continue;
            }
            let world = [first.point[0], first.point[1], first.point[2]];
            let result = prepare(body, faces, [*a, *b], world, max_work - used)?;
            used += result.exact_work;
            if result.prepared.is_some() {
                return Ok(Preparation {
                    prepared: result.prepared,
                    exact_work: used,
                });
            }
        }
    }
    Ok(Preparation {
        prepared: None,
        exact_work: used,
    })
}
impl<'a, E: crate::source_seam_tangency::Carrier> PreparedPair<'a, E> {
    pub(crate) fn matches(&self, body: &Body<E>, faces: [usize; 2]) -> bool {
        std::ptr::eq(self.body, body) && self.faces == faces
    }
    /// All reusable facts remain attached to this immutable original Body.
    /// Query work includes path restrictions, endpoint enclosures and normals.
    pub fn qualify_product_projections(
        &self,
        rectangles: [[[f64; 2]; 2]; 2],
        projections: &[[f64; 3]],
        sine: f64,
        max_spans: usize,
    ) -> Result<(Option<ProductCertificate<'a, E>>, usize)> {
        self.qualify_product_projections_with_normals(
            rectangles,
            projections,
            sine,
            max_spans,
            [None, None],
        )
    }
    /// The caller charges each supplied normal factor before this query.
    /// Native validation keeps factors attached to their original source/pole.
    pub fn qualify_product_projections_with_normals(
        &self,
        rectangles: [[[f64; 2]; 2]; 2],
        projections: &[[f64; 3]],
        sine: f64,
        max_spans: usize,
        normals: [Option<&nurbs_core::factored_surface_normals::Certificate<'_>>; 2],
    ) -> Result<(Option<ProductCertificate<'a, E>>, usize)> {
        let (proof, used) = chord::qualify_polar_anchored_chords_with_projections(
            self.anchors.each_ref(),
            rectangles,
            self.differential.each_ref().map(Option::as_ref),
            normals,
            projections,
            sine,
            max_spans,
        )?;
        Ok((
            proof.map(|_| ProductCertificate {
                body: self.body,
                faces: self.faces,
                rectangles,
                max_sine_squared: sine,
            }),
            used,
        ))
    }
    pub fn qualify_product(
        &self,
        rectangles: [[[f64; 2]; 2]; 2],
        projection: [f64; 3],
        sine: f64,
        max_spans: usize,
    ) -> Result<(Option<ProductCertificate<'a, E>>, usize)> {
        let (proof, used) = chord::qualify_polar_anchored_chords(
            self.anchors.each_ref(),
            rectangles,
            self.differential.each_ref().map(Option::as_ref),
            [None, None],
            projection,
            sine,
            max_spans,
        )?;
        Ok((
            proof.map(|_| ProductCertificate {
                body: self.body,
                faces: self.faces,
                rectangles,
                max_sine_squared: sine,
            }),
            used,
        ))
    }
}
