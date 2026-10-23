//! Exact sufficient retained station-seam jets, separate from source error.
use super::*;
use crate::join::continuity::inspect_surface_projective_strip_jets;

#[derive(Clone, Debug)]
pub struct RetainedDecompositionReport {
    pub order: usize,
    pub expected_joins: usize,
    pub coverage_complete: bool,
    pub all_joins_certified: bool,
    pub exact_work: u64,
    pub joins: Vec<([usize; 2], crate::join::continuity::ExactStripJetReport)>,
    pub reason: Option<&'static str>,
}

#[derive(Clone, Debug)]
pub struct RetainedDecompositionSmoothnessReport {
    pub g2: RetainedDecompositionReport,
    pub g1: Option<RetainedDecompositionReport>,
    pub g1_certified: bool,
    pub exact_work: u64,
    pub max_work: u64,
}

#[derive(Clone, Debug)]
pub struct RetainedSeamReport {
    pub patch: usize,
    pub station: usize,
    pub closure: bool,
    /// Exact shared represented poles/weights; no smoothness implication.
    pub c0_identity: bool,
    pub certified: bool,
    pub regularity_certified: bool,
    pub exact_work: u64,
    pub reason: &'static str,
}
#[derive(Clone, Debug)]
pub struct RetainedSmoothnessReport {
    pub order: usize,
    pub all_station_seams_certified: bool,
    pub exact_work: u64,
    pub seams: Vec<RetainedSeamReport>,
    pub reason: Option<&'static str>,
}
fn strip(s: &Surface, i: usize) -> Surface {
    // Degree-one strips are literal original columns and original knots:
    // no insertion, re-fit, rounded extraction or inferred correspondence.
    let a = s.knots_v[i + 1];
    let b = s.knots_v[i + 2];
    Surface {
        degree_u: s.degree_u,
        degree_v: 1,
        knots_u: s.knots_u.clone(),
        knots_v: vec![a, a, b, b],
        control_points: s
            .control_points
            .iter()
            .map(|r| vec![r[i].clone(), r[i + 1].clone()])
            .collect(),
        weights: s.weights.iter().map(|r| vec![r[i], r[i + 1]]).collect(),
        periodic_u: s.periodic_u,
        periodic_v: false,
    }
}
fn record(
    out: &mut RetainedSmoothnessReport,
    a: &Surface,
    b: &Surface,
    left: usize,
    right: usize,
    patch: usize,
    station: usize,
    closure: bool,
    max_work: u64,
) -> Result<()> {
    if out.seams.len() >= 100000 {
        out.all_station_seams_certified = false;
        out.reason = Some("retained-seam-count-unproved");
        return Ok(());
    }
    let c0_identity = a.control_points.len() == b.control_points.len()
        && a.degree_u == b.degree_u
        && a.knots_u == b.knots_u
        && (0..a.control_points.len()).all(|u| {
            a.control_points[u][left + 1] == b.control_points[u][right]
                && a.weights[u][left + 1] == b.weights[u][right]
        });
    if out.exact_work == max_work {
        out.all_station_seams_certified = false;
        out.reason = Some("retained-seam-work-unproved");
        out.seams.push(RetainedSeamReport {
            patch,
            station,
            closure,
            c0_identity,
            certified: false,
            regularity_certified: false,
            exact_work: 0,
            reason: "retained-seam-work-unproved",
        });
        return Ok(());
    }
    let left_strip = strip(a, left);
    let right_strip = strip(b, right);
    // Numerical scale is only a proposal. Exact projective jet identities
    // over the whole retained boundary and regularity must still prove it.
    let normal_scale = crate::join::continuity::propose_station_normal_scale(
        &left_strip,
        &right_strip,
        "vMax",
        "vMin",
    )?;
    let result = crate::join::continuity::inspect_proposed_station_projective_strip_jets(
        &left_strip,
        &right_strip,
        "vMax",
        "vMin",
        out.order,
        normal_scale,
        max_work - out.exact_work,
    )?;
    out.exact_work += result.work;
    out.all_station_seams_certified &= result.certified;
    if !result.certified {
        out.reason = Some(result.reason);
    }
    out.seams.push(RetainedSeamReport {
        patch,
        station,
        closure,
        c0_identity,
        certified: result.certified,
        regularity_certified: result.regularity_certified,
        exact_work: result.work,
        reason: result.reason,
    });
    Ok(())
}
impl Level {
    /// Exact sufficient G1/G2 across an explicitly identified retained profile
    /// join. Both patches must own the same station interval and basis. This
    /// local report does not establish adjacency of all source profile parts,
    /// source error, cap continuity, or global shell regularity.
    pub fn certify_retained_profile_join(
        &self,
        left_patch: usize,
        right_patch: usize,
        order: usize,
        transverse_scale: f64,
        max_work: u64,
    ) -> Result<crate::join::continuity::ExactStripJetReport> {
        check(
            left_patch != right_patch
                && left_patch < self.patches.len()
                && right_patch < self.patches.len(),
            "Invalid retained profile join patches",
        )?;
        let a = &self.patches[left_patch];
        let b = &self.patches[right_patch];
        a.validate()?;
        b.validate()?;
        check(
            a.degree_v == b.degree_v
                && a.knots_v == b.knots_v
                && a.periodic_v == b.periodic_v
                && a.control_points[0].len() == b.control_points[0].len(),
            "Retained profile join station correspondence mismatch",
        )?;
        inspect_surface_projective_strip_jets(
            a,
            b,
            "uMax",
            "uMin",
            order,
            transverse_scale,
            max_work,
        )
    }

    /// Sufficient full-boundary G1/G2 in the represented retained surfaces.
    /// Does not certify source-frame smoothness, profile joins, cap joins or
    /// surface interiors. Unsupported bases and partial work stay unresolved.
    pub fn certify_retained_station_seams(
        &self,
        order: usize,
        max_work: u64,
    ) -> Result<RetainedSmoothnessReport> {
        check(
            matches!(order, 1 | 2) && max_work <= 1000000,
            "Invalid retained seam proof limits",
        )?;
        let mut out = RetainedSmoothnessReport {
            order,
            all_station_seams_certified: !self.patches.is_empty(),
            exact_work: 0,
            seams: Vec::new(),
            reason: None,
        };
        if self.patches.is_empty() {
            out.reason = Some("retained-seam-surfaces-missing");
        }
        let mut group_start = None;
        let mut previous = None;
        for (patch, s) in self.patches.iter().enumerate() {
            s.validate()?;
            let n = s.control_points[0].len();
            if s.degree_v != 1
                || s.periodic_v
                || s.knots_v.len() != n + 2
                || s.knots_v[0] != s.knots_v[1]
                || s.knots_v[n] != s.knots_v[n + 1]
                || s.knots_v[1..=n].windows(2).any(|w| w[0] >= w[1])
            {
                out.all_station_seams_certified = false;
                out.reason = Some("retained-linear-station-basis-unproved");
                return Ok(out);
            }
            // patches() groups each original profile part into consecutive
            // station chunks, each at most31 spans, spanning exactly[0,1].
            // Group/chunk correspondence is checked, never inferred by samples.
            if s.knots_v[1] == 0. {
                if group_start.is_some() {
                    out.all_station_seams_certified = false;
                    out.reason = Some("retained-station-chunk-correspondence-unproved");
                    return Ok(out);
                }
                group_start = Some(patch);
            } else {
                let Some(prev): Option<usize> = previous else {
                    out.all_station_seams_certified = false;
                    out.reason = Some("retained-station-chunk-correspondence-unproved");
                    return Ok(out);
                };
                let a = &self.patches[prev];
                let na = a.control_points[0].len();
                if group_start.is_none() || a.knots_v[na] != s.knots_v[1] {
                    out.all_station_seams_certified = false;
                    out.reason = Some("retained-station-chunk-correspondence-unproved");
                    return Ok(out);
                }
                record(&mut out, a, s, na - 2, 0, prev, na - 1, false, max_work)?;
            }
            for i in 1..n - 1 {
                if out.seams.len() >= 100000 {
                    out.all_station_seams_certified = false;
                    out.reason = Some("retained-seam-count-unproved");
                    return Ok(out);
                }
                record(&mut out, s, s, i - 1, i, patch, i, false, max_work)?;
            }
            if s.knots_v[n] == 1. {
                let first = group_start.take().unwrap();
                if self.report.closed_path {
                    record(
                        &mut out,
                        s,
                        &self.patches[first],
                        n - 2,
                        0,
                        patch,
                        n - 1,
                        true,
                        max_work,
                    )?;
                }
                previous = None;
            } else {
                previous = Some(patch);
            }
        }
        if group_start.is_some() {
            out.all_station_seams_certified = false;
            out.reason = Some("retained-station-chunk-correspondence-unproved");
        }
        Ok(out)
    }
}

impl MultiLevel {
    /// Try G2 while reserving half the exact budget for a complete G1 fallback.
    /// G2 implies G1; otherwise G1 is separately proved over the same owned set.
    pub fn certify_retained_decomposition_smoothness(
        &self,
        transverse_scale: f64,
        max_work: u64,
    ) -> Result<RetainedDecompositionSmoothnessReport> {
        check(
            max_work <= 1000000,
            "Invalid retained decomposition proof limits",
        )?;
        let g2 = self.certify_retained_decomposition_joins(2, transverse_scale, max_work / 2)?;
        let mut exact_work = g2.exact_work;
        let g1 = if g2.all_joins_certified {
            None
        } else {
            let proof = self.certify_retained_decomposition_joins(
                1,
                transverse_scale,
                max_work - exact_work,
            )?;
            exact_work += proof.exact_work;
            Some(proof)
        };
        let g1_certified =
            g2.all_joins_certified || g1.as_ref().is_some_and(|r| r.all_joins_certified);
        Ok(RetainedDecompositionSmoothnessReport {
            g2,
            g1,
            g1_certified,
            exact_work,
            max_work,
        })
    }

    /// Complete internal decomposition-join audit with one shared exact budget.
    /// Constant transverse reparameterization is a sufficient condition only;
    /// authored profile boundaries, closed profile seams and caps are excluded.
    pub fn certify_retained_decomposition_joins(
        &self,
        order: usize,
        transverse_scale: f64,
        max_work: u64,
    ) -> Result<RetainedDecompositionReport> {
        check(
            matches!(order, 1 | 2)
                && transverse_scale.is_finite()
                && transverse_scale > 0.
                && max_work <= 1000000,
            "Invalid retained decomposition proof limits",
        )?;
        let pairs = self.retained_decomposition_join_pairs()?;
        let mut out = RetainedDecompositionReport {
            order,
            expected_joins: pairs.len(),
            coverage_complete: false,
            all_joins_certified: !pairs.is_empty(),
            exact_work: 0,
            joins: Vec::new(),
            reason: if pairs.is_empty() {
                Some("retained-decomposition-joins-absent")
            } else {
                None
            },
        };
        for pair in pairs {
            if out.exact_work == max_work {
                out.all_joins_certified = false;
                out.reason = Some("retained-decomposition-work-unproved");
                break;
            }
            let proof = inspect_surface_projective_strip_jets(
                &self.patches[pair[0]],
                &self.patches[pair[1]],
                "uMax",
                "uMin",
                order,
                transverse_scale,
                max_work - out.exact_work,
            )?;
            out.exact_work += proof.work;
            out.all_joins_certified &= proof.certified;
            if !proof.certified {
                out.reason = Some(proof.reason);
            }
            out.joins.push((pair, proof));
        }
        out.coverage_complete = out.joins.len() == out.expected_joins;
        out.all_joins_certified &= out.coverage_complete;
        Ok(out)
    }

    /// Exhaustive pairs of internal represented profile-decomposition joins,
    /// one pair per matching station chunk. Authored profile boundaries and
    /// profile closure are excluded: their adjacency needs loop ownership.
    pub fn retained_decomposition_join_pairs(&self) -> Result<Vec<[usize; 2]>> {
        check(
            (1..=64).contains(&self.profile_patch_ranges.len()) && self.patches.len() <= 4096,
            "Invalid retained profile ownership limits",
        )?;
        let mut covered = 0;
        let mut pairs = Vec::new();
        for &[start, end] in &self.profile_patch_ranges {
            check(
                start == covered && start < end && end <= self.patches.len(),
                "Invalid retained profile patch ownership",
            )?;
            let mut groups: Vec<Vec<usize>> = Vec::new();
            let mut current = Vec::new();
            let mut previous_end = 0.;
            for patch in start..end {
                let s = &self.patches[patch];
                s.validate()?;
                let n = s.control_points[0].len();
                check(
                    s.degree_v == 1
                        && !s.periodic_v
                        && s.knots_v.len() == n + 2
                        && s.knots_v[0] == s.knots_v[1]
                        && s.knots_v[n] == s.knots_v[n + 1]
                        && s.knots_v[1..=n].windows(2).all(|w| w[0] < w[1]),
                    "Invalid retained decomposition station basis",
                )?;
                check(
                    s.knots_v[1] == previous_end && s.knots_v[n] <= 1.,
                    "Incomplete retained decomposition station ownership",
                )?;
                current.push(patch);
                previous_end = s.knots_v[n];
                if previous_end == 1. {
                    groups.push(std::mem::take(&mut current));
                    previous_end = 0.;
                }
            }
            check(
                current.is_empty(),
                "Incomplete retained decomposition station coverage",
            )?;
            for adjacent in groups.windows(2) {
                check(
                    adjacent[0].len() == adjacent[1].len(),
                    "Retained decomposition chunk counts mismatch",
                )?;
                for (&left, &right) in adjacent[0].iter().zip(&adjacent[1]) {
                    let a = &self.patches[left];
                    let b = &self.patches[right];
                    check(
                        a.knots_v == b.knots_v,
                        "Retained decomposition chunk correspondence mismatch",
                    )?;
                    pairs.push([left, right]);
                }
            }
            covered = end;
        }
        check(
            covered == self.patches.len(),
            "Orphan retained profile patches",
        )?;
        Ok(pairs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_dense_quadratic_profile_proves_g1_without_promoting_curvature_jumps() {
        // Sixteen exact quadratic pieces with equal endpoint first jets and
        // alternating curvature. Decomposition must preserve these distinctions.
        let mut controls = vec![vec![0.; 3]];
        for i in 0..16 {
            let slope = if i % 2 == 0 { 0. } else { 0.25 };
            controls.push(vec![i as f64 + 0.5, i as f64 * 0.125 + slope * 0.5, 0.]);
            controls.push(vec![(i + 1) as f64, (i + 1) as f64 * 0.125, 0.]);
        }
        let profile = Curve {
            degree: 2,
            knots: [
                vec![0.; 3],
                (1..16).flat_map(|i| [i as f64 / 16.; 2]).collect(),
                vec![1.; 3],
            ]
            .concat(),
            control_points: controls,
            weights: vec![1.; 33],
            periodic: false,
        };
        let path = crate::primitives::line([0.; 3], [0., 0., 4.]).unwrap();
        let scale = constant_vector_law([1., 0., 0.]).unwrap();
        let twist = constant_vector_law([0.; 3]).unwrap();
        let profiles = [profile];
        let options = Options {
            normal: [1., 0., 0.],
            orientation: Orientation::Fixed,
            spacing: Spacing::Parameter,
            initial_sections: 2,
            max_sections: 2,
            max_deviation: 0.01,
        };
        let level = MultiSweep::new(&profiles, &path, &scale, &twist, options)
            .unwrap()
            .preview_at(2)
            .unwrap();
        let proof = level
            .certify_retained_decomposition_smoothness(1., 1000000)
            .unwrap();
        assert_eq!(proof.g2.expected_joins, 15);
        assert!(proof.g2.coverage_complete && !proof.g2.all_joins_certified);
        let g1 = proof.g1.as_ref().unwrap();
        assert!(
            g1.coverage_complete && g1.all_joins_certified && proof.g1_certified,
            "{proof:?}"
        );
        assert_eq!(proof.exact_work, proof.g2.exact_work + g1.exact_work);
        assert!(proof.exact_work <= proof.max_work);
        #[cfg(feature = "transport")]
        {
            let r = crate::transport::dispatch(value_codec::json!({
                "op":"surface_progressive_sweep_decomposition_smoothness",
                "profiles":profiles,"path":path,"scale":scale,"twist":twist,
                "normal":[1.,0.,0.],"orientation":"fixed","spacing":"parameter",
                "initial_sections":2,"max_sections":2,"max_deviation":0.01,
                "preview_sections":2,"transverseScale":1.,"maxExactWork":1000000}))
            .unwrap();
            assert_eq!(r["decompositionG1Certified"], true);
            assert_eq!(r["g2"]["decompositionJoinsCertified"], false);
            assert_eq!(r["g1"]["decompositionJoinsCertified"], true);
            assert_eq!(r["g1"]["requestedOrder"], 1);
            assert_eq!(r["g2"]["requestedOrder"], 2);
            assert_eq!(r["scope"], "within-source-profile-decomposition-only");
            for key in [
                "allProfileJoinsCertified",
                "closedProfileSeamsCertified",
                "sourceFrameSmoothnessCertified",
                "capJoinsCertified",
                "continuousBound",
                "solidCertified",
            ] {
                assert_eq!(r[key], false);
            }
        }
        let zero = level
            .certify_retained_decomposition_smoothness(1., 0)
            .unwrap();
        assert!(!zero.g1_certified && !zero.g2.all_joins_certified);
        // A middle control change creates a true tangent corner, not just G2 loss.
        let mut corner = level;
        corner.patches[1].control_points[1][0][1] += 0.125;
        corner.patches[1].control_points[1][1][1] += 0.125;
        assert!(
            !corner
                .certify_retained_decomposition_smoothness(1., 1000000)
                .unwrap()
                .g1_certified
        );
    }
    #[test]
    fn actual_dense_profile_decomposition_joins_cover_all_station_chunks() {
        // 33 original poles force native profile decomposition; 65 stations
        // force three literal station chunks per profile part.
        let profile = Curve {
            degree: 1,
            knots: [
                vec![0.],
                (0..=32).map(|i| i as f64 / 32.).collect(),
                vec![1.],
            ]
            .concat(),
            control_points: (0..=32).map(|i| vec![i as f64 / 32., 0., 0.]).collect(),
            weights: vec![1.; 33],
            periodic: false,
        };
        let path = crate::primitives::line([0.; 3], [0., 0., 4.]).unwrap();
        let scale = constant_vector_law([1., 0., 0.]).unwrap();
        let twist = constant_vector_law([0.; 3]).unwrap();
        let options = Options {
            normal: [1., 0., 0.],
            orientation: Orientation::Fixed,
            spacing: Spacing::Parameter,
            initial_sections: 5,
            max_sections: 65,
            max_deviation: 0.01,
        };
        let level = Sweep::new(&profile, &path, &scale, &twist, options)
            .unwrap()
            .preview_at(65)
            .unwrap();
        assert_eq!(level.patches.len(), 32 * 3);
        let profiles = [profile.clone(), profile.clone()];
        let multi = MultiSweep::new(&profiles, &path, &scale, &twist, options)
            .unwrap()
            .preview_at(65)
            .unwrap();
        let pairs = multi.retained_decomposition_join_pairs().unwrap();
        assert_eq!(pairs.len(), 2 * 31 * 3);
        let aggregate = multi
            .certify_retained_decomposition_joins(2, 1., 1000000)
            .unwrap();
        assert_eq!(aggregate.expected_joins, 186);
        assert!(!aggregate.all_joins_certified);
        assert_eq!(aggregate.coverage_complete, aggregate.joins.len() == 186);
        assert!(aggregate.exact_work <= 1000000);
        if !aggregate.coverage_complete {
            assert_eq!(
                aggregate.reason,
                Some("retained-decomposition-work-unproved")
            );
        }
        let endpoints = MultiSweep::new(
            &profiles,
            &path,
            &scale,
            &twist,
            Options {
                initial_sections: 2,
                ..options
            },
        )
        .unwrap()
        .preview_at(2)
        .unwrap();
        let positive = endpoints
            .certify_retained_decomposition_joins(2, 1., 1000000)
            .unwrap();
        assert!(
            positive.coverage_complete && positive.all_joins_certified,
            "{positive:?}"
        );
        assert_eq!(positive.expected_joins, 62);
        #[cfg(feature = "transport")]
        {
            let request = value_codec::json!({"op":"surface_progressive_sweep_decomposition_joins",
                "profiles":profiles,"path":path,"scale":scale,"twist":twist,
                "normal":[1.,0.,0.],"orientation":"fixed","spacing":"parameter",
                "initial_sections":2,"max_sections":65,"max_deviation":0.01,
                "preview_sections":2,"order":2,"transverseScale":1.,"maxExactWork":1000000});
            let r = crate::transport::dispatch(request.clone()).unwrap();
            assert_eq!(r["decompositionJoinsCertified"], true);
            assert_eq!(r["coverageComplete"], true);
            assert_eq!(r["expectedJoins"], 62);
            assert_eq!(r["scope"], "within-source-profile-decomposition-only");
            for key in [
                "allProfileJoinsCertified",
                "closedProfileSeamsCertified",
                "sourceFrameSmoothnessCertified",
                "capJoinsCertified",
                "continuousBound",
                "solidCertified",
            ] {
                assert_eq!(r[key], false);
            }
            let mut zero = request;
            zero["maxExactWork"] = value_codec::json!(0);
            let r = crate::transport::dispatch(zero).unwrap();
            assert_eq!(r["coverageComplete"], false);
            assert_eq!(r["decompositionJoinsCertified"], false);
        }
        let short = endpoints
            .certify_retained_decomposition_joins(2, 1., positive.exact_work - 1)
            .unwrap();
        assert!(!short.all_joins_certified && short.exact_work < positive.exact_work);
        let zero = endpoints
            .certify_retained_decomposition_joins(2, 1., 0)
            .unwrap();
        assert!(!zero.coverage_complete && !zero.all_joins_certified && zero.joins.is_empty());
        assert!(!pairs.iter().any(|p| p[0] < 96 && p[1] >= 96));
        for part in 0..31 {
            for chunk in 0..3 {
                assert!(pairs.contains(&[part * 3 + chunk, (part + 1) * 3 + chunk]));
            }
        }
        let mut orphan = multi.clone();
        orphan.profile_patch_ranges.pop();
        assert!(orphan.retained_decomposition_join_pairs().is_err());
        let mut missing = multi;
        missing.patches.remove(1);
        assert!(missing.retained_decomposition_join_pairs().is_err());
        let mut work = 0;
        let mut certified = 0;
        let mut unresolved = 0;
        for part in 0..31 {
            for chunk in 0..3 {
                let proof = level
                    .certify_retained_profile_join(
                        part * 3 + chunk,
                        (part + 1) * 3 + chunk,
                        2,
                        1.,
                        1000000 - work,
                    )
                    .unwrap();
                if proof.certified {
                    certified += 1;
                } else {
                    // Binary64 station generation can leave tiny exact jet
                    // differences in a nominally planar construction. Preserve
                    // refusal rather than infer retained G2 from the source.
                    assert_eq!(
                        proof.reason, "linear-along-jet-smoothness-unproved",
                        "part={part}, chunk={chunk}: {proof:?}"
                    );
                    unresolved += 1;
                }
                work += proof.work;
            }
        }
        assert!(work > 0 && work <= 1000000);
        assert!(certified > 0 && unresolved > 0);
        // Adjacent flattened indices may belong to disjoint station chunks.
        assert!(
            level
                .certify_retained_profile_join(0, 1, 2, 1., 1000000)
                .is_err()
        );
        let mut corner = profile;
        corner.control_points[16][1] = 0.25;
        let corner = Sweep::new(&corner, &path, &scale, &twist, options)
            .unwrap()
            .preview_at(65)
            .unwrap();
        assert!(
            !corner
                .certify_retained_profile_join(15 * 3, 16 * 3, 1, 1., 1000000)
                .unwrap()
                .certified
        );
    }
    #[test]
    fn retained_profile_join_requires_station_ownership_and_exact_jets() {
        let profile = crate::primitives::line([0.; 3], [1., 0., 0.]).unwrap();
        let path = crate::primitives::line([0.; 3], [0., 0., 4.]).unwrap();
        let scale = constant_vector_law([1., 0., 0.]).unwrap();
        let twist = constant_vector_law([0.; 3]).unwrap();
        let options = Options {
            normal: [1., 0., 0.],
            orientation: Orientation::Fixed,
            spacing: Spacing::Parameter,
            initial_sections: 3,
            max_sections: 5,
            max_deviation: 0.01,
        };
        let mut level = Sweep::new(&profile, &path, &scale, &twist, options)
            .unwrap()
            .preview_at(3)
            .unwrap();
        let plane = |x: f64| Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: (0..2)
                .map(|u| {
                    (0..2)
                        .map(|v| vec![x + u as f64, 0., 4. * v as f64])
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        level.patches = vec![plane(0.), plane(1.)];
        let positive = level
            .certify_retained_profile_join(0, 1, 2, 1., 1000000)
            .unwrap();
        assert!(
            positive.certified && positive.regularity_certified,
            "{positive:?}"
        );
        assert!(
            !level
                .certify_retained_profile_join(0, 1, 2, 1., positive.work - 1)
                .unwrap()
                .certified
        );
        let mut wrong_interval = level.clone();
        wrong_interval.patches[1].knots_v = vec![0.5, 0.5, 1., 1.];
        assert!(
            wrong_interval
                .certify_retained_profile_join(0, 1, 2, 1., 1000000)
                .is_err()
        );
        for p in &mut level.patches[1].control_points[1] {
            p[1] = 0.25;
        }
        let corner = level
            .certify_retained_profile_join(0, 1, 1, 1., 1000000)
            .unwrap();
        assert!(!corner.certified && !corner.exact_identity);
        assert!(
            level
                .certify_retained_profile_join(0, 0, 1, 1., 1000000)
                .is_err()
        );
    }
    #[test]
    #[cfg(feature = "transport")]
    fn retained_profile_join_transport_reconstructs_geometry_and_preserves_scope() {
        let profile = crate::primitives::line([0.; 3], [1., 0., 0.]).unwrap();
        let next = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
        let path = crate::primitives::line([0.; 3], [0., 0., 4.]).unwrap();
        let scale = constant_vector_law([1., 0., 0.]).unwrap();
        let twist = constant_vector_law([0.; 3]).unwrap();
        let request = value_codec::json!({"op":"surface_progressive_sweep_profile_join",
            "profiles":[profile,next],"path":path,"scale":scale,"twist":twist,
            "normal":[1.,0.,0.],"orientation":"fixed","spacing":"parameter",
            "initial_sections":3,"max_sections":5,"max_deviation":0.01,
            "preview_sections":3,"leftPatch":0,"rightPatch":1,"order":2,
            "transverseScale":1.,"maxExactWork":1000000});
        let proof = crate::transport::dispatch(request.clone()).unwrap();
        assert_eq!(proof["certified"], true);
        assert_eq!(proof["regularityCertified"], true);
        assert_eq!(proof["scope"], "explicit-retained-profile-join-only");
        for key in [
            "allProfileJoinsCertified",
            "sourceFrameSmoothnessCertified",
            "capJoinsCertified",
            "continuousBound",
            "solidCertified",
        ] {
            assert_eq!(proof[key], false);
        }
        let mut short = request;
        short["maxExactWork"] = value_codec::json!(0);
        assert_eq!(
            crate::transport::dispatch(short).unwrap()["certified"],
            false
        );
    }
    #[test]
    #[cfg(feature = "transport")]
    fn retained_station_jet_transport_keeps_source_and_solid_scope_separate() {
        let profile = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
        let path = crate::primitives::line([0.; 3], [0., 0., 4.]).unwrap();
        let scale = constant_vector_law([1., 0., 0.]).unwrap();
        let twist = constant_vector_law([0.; 3]).unwrap();
        let request = value_codec::json!({"op":"surface_progressive_sweep_station_seams","profiles":[profile],"path":path,"scale":scale,"twist":twist,
            "normal":[1.,0.,0.],"orientation":"rmf","spacing":"parameter","initial_sections":3,"max_sections":5,"max_deviation":0.01,"preview_sections":5,"order":2,"maxExactWork":1000000});
        let proof = crate::transport::dispatch(request.clone()).unwrap();
        assert_eq!(proof["allStationSeamsCertified"], true);
        assert_eq!(proof["requestedOrder"], 2);
        assert_eq!(proof["sourceFrameSmoothnessCertified"], false);
        assert_eq!(proof["solidCertified"], false);
        assert_eq!(proof["scope"], "retained-station-seams-only");
        let mut short = request;
        short["maxExactWork"] = value_codec::json!(0);
        let unresolved = crate::transport::dispatch(short).unwrap();
        assert_eq!(unresolved["allStationSeamsCertified"], false);
    }
    #[test]
    fn rational_multi_span_profile_station_jets_keep_group_and_budget_scope() {
        let profile = Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 0.5, 1., 1., 1., 1.],
            control_points: vec![
                vec![1., 0., 0.],
                vec![2., 0.125, 0.],
                vec![3., 0.25, 0.],
                vec![4., 0.125, 0.],
                vec![5., 0., 0.],
            ],
            weights: vec![1., 0.75, 1.25, 0.875, 1.],
            periodic: false,
        };
        let path = crate::primitives::line([0.; 3], [0., 0., 4.]).unwrap();
        let scale = constant_vector_law([1., 0., 0.]).unwrap();
        let twist = constant_vector_law([0.; 3]).unwrap();
        let sweep = Sweep::new(
            &profile,
            &path,
            &scale,
            &twist,
            Options {
                normal: [1., 0., 0.],
                orientation: Orientation::RotationMinimizing,
                spacing: Spacing::Parameter,
                initial_sections: 3,
                max_sections: 65,
                max_deviation: 0.01,
            },
        )
        .unwrap();
        let level = sweep.preview_at(5).unwrap();
        assert_eq!(level.patches.len(), 1);
        assert!(
            level
                .patches
                .iter()
                .any(|p| p.weights.iter().flatten().any(|w| *w != 1.))
        );
        let proof = level.certify_retained_station_seams(2, 1000000).unwrap();
        assert!(proof.all_station_seams_certified, "{proof:?}");
        assert_eq!(proof.seams.len(), 3);
        assert!(
            proof
                .seams
                .iter()
                .all(|r| r.c0_identity && r.regularity_certified && !r.closure)
        );
        let short = level
            .certify_retained_station_seams(2, proof.exact_work - 1)
            .unwrap();
        assert!(!short.all_station_seams_certified && short.exact_work <= proof.exact_work - 1);
        // A missing station chunk must not turn a partial surface group into
        // a vacuous positive audit, even when other seam predicates pass.
        let mut partial = sweep.preview_at(65).unwrap();
        partial.patches.remove(1);
        let missing = partial.certify_retained_station_seams(1, 1000000).unwrap();
        assert!(!missing.all_station_seams_certified);
        assert_eq!(
            missing.reason,
            Some("retained-station-chunk-correspondence-unproved")
        );
    }
    #[test]
    fn rational_circle_station_g2_uses_exact_along_chain_and_closure_jets() {
        let profile = crate::primitives::circle([0.; 3], [0., 0., 1.], 1.).unwrap();
        let path = crate::primitives::line([0.; 3], [0., 0., 4.]).unwrap();
        let scale = constant_vector_law([1., 0., 0.]).unwrap();
        let twist = constant_vector_law([0.; 3]).unwrap();
        let options = Options {
            normal: [1., 0., 0.],
            orientation: Orientation::RotationMinimizing,
            spacing: Spacing::Parameter,
            initial_sections: 3,
            max_sections: 5,
            max_deviation: 0.01,
        };
        let level = Sweep::new(&profile, &path, &scale, &twist, options)
            .unwrap()
            .preview_at(5)
            .unwrap();
        let proof = level.certify_retained_station_seams(2, 1000000).unwrap();
        assert!(proof.all_station_seams_certified, "{proof:?}");
        assert_eq!(proof.seams.len(), 3);
        assert!(
            proof
                .seams
                .iter()
                .all(|s| s.regularity_certified && s.c0_identity)
        );
        assert!(
            !level
                .certify_retained_station_seams(2, proof.exact_work - 1)
                .unwrap()
                .all_station_seams_certified
        );
        let mut wrong = profile.clone();
        wrong.control_points[3][0] = wrong.control_points[3][0].next_up();
        let level = Sweep::new(&wrong, &path, &scale, &twist, options)
            .unwrap()
            .preview_at(5)
            .unwrap();
        let refused = level.certify_retained_station_seams(2, 1000000).unwrap();
        assert!(!refused.all_station_seams_certified);
        assert!(refused.seams.iter().all(|s| s.c0_identity));
    }
    #[test]
    fn actual_closed_rmf_station_seam_keeps_c0_distinct_from_g1() {
        let profile = crate::primitives::line([1., 0., 1.], [1., 0., 2.]).unwrap();
        let path = crate::primitives::circle([0.; 3], [0., 0., 1.], 1.).unwrap();
        let scale = constant_vector_law([1., 0., 0.]).unwrap();
        let twist = constant_vector_law([0.; 3]).unwrap();
        let sweep = Sweep::new(
            &profile,
            &path,
            &scale,
            &twist,
            Options {
                normal: [0., 0., 1.],
                orientation: Orientation::RotationMinimizing,
                spacing: Spacing::Parameter,
                initial_sections: 5,
                max_sections: 33,
                max_deviation: 1.,
            },
        )
        .unwrap();
        let level = sweep.preview_at(33).unwrap();
        assert!(level.report.closed_path);
        let proof = level.certify_retained_station_seams(1, 1000000).unwrap();
        assert!(!proof.all_station_seams_certified);
        assert!(proof.seams.iter().all(|r| r.c0_identity), "{proof:?}");
        let closure = proof.seams.iter().filter(|r| r.closure).collect::<Vec<_>>();
        assert_eq!(closure.len(), 1);
        assert_eq!(proof.seams.len(), 32);
        assert!(closure.iter().all(|r| !r.certified));
    }
    #[test]
    fn actual_retained_straight_sweep_seams_prove_g2_and_charge_shared_work() {
        let profile = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
        let path = crate::primitives::line([0.; 3], [0., 0., 4.]).unwrap();
        let scale = constant_vector_law([1., 0., 0.]).unwrap();
        let twist = constant_vector_law([0.; 3]).unwrap();
        let sweep = Sweep::new(
            &profile,
            &path,
            &scale,
            &twist,
            Options {
                normal: [1., 0., 0.],
                orientation: Orientation::RotationMinimizing,
                spacing: Spacing::Parameter,
                initial_sections: 3,
                max_sections: 65,
                max_deviation: 0.01,
            },
        )
        .unwrap();
        let across = sweep
            .preview_at(65)
            .unwrap()
            .certify_retained_station_seams(2, 1000000)
            .unwrap();
        assert!(across.all_station_seams_certified, "{across:?}");
        assert_eq!(across.seams.len(), 63);
        let level = sweep.preview_at(5).unwrap();
        let proof = level.certify_retained_station_seams(2, 1000000).unwrap();
        assert!(proof.all_station_seams_certified, "{proof:?}");
        assert_eq!(proof.seams.len(), 3);
        assert!(
            proof
                .seams
                .iter()
                .all(|r| r.c0_identity && r.regularity_certified)
        );
        assert!(proof.exact_work > 0);
        assert!(
            !level
                .certify_retained_station_seams(2, proof.exact_work - 1)
                .unwrap()
                .all_station_seams_certified
        );
        assert!(
            !level
                .certify_retained_station_seams(2, 0)
                .unwrap()
                .all_station_seams_certified
        );
        let mut kink = level.clone();
        kink.patches[0].control_points[1][2][0] += 0.125;
        let refused = kink.certify_retained_station_seams(1, 1000000).unwrap();
        assert!(
            !refused.all_station_seams_certified && refused.seams.iter().all(|r| r.c0_identity)
        );
        let mut singular = level;
        singular.patches[0].control_points[1] = singular.patches[0].control_points[0].clone();
        assert!(
            !singular
                .certify_retained_station_seams(2, 1000000)
                .unwrap()
                .all_station_seams_certified
        );
    }
}

#[cfg(test)]
#[test]
fn nonuniform_retained_stations_require_exact_scaled_g2_not_equal_speed() {
    for stations in [[0., 1., 3., 7., 15.], [0., 3., 10., 21., 34.]] {
        let profiles = [
            crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap(),
            Curve {
                degree: 2,
                knots: vec![0., 0., 0., 0.25, 0.25, 0.5, 0.5, 1., 1., 1.],
                control_points: [1., 2.5, 4., 7.5, 11., 16.5, 22.]
                    .iter()
                    .map(|&x| vec![x, 0., 0.])
                    .collect(),
                weights: vec![1.; 7],
                periodic: false,
            },
        ];
        for profile in profiles {
            let path = Curve {
                degree: 1,
                knots: vec![0., 0., 0.25, 0.5, 0.75, 1., 1.],
                control_points: stations.iter().map(|&z| vec![0., 0., z]).collect(),
                weights: vec![1.; 5],
                periodic: false,
            };
            let scale = constant_vector_law([1., 0., 0.]).unwrap();
            let twist = constant_vector_law([0.; 3]).unwrap();
            let sweep = Sweep::new(
                &profile,
                &path,
                &scale,
                &twist,
                Options {
                    normal: [1., 0., 0.],
                    orientation: Orientation::RotationMinimizing,
                    spacing: Spacing::Parameter,
                    initial_sections: 5,
                    max_sections: 5,
                    max_deviation: 1.,
                },
            )
            .unwrap();
            let level = sweep.preview_at(5).unwrap();
            let proof = level.certify_retained_station_seams(2, 1000000).unwrap();
            assert!(proof.all_station_seams_certified, "{proof:?}");
            assert_eq!(proof.seams.len(), 3);
            assert!(
                proof
                    .seams
                    .iter()
                    .all(|s| s.c0_identity && s.regularity_certified)
            );
            assert!(
                !level
                    .certify_retained_station_seams(2, proof.exact_work - 1)
                    .unwrap()
                    .all_station_seams_certified
            );
            if stations[1] == 3. {
                // Moving an interior station along a straight strip changes its
                // exact speed, not its geometric G2. Prove the represented ratio
                // without restoring or snapping the original station.
                let mut perturbed = level.clone();
                for row in &mut perturbed.patches[0].control_points {
                    row[2][2] = f64::from_bits(row[2][2].to_bits() + 1);
                }
                let report = perturbed
                    .certify_retained_station_seams(2, 1000000)
                    .unwrap();
                assert!(report.all_station_seams_certified,"{report:?}");
                assert!(report.seams.iter().all(|s| s.c0_identity));
                assert_ne!(perturbed.patches[0].control_points,level.patches[0].control_points);
                let mut transverse=perturbed.clone();
                transverse.patches[0].control_points[1][2][0]+=f64::EPSILON*32.;
                assert!(!transverse.certify_retained_station_seams(2,1000000).unwrap().all_station_seams_certified);
            }
            let mut kink = level.clone();
            kink.patches[0].control_points[1][2][0] += 0.125;
            assert!(
                !kink
                    .certify_retained_station_seams(1, 1000000)
                    .unwrap()
                    .all_station_seams_certified
            );
        }
    }
}

#[cfg(test)]
#[test]
fn moving_authored_frame_retained_g2_depends_on_actual_profile_image() {
    let profile = crate::primitives::line([0., 1., 0.], [0., 2., 0.]).unwrap();
    let path = Curve {
        degree: 1,
        knots: vec![0., 0., 0.25, 0.5, 0.75, 1., 1.],
        control_points: [0., 1., 3., 7., 15.]
            .iter()
            .map(|&z| vec![0., 0., z])
            .collect(),
        weights: vec![1.; 5],
        periodic: false,
    };
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let axis = constant_vector_law([0., 1., 0.]).unwrap();
    let normal = crate::primitives::line([1., 0., 0.], [1., 0., 1.]).unwrap();
    let options = Options {
        normal: [1., 0., 0.],
        orientation: Orientation::Fixed,
        spacing: Spacing::Parameter,
        initial_sections: 5,
        max_sections: 5,
        max_deviation: 1.,
    };
    let sweep =
        Sweep::new_authored(&profile, &path, &scale, &twist, &axis, &normal, options).unwrap();
    let level = sweep.preview_at(5).unwrap();
    // This genuinely moving normal leaves a profile on the fixed longitudinal
    // axis unchanged. Verify the stored surface image, not the source frame.
    for (row, y) in level.patches[0].control_points.iter().zip([1., 2.]) {
        for (pole, z) in row.iter().zip([0., 1., 3., 7., 15.]) {
            assert_eq!(pole, &vec![0., y, z]);
        }
    }
    let proof = level.certify_retained_station_seams(2, 1000000).unwrap();
    assert!(proof.all_station_seams_certified, "{proof:?}");
    assert_eq!(proof.seams.len(), 3);
    assert!(
        !level
            .certify_retained_station_seams(2, proof.exact_work - 1)
            .unwrap()
            .all_station_seams_certified
    );
    let sensitive = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
    let level = Sweep::new_authored(&sensitive, &path, &scale, &twist, &axis, &normal, options)
        .unwrap()
        .preview_at(5)
        .unwrap();
    let refused = level.certify_retained_station_seams(1, 1000000).unwrap();
    assert!(!refused.all_station_seams_certified);
    assert!(refused.seams.iter().all(|seam| seam.c0_identity));
}
