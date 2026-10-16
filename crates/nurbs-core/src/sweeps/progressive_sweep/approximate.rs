use super::*;

pub fn approximate(
    profile: &Curve,
    path: &Curve,
    scale: &Curve,
    twist: &Curve,
    options: Options,
) -> Result<Approximation> {
    let sweep = Sweep::new(profile, path, scale, twist, options)?;
    let mut levels = Vec::new();
    let mut patches = None;
    for level in sweep {
        let level = level?;
        if level.report.accepted {
            patches = Some(level.patches);
        }
        levels.push(level.report);
    }
    Ok(Approximation { patches, levels })
}

/// Multiple authored profile curves transported on one shared refinement grid.
/// Curves may describe boundary pieces or separate contours. This operation does
/// not infer nesting, sew adjacent boundaries, add caps, or certify a solid.
#[derive(Clone, Debug)]
pub struct MultiLevel {
    pub patches: Vec<Surface>,
    /// Half-open patch ranges, one per input curve, in authored order.
    pub profile_patch_ranges: Vec<[usize; 2]>,
    pub report: LevelReport,
}

#[derive(Clone, Debug)]
pub struct MultiApproximation {
    pub patches: Option<Vec<Surface>>,
    pub profile_patch_ranges: Option<Vec<[usize; 2]>>,
    pub levels: Vec<LevelReport>,
}

pub struct MultiSweep<'a> {
    pub(super) sweeps: Vec<Sweep<'a>>,
    pub(super) options: Options,
    pub(super) next_sections: Option<usize>,
}

impl<'a> MultiSweep<'a> {
    pub fn new(
        profiles: &'a [Curve],
        path: &'a Curve,
        scale: &'a Curve,
        twist: &'a Curve,
        options: Options,
    ) -> Result<Self> {
        check(
            (1..=64).contains(&profiles.len()),
            "Multi-profile sweep needs 1..64 curves",
        )?;
        let sweeps = profiles
            .iter()
            .map(|profile| Sweep::new(profile, path, scale, twist, options))
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            sweeps,
            options,
            next_sections: Some(options.initial_sections),
        })
    }
}

impl<'a> MultiSweep<'a> {
    pub fn with_spatial_rmf_error_limits(mut self,steps:usize,cells:usize,products:usize)->Result<Self>{
        self.sweeps=self.sweeps.into_iter().map(|s|s.with_spatial_rmf_error_limits(steps,cells,products)).collect::<Result<_>>()?;
        Ok(self)
    }
    /// Retained preview sections in authored profile order, each on the same grid.
    /// Body admission still requires an accepted aggregate level report.
    pub fn sections_at(&self, count: usize) -> Result<Vec<Vec<Curve>>> {
        self.sweeps.iter().map(|s| s.sections_at(count)).collect()
    }
    /// One selected profile anchor defines the transverse fit for every contour.
    /// Hole boundaries therefore retain their relative width and shared joins.
    pub fn with_contact_guide(
        mut self,
        guide: &'a Curve,
        profile_index: usize,
        parameter: f64,
    ) -> Result<Self> {
        check(
            profile_index < self.sweeps.len(),
            "Contact profile index outside authored profiles",
        )?;
        let reference = self
            .sweeps
            .remove(profile_index)
            .with_contact_guide(guide, parameter)?;
        let point = reference.contact_point.unwrap();
        let source = reference.contact_source.unwrap();
        self.sweeps.insert(profile_index, reference);
        self.sweeps = self
            .sweeps
            .into_iter()
            .map(|s| {
                let mut result = s.with_contact_point(guide, point)?;
                result.contact_source = Some(source);
                Ok(result)
            })
            .collect::<Result<_>>()?;
        self.next_sections = Some(self.options.initial_sections);
        Ok(self)
    }
    /// Shared correspondence/orientation for all contours on the aggregate grid.
    pub fn with_orientation_guide(mut self, guide: &'a Curve) -> Result<Self> {
        self.sweeps = self
            .sweeps
            .into_iter()
            .map(|s| s.with_orientation_guide(guide))
            .collect::<Result<_>>()?;
        self.next_sections = Some(self.options.initial_sections);
        Ok(self)
    }
    /// Shared complete orientation for every profile on the aggregate grid.
    pub fn with_frame_laws(
        mut self,
        longitudinal: &'a Curve,
        transverse: &'a Curve,
    ) -> Result<Self> {
        self.sweeps = self
            .sweeps
            .into_iter()
            .map(|s| s.with_frame_laws(longitudinal, transverse))
            .collect::<Result<_>>()?;
        self.next_sections = Some(self.options.initial_sections);
        Ok(self)
    }
    pub fn with_affine_laws(mut self, axis_scale: &'a Curve, center: &'a Curve) -> Result<Self> {
        self.sweeps = self
            .sweeps
            .into_iter()
            .map(|s| s.with_affine_laws(axis_scale, center))
            .collect::<Result<_>>()?;
        self.next_sections = Some(self.options.initial_sections);
        Ok(self)
    }
}

impl MultiSweep<'_> {
    /// One aggregate preview; does not consume or alter iterator progression.
    pub fn preview_at(&self, count: usize) -> Result<MultiLevel> {
        let [_,cells,products]=self.sweeps[0].spatial_rmf_error_limits.unwrap_or([512,10000,1000000]);
        self.preview_at_with_error_limits(count,cells,products)
    }
    /// One caller-owned allowance for the complete wall union. Filled-cap
    /// owners can pass the remaining source allowance without resetting work.
    pub fn preview_at_with_error_limits(&self,count:usize,cell_limit:usize,product_limit:usize)->Result<MultiLevel> {
        check(cell_limit<=100000&&product_limit<=1000000,"Source error work exceeds limits")?;
        check(
            count >= self.options.initial_sections && count <= self.options.max_sections,
            "Preview section count outside configured sweep budget",
        )?;
        let mut patches = Vec::new();
        let mut ranges = Vec::new();
        let mut report: Option<LevelReport> = None;
        // One invocation owns the original path and all profile certificates.
        // Measure that shared source once; per-profile pose and decomposition
        // work still consume the remaining aggregate budget.
        let first=&self.sweeps[0];

        let shared_guided=first.frame_laws.is_none()&&first.orientation_guide.is_some()
            &&first.orientation_guide.is_some_and(|g|!g.periodic)
            &&match first.orientation_guide{Some(g)=>!path_is_closed(g)?,None=>false};
        let shared_length=if cell_limit>0 && (first.spatial_rmf_error_limits.is_some() || ((shared_guided || first.frame_laws.is_some() || (first.options.orientation==Orientation::Fixed||first.options.orientation==Orientation::FixedNormal||first.options.orientation==Orientation::Frenet||first.options.orientation==Orientation::RotationMinimizing||first.options.orientation==Orientation::CorrectedFrenet)
            && first.orientation_guide.is_none() && first.contact_point.is_none())&&(!authored_error::original_line(first.path)||shared_guided&&(first.contact_source.is_some()||first.orientation_guide.is_some_and(|g|!authored_error::original_line(g))))
            && !first.path.periodic&&!path_is_closed(first.path)?)) {
            if let Spacing::ArcLength {tolerance,max_cells}=first.options.spacing {
                check(self.sweeps.iter().all(|s|std::ptr::eq(s.path,first.path)&&s.options.spacing==first.options.spacing),
                    "Shared length certificate requires identical original source and spacing")?;
                let phase=if first.contact_point.is_some() {first.orientation_guide.map(|guide|arc_guide::certify(first.path,guide,max_cells.min(cell_limit))).transpose()?}else{None};
                let phase_cells=phase.as_ref().map_or(0,|p|p.cells);
                let tolerance=phase.as_ref().and_then(|p|p.scale_upper).filter(|scale|*scale>1.).map_or(tolerance,|scale|(tolerance/scale).next_down());
                // The shared source division owns both the exact phase premise
                // and the inverse-length cells, preserving the aggregate ceiling.
                let mut division=crate::curve_measure::divide_by_length(first.path,count-1,tolerance,max_cells.min(cell_limit)-phase_cells)?;
                division.cells+=phase_cells;
                Some(division)
            }else{None}
        }else{None};
        let mut cells=shared_length.as_ref().map_or(0,|report|report.cells);
        let shared_guide=if shared_guided&&shared_length.is_some()&&cells<cell_limit {
            let guide=first.orientation_guide.unwrap();
            check(self.sweeps.iter().all(|s|s.orientation_guide.is_some_and(|g|std::ptr::eq(g,guide))),
                "Shared guide length certificate requires identical original guide")?;
            let Spacing::ArcLength {tolerance,max_cells}=first.options.spacing else{unreachable!()};
            let report=crate::curve_measure::divide_by_length(guide,count-1,tolerance,max_cells.min(cell_limit-cells))?;
            cells+=report.cells;Some(report)
        }else{None};
        // Both station policies share precisely the same original Bishop
        // transport and closing phase. Source arc division is also shared.
        // Every profile query/decomposition consumes the remaining allowance.
        let shared_rmf=if let Some([steps,_,_])=first.spatial_rmf_error_limits {
                check(self.sweeps.iter().all(|s|std::ptr::eq(s.path,first.path)
                    &&s.options.normal==first.options.normal&&s.options.spacing==first.options.spacing
                    &&s.spatial_rmf_error_limits==first.spatial_rmf_error_limits),
                    "Shared RMF certificate requires identical original source and policy")?;
                let remaining=cell_limit-cells;
                let proof=rmf_transport::certify_original_rmf_transport_shared(first.path,first.options.normal,
                    steps,remaining,path_is_closed(first.path)?)?;
                cells+=proof.cells+proof.exact_work as usize;Some(proof)
        }else{None};
        let mut products=0;
        for sweep in &self.sweeps {
            let level = sweep.level_with_error_budget_and_length(count,cell_limit-cells,product_limit-products,shared_length.as_ref(),shared_guide.as_ref(),shared_rmf.as_ref())?;
            cells+=level.report.error_certificate_cells;
            products+=level.report.decomposition_products;
            check(
                patches.len() + level.patches.len() <= 4096,
                "Multi-profile sweep exceeds 4096 total retained patches",
            )?;
            let first = patches.len();
            patches.extend(level.patches);
            ranges.push([first, patches.len()]);
            if let Some(total) = &mut report {
                total.accepted &= level.report.accepted;
                total.sampled_control_deviation = total
                    .sampled_control_deviation
                    .max(level.report.sampled_control_deviation);
                total.continuous_bound &= level.report.continuous_bound;
                total.known_profile_error_upper = match (total.known_profile_error_upper,level.report.known_profile_error_upper) {
                    (Some(a),Some(b))=>Some(a.max(b)),(a,b)=>a.or(b),
                };
                total.continuous_error_upper = match (total.continuous_error_upper,level.report.continuous_error_upper) {
                    (Some(a),Some(b))=>Some(a.max(b)),_=>None,
                };
                total.original_section_endpoint_error_upper = match (total.original_section_endpoint_error_upper,level.report.original_section_endpoint_error_upper) {
                    (Some(a),Some(b))=>Some(a.max(b)),_=>None,
                };
                total.endpoint_contour_error_upper = match (total.endpoint_contour_error_upper,level.report.endpoint_contour_error_upper) {
                    (Some(a),Some(b))=>Some([a[0].max(b[0]),a[1].max(b[1])]),_=>None,
                };
                total.error_certificate_cells += level.report.error_certificate_cells;
                total.decomposition_products += level.report.decomposition_products;
                total.error_certificate_reason = total.error_certificate_reason.or(level.report.error_certificate_reason);
            } else {
                report = Some(level.report);
            }
        }
        let mut report=report.unwrap();
        report.error_certificate_cells=cells;
        Ok(MultiLevel {
            patches,
            profile_patch_ranges: ranges,
            report,
        })
    }
}

impl Iterator for MultiSweep<'_> {
    type Item = Result<MultiLevel>;
    fn next(&mut self) -> Option<Self::Item> {
        let count = self.next_sections.take()?;
        let level = self.preview_at(count);
        if let Ok(result) = &level {
            if !result.report.accepted && count < self.options.max_sections {
                self.next_sections = Some((2 * (count - 1) + 1).min(self.options.max_sections));
            }
        }
        Some(level)
    }
}

pub fn approximate_profiles(
    profiles: &[Curve],
    path: &Curve,
    scale: &Curve,
    twist: &Curve,
    options: Options,
) -> Result<MultiApproximation> {
    collect_profiles(MultiSweep::new(profiles, path, scale, twist, options)?)
}

pub fn approximate_affine_profiles(
    profiles: &[Curve],
    path: &Curve,
    scale: &Curve,
    twist: &Curve,
    axis_scale: &Curve,
    center: &Curve,
    options: Options,
) -> Result<MultiApproximation> {
    collect_profiles(
        MultiSweep::new(profiles, path, scale, twist, options)?
            .with_affine_laws(axis_scale, center)?,
    )
}

/// Original closed spatial RMF error policy, shared across all profiles.
pub fn approximate_spatial_rmf_profiles(
    profiles: &[Curve], path: &Curve, scale: &Curve, twist: &Curve,
    affine: Option<(&Curve, &Curve)>, options: Options,
    transport_steps: usize, max_cells: usize, max_products: usize,
) -> Result<MultiApproximation> {
    let sweep = MultiSweep::new(profiles, path, scale, twist, options)?;
    let sweep = if let Some((axes, center)) = affine {
        sweep.with_affine_laws(axes, center)?
    } else { sweep };
    collect_profiles(sweep.with_spatial_rmf_error_limits(transport_steps, max_cells, max_products)?)
}

/// Complete authored frames, simultaneous affine laws and aggregate admission.
pub fn approximate_authored_profiles(
    profiles: &[Curve],
    path: &Curve,
    scale: &Curve,
    twist: &Curve,
    longitudinal: &Curve,
    transverse: &Curve,
    axis_scale: &Curve,
    center: &Curve,
    mut options: Options,
) -> Result<MultiApproximation> {
    options.orientation = Orientation::Fixed;
    collect_profiles(
        MultiSweep::new(profiles, path, scale, twist, options)?
            .with_frame_laws(longitudinal, transverse)?
            .with_affine_laws(axis_scale, center)?,
    )
}

/// Shared guide-normal correspondence with simultaneous affine laws.
pub fn approximate_guided_profiles(
    profiles: &[Curve],
    path: &Curve,
    scale: &Curve,
    twist: &Curve,
    guide: &Curve,
    axis_scale: &Curve,
    center: &Curve,
    options: Options,
) -> Result<MultiApproximation> {
    collect_profiles(
        MultiSweep::new(profiles, path, scale, twist, options)?
            .with_orientation_guide(guide)?
            .with_affine_laws(axis_scale, center)?,
    )
}

/// Contact fit shared by every profile using one authored reference anchor.
pub fn approximate_contact_profiles(
    profiles: &[Curve],
    path: &Curve,
    scale: &Curve,
    twist: &Curve,
    guide: &Curve,
    profile_index: usize,
    parameter: f64,
    axis_scale: &Curve,
    center: &Curve,
    options: Options,
) -> Result<MultiApproximation> {
    collect_profiles(
        MultiSweep::new(profiles, path, scale, twist, options)?
            .with_contact_guide(guide, profile_index, parameter)?
            .with_affine_laws(axis_scale, center)?,
    )
}

pub(crate) fn collect_profiles(sweep: MultiSweep<'_>) -> Result<MultiApproximation> {
    let mut result = MultiApproximation {
        patches: None,
        profile_patch_ranges: None,
        levels: Vec::new(),
    };
    for level in sweep {
        let level = level?;
        if level.report.accepted {
            result.patches = Some(level.patches);
            result.profile_patch_ranges = Some(level.profile_patch_ranges);
        }
        result.levels.push(level.report);
    }
    Ok(result)
}

/// Geometric closure classification shared with topology resource admission.
/// Tangent/law seam conditions are checked by the sweep itself.
pub fn path_is_closed(path: &Curve) -> Result<bool> {
    path.validate()?;
    check(path.control_points[0].len() == 3, "Sweep path must be3D")?;
    let [a, b] = path.domain();
    let start = path.evaluate(a)?.point;
    let end = path.evaluate(b)?.point;
    closed_extent(
        path,
        [start[0], start[1], start[2]],
        [end[0], end[1], end[2]],
    )
}
pub(super) fn closed_extent(path: &Curve, start: V, end: V) -> Result<bool> {
    let extent = path
        .control_points
        .iter()
        .map(|p| norm(sub([p[0], p[1], p[2]], start)))
        .fold(0_f64, f64::max);
    check(
        extent.is_finite() && extent > 0.,
        "Sweep path extent is zero or unrepresentable",
    )?;
    Ok(start == end || norm(sub(end, start)) / extent <= 64. * f64::EPSILON)
}

pub(super) fn profile_parts(curve: &Curve) -> Result<Vec<Curve>> {
        if curve.control_points.len() <= 32 {
            return Ok(vec![curve.clone()]);
        }
        check(
            !curve.periodic,
            "Dense periodic sweep profiles need explicit clamped encoding",
        )?;
        Ok(curve
            .decompose()?
            .iter()
            .map(|s| s.definition().clone())
            .collect())
}
pub(super) fn patches(sections: &[Curve]) -> Result<Vec<Surface>> {
    let rows = sections.iter().map(profile_parts).collect::<Result<Vec<_>>>()?;
    let count = sections.len();
    let columns = (count - 1).div_ceil(31);
    check(
        rows[0].len() * columns <= 4096,
        "Progressive sweep exceeds 4096 retained patches",
    )?;
    let mut result = Vec::new();
    for u in 0..rows[0].len() {
        let mut first = 0;
        while first + 1 < count {
            let last = (first + 31).min(count - 1);
            let curves = (first..=last)
                .map(|v| rows[v][u].clone())
                .collect::<Vec<_>>();
            let mut surface = crate::surface::loft(&curves)?;
            for k in &mut surface.knots_v {
                *k = (*k + first as f64) / (count - 1) as f64;
            }
            surface.validate()?;
            result.push(surface);
            first = last;
        }
    }
    Ok(result)
}
