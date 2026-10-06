//! Continuous lower qualification over every original source face pair.
//! Curved self pairs require a continuous intrinsic exclusion; otherwise they remain unresolved.
use crate::{material_wall_coverage as geometry, source_face_gap, source_volume::Body};
use nurbs_core::{Error, Result};
pub struct Limits {
    pub pairs: usize,
    pub plane_controls: usize,
    pub normal_spans: usize,
    pub gap: source_face_gap::Limits,
    pub max_sine_squared: f64,
}
pub struct Pair {
    pub faces: [usize; 2],
    pub reason: &'static str,
    pub proven: bool,
}
pub struct Certificate<'a> {
    body: &'a Body,
    minimum_mm: f64,
    max_sine_squared: f64,
    gaps: Vec<source_face_gap::Certificate<'a>>,
    self_chords: Vec<nurbs_core::surface_self_chord::Certificate<'a>>,
}
impl<'a> Certificate<'a> {
    pub fn body(&self) -> &'a Body {
        self.body
    }
    pub fn minimum_mm(&self) -> f64 {
        self.minimum_mm
    }
    pub fn max_sine_squared(&self) -> f64 {
        self.max_sine_squared
    }
    pub fn self_chord_certificates(&self) -> &[nurbs_core::surface_self_chord::Certificate<'a>] {
        &self.self_chords
    }
    pub fn gap_certificates(&self) -> &[source_face_gap::Certificate<'a>] {
        &self.gaps
    }
}
pub struct Report<'a> {
    pub certificate: Option<Certificate<'a>>,
    pub pairs: Vec<Pair>,
    pub total_pairs: usize,
    pub enumeration_complete: bool,
    pub plane_controls: usize,
    pub normal_spans: usize,
    pub cells: usize,
    pub spans: usize,
    pub reason: &'static str,
}
fn domain(s: &nurbs_core::surface::Surface) -> [[f64; 2]; 2] {
    [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ]
}
/// Proves a lower threshold for aligned material chords, including unselected
/// walls, cavities, same-face pairs and root-ended trim regions. Exclusions and
/// distance bounds use complete original charts as conservative supersets.
pub fn qualify<'a>(body: &'a Body, minimum_mm: f64, limits: Limits) -> Result<Report<'a>> {
    if !minimum_mm.is_finite()
        || minimum_mm <= 0.
        || !(1..=100000).contains(&limits.pairs)
        || !(1..=1000000).contains(&limits.plane_controls)
        || !(1..=100000).contains(&limits.normal_spans)
        || !(1..=100000).contains(&limits.gap.cells)
        || !(1..=100000).contains(&limits.gap.spans)
        || !limits.max_sine_squared.is_finite()
        || !(0. ..1.).contains(&limits.max_sine_squared)
    {
        return Err(Error::new(
            "BREP_SOURCE_WALL_COVERAGE_INPUT",
            "Choose positive whole-wall threshold and bounded continuous work",
        ));
    }
    let regions = body.geometry().shell().regions().unwrap();
    let surfaces: Vec<_> = regions.iter().map(|r| r.loops()[0][0].surface()).collect();
    let total_pairs = surfaces
        .len()
        .checked_mul(surfaces.len() + 1)
        .and_then(|n| n.checked_div(2))
        .ok_or_else(|| {
            Error::new(
                "BREP_SOURCE_WALL_COVERAGE_LIMIT",
                "Original face pair count overflow",
            )
        })?;
    let mut out = Report {
        certificate: None,
        pairs: Vec::with_capacity(total_pairs.min(limits.pairs)),
        total_pairs,
        enumeration_complete: false,
        plane_controls: 0,
        normal_spans: 0,
        cells: 0,
        spans: 0,
        reason: "source-wall-coverage-unproven",
    };
    let planes: Vec<_> = surfaces
        .iter()
        .map(|s| {
            let (p, used) = geometry::plane(s, limits.plane_controls - out.plane_controls);
            out.plane_controls += used;
            p
        })
        .collect();
    let necessary = geometry::necessary_normal_sine(limits.max_sine_squared)?;
    let mut gaps = Vec::new();
    let mut self_chords = Vec::new();
    'pairs: for a in 0..surfaces.len() {
        for b in a..surfaces.len() {
            if out.pairs.len() == limits.pairs {
                break 'pairs;
            }
            let mut pair = Pair {
                faces: [a, b],
                reason: "source-wall-pair-unproven",
                proven: false,
            };
            if a == b {
                if planes[a].is_some() && limits.max_sine_squared < 1. {
                    pair.reason = "source-wall-planar-self-excluded";
                    pair.proven = true;
                } else {
                    pair.reason = "source-wall-curved-self-unproven";
                    if out.normal_spans < limits.normal_spans {
                        let r = nurbs_core::surface_self_chord::qualify(
                            surfaces[a],
                            limits.max_sine_squared,
                            limits.normal_spans - out.normal_spans,
                        )?;
                        out.normal_spans += r.spans;
                        if let Some(certificate) = r.certificate {
                            pair.reason = "source-wall-curved-self-excluded";
                            pair.proven = true;
                            self_chords.push(certificate);
                        }
                    }
                }
            } else if matches!((planes[a],planes[b]),(Some(x),Some(y)) if geometry::same_plane(x,y))
            {
                pair.reason = "source-wall-coplanar-excluded";
                pair.proven = true;
            } else {
                if let Some(threshold) =
                    necessary.filter(|_| out.normal_spans < limits.normal_spans)
                {
                    let r = nurbs_core::normal_alignment::inspect_pair(
                        [surfaces[a], surfaces[b]],
                        [domain(surfaces[a]), domain(surfaces[b])],
                        threshold,
                        limits.normal_spans - out.normal_spans,
                    )?;
                    out.normal_spans += r.spans;
                    if r.aligned == Some(false) {
                        pair.reason = "source-wall-normal-pair-excluded";
                        pair.proven = true;
                    }
                }
                if !pair.proven && out.cells < limits.gap.cells && out.spans < limits.gap.spans {
                    let r = source_face_gap::qualify(
                        body,
                        [&[a], &[b]],
                        minimum_mm,
                        source_face_gap::Limits {
                            cells: limits.gap.cells - out.cells,
                            spans: limits.gap.spans - out.spans,
                        },
                    )?;
                    out.cells += r.cells;
                    out.spans += r.spans;
                    pair.reason = r.reason;
                    if let Some(c) = r.certificate {
                        pair.proven = true;
                        gaps.push(c);
                    }
                }
            }
            out.pairs.push(pair);
        }
    }
    out.enumeration_complete = out.pairs.len() == total_pairs;
    if out.enumeration_complete && out.pairs.iter().all(|p| p.proven) {
        out.certificate = Some(Certificate {
            body,
            minimum_mm,
            max_sine_squared: limits.max_sine_squared,
            gaps,
            self_chords,
        });
        out.reason = "source-whole-wall-lower-qualified";
    } else if !out.enumeration_complete {
        out.reason = "source-wall-coverage-pair-limit";
    }
    Ok(out)
}
