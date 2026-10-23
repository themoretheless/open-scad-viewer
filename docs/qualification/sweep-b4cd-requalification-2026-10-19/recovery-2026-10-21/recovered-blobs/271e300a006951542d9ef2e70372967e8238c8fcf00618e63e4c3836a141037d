//! Whole retained-level error certificate. Independent of regularity,
//! embedding and continuity certificates; positive fixed profile weights
//! transfer the control-trajectory bound to every profile parameter.
use super::{Sweep, frame_certificate::stored_point_error_upper, scalar_certificate::Status};
use crate::{Result, check, curve::Curve, distance_bounds::Interval as I};
#[derive(Clone, Debug)]
pub struct Report {
    pub status: Status,
    pub cells: usize,
    pub error_upper: Option<f64>,
    /// Whole endpoint contour deviation from the authored frame/law family.
    /// Does not by itself bound the filled cap material region.
    pub endpoint_contour_error_upper: Option<[f64;2]>,
    pub reason: Option<&'static str>,
}
fn unresolved(cells: usize, reason: &'static str) -> Report {
    Report {
        status: Status::Unresolved,
        cells,
        error_upper: None,
        endpoint_contour_error_upper: None,
        reason: Some(reason),
    }
}
#[derive(Clone, Debug)]
pub struct ProfileRegularityReport {
    pub spanwise_regular: bool,
    pub cells: usize,
    pub unresolved_profiles: Vec<usize>,
}
#[derive(Clone, Debug)]
pub struct WallRegularityReport {
    pub spanwise_regular: bool,
    pub cells: usize,
    /// Retained interval and profile indices whose Jacobian is unresolved.
    pub unresolved_patches: Vec<[usize; 2]>,
}
impl Sweep<'_> {
    /// Separate R obligation for all authored profiles, with a shared budget.
    /// Does not imply surface Jacobian regularity or global embedding.
    pub fn certify_profile_regularity(&self, max_cells: usize) -> Result<ProfileRegularityReport> {
        check(
            max_cells <= 100000,
            "Profile regularity budget exceeds100000",
        )?;
        let mut report = ProfileRegularityReport {
            spanwise_regular: true,
            cells: 0,
            unresolved_profiles: Vec::new(),
        };
        for (index, profile) in self.profiles.iter().enumerate() {
            let r = crate::curve_regularity::inspect(profile, max_cells - report.cells)?;
            report.cells += r.cells;
            if !r.spanwise_regular {
                report.unresolved_profiles.push(index);
            }
        }
        report.spanwise_regular = report.unresolved_profiles.is_empty();
        Ok(report)
    }
    /// R certificate for the actual retained ruled walls. Fixed positive
    /// profile weights make this exactly the linear section interpolation.
    /// Corner sides and the cyclic closing interval are checked independently.
    /// Caps, ideal unsampled trajectories and global embedding are separate.
    pub fn certify_wall_regularity(
        &self,
        sections: &[Vec<Curve>],
        max_cells: usize,
    ) -> Result<WallRegularityReport> {
        check(
            max_cells <= 100000 && (2..=1025).contains(&sections.len()),
            "Invalid wall regularity budget/sections",
        )?;
        for section in sections {
            check(
                section.len() == self.profiles.len(),
                "Wall profile count mismatch",
            )?;
            for (c, original) in section.iter().zip(self.profiles) {
                c.validate()?;
                check(
                    c.degree == original.degree
                        && c.knots == original.knots
                        && c.weights == original.weights
                        && c.periodic == original.periodic
                        && c.control_points.len() == original.control_points.len()
                        && c.control_points.iter().all(|p| p.len() == 3),
                    "Wall regularity requires unchanged profile correspondence",
                )?;
            }
        }
        let mut out = WallRegularityReport {
            spanwise_regular: false,
            cells: 0,
            unresolved_patches: Vec::new(),
        };
        for (index, pair) in sections.windows(2).enumerate() {
            for profile in 0..self.profiles.len() {
                let a = &pair[0][profile];
                let b = &pair[1][profile];
                let remaining = max_cells - out.cells;
                if remaining == 0 {
                    out.unresolved_patches.push([index, profile]);
                    continue;
                }
                let wall = crate::surface::Surface {
                    degree_u: a.degree,
                    degree_v: 1,
                    knots_u: a.knots.clone(),
                    knots_v: vec![0., 0., 1., 1.],
                    control_points: a
                        .control_points
                        .iter()
                        .zip(&b.control_points)
                        .map(|(p, q)| vec![p.clone(), q.clone()])
                        .collect(),
                    weights: a.weights.iter().map(|w| vec![*w, *w]).collect(),
                    periodic_u: a.periodic,
                    periodic_v: false,
                };
                match crate::surface_regularity::inspect(&wall, remaining) {
                    Ok(r) => {
                        out.cells += r.cells;
                        if !r.spanwise_regular {
                            out.unresolved_patches.push([index, profile]);
                        }
                    }
                    Err(_) => {
                        // Numeric failure never yields a partial R certificate.
                        out.cells = max_cells;
                        out.unresolved_patches.push([index, profile]);
                    }
                }
            }
        }
        out.spanwise_regular = out.unresolved_patches.is_empty();
        Ok(out)
    }
    /// Inter-patch I evidence with path-neighbor ownership derived from the
    /// retained station layout. Does not certify shared boundary interiors,
    /// self-patch injectivity, cross-profile loop ownership or containment.
    pub fn inspect_wall_separation(
        &self,
        sections: &[Vec<Curve>],
        clearance: f64,
        distance_tolerance: f64,
        max_pairs: usize,
        max_cells: usize,
    ) -> Result<crate::sweep_pair_audit::Report> {
        let (walls, shared) = self.retained_wall_charts(sections)?;
        crate::sweep_pair_audit::inspect(
            &walls,
            &shared,
            clearance,
            distance_tolerance,
            max_pairs,
            max_cells,
        )
    }
    /// Combined whole-chart injectivity and pair compatibility. Cross-profile
    /// loop boundary ownership, caps and shell containment remain separate.
    pub fn inspect_wall_geometry(
        &self,
        sections: &[Vec<Curve>],
        clearance: f64,
        distance_tolerance: f64,
        max_injectivity_cells: usize,
        max_pairs: usize,
        max_pair_cells: usize,
    ) -> Result<crate::sweep_wall_audit::Report> {
        let (walls, shared) = self.retained_wall_charts(sections)?;
        crate::sweep_wall_audit::inspect(
            &walls,
            &shared,
            clearance,
            distance_tolerance,
            max_injectivity_cells,
            max_pairs,
            max_pair_cells,
        )
    }
    /// Explicit closed-loop partition establishes profile-side ownership.
    /// No shared-boundary declaration is inferred between different loops.
    pub fn inspect_wall_geometry_with_loops(
        &self, sections: &[Vec<Curve>], loop_sizes: &[usize],
        clearance: f64, distance_tolerance: f64,
        max_injectivity_cells: usize, max_pairs: usize, max_pair_cells: usize,
    ) -> Result<crate::sweep_wall_audit::Report> {
        check((1..=16).contains(&loop_sizes.len())
            && loop_sizes.iter().all(|n| *n > 0 && *n <= self.profiles.len())
            && loop_sizes.iter().sum::<usize>() == self.profiles.len(),
            "Invalid wall audit loop partition")?;
        let (walls, mut shared) = self.retained_wall_charts(sections)?;
        let count = self.profiles.len();
        for section in sections {
            let mut offset = 0;
            for size in loop_sizes {
                for local in 0..*size {
                    let a = &section[offset+local];
                    let b = &section[offset+(local+1)%size];
                    let clamped = |c: &Curve| !c.periodic
                        && c.knots[..=c.degree].iter().all(|k| *k == c.knots[c.degree])
                        && c.knots[c.control_points.len()..].iter()
                            .all(|k| *k == c.knots[c.control_points.len()]);
                    let endpoints = |c: &Curve| -> Option<(Vec<f64>, Vec<f64>)> {
                        if clamped(c) {
                            return Some((c.control_points.first()?.clone(), c.control_points.last()?.clone()));
                        }
                        // Exact blossom extraction proves active-domain endpoints;
                        // exterior poles are not endpoints of an unclamped curve.
                        let pieces = crate::exact_curve_segments::inspect(c)?;
                        Some((pieces.first()?.control_points.first()?.clone(),
                            pieces.last()?.control_points.last()?.clone()))
                    };
                    let same_curve_seam = *size == 1
                        && crate::exact_curve_segments::translated_active_seam_closed(a);
                    check(same_curve_seam || endpoints(a).zip(endpoints(b)).is_some_and(|(a, b)| a.1 == b.0),
                        "Wall audit loop requires exact active-domain endpoint closure")?;
                }
                offset += size;
            }
        }
        for interval in 0..sections.len()-1 {
            let mut offset = 0;
            for size in loop_sizes {
                if *size > 1 {
                    for local in 0..*size {
                        let a = interval*count + offset + local;
                        let b = interval*count + offset + (local+1)%size;
                        shared.push([a.min(b),a.max(b)]);
                    }
                }
                offset += size;
            }
        }
        shared.sort_unstable();
        shared.dedup();
        crate::sweep_wall_audit::inspect(&walls, &shared, clearance,
            distance_tolerance, max_injectivity_cells, max_pairs, max_pair_cells)
    }
    fn retained_wall_charts(
        &self,
        sections: &[Vec<Curve>],
    ) -> Result<(Vec<crate::surface::Surface>, Vec<[usize; 2]>)> {
        let count = self.profiles.len();
        check(
            (2..=1025).contains(&sections.len()) && (sections.len() - 1) * count <= 1024,
            "Wall pair audit exceeds1024 patches",
        )?;
        for section in sections {
            check(section.len() == count, "Wall audit profile count mismatch")?;
            for (c, original) in section.iter().zip(self.profiles) {
                c.validate()?;
                check(
                    c.degree == original.degree
                        && c.knots == original.knots
                        && c.weights == original.weights
                        && c.periodic == original.periodic
                        && c.control_points.len() == original.control_points.len()
                        && c.control_points.iter().all(|p| p.len() == 3),
                    "Wall audit requires retained profile correspondence",
                )?;
            }
        }
        if self.options.closed {
            check(
                sections[0]
                    .iter()
                    .zip(sections.last().unwrap())
                    .all(|(a, b)| a.control_points == b.control_points),
                "Closed wall audit requires exact retained seam",
            )?;
        }
        let mut walls = Vec::new();
        for pair in sections.windows(2) {
            for p in 0..count {
                let a = &pair[0][p];
                let b = &pair[1][p];
                walls.push(crate::surface::Surface {
                    degree_u: a.degree,
                    degree_v: 1,
                    knots_u: a.knots.clone(),
                    knots_v: vec![0., 0., 1., 1.],
                    control_points: a
                        .control_points
                        .iter()
                        .zip(&b.control_points)
                        .map(|(x, y)| vec![x.clone(), y.clone()])
                        .collect(),
                    weights: a.weights.iter().map(|w| vec![*w, *w]).collect(),
                    periodic_u: a.periodic,
                    periodic_v: false,
                });
            }
        }
        let intervals = sections.len() - 1;
        let mut shared = Vec::new();
        for e in 0..intervals - 1 {
            for p in 0..count {
                shared.push([e * count + p, (e + 1) * count + p]);
            }
        }
        if self.options.closed && intervals > 2 {
            for p in 0..count {
                shared.push([p, (intervals - 1) * count + p]);
            }
        }
        Ok((walls, shared))
    }
    /// Each cell certifies one control trajectory on one retained interval.
    /// Scalar work has its own per-call budget. On exhaustion no partial
    /// maximum is returned. Incoming/outgoing and cyclic ownership are
    /// accounted for by comparing the actual stored endpoint to that edge's
    /// ideal endpoint, rather than assuming numerically identical frames.
    pub fn certify_level(
        &self,
        steps: usize,
        sections: &[Vec<Curve>],
        max_cells: usize,
        law_cells: usize,
    ) -> Result<Report> {
        check(
            steps > 0 && steps <= 1024 / self.tangents.len(),
            "Invalid certificate refinement",
        )?;
        check(
            max_cells <= 100000 && law_cells <= 100000,
            "Certificate budget exceeds100000",
        )?;
        check(
            sections.len() == self.tangents.len() * steps + 1,
            "Certificate section count mismatch",
        )?;
        for section in sections {
            check(
                section.len() == self.profiles.len(),
                "Certificate profile count mismatch",
            )?;
            for (stored, authored) in section.iter().zip(self.profiles) {
                stored.validate()?;
                check(
                    stored.degree == authored.degree
                        && stored.knots == authored.knots
                        && stored.weights == authored.weights
                        && stored.periodic == authored.periodic
                        && stored.control_points.len() == authored.control_points.len()
                        && stored.control_points.iter().all(|p| p.len() == 3),
                    "Certificate requires unchanged positive-weight profile correspondence",
                )?;
            }
        }
        let fraction = |i: usize| -> Result<I> {
            I::point(i as f64)
                .div(I::point(steps as f64))?
                .intersect(0., 1.)
        };
        let mut cells = 0;
        let mut maximum = 0_f64;
        let mut endpoint_maximum = [0_f64;2];
        for edge in 0..self.tangents.len() {
            for i in 0..steps {
                let a = fraction(i)?;
                let b = fraction(i + 1)?;
                let index = edge * steps + i;
                for (profile_index, profile) in self.profiles.iter().enumerate() {
                    for (control_index, p) in profile.control_points.iter().enumerate() {
                        if cells == max_cells {
                            return Ok(unresolved(cells, "cell-budget-exhausted"));
                        }
                        cells += 1;
                        let result = (|| -> Result<Option<f64>> {
                            let authored = [p[0], p[1], p[2]];
                            let frame = &self.transport_certificate;
                            let remainder=if self.frame_laws.is_some() || self.orientation_guide.is_some() {
                                let source=self.frame_source().unwrap();
                                let Some(local)=frame.profile_local_enclosure(authored)? else {return Ok(None);};
                                frame.interpolation_upper_frame_affine(edge,[a.lo,b.hi],local,source,self.scale,self.twist,self.affine_laws,law_cells)?
                            } else {
                                frame.interpolation_upper_affine(edge,[a.lo,b.hi],authored,self.scale,self.twist,self.affine_laws,law_cells)?
                            };
                            let Some(remainder)=remainder else {return Ok(None);};
                            let mut endpoint = 0_f64;
                            for (f, station) in [(a, index), (b, index + 1)] {
                                let ideal=if self.frame_laws.is_some() || self.orientation_guide.is_some() {
                                let source=self.frame_source().unwrap();
                                    let Some(local)=frame.profile_local_enclosure(authored)? else {return Ok(None);};
                                    frame.station_frame_affine(edge,[f.lo,f.hi],local,source,self.scale,self.twist,self.affine_laws,law_cells)?
                                } else {
                                    frame.station_with_affine_laws(edge,[f.lo,f.hi],authored,self.scale,self.twist,self.affine_laws,law_cells)?
                                };
                                let Some(ideal)=ideal else {return Ok(None);};
                                let stored =
                                    &sections[station][profile_index].control_points[control_index];
                                let point_error=stored_point_error_upper(
                                    [stored[0], stored[1], stored[2]],
                                    ideal,
                                )?;
                                endpoint = endpoint.max(point_error);
                                if station==0 { endpoint_maximum[0]=endpoint_maximum[0].max(point_error); }
                                if station+1==sections.len() { endpoint_maximum[1]=endpoint_maximum[1].max(point_error); }
                            }
                            Ok(Some(I::point(remainder).add(I::point(endpoint))?.hi))
                        })();
                        match result {
                            Ok(Some(bound)) => maximum = maximum.max(bound),
                            _ => return Ok(unresolved(cells, "trajectory-enclosure-unresolved")),
                        }
                    }
                }
            }
        }
        Ok(Report {
            status: Status::Certified,
            cells,
            error_upper: Some(maximum),
            endpoint_contour_error_upper: Some(endpoint_maximum),
            reason: None,
        })
    }
}
