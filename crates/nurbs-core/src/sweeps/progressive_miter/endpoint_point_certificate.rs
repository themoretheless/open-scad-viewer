//! Enclosures of the authored endpoint affine maps, including all frame/law
//! sources. Domain coverage and association with retained faces are separate.
use super::{Sweep, frame_certificate::VectorEnclosure, scalar_certificate::Status};
use crate::{Result, check, curve::Curve};

pub struct Report {
    pub points: Option<Vec<[VectorEnclosure; 2]>>,
    pub cells: usize,
    pub reason: Option<&'static str>,
}
impl Sweep<'_> {
    /// Borrow the original profiles owned by this sweep's certificate source.
    pub fn source_profiles(&self) -> &[Curve] {
        self.profiles
    }

    /// Enclose both ideal endpoint images of each original-space point. The
    /// map is affine in the point: this also applies to exact Bezier extraction
    /// coefficients, without treating a rounded stored station as the source.
    /// `max_cells` is shared across all points and both endpoints; `law_cells`
    /// is the existing per-station scalar/frame allowance.
    pub fn certify_endpoint_points(
        &self,
        points: &[[f64; 3]],
        max_cells: usize,
        law_cells: usize,
    ) -> Result<Report> {
        check(
            points.len() <= 4096
                && points.iter().flatten().all(|v| v.is_finite())
                && max_cells <= 100_000
                && law_cells <= 100_000,
            "Invalid endpoint point certificate input/budget",
        )?;
        let mut out = Report {
            points: None,
            cells: 0,
            reason: Some("endpoint-frame-unproved"),
        };
        if self.options.closed {
            out.reason = Some("closed-path-has-no-caps");
            return Ok(out);
        }
        let frame = &self.transport_certificate;
        if frame.status != Status::Certified {
            return Ok(out);
        }
        if points.is_empty() {
            out.reason = Some("empty-point-family");
            return Ok(out);
        }
        let mut result = Vec::with_capacity(points.len());
        for &point in points {
            let mut pair = [[[0.; 2]; 3]; 2];
            for endpoint in 0..2 {
                if out.cells == max_cells {
                    out.reason = Some("endpoint-cell-budget-exhausted");
                    return Ok(out);
                }
                out.cells += 1;
                let edge = if endpoint == 0 {
                    0
                } else {
                    self.tangents.len() - 1
                };
                let fraction = [endpoint as f64; 2];
                let ideal = if let Some(source) = self.frame_source() {
                    let Some(local) = frame.profile_local_enclosure(point)? else {
                        return Ok(out);
                    };
                    frame.station_frame_affine(
                        edge,
                        fraction,
                        local,
                        source,
                        self.scale,
                        self.twist,
                        self.affine_laws,
                        law_cells,
                    )?
                } else {
                    frame.station_with_affine_laws(
                        edge,
                        fraction,
                        point,
                        self.scale,
                        self.twist,
                        self.affine_laws,
                        law_cells,
                    )?
                };
                let Some(ideal) = ideal else { return Ok(out) };
                pair[endpoint] = ideal;
            }
            result.push(pair);
        }
        out.points = Some(result);
        out.reason = None;
        Ok(out)
    }
}
