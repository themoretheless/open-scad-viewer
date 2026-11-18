//! Offset a creased network of planar bilinear rational faces by intersecting
//! their oriented offset supports. Shared vertex proposals are qualified by
//! whole-carrier signed plane-distance bounds and fresh closed BRep checks.
//! This is not an arbitrary freeform-support intersection or embedding theorem.
pub use super::planar_network::{Boundary, Cut, CutKind, Limits};
use super::{network::Built, planar_network::frame};
use crate::feature_ops::region_pattern::{Verification, qualification};
use crate::{Error, Model, Result};
use nurbs_core::{interval_eval::Interval as I, robust_solvers, surface::Surface};
use std::collections::BTreeMap;
fn refused(s: &str) -> Error {
    Error::new("BREP_MITER_NETWORK_REFUSED", s)
}

pub struct Thickness {
    sources: Vec<Surface>,
    reversed: Vec<bool>,
    distance: f64,
    tolerance: f64,
    miter_limit: f64,
    limits: Limits,
    caps: Vec<Surface>,
    built: Built,
    error: f64,
    displacement: f64,
}
impl Thickness {
    pub fn model(&self) -> &Model {
        &self.built.model
    }
    pub fn source_patches(&self) -> &[Surface] {
        &self.sources
    }
    pub fn cap_patches(&self) -> &[Surface] {
        &self.caps
    }
    pub fn exposed_boundaries(&self) -> &[Boundary] {
        &self.built.boundaries
    }
    pub fn cuts(&self) -> &[Cut] {
        &self.built.cuts
    }
    pub fn evidence(&self) -> &Verification {
        &self.built.evidence
    }
    /// Absolute error in signed distance to each ORIGINAL oriented support
    /// plane, everywhere on its saved cap. Does not bound lateral displacement,
    /// closest distance to the bounded original face, or wall thickness.
    pub fn support_offset_error_upper(&self) -> f64 {
        self.error
    }
    /// L1 displacement bound for saved vertices; includes binary64 addition.
    pub fn miter_displacement_upper(&self) -> f64 {
        self.displacement
    }
    pub fn exact_work(&self) -> u64 {
        self.built.exact_work
    }
    pub fn reverify(&self) -> Result<Verification> {
        let fresh = thicken(
            &self.sources,
            &self.reversed,
            self.distance,
            self.tolerance,
            self.miter_limit,
            self.limits,
        )?;
        if fresh.caps != self.caps
            || fresh.built.model != self.built.model
            || fresh.built.boundaries != self.built.boundaries
            || fresh.built.cuts != self.built.cuts
        {
            return Err(refused(
                "Saved miter skin differs from original support reconstruction",
            ));
        }
        Ok(fresh.built.evidence)
    }
}
fn key(p: &[f64]) -> [u64; 3] {
    std::array::from_fn(|k| if p[k] == 0. { 0 } else { p[k].to_bits() })
}
fn spend(cells: &mut usize, max: usize) -> Result<()> {
    if *cells >= max {
        return Err(refused("Shared support-offset proof allowance exhausted"));
    }
    *cells += 1;
    Ok(())
}
/// A connected network of 1..32 full degree-(1,1), exactly planar positive
/// rational charts. Controls and natural UV charts are retained on source caps.
/// At each shared source vertex a native rank-revealing solver PROPOSES a common
/// displacement satisfying all incident oriented support offsets. Identical
/// literal target vertices sew incident cap edges; complete actual lifts and
/// closed chart/trim/material-role qualification remain mandatory.
///
/// `miter_limit` bounds L1 vertex displacement / |thickness|. This rejects large
/// near-parallel miters; inconsistent normals, cap collapse, sewing failures
/// and exhausted aggregate work refuse the operation. No self-trimming occurs.
pub fn thicken(
    sources: &[Surface],
    reversed: &[bool],
    signed_thickness: f64,
    tolerance: f64,
    miter_limit: f64,
    limits: Limits,
) -> Result<Thickness> {
    qualification::limits(limits.proof)?;
    if sources.is_empty()
        || sources.len() > 32
        || sources.len() != reversed.len()
        || !signed_thickness.is_finite()
        || signed_thickness == 0.
        || signed_thickness.abs() > 1e6
        || !tolerance.is_finite()
        || !(1e-10..=1e-2).contains(&tolerance)
        || tolerance >= signed_thickness.abs()
        || !miter_limit.is_finite()
        || !(1.0..=1e6).contains(&miter_limit)
        || !(1..=1_000_000).contains(&limits.exact_work)
    {
        return Err(refused(
            "Expected bounded oriented planar quadrilateral network, thickness, tolerance and miter limit",
        ));
    }
    for s in sources {
        s.validate()?;
        if s.degree_u != 1
            || s.degree_v != 1
            || s.periodic_u
            || s.periodic_v
            || !crate::rounded_patch_network::clamped_single(&s.knots_u, 1, s.control_points.len())
            || !crate::rounded_patch_network::clamped_single(
                &s.knots_v,
                1,
                s.control_points[0].len(),
            )
            || s.weights.iter().flatten().any(|w| *w <= 0.)
            || s.control_points
                .iter()
                .flatten()
                .any(|p| p.len() != 3 || p.iter().any(|x| x.abs() > 1e6))
        {
            return Err(refused(
                "Miter supports require bounded full rational bilinear source charts",
            ));
        }
    }
    let mut cells = 0;
    let frames = sources
        .iter()
        .zip(reversed)
        .map(|(s, r)| {
            frame::offset(
                std::slice::from_ref(s),
                std::slice::from_ref(r),
                signed_thickness,
                tolerance,
                limits.proof.max_cells,
                &mut cells,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    let mut vertices = BTreeMap::<[u64; 3], Vec<(usize, usize, usize)>>::new();
    for (face, s) in sources.iter().enumerate() {
        for (i, row) in s.control_points.iter().enumerate() {
            for (j, p) in row.iter().enumerate() {
                vertices.entry(key(p)).or_default().push((face, i, j));
            }
        }
    }
    let mut caps = sources.to_vec();
    let mut displacement = 0_f64;
    for addresses in vertices.values() {
        spend(&mut cells, limits.proof.max_cells)?;
        let rows = addresses
            .iter()
            .map(|(f, _, _)| {
                frames[*f]
                    .unit_normal
                    .iter()
                    .map(|n| n.lo * 0.5 + n.hi * 0.5)
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        // A numerical proposal only. Every pole is independently checked against
        // exact original planes via the interval normal enclosure below.
        let delta = robust_solvers::rrqr(&rows, None)?
            .solve_least_squares(&vec![signed_thickness; rows.len()])?;
        let (f, i, j) = addresses[0];
        let old = &sources[f].control_points[i][j];
        let new = (0..3).map(|k| old[k] + delta[k]).collect::<Vec<_>>();
        if new.iter().any(|x| !x.is_finite() || x.abs() > 1e6) {
            return Err(refused("Miter proposal exceeds finite coordinate bounds"));
        }
        let mut length = I::point(0.);
        for k in 0..3 {
            let d = I::point(new[k]).sub(I::point(old[k]))?;
            length = length.add(I::point(d.lo.abs().max(d.hi.abs())))?;
        }
        // Compare against an outward LOWER bound of the caller's allowed
        // displacement, so rounding cannot enlarge the admitted miter limit.
        let allowance = I::point(miter_limit)
            .mul(I::point(signed_thickness.abs()))?
            .lo;
        if length.hi > allowance {
            return Err(refused("Miter displacement exceeds caller limit"));
        }
        displacement = displacement.max(length.hi);
        for &(face, i, j) in addresses {
            caps[face].control_points[i][j] = new.clone();
        }
    }
    let mut error = 0_f64;
    for (cap, frame) in caps.iter().zip(&frames) {
        for (new, base) in cap
            .control_points
            .iter()
            .flatten()
            .zip(frame.patches[0].control_points.iter().flatten())
        {
            spend(&mut cells, limits.proof.max_cells)?;
            let mut residual = I::point(0.);
            for k in 0..3 {
                residual = residual.add(
                    I::point(new[k])
                        .sub(I::point(base[k]))?
                        .mul(frame.unit_normal[k])?,
                )?;
            }
            let bound = I::point(residual.lo.abs().max(residual.hi.abs()))
                .add(I::point(frame.error_upper))?
                .hi;
            error = error.max(bound);
            if error > tolerance {
                return Err(refused(
                    "Whole cap signed support-offset error exceeds tolerance",
                ));
            }
        }
    }
    for ((cap, frame), reversed) in caps.iter().zip(&frames).zip(reversed) {
        same_orientation(
            cap,
            frame.unit_normal,
            *reversed,
            &mut cells,
            limits.proof.max_cells,
        )?;
    }
    // Identical positive weights make every signed plane residual a convex
    // combination of the bounded pole residuals, covering the full cap image.
    let built = super::network::construct(
        sources,
        &caps,
        reversed,
        signed_thickness,
        tolerance,
        limits,
        &mut cells,
    )?;
    Ok(Thickness {
        sources: sources.to_vec(),
        reversed: reversed.to_vec(),
        distance: signed_thickness,
        tolerance,
        miter_limit,
        limits,
        caps,
        built,
        error,
        displacement,
    })
}

// Rank alone does not distinguish a cap turned through its source support.
// Reuse the native complete original-cap normal hull, never a corner sample.
fn same_orientation(
    cap: &Surface,
    original_normal: [I; 3],
    reversed: bool,
    cells: &mut usize,
    max: usize,
) -> Result<()> {
    let domain = [
        [cap.knots_u[1], cap.knots_u[2]],
        [cap.knots_v[1], cap.knots_v[2]],
    ];
    let mut pending = vec![(domain, 0usize)];
    while let Some((rect, depth)) = pending.pop() {
        spend(cells, max)?;
        let (hull, _) = nurbs_core::normal_alignment::normal_bounds(cap, rect, 1)?;
        let mut dot = I::point(0.);
        if let Some(hull) = hull {
            for k in 0..3 {
                dot = dot.add(I::new(hull[k][0], hull[k][1])?.mul(original_normal[k])?)?;
            }
            if reversed {
                dot = I::new(-dot.hi, -dot.lo)?;
            }
            if dot.lo > 0. {
                continue;
            }
            if dot.hi <= 0. {
                return Err(refused(
                    "Miter cap reverses the original support orientation",
                ));
            }
        }
        if depth == 32 {
            return Err(refused("Miter cap orientation remains unresolved"));
        }
        let axis = depth % 2;
        let middle = rect[axis][0] * 0.5 + rect[axis][1] * 0.5;
        if middle <= rect[axis][0] || middle >= rect[axis][1] {
            return Err(refused("Miter cap orientation clock exhausted"));
        }
        let mut a = rect;
        let mut b = rect;
        a[axis][1] = middle;
        b[axis][0] = middle;
        pending.push((b, depth + 1));
        pending.push((a, depth + 1));
    }
    Ok(())
}
