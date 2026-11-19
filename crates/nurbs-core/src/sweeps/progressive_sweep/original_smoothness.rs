//! Sufficient open original-frame C1/C2, separate from retained seams/error.
use super::*;
mod guided_arc;
pub(super) mod closed_frenet;
pub(super) mod contact_motion;
use crate::sweeps::progressive_miter::{
    authored_frame_certificate as frame, scalar_certificate::Status,
};
#[derive(Clone, Debug)]
pub struct OriginalFrameSmoothnessReport {
    pub order: usize,
    pub source_frame_smoothness_certified: bool,
    pub cells: usize,
    pub exact_work: u64,
    pub reason: Option<&'static str>,
}
fn basis_continuity(curve: &Curve, order: usize) -> bool {
    if curve.periodic {
        return false;
    }
    let [a, b] = curve.domain();
    let mut idx = 0;
    while idx < curve.knots.len() {
        let u = curve.knots[idx];
        let mut count = 1;
        while idx + count < curve.knots.len() && curve.knots[idx + count] == u {
            count += 1;
        }
        if a < u && u < b && (curve.degree < order || count > curve.degree - order) {
            return false;
        }
        idx += count;
    }
    true
}
fn knot_continuity(curve:&Curve,order:usize,work:&mut u64,max_work:u64)->Result<bool>{
    if basis_continuity(curve,order){return Ok(true);}
    let proof=super::endpoint_jets::certify_knots(curve,order,max_work-*work)?;
    *work+=proof.exact_work;
    Ok(proof.certified)
}
impl Sweep<'_> {
    /// A forward line image has a constant tangent even at a knot where its
    /// parameter speed changes. A successful reduction still needs a valid
    /// perpendicular seed and twist cover. Charge the line trial even on failure.
    fn try_line_frame_cover(
        &self,
        out: &mut OriginalFrameSmoothnessReport,
        max_cells: usize,
        max_exact_work: u64,
    ) -> Result<bool> {
        if self.frame_laws.is_some()
            || self.orientation_guide.is_some()
            || !matches!(self.options.orientation, Orientation::RotationMinimizing | Orientation::CorrectedFrenet)
            || authored_error::original_line(self.path)
        {
            return Ok(false);
        }
        let budget = max_cells.min((max_exact_work - out.exact_work) as usize);
        let line = super::source_line::certify(self.path, budget)?;
        out.cells += line.cells;
        out.exact_work += line.cells as u64;
        if !line.certified {
            return Ok(false);
        }
        let poles = &self.path.control_points;
        let reference = Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![poles[0].clone(), poles[poles.len() - 1].clone()],
            weights: vec![1., 1.],
            periodic: false,
        };
        let cover = frame::certify_fixed_cover(
            &reference, self.options.normal, self.twist, max_cells - out.cells,
        )?;
        out.cells += cover.cells;
        out.source_frame_smoothness_certified = cover.status == Status::Certified;
        out.reason = if out.source_frame_smoothness_certified { None } else { cover.reason };
        Ok(true)
    }

    /// Sufficient original frame Ck on the entire open traversal domain.
    /// Does not certify retained Gk, trajectory/surface regularity, or a seam.
    /// Arc composition uses proved positive speed and the inverse function
    /// theorem. Guided arc frames additionally require both source speed
    /// covers and independent joint nondegeneracy. No inverse derivative
    /// bounds or retained numerical-frame correspondence are claimed.
    pub fn certify_original_frame_smoothness(
        &self,
        order: usize,
        max_cells: usize,
        max_exact_work: u64,
    ) -> Result<OriginalFrameSmoothnessReport> {
        check(
            matches!(order, 1 | 2) && max_cells <= 100000 && max_exact_work <= 1000000,
            "Invalid original frame smoothness limits",
        )?;
        let mut out = OriginalFrameSmoothnessReport {
            order,
            source_frame_smoothness_certified: false,
            cells: 0,
            exact_work: 0,
            reason: Some("original-frame-smoothness-unproved"),
        };
        if self.path.periodic || path_is_closed(self.path)? {
            out.reason = Some("original-frame-closed-seam-unproved");
            return Ok(out);
        }
        if !knot_continuity(self.twist,order,&mut out.exact_work,max_exact_work)? {
            out.reason = Some("original-frame-law-knot-continuity-unproved");
            return Ok(out);
        }
        if self.try_line_frame_cover(&mut out, max_cells, max_exact_work)? {
            return Ok(out);
        }
        let max_cells=max_cells-out.cells;
        let cover = if let Some((axis, normal)) = self.frame_laws {
            if !knot_continuity(axis,order,&mut out.exact_work,max_exact_work)? || !knot_continuity(normal,order,&mut out.exact_work,max_exact_work)? {
                out.reason = Some("original-frame-law-knot-continuity-unproved");
                return Ok(out);
            }
            frame::certify_twisted_cover(axis, normal, self.twist, max_cells)?
        } else if let Some(guide) = self.orientation_guide {
            if !knot_continuity(self.path,order+1,&mut out.exact_work,max_exact_work)? || !knot_continuity(guide,order,&mut out.exact_work,max_exact_work)? {
                out.reason = Some("original-frame-path-knot-continuity-unproved");
                return Ok(out);
            }
            if self.options.spacing != Spacing::Parameter {
                // Positive speed gives Ck inverse normalized-length maps by
                // the inverse function theorem. The independent Cartesian
                // cover proves nondegeneracy for their entire joint image;
                // it requires no unproved inverse derivative estimates.
                for curve in [self.path,guide] {
                    let regular=crate::curve_regularity::inspect(curve,max_cells-out.cells)?;
                    out.cells+=regular.cells;
                    if !regular.spanwise_regular {
                        out.reason=Some("guided-arc-source-speed-unproved");
                        return Ok(out);
                    }
                }
                let (status,cells)=frame::certify_independent_guided_cover(
                    self.path,guide,self.twist,max_cells-out.cells)?;
                out.cells+=cells;
                out.source_frame_smoothness_certified=status==Status::Certified;
                out.reason=if out.source_frame_smoothness_certified {None}
                    else {Some("guided-arc-independent-frame-cover-unproved")};
                return Ok(out);
            }
            frame::certify_guided_cover(self.path, guide, self.twist, max_cells)?
        } else {
            match self.options.orientation {
                Orientation::Fixed => frame::certify_fixed_cover(
                    self.path,
                    self.options.normal,
                    self.twist,
                    max_cells,
                )?,
                Orientation::FixedNormal | Orientation::RotationMinimizing => {
                    if !knot_continuity(self.path,order+1,&mut out.exact_work,max_exact_work)? {
                        out.reason = Some("original-frame-path-knot-continuity-unproved");
                        return Ok(out);
                    }
                    if self.options.orientation==Orientation::RotationMinimizing
                        && self.spatial_rmf_error_limits.is_some() {
                        // The original Bishop ODE N'=((P' x P'')/|P'|²) x N
                        // has C(k-1) coefficients on a regular C(k+1) path.
                        // Its unique Ck orthonormal solution exists on this
                        // compact OPEN domain. No numerical integration error
                        // or retained seam credit follows from this theorem.
                        let regular=crate::curve_regularity::inspect(self.path,max_cells)?;
                        out.cells+=regular.cells;
                        if !regular.spanwise_regular {
                            out.reason=Some("original-spatial-bishop-speed-unproved");return Ok(out);
                        }
                        let zero=constant_vector_law([0.;3])?;
                        let seed=frame::certify_fixed_path_values(self.path,self.options.normal,&zero,[0.,0.],max_cells-regular.cells)?;
                        out.cells+=seed.cells;
                        out.source_frame_smoothness_certified=seed.status==Status::Certified;
                        out.reason=if out.source_frame_smoothness_certified {None}else{Some("original-spatial-bishop-seed-unproved")};
                        return Ok(out);
                    }
                    if self.options.orientation == Orientation::RotationMinimizing
                        && !authored_error::original_line(self.path)
                        && !authored_error::original_planar_rmf(self.path, self.options.normal)
                    {
                        let plane = super::source_plane::certify(
                            self.path,
                            self.options.normal,
                            max_exact_work-out.exact_work,
                        )?;
                        out.exact_work += plane.exact_work;
                        if !plane.proved {
                            out.reason = plane.reason;
                            return Ok(out);
                        }
                    }
                    if self.options.orientation == Orientation::RotationMinimizing
                        && authored_error::original_line(self.path)
                    {
                        frame::certify_fixed_cover(
                            self.path,
                            self.options.normal,
                            self.twist,
                            max_cells,
                        )?
                    } else {
                        frame::certify_fixed_normal_cover(
                            self.path,
                            self.options.normal,
                            self.twist,
                            max_cells,
                        )?
                    }
                }
                Orientation::Frenet => {
                    if !knot_continuity(self.path,order+2,&mut out.exact_work,max_exact_work)? {
                        out.reason = Some("original-frame-path-knot-continuity-unproved");
                        return Ok(out);
                    }
                    frame::certify_frenet_cover(self.path, self.twist, max_cells)?
                }
                Orientation::CorrectedFrenet => {
                    if !knot_continuity(self.path,order+1,&mut out.exact_work,max_exact_work)? {
                        out.reason = Some("original-frame-path-knot-continuity-unproved");
                        return Ok(out);
                    }
                    if authored_error::original_line(self.path) {
                        // No principal direction: the perpendicular seed and
                        // source tangent are constant on an original line.
                        frame::certify_fixed_cover(self.path,self.options.normal,self.twist,max_cells)?
                    } else {
                        let plane = super::source_plane::certify(
                            self.path,self.options.normal,max_exact_work-out.exact_work)?;
                        out.exact_work += plane.exact_work;
                        if !plane.proved {
                            if plane.reason != Some("source-plane-coefficients-nonzero")
                                || self.contact_point.is_some()
                            {
                                out.reason = plane.reason;
                                return Ok(out);
                            }
                            if !knot_continuity(self.path, order + 2, &mut out.exact_work, max_exact_work)? {
                                out.reason = Some("original-frame-path-knot-continuity-unproved");
                                return Ok(out);
                            }
                            let (regular, work) = authored_error::certify_corrected_regular_branch(
                                self.path, max_cells.min((max_exact_work - out.exact_work) as usize),
                            )?;
                            out.cells += work;
                            out.exact_work += work as u64;
                            if !regular {
                                out.reason = Some("original-corrected-regular-branch-unproved");
                                return Ok(out);
                            }
                            frame::certify_frenet_cover(self.path, self.twist, max_cells - work)?
                        } else {
                            // A regular C(k+1) planar path has a Ck tangent T.
                            // For its constant plane normal B, B x T extends the
                            // signed principal normal through curvature zeros.
                            // Planar RMF fallback preserves this basis (or any
                            // constant transverse phase on an entirely straight
                            // path). Thus no nonzero-curvature premise is needed.
                            // Use this cover only to prove speed/nondegeneracy;
                            // its T/B jets are not corrected-Frenet frame jets.
                            // Retained arithmetic/correspondence is not certified.
                            frame::certify_fixed_normal_cover(
                                self.path,self.options.normal,self.twist,max_cells)?
                        }
                    }
                }
            }
        };
        out.cells += cover.cells;
        out.source_frame_smoothness_certified = cover.status == Status::Certified;
        out.reason = if out.source_frame_smoothness_certified {
            None
        } else {
            cover.reason
        };
        Ok(out)
    }
}

#[cfg(test)]
#[path = "tests/original_smoothness.rs"]
mod tests;

impl MultiSweep<'_> {
    /// All contours share the original orientation laws; certify that frame once.
    /// Profile regularity and retained joins remain independent obligations.
    pub fn certify_original_frame_smoothness(
        &self, order: usize, max_cells: usize, max_exact_work: u64,
    ) -> Result<OriginalFrameSmoothnessReport> {
        self.sweeps[0].certify_original_frame_smoothness(order, max_cells, max_exact_work)
    }

    pub fn certify_closed_path_frame_smoothness(&self,order:usize,max_cells:usize,max_exact_work:u64)
        ->Result<ClosedPathFrameSmoothnessReport>{
        self.sweeps[0].certify_closed_path_frame_smoothness(order,max_cells,max_exact_work)
    }
    pub fn certify_closed_guided_frame_smoothness(&self,order:usize,max_cells:usize,max_exact_work:u64)
        ->Result<ClosedPathFrameSmoothnessReport>{
        self.sweeps[0].certify_closed_guided_frame_smoothness(order,max_cells,max_exact_work)
    }
}

#[derive(Clone,Debug)]
pub struct ClosedPathFrameSmoothnessReport {
    pub order:usize,
    pub closed_source_frame_smoothness_certified:bool,
    pub cells:usize,
    pub exact_work:u64,
    pub reason:Option<&'static str>,
}
impl Sweep<'_> {
    /// Closed original guided frame; retained joins and error remain separate.
    pub fn certify_closed_guided_frame_smoothness(&self,order:usize,max_cells:usize,max_exact_work:u64)->Result<ClosedPathFrameSmoothnessReport>{
        check(matches!(order,1|2)&&max_cells<=100000&&max_exact_work<=1000000,"Invalid closed guided frame proof limits")?;
        let mut out=ClosedPathFrameSmoothnessReport{order,closed_source_frame_smoothness_certified:false,cells:0,exact_work:0,reason:Some("closed-guided-frame-mode-unproved")};
        let Some(guide)=self.orientation_guide else{return Ok(out);};
        if self.frame_laws.is_some()||!path_is_closed(self.path)?||!path_is_closed(guide)?{return Ok(out);}
        // Keep the cheaper sufficient Cartesian route. A failed source jet
        // query is charged before trying the geometric arc owner.
        let mut cartesian=true;
        for (curve,k) in [(self.path,order+1),(guide,order),(self.twist,order)] {
            if !knot_continuity(curve,k,&mut out.exact_work,max_exact_work)? {
                out.reason=Some("closed-guided-frame-knot-jets-unproved");cartesian=false;break;
            }
            let seam=super::endpoint_jets::certify(curve,k,max_exact_work-out.exact_work)?;
            out.exact_work+=seam.exact_work;
            if !seam.certified {
                out.reason=Some("closed-guided-frame-endpoint-jets-unproved");cartesian=false;break;
            }
        }
        if !cartesian {
            return if self.options.spacing==Spacing::Parameter {Ok(out)}
                else {guided_arc::certify(self,guide,out,false,max_cells,max_exact_work)};
        }
        if self.options.spacing!=Spacing::Parameter {
            for curve in [self.path,guide] {
                let regular=crate::curve_regularity::inspect(curve,max_cells-out.cells)?;out.cells+=regular.cells;
                if !regular.spanwise_regular {out.reason=Some("closed-guided-arc-source-speed-unproved");return Ok(out);}
            }
            // This strong Cartesian-product proposal is bounded independently
            // of the caller allowance. A true station correspondence can be
            // regular even when unrelated path/guide parameters are singular.
            let (status,cells)=frame::certify_independent_guided_cover(self.path,guide,self.twist,(max_cells-out.cells).min(4096))?;
            out.cells+=cells;
            if status!=Status::Certified {
                return guided_arc::certify(self,guide,out,true,max_cells,max_exact_work);
            }
            out.closed_source_frame_smoothness_certified=true;out.reason=None;
        }else {
            let cover=frame::certify_guided_cover(self.path,guide,self.twist,max_cells)?;
            out.cells=cover.cells;out.closed_source_frame_smoothness_certified=cover.status==Status::Certified;
            out.reason=if out.closed_source_frame_smoothness_certified{None}else{cover.reason};
        }
        Ok(out)
    }
    /// Closed original path frame only; exact direction jets can replace
    /// the sufficient Cartesian jets. Whole regularity/plane remain owned.
    pub fn certify_closed_path_frame_smoothness(&self,order:usize,max_cells:usize,max_exact_work:u64)->Result<ClosedPathFrameSmoothnessReport>{
        check(matches!(order,1|2)&&max_cells<=100000&&max_exact_work<=1000000,"Invalid closed path frame proof limits")?;
        let mut out=ClosedPathFrameSmoothnessReport{order,closed_source_frame_smoothness_certified:false,cells:0,exact_work:0,reason:Some("closed-path-frame-mode-unproved")};
        let spatial_bishop = self.options.orientation == Orientation::RotationMinimizing
            && self.spatial_rmf_error_limits.is_some();
        if self.options.orientation == Orientation::Fixed
            && self.frame_laws.is_none()
            && self.orientation_guide.is_none()
            && path_is_closed(self.path)?
        {
            // The basis depends on the initial tangent only. Throughout
            // the traversal the exact periodic twist rotates this basis.
            if !knot_continuity(self.twist, order, &mut out.exact_work, max_exact_work)? {
                out.reason = Some("closed-fixed-twist-knots-unproved");
                return Ok(out);
            }
            let twist = super::endpoint_jets::certify(
                self.twist, order, max_exact_work - out.exact_work,
            )?;
            out.exact_work += twist.exact_work;
            if !twist.certified {
                out.reason = Some("closed-fixed-twist-seam-unproved");
                return Ok(out);
            }
            let basis = frame::certify_fixed_path(
                self.path, self.options.normal, self.twist, [0., 0.], max_cells,
            )?;
            out.cells = basis.cells;
            out.closed_source_frame_smoothness_certified = basis.status == Status::Certified;
            out.reason = basis.reason;
            return Ok(out);
        }
        let periodic_frame = spatial_bishop
            || matches!(self.options.orientation, Orientation::FixedNormal | Orientation::Frenet | Orientation::CorrectedFrenet);
        if (self.path.periodic && !periodic_frame)
            || !path_is_closed(self.path)?
            || self.frame_laws.is_some()
            || self.orientation_guide.is_some()
            || !matches!(
                self.options.orientation,
                Orientation::FixedNormal | Orientation::RotationMinimizing
                    | Orientation::CorrectedFrenet | Orientation::Frenet
            )
        {
            return Ok(out);
        }
        if self.options.orientation==Orientation::Frenet {
            return closed_frenet::certify(self,order,max_cells,max_exact_work);
        }
        if self.options.orientation == Orientation::CorrectedFrenet {
            let plane = super::source_plane::certify(self.path, self.options.normal, max_exact_work)?;
            out.exact_work += plane.exact_work;
            if self.path.periodic || plane.reason == Some("source-plane-coefficients-nonzero") {
                let mut principal = closed_frenet::certify(
                    self, order, max_cells, max_exact_work - out.exact_work,
                )?;
                principal.exact_work += out.exact_work;
                return Ok(principal);
            }
            if !plane.proved {
                out.reason = plane.reason;
                return Ok(out);
            }
        }
        // Preserve the cheaper sufficient Cartesian route. Failed work is
        // retained when trying exact direction charts under the same owner.
        let mut path_ok=knot_continuity(self.path,order+1,&mut out.exact_work,max_exact_work)?;
        if path_ok {
            let seam=super::endpoint_jets::certify(self.path,order+1,max_exact_work-out.exact_work)?;
            out.exact_work+=seam.exact_work;path_ok=seam.certified;
        }
        if !path_ok {
            for endpoints in [false,true] {
                if self.path.periodic {
                    let position = super::endpoint_jets::certify_position(
                        self.path, endpoints, max_exact_work - out.exact_work,
                    )?;
                    out.exact_work += position.exact_work;
                    if !position.certified {
                        out.reason = Some("closed-path-position-jets-unproved");
                        return Ok(out);
                    }
                }
                let proof=super::endpoint_jets::certify_direction(self.path,order,endpoints,self.options.spacing!=Spacing::Parameter,max_exact_work-out.exact_work)?;
                out.exact_work+=proof.exact_work;
                if !proof.certified {out.reason=Some("closed-path-direction-jets-unproved");return Ok(out);}
            }
        }
        if !knot_continuity(self.twist,order,&mut out.exact_work,max_exact_work)? {out.reason=Some("closed-path-frame-knot-jets-unproved");return Ok(out);}
        let seam=super::endpoint_jets::certify(self.twist,order,max_exact_work-out.exact_work)?;
        out.exact_work+=seam.exact_work;
        if !seam.certified {out.reason=Some("closed-path-frame-endpoint-jets-unproved");return Ok(out);}
        if spatial_bishop {
            // The ORIGINAL Bishop ODE exists on the positive-speed source.
            // Closing its holonomy by alpha=angle*normalized_arc makes N(1)=N(0).
            // Its corrected angular field is omega+alpha' T. Periodic Ck tangent
            // jets and periodic correction rates give a periodic Ck solution.
            // In parameter mode the speed jets must also match: direction-only
            // identities do not prove periodic derivatives of alpha(t).
            // In original arc units the exact Ck unit-tangent joins already
            // prove the periodic angular field. Raw speed factors cancel in
            // arc composition; parameter-mode correction still needs them.
            if !path_ok && self.options.spacing==Spacing::Parameter {
                out.reason=Some("closed-bishop-source-cartesian-jets-unproved");return Ok(out);
            }
            let regular=crate::curve_regularity::inspect(self.path,max_cells-out.cells)?;
            out.cells+=regular.cells;
            if !regular.spanwise_regular {out.reason=Some("closed-bishop-source-speed-unproved");return Ok(out);}
            let initial=frame::certify_fixed_path_values(self.path,self.options.normal,self.twist,[0.,0.],max_cells-out.cells)?;
            out.cells+=initial.cells;
            out.closed_source_frame_smoothness_certified=initial.status==Status::Certified;
            out.reason=if out.closed_source_frame_smoothness_certified {None}else{Some("closed-bishop-initial-seed-unproved")};
            // Frame well-posedness/smoothness only. This is neither a corrected
            // derivative enclosure nor a retained/source boundary certificate.
            return Ok(out);
        }
        if self.options.orientation == Orientation::RotationMinimizing {
            let plane=super::source_plane::certify(self.path,self.options.normal,max_exact_work-out.exact_work)?;
            out.exact_work+=plane.exact_work;
            if !plane.proved {out.reason=plane.reason;return Ok(out);}
        }
        // Complete cover proves nonzero speed and nonparallel constant seed.
        // Arc composition follows the regular inverse map with matching seam
        // jets. No numerical inverse-jet or retained frame proof is inferred.
        let cover=frame::certify_fixed_normal_cover(self.path,self.options.normal,self.twist,max_cells-out.cells)?;
        out.cells+=cover.cells;
        out.closed_source_frame_smoothness_certified=cover.status==Status::Certified;
        out.reason=if out.closed_source_frame_smoothness_certified {None}else{cover.reason};
        Ok(out)
    }
}

#[derive(Clone,Debug)]
pub struct ClosedAuthoredFrameSmoothnessReport {
    pub order:usize,
    pub closed_source_frame_smoothness_certified:bool,
    pub cells:usize,
    pub exact_work:u64,
    pub reason:Option<&'static str>,
}
impl Sweep<'_> {
    /// Sufficient periodic Ck authored frame field, including its source seam.
    /// Fixed orientation excludes constructor RMF holonomy correction. Does
    /// not certify closed path jets, profile trajectories or retained seams.
    pub fn certify_closed_authored_frame_smoothness(&self,order:usize,max_cells:usize,max_exact_work:u64)
        ->Result<ClosedAuthoredFrameSmoothnessReport>{
        check(matches!(order,1|2)&&max_cells<=100000&&max_exact_work<=1000000,"Invalid closed authored frame proof limits")?;
        let mut out=ClosedAuthoredFrameSmoothnessReport {order,closed_source_frame_smoothness_certified:false,
            cells:0,exact_work:0,reason:Some("closed-authored-frame-mode-unproved")};
        if !path_is_closed(self.path)?||self.options.orientation!=Orientation::Fixed||self.orientation_guide.is_some(){return Ok(out);}
        let Some((axis,normal))=self.frame_laws else{return Ok(out);};
        for law in [axis,normal,self.twist]{
            if !knot_continuity(law,order,&mut out.exact_work,max_exact_work)?{out.reason=Some("closed-authored-law-knot-continuity-unproved");return Ok(out);}
            let proof=super::endpoint_jets::certify(law,order,max_exact_work-out.exact_work)?;
            out.exact_work+=proof.exact_work;
            if !proof.certified {out.reason=Some("closed-authored-endpoint-jets-unproved");return Ok(out);}
        }
        let cover=frame::certify_twisted_cover(axis,normal,self.twist,max_cells)?;
        out.cells=cover.cells;
        out.closed_source_frame_smoothness_certified=cover.status==Status::Certified;
        out.reason=if out.closed_source_frame_smoothness_certified {None} else {cover.reason};
        Ok(out)
    }
}
#[cfg(test)]
mod closed_authored_tests;

impl MultiSweep<'_> {
    /// Shared original authored orientation field for all contours.
    pub fn certify_closed_authored_frame_smoothness(&self,order:usize,max_cells:usize,max_exact_work:u64)
        ->Result<ClosedAuthoredFrameSmoothnessReport>{
        self.sweeps[0].certify_closed_authored_frame_smoothness(order,max_cells,max_exact_work)
    }
}

#[cfg(test)]
#[test]
fn closed_conic_frame_c1_owns_direction_charts_and_refuses_false_c2(){
 let mut path=crate::primitives::circle([0.;3],[0.,0.,1.],1.).unwrap();
 for w in &mut path.weights {if *w!=1. {*w=0.5;}}
 let profile=crate::primitives::line([1.,0.,0.1],[1.,0.,0.2]).unwrap();
 let scale=super::constant_vector_law([1.,0.,0.]).unwrap();let twist=super::constant_vector_law([0.;3]).unwrap();
 for spacing in [Spacing::Parameter,Spacing::ArcLength{tolerance:0.001,max_cells:100000}] {
  let options=Options{normal:[0.,0.,1.],orientation:Orientation::RotationMinimizing,spacing,initial_sections:5,max_sections:17,max_deviation:2.};
  let sweep=Sweep::new(&profile,&path,&scale,&twist,options).unwrap();
  let proof=sweep.certify_closed_path_frame_smoothness(1,10000,1000000).unwrap();
  assert!(proof.closed_source_frame_smoothness_certified,"{spacing:?}/{proof:?}");
  assert!(!sweep.certify_closed_path_frame_smoothness(2,10000,1000000).unwrap().closed_source_frame_smoothness_certified);
  assert!(!sweep.certify_closed_path_frame_smoothness(1,10000,proof.exact_work-1).unwrap().closed_source_frame_smoothness_certified);
 }
}
