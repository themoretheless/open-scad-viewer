//! Sufficient open original-frame C1/C2, separate from retained seams/error.
use super::*;
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
    let [a, b] = curve.domain();
    !curve.periodic
        && curve.knots.iter().filter(|&&u| a < u && u < b).all(|u| {
            curve.degree >= order
                && curve.knots.iter().filter(|x| *x == u).count() <= curve.degree - order
        })
}
fn knot_continuity(curve:&Curve,order:usize,work:&mut u64,max_work:u64)->Result<bool>{
    if basis_continuity(curve,order){return Ok(true);}
    let proof=super::endpoint_jets::certify_knots(curve,order,max_work-*work)?;
    *work+=proof.exact_work;
    Ok(proof.certified)
}
impl Sweep<'_> {
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
        if self.frame_laws.is_none()&&self.orientation_guide.is_none()
            &&matches!(self.options.orientation,Orientation::RotationMinimizing|Orientation::CorrectedFrenet)
            &&!authored_error::original_line(self.path) {
            // A proved forward line image has constant tangent orientation,
            // even when source speed changes at a knot. This establishes only
            // the frame, not positional Ck or retained surface continuity.
            // Conservatively charge all line work to both caller limits.
            let line=super::source_line::certify(self.path,max_cells.min((max_exact_work-out.exact_work) as usize))?;
            out.cells+=line.cells;out.exact_work+=line.cells as u64;
            if line.certified {
                let reference=Curve {degree:1,knots:vec![0.,0.,1.,1.],
                    control_points:vec![self.path.control_points[0].clone(),self.path.control_points.last().unwrap().clone()],
                    weights:vec![1.,1.],periodic:false};
                let cover=frame::certify_fixed_cover(&reference,self.options.normal,self.twist,max_cells-out.cells)?;
                out.cells+=cover.cells;
                out.source_frame_smoothness_certified=cover.status==Status::Certified;
                out.reason=if out.source_frame_smoothness_certified {None}else{cover.reason};
                return Ok(out);
            }
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
                            out.reason = plane.reason;
                            return Ok(out);
                        }
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
mod tests {
    use super::*;
    #[test]
    fn forward_collinear_frames_are_c2_across_speed_knots_without_positional_claim(){
        let profile=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
        let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=crate::primitives::line([0.;3],[0.125,0.,0.]).unwrap();
        let cubic=Curve {degree:3,knots:vec![2.,2.,2.,2.,5.,5.,5.,5.],
            control_points:vec![vec![0.;3],vec![0.,0.,1.],vec![0.,0.,7.],vec![0.,0.,10.]],weights:vec![1.,2.,2.,1.],periodic:false};
        let multispan=Curve {degree:1,knots:vec![2.,2.,3.,5.,5.],
            control_points:vec![vec![0.;3],vec![0.,0.,1.],vec![0.,0.,10.]],weights:vec![1.;3],periodic:false};
        assert!(!basis_continuity(&multispan,1));
        for path in [&cubic,&multispan] {for orientation in [Orientation::RotationMinimizing,Orientation::CorrectedFrenet] {
          for spacing in [Spacing::Parameter,Spacing::ArcLength {tolerance:0.001,max_cells:100000}] {for order in [1,2] {
            let options=Options {normal:[1.,0.,1.],orientation,spacing,initial_sections:3,max_sections:9,max_deviation:1.};
            let audit=|source:&Curve,cells,work|Sweep::new(&profile,source,&scale,&twist,options).unwrap().certify_original_frame_smoothness(order,cells,work).unwrap();
            let proof=audit(path,10000,1000000);
            assert!(proof.source_frame_smoothness_certified,"{orientation:?}/{spacing:?}/{order}/{proof:?}");
            assert!(proof.cells>0&&proof.exact_work>0);
            assert!(!audit(path,proof.cells-1,1000000).source_frame_smoothness_certified);
            assert!(!audit(path,10000,proof.exact_work-1).source_frame_smoothness_certified);
            assert!(!audit(path,0,1000000).source_frame_smoothness_certified);
            assert!(!audit(path,10000,0).source_frame_smoothness_certified);
            let mut backtrack=multispan.clone();backtrack.control_points[1][2]=11.;
            assert!(Sweep::new(&profile,&backtrack,&scale,&twist,options).is_err());
          }}
        }}
    }
    #[test]
    fn corrected_planar_source_frames_extend_through_inflections_with_shared_limits() {
        let profile=crate::primitives::line([0.,0.,1.],[0.,0.,2.]).unwrap();
        let scale=constant_vector_law([1.,0.,0.]).unwrap();
        let twist=crate::primitives::line([0.;3],[0.125,0.,0.]).unwrap();
        let path=crate::paths::bezier(vec![vec![0.,0.,0.],vec![1.,1.,0.],
            vec![2.,-1.,0.],vec![3.,0.,0.]],None).unwrap();
        for rational in [false,true] {
            let mut source=path.clone();
            if rational {source.weights=vec![1.,2.,2.,1.];}
            let before=source.clone();
            assert!(source.evaluate(0.5).unwrap().d2.unwrap().iter().all(|x|*x==0.));
            for order in [1,2] {
                for spacing in [Spacing::Parameter,Spacing::ArcLength{tolerance:0.001,max_cells:100000}] {
                    let options=Options{normal:[0.,0.,1.],orientation:Orientation::CorrectedFrenet,
                        spacing,initial_sections:3,max_sections:9,max_deviation:1.};
                    let audit=|curve:&Curve,cells,work|Sweep::new(&profile,curve,&scale,&twist,options).unwrap()
                        .certify_original_frame_smoothness(order,cells,work).unwrap();
                    let proof=audit(&source,10000,1000000);
                    assert!(proof.source_frame_smoothness_certified,"{rational}/{order}/{spacing:?}: {proof:?}");
                    assert!(proof.cells>0&&proof.exact_work>0);
                    assert!(!audit(&source,proof.cells-1,1000000).source_frame_smoothness_certified);
                    assert!(!audit(&source,10000,proof.exact_work-1).source_frame_smoothness_certified);
                    assert!(!audit(&source,0,1000000).source_frame_smoothness_certified);
                    assert!(!audit(&source,10000,0).source_frame_smoothness_certified);
                    let mut off_plane=source.clone();off_plane.control_points[1][2]=f64::from_bits(1);
                    assert!(!audit(&off_plane,10000,1000000).source_frame_smoothness_certified);
                }
            }
            assert_eq!(source.control_points,before.control_points);
            assert_eq!(source.weights,before.weights);assert_eq!(source.knots,before.knots);
        }
        let stationary=crate::paths::bezier(vec![vec![0.,0.,0.],vec![1.,0.,0.],vec![0.,0.,0.]],None).unwrap();
        // This path is closed; choose an open source with an interior zero speed.
        let singular=crate::paths::bezier(vec![vec![0.,0.,0.],vec![1.,0.,0.],
            vec![0.,0.,0.],vec![1.,0.,0.]],None).unwrap();
        let options=Options{normal:[0.,0.,1.],orientation:Orientation::CorrectedFrenet,
            spacing:Spacing::Parameter,initial_sections:3,max_sections:9,max_deviation:1.};
        for curve in [&stationary,&singular] {
            assert!(!Sweep::new(&profile,curve,&scale,&twist,options).unwrap()
                .certify_original_frame_smoothness(2,10000,1000000).unwrap().source_frame_smoothness_certified);
        }
        let straight=crate::primitives::line([0.;3],[1.,2.,3.]).unwrap();
        assert!(Sweep::new(&profile,&straight,&scale,&twist,options).unwrap()
            .certify_original_frame_smoothness(2,10000,0).unwrap().source_frame_smoothness_certified);
    }
    #[test]
    fn mixed_path_knots_preserve_all_c2_frame_modes_and_shared_budget(){
        let path=Curve{degree:2,knots:vec![0.,0.,0.,0.25,0.5,0.5,1.,1.,1.],
            control_points:vec![vec![0.,0.,0.],vec![0.,0.,0.125],vec![0.,0.125,0.375],
                vec![0.,0.25,0.5],vec![0.,0.5,0.75],vec![0.,1.,1.]],weights:vec![1.;6],periodic:false};
        let profile=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
        let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
        for orientation in [Orientation::FixedNormal,Orientation::Frenet,Orientation::RotationMinimizing,Orientation::CorrectedFrenet]{
            for spacing in [Spacing::Parameter,Spacing::ArcLength{tolerance:0.001,max_cells:100000}]{
                let options=Options{normal:[1.,0.,0.],orientation,spacing,initial_sections:3,max_sections:9,max_deviation:1.};
                let audit=|curve:&Curve,work|Sweep::new(&profile,curve,&scale,&twist,options).unwrap()
                    .certify_original_frame_smoothness(2,10000,work).unwrap();
                let proof=audit(&path,1000000);
                assert!(proof.source_frame_smoothness_certified,"{orientation:?} {spacing:?}: {proof:?}");
                assert!(!audit(&path,proof.exact_work-1).source_frame_smoothness_certified);
                let mut bad=path.clone();bad.control_points[3][1]=bad.control_points[3][1].next_up();
                assert!(!audit(&bad,1000000).source_frame_smoothness_certified);
            }
        }
    }
    #[test]
    fn original_curved_path_high_jets_cover_simple_knots_for_c2_frames(){
        let profile=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
        let scale=constant_vector_law([1.,0.,0.]).unwrap();
        let twist=constant_vector_law([0.;3]).unwrap();
        for rational in [false,true] {
            let path=Curve{degree:2,knots:vec![0.,0.,0.,0.5,1.,1.,1.],
                control_points:if rational{vec![vec![0.,0.,0.],vec![0.,0.,0.375],vec![0.,0.25,0.625],vec![0.,1.,1.]]}
                    else{vec![vec![0.,0.,0.],vec![0.,0.,0.25],vec![0.,0.5,0.75],vec![0.,1.,1.]]},
                weights:if rational{vec![1.,2.,2.,1.]}else{vec![1.;4]},periodic:false};
            assert!(!basis_continuity(&path,3));
            for orientation in [Orientation::FixedNormal,Orientation::Frenet,Orientation::CorrectedFrenet] {
                for spacing in [Spacing::Parameter,Spacing::ArcLength{tolerance:0.001,max_cells:100000}] {
                    let options=Options{normal:[1.,0.,0.],orientation,spacing,initial_sections:3,max_sections:9,max_deviation:1.};
                    let audit=|curve:&Curve,work,cells|Sweep::new(&profile,curve,&scale,&twist,options).unwrap()
                        .certify_original_frame_smoothness(2,cells,work).unwrap();
                    let proof=audit(&path,1000000,10000);
                    assert!(proof.source_frame_smoothness_certified,"{rational} {orientation:?} {spacing:?}: {proof:?}");
                    assert!(proof.exact_work>0);
                    assert!(!audit(&path,proof.exact_work-1,10000).source_frame_smoothness_certified);
                    assert!(!audit(&path,1000000,0).source_frame_smoothness_certified);
                    let mut bad=path.clone();bad.control_points[2][1]=bad.control_points[2][1].next_up();
                    assert!(!audit(&bad,1000000,10000).source_frame_smoothness_certified);
                }
            }
        }
    }
    #[test]
    fn original_multispan_frame_jets_override_basis_continuity_only_when_exact(){
        let profile=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
        let path=crate::primitives::line([0.;3],[0.,0.,4.]).unwrap();
        let scale=constant_vector_law([1.,0.,0.]).unwrap();
        let twist=constant_vector_law([0.;3]).unwrap();
        let normal=constant_vector_law([1.,0.,0.]).unwrap();
        let axis=Curve {degree:2,knots:vec![0.,0.,0.,0.25,0.25,1.,1.,1.],
            control_points:vec![vec![0.,0.,1.],vec![0.125,0.,1.],vec![0.25,0.,1.],vec![0.625,0.,1.],vec![1.,0.,1.]],
            weights:vec![1.;5],periodic:false};
        let before=axis.clone();
        let options=Options {normal:[1.,0.,0.],orientation:Orientation::Fixed,spacing:Spacing::Parameter,
            initial_sections:3,max_sections:9,max_deviation:1.};
        let audit=|law:&Curve,order,work|Sweep::new(&profile,&path,&scale,&twist,options)
            .unwrap().with_frame_laws(law,&normal).unwrap()
            .certify_original_frame_smoothness(order,10000,work).unwrap();
        assert!(!basis_continuity(&axis,2));
        let proof=audit(&axis,2,1000000);
        assert!(proof.source_frame_smoothness_certified,"{proof:?}");
        assert!(proof.exact_work>0);
        assert!(!audit(&axis,2,proof.exact_work-1).source_frame_smoothness_certified);
        assert!(!audit(&axis,2,0).source_frame_smoothness_certified);
        let mut wrong=axis.clone();wrong.control_points[4][0]=1_f64.next_up();
        assert!(!audit(&wrong,2,1000000).source_frame_smoothness_certified);
        assert!(audit(&wrong,1,1000000).source_frame_smoothness_certified);
        wrong=axis.clone();wrong.knots[3]=0.5;wrong.knots[4]=0.5;
        assert!(!audit(&wrong,1,1000000).source_frame_smoothness_certified);
        let rational=Curve{knots:vec![0.,0.,0.,0.5,0.5,1.,1.,1.],
            control_points:vec![vec![0.,0.,1.];5],weights:vec![1.,0.5,1.,1.5,3.],..axis.clone()};
        assert!(audit(&rational,2,1000000).source_frame_smoothness_certified);
        assert_eq!(axis.control_points,before.control_points);assert_eq!(axis.knots,before.knots);
        let sweep=Sweep::new(&profile,&path,&scale,&twist,options).unwrap().with_frame_laws(&axis,&normal).unwrap();
        assert!(!sweep.certify_original_frame_smoothness(2,0,1000000).unwrap().source_frame_smoothness_certified);
    }
    #[test]
    fn original_nonaxial_planar_frame_c2_requires_plane_regular_cover_and_budget() {
        let path = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0.; 3], vec![0.5, -0.5, 0.], vec![1., -1., 1.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let profile = crate::primitives::line([1., 1., 0.], [2., 2., 0.]).unwrap();
        let scale = constant_vector_law([1., 0., 0.]).unwrap();
        let twist = crate::primitives::line([0.; 3], [0.125, 0., 0.]).unwrap();
        for spacing in [
            Spacing::Parameter,
            Spacing::ArcLength {
                tolerance: 0.001,
                max_cells: 100000,
            },
        ] {
            let options = Options {
                normal: [1., 1., 0.],
                orientation: Orientation::RotationMinimizing,
                spacing,
                initial_sections: 3,
                max_sections: 17,
                max_deviation: 2.,
            };
            let sweep = Sweep::new(&profile, &path, &scale, &twist, options).unwrap();
            let proof = sweep
                .certify_original_frame_smoothness(2, 10000, 1000000)
                .unwrap();
            assert!(proof.source_frame_smoothness_certified, "{proof:?}");
            assert!(proof.cells > 0 && proof.exact_work > 0);
            assert!(
                !sweep
                    .certify_original_frame_smoothness(2, proof.cells - 1, 1000000)
                    .unwrap()
                    .source_frame_smoothness_certified
            );
            assert!(
                !sweep
                    .certify_original_frame_smoothness(2, 10000, proof.exact_work - 1)
                    .unwrap()
                    .source_frame_smoothness_certified
            );
            let mut wrong = path.clone();
            wrong.control_points[1][1] = (-0.5_f64).next_up();
            assert!(
                !Sweep::new(&profile, &wrong, &scale, &twist, options)
                    .unwrap()
                    .certify_original_frame_smoothness(2, 10000, 1000000)
                    .unwrap()
                    .source_frame_smoothness_certified
            );
        }
    }
    #[test]
    fn guided_arc_curved_independent_domains_prove_c2_without_inverse_jets() {
        let profile=crate::primitives::line([0.1,0.,0.],[0.2,0.,0.]).unwrap();
        let path=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
            control_points:vec![vec![0.;3],vec![0.,0.,0.5],vec![0.,1.,1.]],weights:vec![1.;3],periodic:false};
        let guide=Curve {degree:2,knots:vec![-3.,-3.,-3.,7.,7.,7.],
            control_points:vec![vec![1.,0.,0.],vec![1.,0.,0.5],vec![1.,2.,1.]],weights:vec![1.;3],periodic:false};
        let scale=constant_vector_law([1.,0.,0.]).unwrap();
        let twist=crate::primitives::line([0.;3],[0.125,0.,0.]).unwrap();
        let options=Options {normal:[1.,0.,0.],orientation:Orientation::RotationMinimizing,
            spacing:Spacing::ArcLength {tolerance:0.001,max_cells:100000},
            initial_sections:3,max_sections:17,max_deviation:2.};
        let sweep=Sweep::new(&profile,&path,&scale,&twist,options).unwrap().with_orientation_guide(&guide).unwrap();
        let proof=sweep.certify_original_frame_smoothness(2,10000,1000000).unwrap();
        assert!(proof.source_frame_smoothness_certified,"{proof:?}");
        assert!(!sweep.certify_original_frame_smoothness(2,proof.cells-1,1000000).unwrap().source_frame_smoothness_certified);
        #[cfg(feature="transport")]
        {
            let request=value_codec::json!({"op":"surface_progressive_sweep_frame_smoothness",
                "profiles":[profile],"path":path,"scale":scale,"twist":twist,"orientation_guide":guide,
                "normal":[1.,0.,0.],"orientation":"rmf","spacing":"arc_length",
                "length_tolerance":0.001,"length_max_cells":100000,
                "initial_sections":3,"max_sections":17,"max_deviation":2.,
                "order":2,"maxCells":10000,"maxExactWork":1000000});
            let transported=crate::transport::dispatch(request).unwrap();
            assert_eq!(transported["sourceFrameSmoothnessCertified"],true);
            assert_eq!(transported["continuousBound"],false);
            assert_eq!(transported["solidCertified"],false);
        }
        let mut rational=guide.clone();rational.weights[1]=0.5;
        let rational_sweep=Sweep::new(&profile,&path,&scale,&twist,options).unwrap().with_orientation_guide(&rational).unwrap();
        let rational_proof=rational_sweep.certify_original_frame_smoothness(2,10000,1000000).unwrap();
        assert!(rational_proof.source_frame_smoothness_certified,"{rational_proof:?}");
        assert!(!rational_sweep.certify_original_frame_smoothness(2,rational_proof.cells-1,1000000).unwrap().source_frame_smoothness_certified);
        let mut crossing=guide.clone();crossing.control_points.iter_mut().for_each(|p|p[0]=0.);
        let unresolved=Sweep::new(&profile,&path,&scale,&twist,options).unwrap().with_orientation_guide(&crossing).unwrap()
            .certify_original_frame_smoothness(2,200,1000000).unwrap();
        assert!(!unresolved.source_frame_smoothness_certified);
    }
    #[test]
    fn guided_arc_c2_requires_both_speed_covers_and_joint_frame_budget() {
        let profile=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
        let path=crate::primitives::line([0.;3],[0.,0.,4.]).unwrap();
        let guide=crate::primitives::line([1.,0.,0.],[1.,0.,4.]).unwrap();
        let scale=constant_vector_law([1.,0.,0.]).unwrap();
        let twist=constant_vector_law([0.;3]).unwrap();
        let options=Options {normal:[1.,0.,0.],orientation:Orientation::RotationMinimizing,
            spacing:Spacing::ArcLength {tolerance:0.001,max_cells:10000},
            initial_sections:3,max_sections:5,max_deviation:0.01};
        let sweep=Sweep::new(&profile,&path,&scale,&twist,options).unwrap().with_orientation_guide(&guide).unwrap();
        let proof=sweep.certify_original_frame_smoothness(2,10000,1000000).unwrap();
        assert!(proof.source_frame_smoothness_certified,"{proof:?}");
        assert!(proof.cells>2);
        assert!(!sweep.certify_original_frame_smoothness(2,proof.cells-1,1000000).unwrap().source_frame_smoothness_certified);
        let kink=Curve {degree:1,knots:vec![0.,0.,0.5,1.,1.],
            control_points:vec![vec![1.,0.,0.],vec![1.,0.,1.],vec![2.,1.,4.]],
            weights:vec![1.;3],periodic:false};
        for (source,rail) in [(&path,&kink)] {
            let refused=Sweep::new(&profile,source,&scale,&twist,options).unwrap().with_orientation_guide(rail).unwrap()
                .certify_original_frame_smoothness(2,10000,1000000).unwrap();
            assert!(!refused.source_frame_smoothness_certified);
            assert_eq!(refused.reason,Some("original-frame-path-knot-continuity-unproved"));
        }
        assert!(Sweep::new(&profile,&kink,&scale,&twist,options).is_err());
        let stopped=constant_vector_law([1.,0.,0.]).unwrap();
        let bad=Sweep::new(&profile,&path,&scale,&twist,options).unwrap().with_orientation_guide(&stopped).unwrap()
            .certify_original_frame_smoothness(2,100,1000000).unwrap();
        assert!(!bad.source_frame_smoothness_certified);
        assert_eq!(bad.reason,Some("guided-arc-source-speed-unproved"));
    }
    #[test]
    fn original_curved_fixed_normal_and_frenet_c2_cover_both_spacings() {
        let path=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
            control_points:vec![vec![0.,0.,0.],vec![0.5,-0.5,0.],vec![1.,-1.,1.]],
            weights:vec![1.;3],periodic:false};
        let profile=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
        let scale=constant_vector_law([1.,0.,0.]).unwrap();
        let twist=constant_vector_law([0.;3]).unwrap();
        for orientation in [Orientation::FixedNormal,Orientation::Frenet] {
            for spacing in [Spacing::Parameter,Spacing::ArcLength {tolerance:0.001,max_cells:100000}] {
                let options=Options {normal:[1.,1.,0.],orientation,spacing,
                    initial_sections:3,max_sections:5,max_deviation:0.01};
                let sweep=Sweep::new(&profile,&path,&scale,&twist,options).unwrap();
                let proof=sweep.certify_original_frame_smoothness(2,10000,1000000).unwrap();
                assert!(proof.source_frame_smoothness_certified,"{orientation:?} {spacing:?}: {proof:?}");
                assert!(!sweep.certify_original_frame_smoothness(2,0,1000000).unwrap().source_frame_smoothness_certified);
            }
        }
    }
    #[test]
    #[cfg(feature = "transport")]
    fn original_frame_transport_preserves_scope_budget_and_closed_refusal() {
        let profile = crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
        let path = crate::primitives::line([0.;3],[0.,0.,4.]).unwrap();
        let scale = constant_vector_law([1.,0.,0.]).unwrap();
        let twist = constant_vector_law([0.;3]).unwrap();
        let request=value_codec::json!({"op":"surface_progressive_sweep_frame_smoothness",
            "profiles":[profile],"path":path,"scale":scale,"twist":twist,
            "normal":[1.,0.,0.],"orientation":"rmf","spacing":"parameter",
            "initial_sections":3,"max_sections":5,"max_deviation":0.01,
            "order":2,"maxCells":10000,"maxExactWork":1000000});
        let proof=crate::transport::dispatch(request.clone()).unwrap();
        assert_eq!(proof["sourceFrameSmoothnessCertified"],true);
        assert_eq!(proof["scope"],"open-original-frame-only");
        for key in ["retainedSeamsCertified","profileJoinsCertified","capJoinsCertified","continuousBound","solidCertified"] {
            assert_eq!(proof[key],false);
        }
        let mut short=request.clone(); short["maxCells"]=value_codec::json!(0);
        assert_eq!(crate::transport::dispatch(short).unwrap()["sourceFrameSmoothnessCertified"],false);
        let mut invalid=request.clone(); invalid["order"]=value_codec::json!(3);
        assert!(crate::transport::dispatch(invalid).is_err());
        let closed=Curve { degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
            control_points:vec![vec![0.,0.,0.],vec![0.,0.,2.],vec![0.,0.,0.]],
            weights:vec![1.;3],periodic:false };
        let mut closed_request=request; closed_request["path"]=value_codec::json!(closed);
        let refused=crate::transport::dispatch(closed_request).unwrap();
        assert_eq!(refused["sourceFrameSmoothnessCertified"],false);
        assert_eq!(refused["reason"],"original-frame-closed-seam-unproved");
    }
    #[test]
    fn original_authored_frame_c2_does_not_follow_from_piecewise_value_cover() {
        let profile = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
        let path = crate::primitives::line([0.; 3], [0., 0., 4.]).unwrap();
        let scale = constant_vector_law([1., 0., 0.]).unwrap();
        let twist = constant_vector_law([0.; 3]).unwrap();
        let axis = crate::primitives::line([0., 0., 1.], [0.25, 0., 1.]).unwrap();
        let normal = crate::primitives::line([1., 0., 0.], [1., 0.25, 0.]).unwrap();
        let options = Options {
            normal: [1., 0., 0.],
            orientation: Orientation::Fixed,
            spacing: Spacing::Parameter,
            initial_sections: 3,
            max_sections: 5,
            max_deviation: 1.,
        };
        let sweep = Sweep::new(&profile, &path, &scale, &twist, options)
            .unwrap()
            .with_frame_laws(&axis, &normal)
            .unwrap();
        assert!(
            sweep
                .certify_original_frame_smoothness(2, 10000, 1000000)
                .unwrap()
                .source_frame_smoothness_certified
        );
        let broken = Curve {
            degree: 1,
            knots: vec![0., 0., 0.5, 1., 1.],
            control_points: vec![vec![1., 0., 0.], vec![1., 0.125, 0.], vec![1., 0.5, 0.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let refused = Sweep::new(&profile, &path, &scale, &twist, options)
            .unwrap()
            .with_frame_laws(&axis, &broken)
            .unwrap()
            .certify_original_frame_smoothness(2, 10000, 1000000)
            .unwrap();
        assert!(!refused.source_frame_smoothness_certified);
        assert_eq!(
            refused.reason,
            Some("original-frame-law-knot-continuity-unproved")
        );
    }
}

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
        if self.frame_laws.is_some()||self.path.periodic||guide.periodic||!path_is_closed(self.path)?||!path_is_closed(guide)?{return Ok(out);}
        for (curve,k) in [(self.path,order+1),(guide,order),(self.twist,order)] {
            if !knot_continuity(curve,k,&mut out.exact_work,max_exact_work)?{out.reason=Some("closed-guided-frame-knot-jets-unproved");return Ok(out);}
            let seam=super::endpoint_jets::certify(curve,k,max_exact_work-out.exact_work)?;out.exact_work+=seam.exact_work;
            if !seam.certified{out.reason=Some("closed-guided-frame-endpoint-jets-unproved");return Ok(out);}
        }
        if self.options.spacing!=Spacing::Parameter {
            for curve in [self.path,guide] {
                let regular=crate::curve_regularity::inspect(curve,max_cells-out.cells)?;out.cells+=regular.cells;
                if !regular.spanwise_regular{out.reason=Some("closed-guided-arc-source-speed-unproved");return Ok(out);}
            }
            let (status,cells)=frame::certify_independent_guided_cover(self.path,guide,self.twist,max_cells-out.cells)?;
            out.cells+=cells;out.closed_source_frame_smoothness_certified=status==Status::Certified;
            out.reason=if out.closed_source_frame_smoothness_certified{None}else{Some("closed-guided-independent-frame-cover-unproved")};
        }else{
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
        if self.path.periodic||!path_is_closed(self.path)?||self.frame_laws.is_some()||self.orientation_guide.is_some()||!matches!(self.options.orientation,Orientation::FixedNormal|Orientation::RotationMinimizing|Orientation::CorrectedFrenet){return Ok(out);}
        // Preserve the cheaper sufficient Cartesian route. Failed work is
        // retained when trying exact direction charts under the same owner.
        let mut path_ok=knot_continuity(self.path,order+1,&mut out.exact_work,max_exact_work)?;
        if path_ok {
            let seam=super::endpoint_jets::certify(self.path,order+1,max_exact_work-out.exact_work)?;
            out.exact_work+=seam.exact_work;path_ok=seam.certified;
        }
        if !path_ok {
            for endpoints in [false,true] {
                let proof=super::endpoint_jets::certify_direction(self.path,order,endpoints,self.options.spacing!=Spacing::Parameter,max_exact_work-out.exact_work)?;
                out.exact_work+=proof.exact_work;
                if !proof.certified {out.reason=Some("closed-path-direction-jets-unproved");return Ok(out);}
            }
        }
        if !knot_continuity(self.twist,order,&mut out.exact_work,max_exact_work)? {out.reason=Some("closed-path-frame-knot-jets-unproved");return Ok(out);}
        let seam=super::endpoint_jets::certify(self.twist,order,max_exact_work-out.exact_work)?;
        out.exact_work+=seam.exact_work;
        if !seam.certified {out.reason=Some("closed-path-frame-endpoint-jets-unproved");return Ok(out);}
        if self.options.orientation!=Orientation::FixedNormal {
            let plane=super::source_plane::certify(self.path,self.options.normal,max_exact_work-out.exact_work)?;
            out.exact_work+=plane.exact_work;
            if !plane.proved {out.reason=plane.reason;return Ok(out);}
        }
        // Complete cover proves nonzero speed and nonparallel constant seed.
        // Arc composition follows the regular inverse map with matching seam
        // jets. No numerical inverse-jet or retained frame proof is inferred.
        let cover=frame::certify_fixed_normal_cover(self.path,self.options.normal,self.twist,max_cells)?;
        out.cells=cover.cells;
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
mod closed_authored_tests {
    use super::*;
    #[test]
    fn closed_guided_frames_own_both_seams_independent_speed_and_joint_cover(){
        let path=Curve {degree:7,knots:[vec![2.;8],vec![5.;8]].concat(),
            control_points:vec![vec![0.,0.,0.],vec![1.,0.,0.],vec![2.,1.,0.],vec![3.,3.,0.],
                vec![-3.,3.,0.],vec![-2.,1.,0.],vec![-1.,0.,0.],vec![0.,0.,0.]],weights:vec![1.;8],periodic:false};
        let mut guide=path.clone();guide.knots=[vec![-3.;8],vec![7.;8]].concat();
        for p in &mut guide.control_points {p[2]=1.;}
        let profile=crate::primitives::line([0.,0.,0.1],[0.,0.,0.2]).unwrap();
        let scale=constant_vector_law([1.,0.,0.]).unwrap();
        let twist=Curve {degree:5,knots:[vec![7.;6],vec![9.;6]].concat(),
            control_points:[0.,0.125,0.25,-0.25,-0.125,0.].into_iter().map(|x|vec![x,0.,0.]).collect(),weights:vec![1.;6],periodic:false};
        for spacing in [Spacing::Parameter,Spacing::ArcLength {tolerance:0.001,max_cells:100000}] {for order in [1,2] {
            let opts=Options {normal:[0.,0.,1.],orientation:Orientation::RotationMinimizing,spacing,initial_sections:5,max_sections:17,max_deviation:1.};
            let audit=|g:&Curve,cells,work|Sweep::new(&profile,&path,&scale,&twist,opts).unwrap().with_orientation_guide(g).unwrap()
                .certify_closed_guided_frame_smoothness(order,cells,work).unwrap();
            let proof=audit(&guide,10000,1000000);
            assert!(proof.closed_source_frame_smoothness_certified,"{spacing:?}/{order}/{proof:?}");
            assert!(proof.cells>0&&proof.exact_work>0);
            assert!(!audit(&guide,proof.cells-1,1000000).closed_source_frame_smoothness_certified);
            assert!(!audit(&guide,10000,proof.exact_work-1).closed_source_frame_smoothness_certified);
            assert!(!audit(&guide,0,1000000).closed_source_frame_smoothness_certified);
            assert!(!audit(&guide,10000,0).closed_source_frame_smoothness_certified);
            assert!(!audit(&path,10000,1000000).closed_source_frame_smoothness_certified);
            let mut damaged=guide.clone();damaged.control_points[2][2]=1_f64.next_up();
            assert_eq!(audit(&damaged,10000,1000000).closed_source_frame_smoothness_certified,order==1);
            let stopped=constant_vector_law([0.,0.,1.]).unwrap();
            let stopped_sweep=Sweep::new(&profile,&path,&scale,&twist,opts).unwrap().with_orientation_guide(&stopped);
            let stopped_proof=stopped_sweep.and_then(|s|s.certify_closed_guided_frame_smoothness(order,10000,1000000));
            assert!(stopped_proof.is_err()||!stopped_proof.unwrap().closed_source_frame_smoothness_certified);
        }}
    }
    #[test]
    #[cfg(feature="transport")]
    fn closed_guided_frame_transport_preserves_scope_and_partial_budget_refusal(){
        let path=Curve {degree:7,knots:[vec![2.;8],vec![5.;8]].concat(),
            control_points:vec![vec![0.,0.,0.],vec![1.,0.,0.],vec![2.,1.,0.],vec![3.,3.,0.],
                vec![-3.,3.,0.],vec![-2.,1.,0.],vec![-1.,0.,0.],vec![0.,0.,0.]],weights:vec![1.;8],periodic:false};
        let profile=crate::primitives::line([0.,0.,0.1],[0.,0.,0.2]).unwrap();
        let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
        let mut guide=path.clone();for p in &mut guide.control_points {p[2]=1.;}
        let request=value_codec::json!({"op":"surface_progressive_sweep_closed_guided_frame_smoothness",
            "profiles":[profile],"path":path,"orientation_guide":guide,"scale":scale,"twist":twist,"normal":[0.,0.,1.],
            "orientation":"rmf","spacing":"parameter","initial_sections":3,"max_sections":17,"max_deviation":1.,
            "order":2,"maxCells":10000,"maxExactWork":1000000});
        let proof=crate::transport::dispatch(request.clone()).unwrap();
        assert_eq!(proof["closedSourceFrameSmoothnessCertified"],true);
        assert_eq!(proof["scope"],"closed-original-guided-frame-only");
        for key in ["pathSeamCertified","retainedSeamsCertified","profileJoinsCertified","capJoinsCertified","continuousBound","solidCertified"] {
            assert_eq!(proof[key],false);
        }
        for (field,value) in [("maxCells",0),("maxExactWork",0),
            ("maxCells",proof["cells"].as_u64().unwrap()-1),("maxExactWork",proof["exactWork"].as_u64().unwrap()-1)] {
            let mut short=request.clone();short[field]=value_codec::json!(value);
            assert_eq!(crate::transport::dispatch(short).unwrap()["closedSourceFrameSmoothnessCertified"],false);
        }
    }
    #[test]
    #[cfg(feature="transport")]
    fn closed_path_frame_transport_preserves_scope_and_partial_budget_refusal(){
        let path=Curve {degree:7,knots:[vec![2.;8],vec![5.;8]].concat(),
            control_points:vec![vec![0.,0.,0.],vec![1.,0.,0.],vec![2.,1.,0.],vec![3.,3.,0.],
                vec![-3.,3.,0.],vec![-2.,1.,0.],vec![-1.,0.,0.],vec![0.,0.,0.]],weights:vec![1.;8],periodic:false};
        let profile=crate::primitives::line([0.,0.,0.1],[0.,0.,0.2]).unwrap();
        let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
        let request=value_codec::json!({"op":"surface_progressive_sweep_closed_path_frame_smoothness",
            "profiles":[profile],"path":path,"scale":scale,"twist":twist,"normal":[0.,0.,1.],
            "orientation":"rmf","spacing":"parameter","initial_sections":3,"max_sections":17,"max_deviation":1.,
            "order":2,"maxCells":10000,"maxExactWork":1000000});
        let proof=crate::transport::dispatch(request.clone()).unwrap();
        assert_eq!(proof["closedSourceFrameSmoothnessCertified"],true);
        assert_eq!(proof["scope"],"closed-original-path-frame-only");
        for key in ["pathSeamCertified","retainedSeamsCertified","profileJoinsCertified","capJoinsCertified","continuousBound","solidCertified"] {
            assert_eq!(proof[key],false);
        }
        for (field,value) in [("maxCells",0),("maxExactWork",0),
            ("maxCells",proof["cells"].as_u64().unwrap()-1),("maxExactWork",proof["exactWork"].as_u64().unwrap()-1)] {
            let mut short=request.clone();short[field]=value_codec::json!(value);
            assert_eq!(crate::transport::dispatch(short).unwrap()["closedSourceFrameSmoothnessCertified"],false);
        }
    }
    #[test]
    fn closed_path_frames_need_exact_c3_seam_full_cover_and_planar_transport(){
        let profile=crate::primitives::line([0.,0.,0.1],[0.,0.,0.2]).unwrap();
        let scale=constant_vector_law([1.,0.,0.]).unwrap();
        let twist=Curve {degree:5,knots:[vec![7.;6],vec![9.;6]].concat(),
            control_points:[0.,0.125,0.25,-0.25,-0.125,0.].into_iter().map(|x|vec![x,0.,0.]).collect(),
            weights:vec![1.;6],periodic:false};
        let path=Curve {degree:7,knots:[vec![2.;8],vec![5.;8]].concat(),
            control_points:vec![vec![0.,0.,0.],vec![1.,0.,0.],vec![2.,1.,0.],vec![3.,3.,0.],
                vec![-3.,3.,0.],vec![-2.,1.,0.],vec![-1.,0.,0.],vec![0.,0.,0.]],weights:vec![1.;8],periodic:false};
        let rational=Curve {degree:9,knots:[vec![2.;10],vec![5.;10]].concat(),
            control_points:vec![vec![0.,0.,0.],vec![1.,0.,0.],vec![2.,1.,0.],vec![3.,3.,0.],vec![2.,4.,0.],
                vec![-2.,4.,0.],vec![-3.,3.,0.],vec![-2.,1.,0.],vec![-1.,0.,0.],vec![0.,0.,0.]],
            weights:vec![1.,1.,1.,1.,2.,2.,1.,1.,1.,1.],periodic:false};
        for path in [&path,&rational] {
        for orientation in [Orientation::FixedNormal,Orientation::RotationMinimizing,Orientation::CorrectedFrenet] {
          for spacing in [Spacing::Parameter,Spacing::ArcLength {tolerance:0.001,max_cells:100000}] {
            let opts=Options {normal:[0.,0.,1.],orientation,spacing,initial_sections:3,max_sections:17,max_deviation:1.};
            let audit=|p:&Curve,order,cells,work|Sweep::new(&profile,p,&scale,&twist,opts).unwrap().certify_closed_path_frame_smoothness(order,cells,work).unwrap();
            let proof=audit(&path,2,10000,1000000);
            assert!(proof.closed_source_frame_smoothness_certified,"{orientation:?}/{spacing:?}/{proof:?}");
            assert!(proof.cells>0&&proof.exact_work>0);
            assert!(!audit(&path,2,proof.cells-1,1000000).closed_source_frame_smoothness_certified);
            assert!(!audit(&path,2,10000,proof.exact_work-1).closed_source_frame_smoothness_certified);
            assert!(!audit(&path,2,0,1000000).closed_source_frame_smoothness_certified);
            assert!(!audit(&path,2,10000,0).closed_source_frame_smoothness_certified);
            let mut bad=path.clone();bad.control_points[3][1]=3_f64.next_up();
            assert!(!audit(&bad,2,10000,1000000).closed_source_frame_smoothness_certified);
            assert!(audit(&bad,1,10000,1000000).closed_source_frame_smoothness_certified);
            let mut spatial=path.clone();spatial.control_points[3][2]=1.;
            let opposite=spatial.control_points.len()-4;spatial.control_points[opposite][2]=-1.;
            assert_eq!(audit(&spatial,2,10000,1000000).closed_source_frame_smoothness_certified,orientation==Orientation::FixedNormal);
            let mut bad_twist=twist.clone();bad_twist.control_points[2][0]=0.25_f64.next_up();
            let damaged=Sweep::new(&profile,&path,&scale,&bad_twist,opts).unwrap();
            assert!(!damaged.certify_closed_path_frame_smoothness(2,10000,1000000).unwrap().closed_source_frame_smoothness_certified);
            assert!(damaged.certify_closed_path_frame_smoothness(1,10000,1000000).unwrap().closed_source_frame_smoothness_certified);
          }
        }
        }
    }
    #[test]
    fn closed_cartesian_frame_jets_need_no_constant_homogeneous_seam_scale(){
        let profile=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
        let path=crate::primitives::circle([0.;3],[0.,0.,1.],4.).unwrap();
        let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
        let normal=constant_vector_law([1.,0.,0.]).unwrap();
        let moving=Curve{degree:5,knots:[vec![0.;6],vec![1.;6]].concat(),
            control_points:vec![vec![0.,0.,1.],vec![0.125,0.,1.],vec![0.046875,0.125,1.],
                vec![-0.28125,0.125,1.],vec![-0.03125,0.,1.],vec![0.,0.,1.]],
            weights:vec![1.,0.5,1.,1.,2.,1.],periodic:false};
        let nonclamped=Curve{degree:2,knots:vec![-2.,-1.,0.,0.5,1.,2.,3.],
            control_points:vec![vec![0.,0.,1.];4],weights:vec![1.,2.,3.,4.],periodic:false};
        for axis in [&moving,&nonclamped]{
            for spacing in [Spacing::Parameter,Spacing::ArcLength{tolerance:0.001,max_cells:100000}]{
                let options=Options {normal:[1.,0.,0.],orientation:Orientation::Fixed,spacing,
                    initial_sections:5,max_sections:17,max_deviation:2.};
                let audit=|law:&Curve,order,work|Sweep::new(&profile,&path,&scale,&twist,options).unwrap()
                    .with_frame_laws(law,&normal).unwrap().certify_closed_authored_frame_smoothness(order,10000,work).unwrap();
                let proof=audit(axis,2,1000000);
                assert!(proof.closed_source_frame_smoothness_certified,"{spacing:?}: {proof:?}");
                assert!(proof.exact_work>0);
                assert!(!audit(axis,2,proof.exact_work-1).closed_source_frame_smoothness_certified);
                assert!(!audit(axis,2,0).closed_source_frame_smoothness_certified);
            }
        }
        let options=Options {normal:[1.,0.,0.],orientation:Orientation::Fixed,spacing:Spacing::Parameter,
            initial_sections:5,max_sections:17,max_deviation:2.};
        let mut bad=moving.clone();bad.control_points[3][0]=bad.control_points[3][0].next_up();
        let sweep=Sweep::new(&profile,&path,&scale,&twist,options).unwrap().with_frame_laws(&bad,&normal).unwrap();
        assert!(!sweep.certify_closed_authored_frame_smoothness(2,10000,1000000).unwrap().closed_source_frame_smoothness_certified);
        assert!(sweep.certify_closed_authored_frame_smoothness(1,10000,1000000).unwrap().closed_source_frame_smoothness_certified);
    }
    #[test]
    fn closed_piecewise_bezier_frame_needs_internal_and_endpoint_jets(){
        let profile=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
        let path=crate::primitives::circle([0.;3],[0.,0.,1.],4.).unwrap();
        let scale=constant_vector_law([1.,0.,0.]).unwrap();
        let twist=constant_vector_law([0.;3]).unwrap();
        let normal=constant_vector_law([1.,0.,0.]).unwrap();
        let axis=Curve{degree:5,knots:[vec![0.;6],vec![0.5;5],vec![1.;6]].concat(),
            control_points:vec![vec![0.,0.,1.],vec![0.0625,0.,1.],vec![0.125,0.03125,1.],
                vec![0.109375,0.0625,1.],vec![0.0546875,0.078125,1.],vec![0.,0.078125,1.],
                vec![-0.0546875,0.078125,1.],vec![-0.109375,0.0625,1.],vec![-0.125,0.03125,1.],
                vec![-0.0625,0.,1.],vec![0.,0.,1.]],weights:vec![1.;11],periodic:false};
        for spacing in [Spacing::Parameter,Spacing::ArcLength{tolerance:0.001,max_cells:100000}]{
            let options=Options {normal:[1.,0.,0.],orientation:Orientation::Fixed,spacing,
                initial_sections:5,max_sections:17,max_deviation:2.};
            let audit=|law:&Curve,order,work|Sweep::new(&profile,&path,&scale,&twist,options)
                .unwrap().with_frame_laws(law,&normal).unwrap()
                .certify_closed_authored_frame_smoothness(order,10000,work).unwrap();
            assert!(!basis_continuity(&axis,2));
            let proof=audit(&axis,2,1000000);
            assert!(proof.closed_source_frame_smoothness_certified,"{spacing:?}: {proof:?}");
            assert!(!audit(&axis,2,proof.exact_work-1).closed_source_frame_smoothness_certified);
            let mut bad=axis.clone();bad.control_points[7][0]=bad.control_points[7][0].next_up();
            assert!(!audit(&bad,2,1000000).closed_source_frame_smoothness_certified);
            assert!(audit(&bad,1,1000000).closed_source_frame_smoothness_certified);
            bad=axis.clone();bad.control_points[1][0]=bad.control_points[1][0].next_up();
            assert!(!audit(&bad,1,1000000).closed_source_frame_smoothness_certified);
        }
    }
    #[test]
    fn moving_closed_authored_c2_needs_exact_seam_and_full_regular_cover(){
        let profile=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
        let path=crate::primitives::circle([0.;3],[0.,0.,1.],4.).unwrap();
        let scale=constant_vector_law([1.,0.,0.]).unwrap();
        let twist=constant_vector_law([0.;3]).unwrap();
        let normal=constant_vector_law([1.,0.,0.]).unwrap();
        let axis=Curve {degree:5,knots:vec![0.,0.,0.,0.,0.,0.,1.,1.,1.,1.,1.,1.],
            control_points:vec![vec![0.,0.,1.],vec![0.125,0.,1.],vec![0.25,0.125,1.],
                vec![-0.25,0.125,1.],vec![-0.125,0.,1.],vec![0.,0.,1.]],
            weights:vec![1.;6],periodic:false};
        let options=Options {normal:[1.,0.,0.],orientation:Orientation::Fixed,spacing:Spacing::Parameter,
            initial_sections:5,max_sections:17,max_deviation:2.};
        let sweep=Sweep::new(&profile,&path,&scale,&twist,options).unwrap().with_frame_laws(&axis,&normal).unwrap();
        let proof=sweep.certify_closed_authored_frame_smoothness(2,10000,1000000).unwrap();
        assert!(proof.closed_source_frame_smoothness_certified,"{proof:?}");
        #[cfg(feature="transport")]
        {
            let request=value_codec::json!({"op":"surface_progressive_sweep_closed_frame_smoothness",
                "profiles":[profile],"path":path,"scale":scale,"twist":twist,
                "frame_axis":axis,"frame_normal":normal,"normal":[1.,0.,0.],
                "orientation":"authored","spacing":"parameter","initial_sections":5,
                "max_sections":17,"max_deviation":2.,"order":2,"maxCells":10000,"maxExactWork":1000000});
            let transported=crate::transport::dispatch(request.clone()).unwrap();
            assert_eq!(transported["closedSourceFrameSmoothnessCertified"],true);
            assert_eq!(transported["scope"],"closed-original-authored-frame-only");
            for key in ["pathSeamCertified","retainedSeamsCertified","continuousBound","solidCertified"] {
                assert_eq!(transported[key],false);
            }
            let mut short=request;short["maxExactWork"]=value_codec::json!(proof.exact_work-1);
            assert_eq!(crate::transport::dispatch(short).unwrap()["closedSourceFrameSmoothnessCertified"],false);
        }

        assert!(!sweep.certify_closed_authored_frame_smoothness(2,proof.cells-1,1000000).unwrap().closed_source_frame_smoothness_certified);
        assert!(!sweep.certify_closed_authored_frame_smoothness(2,10000,proof.exact_work-1).unwrap().closed_source_frame_smoothness_certified);
        let mut wrong=axis.clone();wrong.control_points[2][0]=0.25_f64.next_up();
        let bad=Sweep::new(&profile,&path,&scale,&twist,options).unwrap().with_frame_laws(&wrong,&normal).unwrap();
        assert!(!bad.certify_closed_authored_frame_smoothness(2,10000,1000000).unwrap().closed_source_frame_smoothness_certified);
        assert!(bad.certify_closed_authored_frame_smoothness(1,10000,1000000).unwrap().closed_source_frame_smoothness_certified);
        assert!(!sweep.certify_original_frame_smoothness(2,10000,1000000).unwrap().source_frame_smoothness_certified);
        let multi=Curve {degree:3,knots:vec![0.,0.,0.,0.,0.25,0.5,0.75,1.,1.,1.,1.],
            control_points:vec![vec![0.,0.,1.],vec![0.125,0.,1.],vec![0.375,0.125,1.],
                vec![0.,0.25,1.],vec![-0.375,0.125,1.],vec![-0.125,0.,1.],vec![0.,0.,1.]],
            weights:vec![1.;7],periodic:false};
        let multi_sweep=Sweep::new(&profile,&path,&scale,&twist,options).unwrap().with_frame_laws(&multi,&normal).unwrap();
        let multi_proof=multi_sweep.certify_closed_authored_frame_smoothness(2,10000,1000000).unwrap();
        assert!(multi_proof.closed_source_frame_smoothness_certified,"{multi_proof:?}");
        assert!(!multi_sweep.certify_closed_authored_frame_smoothness(2,10000,multi_proof.exact_work-1).unwrap().closed_source_frame_smoothness_certified);
        #[cfg(feature="transport")]
        {
            let request=value_codec::json!({"op":"surface_progressive_sweep_closed_frame_smoothness",
                "profiles":[profile],"path":path,"scale":scale,"twist":twist,
                "frame_axis":multi,"frame_normal":normal,"normal":[1.,0.,0.],
                "orientation":"authored","spacing":"parameter","initial_sections":5,
                "max_sections":17,"max_deviation":2.,"order":2,"maxCells":10000,"maxExactWork":1000000});
            let report=crate::transport::dispatch(request.clone()).unwrap();
            assert_eq!(report["closedSourceFrameSmoothnessCertified"],true);
            assert_eq!(report["exactWork"],value_codec::json!(multi_proof.exact_work));
            assert_eq!(report["scope"],"closed-original-authored-frame-only");
            for key in ["pathSeamCertified","retainedSeamsCertified","continuousBound","solidCertified"] {
                assert_eq!(report[key],false);
            }
            let mut short=request;short["maxExactWork"]=value_codec::json!(multi_proof.exact_work-1);
            assert_eq!(crate::transport::dispatch(short).unwrap()["closedSourceFrameSmoothnessCertified"],false);
        }
        let mut rational_multi=multi.clone();rational_multi.weights[3]=2.;
        for u in &mut rational_multi.knots {*u=-3.+8.* *u;}
        let rational_multi=Sweep::new(&profile,&path,&scale,&twist,options).unwrap().with_frame_laws(&rational_multi,&normal).unwrap();
        assert!(rational_multi.certify_closed_authored_frame_smoothness(2,10000,1000000).unwrap().closed_source_frame_smoothness_certified);
        let mut projective_multi=multi.clone();projective_multi.weights=vec![1.,1.,1.,1.5,2.,2.,2.];
        let projective=Sweep::new(&profile,&path,&scale,&twist,options).unwrap().with_frame_laws(&projective_multi,&normal).unwrap();
        let projective_proof=projective.certify_closed_authored_frame_smoothness(2,10000,1000000).unwrap();
        assert!(projective_proof.closed_source_frame_smoothness_certified,"{projective_proof:?}");
        assert!(!projective.certify_closed_authored_frame_smoothness(2,10000,projective_proof.exact_work-1).unwrap().closed_source_frame_smoothness_certified);
        projective_multi.control_points[2][0]=0.375_f64.next_up();
        let projective_bad=Sweep::new(&profile,&path,&scale,&twist,options).unwrap().with_frame_laws(&projective_multi,&normal).unwrap();
        assert!(!projective_bad.certify_closed_authored_frame_smoothness(2,10000,1000000).unwrap().closed_source_frame_smoothness_certified);
        assert!(projective_bad.certify_closed_authored_frame_smoothness(1,10000,1000000).unwrap().closed_source_frame_smoothness_certified);
        // Matching endpoint jets do not promote an internal C0-only knot to C2.
        let mut c0_only=multi.clone();
        c0_only.knots.splice(6..6,[0.5,0.5]);
        c0_only.control_points.splice(3..3,[vec![0.,0.25,1.],vec![0.,0.25,1.]]);
        c0_only.weights.splice(3..3,[1.,1.]);
        assert!(super::super::endpoint_jets::certify(&c0_only,2,1000000).unwrap().certified);
        let c0_only=Sweep::new(&profile,&path,&scale,&twist,options).unwrap().with_frame_laws(&c0_only,&normal).unwrap();
        let refused=c0_only.certify_closed_authored_frame_smoothness(2,10000,1000000).unwrap();
        assert!(!refused.closed_source_frame_smoothness_certified);
        assert_eq!(refused.reason,Some("closed-authored-law-knot-continuity-unproved"));
        let mut damaged=multi.clone();damaged.control_points[2][0]=0.375_f64.next_up();
        let damaged=Sweep::new(&profile,&path,&scale,&twist,options).unwrap().with_frame_laws(&damaged,&normal).unwrap();
        assert!(!damaged.certify_closed_authored_frame_smoothness(2,10000,1000000).unwrap().closed_source_frame_smoothness_certified);
        assert!(damaged.certify_closed_authored_frame_smoothness(1,10000,1000000).unwrap().closed_source_frame_smoothness_certified);
        let mut projective_bezier=axis.clone();projective_bezier.weights=vec![1.,1.,1.,2.,2.,2.];
        let projective_bezier=Sweep::new(&profile,&path,&scale,&twist,options).unwrap().with_frame_laws(&projective_bezier,&normal).unwrap();
        assert!(projective_bezier.certify_closed_authored_frame_smoothness(2,10000,1000000).unwrap().closed_source_frame_smoothness_certified);
        let mut rational=axis.clone();
        rational.knots=vec![-3.,-3.,-3.,-3.,-3.,-3.,7.,7.,7.,7.,7.,7.];
        rational.weights=vec![1.,1.5,2.5,0.5,0.5,1.];
        rational.control_points=vec![vec![0.,0.,1.],vec![0.0625,0.,1.],vec![0.125,0.0625,1.],
            vec![-0.125,0.3125,1.],vec![-0.1875,0.,1.],vec![0.,0.,1.]];
        let arc_options=Options {spacing:Spacing::ArcLength {tolerance:0.001,max_cells:100000},..options};
        let arc=Sweep::new(&profile,&path,&scale,&twist,arc_options).unwrap().with_frame_laws(&rational,&normal).unwrap();
        let arc_proof=arc.certify_closed_authored_frame_smoothness(2,10000,1000000).unwrap();
        assert!(arc_proof.closed_source_frame_smoothness_certified,"{arc_proof:?}");
        assert!(!arc.certify_closed_authored_frame_smoothness(2,10000,arc_proof.exact_work-1).unwrap().closed_source_frame_smoothness_certified);

    }
}

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
