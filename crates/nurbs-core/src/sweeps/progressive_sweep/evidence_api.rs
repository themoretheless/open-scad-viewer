use super::*;

impl<'a> Sweep<'a> {
/// Certify authored direction regularity over the entire normalized law
    /// traversal, independently of the preview station grid. No report is
    /// returned for transport modes without authored direction laws. This is
    /// one premise of a continuous sweep bound, not that complete bound.
    pub fn authored_frame_regularity(
        &self,
        max_cells: usize,
    ) -> Result<Option<crate::sweeps::progressive_miter::authored_frame_certificate::RegularityReport>> {
        self.frame_laws
            .map(|(axis, normal)| {
                crate::sweeps::progressive_miter::authored_frame_certificate::certify_regularity(
                    axis, normal, max_cells,
                )
            })
            .transpose()
    }

    /// Full twisted-frame jets for later retained-wall error composition.
    /// Publishing this premise does not certify the complete sweep boundary.
    pub fn authored_frame_jet_cover(
        &self,
        max_cells: usize,
    ) -> Result<Option<crate::sweeps::progressive_miter::authored_frame_certificate::RegularityReport>> {
        self.frame_laws
            .map(|(axis, normal)| {
                crate::sweeps::progressive_miter::authored_frame_certificate::certify_twisted_cover(
                    axis, normal, self.twist, max_cells,
                )
            })
            .transpose()
    }

    /// Original-law initial profile coordinates including outward arithmetic.
    /// A separate premise, not a retained-wall or complete boundary bound.
    pub fn authored_initial_coordinates(&self, max_cells: usize) -> Result<InitialCoordinatesReport> {
        authored_error::initial_coordinates(self, max_cells)
    }

    pub fn fixed_initial_coordinates(&self,max_cells:usize)->Result<InitialCoordinatesReport>{
        authored_error::fixed_initial_coordinates(self,max_cells)
    }
    pub fn fixed_patch_error_bound(&self,count:usize,max_cells:usize,max_products:usize)->Result<PatchErrorReport>{
        authored_error::fixed_patch_error(self,count,max_cells,max_products)
    }
    /// Original straight-axis RMF transport only; general RMF remains unresolved.
    pub fn rmf_straight_patch_error_bound(&self,count:usize,max_cells:usize,max_products:usize)->Result<PatchErrorReport>{
        authored_error::rmf_straight_patch_error(self,count,max_cells,max_products)
    }
    /// Explicit original spatial RMF integration resolution and shared work.
    /// This proves retained patch error only; global/cap/Solid obligations stay separate.
    pub fn with_spatial_rmf_error_limits(mut self,steps:usize,max_cells:usize,max_products:usize)->Result<Self>{
        check(steps.is_power_of_two() && (2..=16384).contains(&steps)
            && max_cells<=100000 && max_products<=1000000,
            "Invalid spatial RMF proof limits")?;
        check(self.options.orientation==Orientation::RotationMinimizing && self.frame_laws.is_none()
            && self.orientation_guide.is_none() && self.contact_point.is_none(),
            "Spatial RMF proof limits require an unguided RMF source")?;
        self.spatial_rmf_error_limits=Some([steps,max_cells,max_products]);Ok(self)
    }
    pub fn rmf_spatial_patch_error_bound(&self,count:usize,transport_steps:usize,max_cells:usize,max_products:usize)->Result<PatchErrorReport>{
        authored_error::rmf_spatial_patch_error(self,count,transport_steps,max_cells,max_products)
    }
    pub fn rmf_planar_patch_error_bound(&self,count:usize,max_cells:usize,max_products:usize)->Result<PatchErrorReport>{
        authored_error::rmf_planar_patch_error(self,count,max_cells,max_products)
    }
    pub fn fixed_normal_patch_error_bound(&self,count:usize,max_cells:usize,max_products:usize)->Result<PatchErrorReport>{
        authored_error::fixed_normal_patch_error(self,count,max_cells,max_products)
    }
    pub fn frenet_patch_error_bound(&self,count:usize,max_cells:usize,max_products:usize)->Result<PatchErrorReport>{
        authored_error::frenet_patch_error(self,count,max_cells,max_products)
    }

    /// Guided initial basis before twist/affine laws; not a contact/error proof.
    pub fn guided_initial_coordinates(&self,max_cells:usize)->Result<InitialCoordinatesReport> {
        authored_error::guided_initial_coordinates(self,max_cells)
    }

    /// Original selected profile point in the initial guided basis.
    /// This premise does not establish continuous rail contact or retained error.
    pub fn contact_anchor_bound(&self, max_cells: usize) -> Result<ContactAnchorReport> {
        contact_anchor::certify(self, max_cells)
    }

    /// Constructor-owned contact-width quotient jets; not retained geometry E.
    pub fn contact_fit_jet(&self, traversal: [f64; 2], max_cells: usize) -> Result<ContactFitReport> {
        contact_anchor::fit(self, traversal, max_cells)
    }

    /// Original contact control jets with constructor-owned coordinates/anchor.
    /// This does not certify retained interpolation or continuous rail contact.
    pub fn contact_control_trajectory(&self, control: usize, traversal: [f64; 2], max_cells: usize) -> Result<crate::sweeps::progressive_miter::authored_frame_certificate::TrajectoryReport> {
        contact_anchor::control_trajectory(self, control, traversal, max_cells)
    }

    /// Original contact control value, including point-safe station evaluation.
    pub fn contact_control_value(&self, control: usize, traversal: [f64; 2], max_cells: usize) -> Result<crate::sweeps::progressive_miter::authored_frame_certificate::ControlValueReport> {
        contact_anchor::control_value(self, control, traversal, max_cells)
    }

    /// Original guided control image; contact/closed/arc-length proofs remain separate.
    pub fn guided_control_value(&self,profile_control:usize,traversal:[f64;2],max_cells:usize)->Result<crate::sweeps::progressive_miter::authored_frame_certificate::ControlValueReport> {
        authored_error::guided_control_value(self,profile_control,traversal,max_cells)
    }

    /// Constructor-owned ideal control trajectory, including initial-coordinate
    /// enclosures. Retained interpolation and endpoint displacement are separate.
    pub fn authored_control_trajectory(
        &self,
        profile_control: usize,
        traversal: [f64; 2],
        max_cells: usize,
    ) -> Result<crate::sweeps::progressive_miter::authored_frame_certificate::TrajectoryReport> {
        authored_error::control_trajectory(self, profile_control, traversal, max_cells)
    }

    /// Complete original-profile section interpolation bound before retained
    /// profile decomposition. This is not the complete patch/boundary bound.
    pub fn authored_section_interpolation_bound(&self, count: usize, max_cells: usize) -> Result<SectionInterpolationReport> {
        authored_error::section_interpolation(self, count, max_cells)
    }
    /// Whole original-profile guided interpolation, before decomposition/caps.
    pub fn guided_section_interpolation_bound(&self,count:usize,max_cells:usize)->Result<SectionInterpolationReport> {
        authored_error::guided_section_interpolation(self,count,max_cells)
    }
    /// Contact fitted original-profile interpolation; caps/contact identity separate.
    pub fn contact_section_interpolation_bound(&self,count:usize,max_cells:usize)->Result<SectionInterpolationReport> {
        authored_error::contact_section_interpolation(self,count,max_cells)
    }
    pub fn authored_patch_error_bound(&self, count: usize, max_cells: usize, max_products: usize) -> Result<PatchErrorReport> {
        authored_error::patch_error(self,count,max_cells,max_products)
    }

    /// Guided retained-patch bound including source-profile decomposition.
    pub fn guided_patch_error_bound(&self,count:usize,max_cells:usize,max_products:usize)->Result<PatchErrorReport> {
        authored_error::guided_patch_error(self,count,max_cells,max_products)
    }
    /// Contact fitted retained patches including original profile decomposition.
    pub fn contact_patch_error_bound(&self,count:usize,max_cells:usize,max_products:usize)->Result<PatchErrorReport> {
        authored_error::contact_patch_error(self,count,max_cells,max_products)
    }
}
