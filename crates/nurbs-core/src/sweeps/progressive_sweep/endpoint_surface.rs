//! Filled rational cap transported by the constructor's original endpoint pose.
//! Original frame/path/scale/affine/guide laws are enclosed before proposing any
//! stored controls. Convex positive rational weights bound the full cap interior.
use super::*;
use crate::distance_bounds::Interval as I;
use crate::numerics::interval_vec3::{norm as interval_norm, sub as interval_sub};
use crate::sweeps::progressive_miter::{
    authored_frame_certificate as frame, scalar_certificate::Status, vector_certificate,
};

#[derive(Clone, Debug)]
pub struct EndpointSurfaceReport {
    pub surface: Option<Surface>,
    pub error_upper: Option<f64>,
    pub cells: usize,
    pub products: usize,
    pub reason: Option<&'static str>,
}
impl MultiSweep<'_> {
    pub fn source_is_closed(&self) -> Result<bool> {
        path_is_closed(self.sweeps[0].path)
    }
    /// Both endpoint policies (parameter/arc length) reach the same exact source
    /// endpoints. This operation cannot replace the complete wall interpolation proof.
    pub fn endpoint_surface_image(
        &self,
        surface: &Surface,
        endpoint: usize,
        max_cells: usize,
        max_products: usize,
    ) -> Result<EndpointSurfaceReport> {
        surface.validate()?;
        check(
            endpoint < 2 && max_cells <= 100000 && max_products <= 1000000,
            "Invalid endpoint image work limits",
        )?;
        let mut out = EndpointSurfaceReport {
            surface: None,
            error_upper: None,
            cells: 0,
            products: 0,
            reason: Some("endpoint-image-work-limit"),
        };
        let n = surface.control_points.len() * surface.control_points[0].len();
        // Fixed conservative charge for coordinate/pose/control error products,
        // in addition to the separately charged original-law enclosure cells.
        if n > 32768 || 64 * n > max_products {
            return Ok(out);
        }
        out.products = 64 * n;
        let sweep = &self.sweeps[0];
        if self.source_is_closed()? {
            out.reason = Some("closed-source-has-no-end-caps");
            return Ok(out);
        }
        let zero = constant_vector_law([0.; 3])?;
        let Some(initial) =
            super::profile_domain::original_frame(sweep, &zero, [0., 0.], max_cells, true)?
        else {
            out.cells = max_cells;
            out.reason = Some("initial-source-frame-unproved");
            return Ok(out);
        };
        out.cells += initial.cells;
        if initial.status != Status::Certified {
            out.reason = Some("initial-source-frame-unproved");
            return Ok(out);
        }
        let start = vector_certificate::certify_values_traversal(
            sweep.path,
            [0., 0.],
            max_cells - out.cells,
            false,
        )?;
        out.cells += start.cells;
        let Some(start) = start.value else {
            out.reason = Some("path-start-unproved");
            return Ok(out);
        };
        // Use the existing source coordinate owner on an unrounded flattening
        // of the cap net. Degree/knots here are irrelevant to that linear map.
        let mut poles = sweep.profile.clone();
        poles.control_points = surface.control_points.iter().flatten().cloned().collect();
        let coordinates = super::authored_error::coordinates_in_basis(&poles, start, initial)?;
        let t = [endpoint as f64; 2];
        let Some(mut values) = super::profile_domain::original_frame(
            sweep,
            sweep.twist,
            t,
            max_cells - out.cells,
            endpoint == 0,
        )?
        else {
            out.cells = max_cells;
            out.reason = Some("endpoint-source-frame-unproved");
            return Ok(out);
        };
        let fit = if sweep.contact_point.is_some() {
            let anchor =
                super::contact_anchor::certify(sweep, max_cells - out.cells - values.cells)?;
            values.cells += anchor.cells;
            if anchor.status != Status::Certified {
                out.cells += values.cells;
                out.reason = anchor.reason;
                return Ok(out);
            }
            let point = anchor.coordinates.unwrap()[0];
            let report = frame::certify_contact_fit_value(
                sweep.path,
                sweep.orientation_guide.unwrap(),
                sweep.scale,
                sweep.affine_laws,
                point,
                t,
                max_cells - out.cells - values.cells,
            )?;
            values.cells += report.cells;
            let Some(value) = report.value else {
                out.cells += values.cells;
                out.reason = Some("endpoint-contact-fit-unproved");
                return Ok(out);
            };
            if value[0] <= 0. {
                out.cells += values.cells;
                out.reason = Some("endpoint-contact-fit-positivity-unproved");
                return Ok(out);
            }
            Some(I::new(value[0], value[1])?)
        } else {
            None
        };
        let result = frame::control_values_with_fit(
            sweep.path,
            sweep.scale,
            sweep.affine_laws,
            &coordinates,
            t,
            max_cells - out.cells,
            values,
            fit,
        )?;
        out.cells += result.cells;
        let Some(images) = result.values else {
            out.reason = result.reason;
            return Ok(out);
        };
        let mut stored = surface.clone();
        let mut maximum = 0_f64;
        for (pole, image) in stored.control_points.iter_mut().flatten().zip(images) {
            let intervals = [
                I::new(image[0][0], image[0][1])?,
                I::new(image[1][0], image[1][1])?,
                I::new(image[2][0], image[2][1])?,
            ];
            let proposal = intervals.map(|i| crate::distance_bounds::midpoint(i.lo, i.hi));
            let point = proposal.map(I::point);
            maximum = maximum.max(interval_norm(interval_sub(point, intervals)?)?.hi);
            *pole = proposal.to_vec();
        }
        stored.validate()?;
        out.surface = Some(stored);
        out.error_upper = Some(maximum);
        out.reason = None;
        Ok(out)
    }
}
