//! source profile body operations: definition.
use super::*;
impl<'s, 'p, 'b> TransitionBoundary<'s, 'p, 'b> {
#[cfg(feature = "codec")]
    pub fn definition(&self) -> Result<value_codec::Value> {
        Ok(
            value_codec::json!({"version":1,"kind":"owned-profile-transition-boundary",
            "sides":[self.rooted[0].definition()?,self.rooted[1].definition()?]}),
        )
    }
/// Recompute both cut roots for one section and bind complementary contours
    /// to this exact reconstructed transition boundary.
    pub fn replay_section_contours<'a>(
        &'a self,
        high: bool,
        max_cut_spans: usize,
        max_mapping_cells: usize,
        max_fragments: usize,
    ) -> Result<TransitionSectionReport<'a, 's, 'p, 'b>> {
        if max_cut_spans > 100000 || max_mapping_cells > 100000 || max_fragments > 100000 {
            return Err(Error::new(
                "BREP_PROFILE_SECTION_RESTORE",
                "Bound section root and contour replay work",
            ));
        }
        let pending = |reason| TransitionSectionReport {
            section: None,
            fragments: 0,
            reason,
        };
        let root = self.rooted[0];
        let proposal = root.restricted.proposal;
        let parameter = root.parameters[usize::from(high)];
        let first = proposal.cut_endpoint(
            root.face,
            root.axis,
            parameter,
            false,
            root.restricted.edge,
            max_cut_spans,
        )?;
        let Some(a) = first.endpoint else {
            return Ok(pending(first.reason));
        };
        let second = proposal.cut_endpoint(
            root.face,
            root.axis,
            parameter,
            true,
            self.rooted[1].restricted.edge,
            max_cut_spans.saturating_sub(first.spans),
        )?;
        let Some(b) = second.endpoint else {
            return Ok(pending(second.reason));
        };
        if max_mapping_cells == 0 {
            return Ok(pending("profile-section-replay-mapping-work-limit"));
        }
        let first = a.split_source_use(max_mapping_cells)?;
        let Some(pa) = first.partition else {
            return Ok(pending(first.reason));
        };
        let remaining = max_mapping_cells.saturating_sub(first.mapping_cells);
        if remaining == 0 {
            return Ok(pending("profile-section-replay-mapping-work-limit"));
        }
        let second = b.split_source_use(remaining)?;
        let Some(pb) = second.partition else {
            return Ok(pending(second.reason));
        };
        self.propose_section_contours(high, [&pa, &pb], max_fragments)
    }
/// Bind a terminal/entry section cut to this exact transition's carrier roots.
    pub fn propose_section_contours<'a>(
        &'a self,
        high: bool,
        parts: [&CutPartition<'_, 'p, 'b>; 2],
        max_fragments: usize,
    ) -> Result<TransitionSectionReport<'a, 's, 'p, 'b>> {
        let mut out = TransitionSectionReport {
            section: None,
            fragments: 0,
            reason: "profile-transition-section-ownership-unproven",
        };
        let index = usize::from(high);
        let recipe = self.rooted[0];
        for rail in 0..2 {
            let part = parts[rail];
            let endpoint = part.endpoint;
            if !std::ptr::eq(endpoint.proposal, recipe.restricted.proposal)
                || endpoint.face != recipe.face
                || endpoint.axis != recipe.axis
                || endpoint.parameter != recipe.parameters[index]
                || endpoint.high != (rail == 1)
                || endpoint.edge != self.rooted[rail].restricted.edge
                || !part
                    .point
                    .same_definition(self.rooted[rail].points[index].root())
            {
                return Ok(out);
            }
        }
        let contours = propose_cut_contours(parts[0], parts[1], max_fragments)?;
        out.fragments = contours.fragments;
        out.reason = contours.reason;
        let Some(contours) = contours.proposal else {
            return Ok(out);
        };
        out.section = Some(TransitionSection {
            boundary: self,
            high,
            contours,
        });
        out.reason = "profile-transition-section-contours-bound";
        Ok(out)
    }
/// Fresh whole-domain injectivity also proves immersion. Authority remains
    /// attached to this boundary and its exact full-Hermite recipe.
    pub fn qualify_face<'a>(
        &'a self,
        projection: [[f64; 3]; 2],
        controls: usize,
        cells: usize,
        spans: usize,
    ) -> Result<TransitionFaceReport<'a, 's, 'p, 'b>> {
        self.qualify_face_policy(projection, controls, cells, spans, false)
    }
/// Fresh full-domain admission with source-rail bounds. A caller's
    /// diagnostic report cannot create an owned transition face.
    pub fn qualify_face_from_rails<'a>(
        &'a self,
        projection: [[f64; 3]; 2],
        controls: usize,
        cells: usize,
        spans: usize,
    ) -> Result<TransitionFaceReport<'a, 's, 'p, 'b>> {
        self.qualify_face_policy(projection, controls, cells, spans, true)
    }
pub(super) fn qualify_face_policy<'a>(
        &'a self,
        projection: [[f64; 3]; 2],
        controls: usize,
        cells: usize,
        spans: usize,
        from_rails: bool,
    ) -> Result<TransitionFaceReport<'a, 's, 'p, 'b>> {
        let root = self.rooted[0];
        let profile = root.restricted.proposal.profile(root.face).unwrap();
        let sections = [
            profile.section(root.axis, root.parameters[0])?,
            profile.section(root.axis, root.parameters[1])?,
        ];
        let range = sections[0].range();
        let width = root.parameters[1] - root.parameters[0];
        let [first, last] = sections;
        let bridge =
            nurbs_core::normal_alignment::ProfileSectionBridge::new(first, last, [width; 2])?;
        let (bridge, _) = bridge.constrain_original_rails(controls)?;
        let Some(bridge) = bridge else {
            return Ok(TransitionFaceReport {
                face: None,
                interior: None,
                reason: "profile-transition-face-recipe-unproven",
            });
        };
        let interior = if from_rails {
            bridge.inspect_transverse_corrected_injectivity_from_rails(
                range,
                [0., 1.],
                projection,
                cells,
                spans,
            )?
        } else {
            bridge.inspect_transverse_corrected_injectivity(
                range,
                [0., 1.],
                projection,
                cells,
                spans,
            )?
        };
        let face = if interior.complete_regular {
            Some(TransitionFace {
                boundary: self,
                projection,
            })
        } else {
            None
        };
        let reason = interior.reason;
        Ok(TransitionFaceReport {
            face,
            interior: Some(interior),
            reason,
        })
    }
pub fn corners(&self) -> [&crate::source_carrier_point::Point; 4] {
        [
            &self.rooted[0].points[0],
            &self.rooted[0].points[1],
            &self.rooted[1].points[1],
            &self.rooted[1].points[0],
        ]
    }
pub fn sides(&self) -> [&RootEndedSeam<'_, '_>; 2] {
        self.rooted
    }
/// Low rail, terminal section, reversed high rail, reversed entry section.
    /// Each segment uses the exact owned profile boundary specialization.
    pub fn enclose_edge(
        &self,
        index: usize,
        parameter: [f64; 2],
        spans: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        use nurbs_core::interval_eval::Interval as I;
        if index > 3
            || !parameter.iter().all(|x| x.is_finite())
            || parameter[0] < 0.
            || parameter[1] > 1.
            || parameter[0] > parameter[1]
        {
            return Err(Error::new(
                "BREP_PROFILE_TRANSITION_BOUNDARY",
                "Bound edge index and normalized parameter",
            ));
        }
        let root = self.rooted[0];
        let profile = root.restricted.proposal.profile(root.face).unwrap();
        let range = profile.section(root.axis, root.parameters[0])?.range();
        let affine = |ends: [f64; 2]| -> Result<[f64; 2]> {
            if parameter == [0., 0.] {
                return Ok([ends[0]; 2]);
            }
            if parameter == [1., 1.] {
                return Ok([ends[1]; 2]);
            }
            let value = I::point(ends[0])
                .add(
                    I::new(parameter[0], parameter[1])?
                        .mul(I::point(ends[1]).sub(I::point(ends[0]))?)?,
                )?
                .intersect(ends[0].min(ends[1]), ends[0].max(ends[1]))?;
            Ok([value.lo, value.hi])
        };
        let mut domain = [[0.; 2]; 2];
        match index {
            0 => {
                domain[root.axis] = affine(root.parameters)?;
                domain[1 - root.axis] = [range[0]; 2];
            }
            1 => {
                domain[root.axis] = [root.parameters[1]; 2];
                domain[1 - root.axis] = affine(range)?;
            }
            2 => {
                domain[root.axis] = affine([root.parameters[1], root.parameters[0]])?;
                domain[1 - root.axis] = [range[1]; 2];
            }
            _ => {
                domain[root.axis] = [root.parameters[0]; 2];
                domain[1 - root.axis] = affine([range[1], range[0]])?;
            }
        }
        profile.enclose(domain, spans)
    }
}
