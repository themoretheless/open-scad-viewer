//! Shared retained source edge with exact rational-linear parameter maps.
//! Original definitions are replayed; this does not admit a sewn Model.
use crate::{
    source_boundary_fragment::Fragment,
    source_projective_use::{self, ProjectiveUse},
};
use nurbs_core::{Error, Result, curve::Curve};
#[cfg(feature = "codec")]
use value_codec::{Deserialize, Value, json};
type Plane = [[f64; 3]; 3];
type Candidates = [[Option<[f64; 2]>; 2]; 2];
pub type FractionCandidates = [[Option<[[f64; 2]; 2]>; 2]; 2];
#[derive(Clone)]
pub struct Edge {
    uses: [ProjectiveUse; 2],
    planes: [Option<Plane>; 2],
    candidates: Candidates,
    fraction_candidates: Option<FractionCandidates>,
}
impl Edge {
    pub fn same_definition(&self, other: &Self) -> bool {
        self.planes == other.planes
            && self.candidates == other.candidates
            && self.fraction_candidates == other.fraction_candidates
            && self.uses.iter().zip(&other.uses).all(|(a, b)| {
                a.world() == b.world()
                    && a.range() == b.range()
                    && a.weights() == b.weights()
                    && a.fragment().same_definition(b.fragment())
            })
    }

    pub fn world(&self) -> &Curve {
        self.uses[0].world()
    }
    pub fn uses(&self) -> &[ProjectiveUse; 2] {
        &self.uses
    }
    pub fn sample_interior(
        &self,
        canonical: f64,
        max_spans: usize,
    ) -> Result<crate::source_shared_edge::BoundarySampleReport<'_, Self>> {
        crate::source_shared_edge::sample::sample_interior(self, canonical, max_spans)
    }
    pub fn sample_interior_chain(
        &self,
        canonical: &[f64],
        max_spans: usize,
    ) -> Result<crate::source_shared_edge::BoundaryChainReport<'_, Self>> {
        crate::source_shared_edge::sample::sample_interior_chain(self, canonical, max_spans)
    }
    pub fn sample_endpoint(
        &self,
        end: usize,
        max_spans: usize,
    ) -> Result<crate::source_shared_edge::BoundaryEndpointReport<'_, Self>> {
        crate::source_shared_edge::endpoint::sample_endpoint(self, end, max_spans)
    }
    pub fn prepare_boundary_mesh(
        &self,
        limits: crate::source_shared_edge::BoundaryMeshLimits,
    ) -> Result<crate::source_shared_edge::BoundaryMeshReport> {
        crate::source_shared_edge::mesh::prepare_boundary_mesh(self, limits)
    }
    #[cfg(feature = "codec")]
    pub fn definition(&self) -> Value {
        let mut definition = json!({"version":1,"uses":self.uses.each_ref().map(|u|u.definition()),
            "planes":self.planes,"candidates":self.candidates});
        if let Some(candidates) = self.fraction_candidates {
            definition["fractionCandidates"] = json!(candidates);
        }
        definition
    }
}
pub struct Report {
    pub edge: Option<Edge>,
    pub exact_work: u64,
    pub root_checks: usize,
    pub driver_cells: usize,
    pub reason: &'static str,
}
fn invalid() -> Error {
    Error::new(
        "BREP_SOURCE_PROJECTIVE_EDGE",
        "Invalid projective shared-edge input or budget",
    )
}
fn initial(max_work: u64, max_driver: usize) -> Result<Report> {
    if !(1..=100000000).contains(&max_work) || max_driver > 100000 {
        return Err(invalid());
    }
    Ok(Report {
        edge: None,
        exact_work: 0,
        root_checks: 0,
        driver_cells: 0,
        reason: "projective-shared-composition-unproven",
    })
}
fn finish(
    uses: [ProjectiveUse; 2],
    planes: [Option<Plane>; 2],
    candidates: Candidates,
    fraction_candidates: Option<FractionCandidates>,
    mut report: Report,
    max_work: u64,
    max_driver: usize,
) -> Result<Report> {
    report.reason = "projective-shared-uses-not-opposite";
    if uses[0].world() != uses[1].world() || uses[0].reversed() == uses[1].reversed() {
        return Ok(report);
    }
    report.reason = "projective-shared-endpoints-unproven";
    for i in 0..2 {
        if report.exact_work == max_work {
            return Ok(report);
        }
        let (mut same, work, checks) = source_projective_use::same_parameter_endpoint_counted(
            uses.each_ref(),
            [i, 1 - i],
            [candidates[0][i], candidates[1][1 - i]],
            max_work - report.exact_work,
        )?;
        report.exact_work += work;
        report.root_checks += checks;
        if !same && report.exact_work < max_work {
            if let Some(candidates) = fraction_candidates {
                let (proved, work, checks) = source_projective_use::same_fraction_endpoint_counted(
                    uses.each_ref(),
                    [i, 1 - i],
                    [candidates[0][i], candidates[1][1 - i]],
                    max_work - report.exact_work,
                )?;
                report.exact_work += work;
                report.root_checks += checks;
                same = proved;
            }
        }
        if !same && report.exact_work < max_work {
            let (proved, work, checks) = source_projective_use::same_linear_endpoint_counted(
                uses.each_ref(),
                [i, 1 - i],
                max_work - report.exact_work,
            )?;
            report.exact_work += work;
            report.root_checks += checks;
            same = proved;
        }
        if !same {
            if let Some(plane) = planes[i] {
                if report.exact_work == max_work {
                    return Ok(report);
                }
                let (proved, root) = source_projective_use::same_crossing_endpoint(
                    uses.each_ref(),
                    [i, 1 - i],
                    plane,
                    max_work - report.exact_work,
                    max_driver - report.driver_cells,
                )?;
                report.exact_work += root.work_used;
                report.driver_cells += root.driver_cells;
                report.root_checks += root.root_checks;
                same = proved;
            }
        }
        if !same {
            return Ok(report);
        }
    }
    report.edge = Some(Edge {
        uses,
        planes,
        candidates,
        fraction_candidates,
    });
    report.reason = "projective-shared-edge-qualified";
    Ok(report)
}
/// Both original source compositions and both retained endpoint identities
/// share one exact-work budget. Root driving shares one cell budget.
pub fn qualify(
    world: &Curve,
    fragments: [&Fragment; 2],
    ranges: [[[f64; 2]; 2]; 2],
    weights: [[f64; 2]; 2],
    planes: [Option<Plane>; 2],
    candidates: Candidates,
    max_work: u64,
    max_driver: usize,
) -> Result<Report> {
    qualify_impl(
        world, fragments, ranges, weights, planes, candidates, None, max_work, max_driver,
    )
}
/// Rational parameter proposals are freshly checked against original roots.
pub fn qualify_with_fraction_candidates(
    world: &Curve,
    fragments: [&Fragment; 2],
    ranges: [[[f64; 2]; 2]; 2],
    weights: [[f64; 2]; 2],
    planes: [Option<Plane>; 2],
    candidates: Candidates,
    fraction_candidates: FractionCandidates,
    max_work: u64,
    max_driver: usize,
) -> Result<Report> {
    qualify_impl(
        world,
        fragments,
        ranges,
        weights,
        planes,
        candidates,
        Some(fraction_candidates),
        max_work,
        max_driver,
    )
}
/// Discover bounded rational endpoint proposals, then freshly qualify both
/// original compositions and canonical endpoint identities with the remainder.
pub fn qualify_with_discovered_fraction_candidates(
    world: &Curve,
    fragments: [&Fragment; 2],
    ranges: [[[f64; 2]; 2]; 2],
    weights: [[f64; 2]; 2],
    planes: [Option<Plane>; 2],
    candidates: Candidates,
    max_denominator: usize,
    max_candidates_per_axis: usize,
    max_work: u64,
    max_driver: usize,
) -> Result<Report> {
    use crate::source_boundary_fragment::Endpoint;
    let mut spent = initial(max_work, max_driver)?;
    if !(1..=4096).contains(&max_denominator) || !(1..=64).contains(&max_candidates_per_axis) {
        return Err(invalid());
    }
    let mut fractions: FractionCandidates = [[None; 2]; 2];
    for side in 0..2 {
        for end in 0..2 {
            if let Endpoint::Crossing { point, .. } = &fragments[side].endpoints()[end] {
                if spent.exact_work == max_work {
                    spent.reason = "projective-root-discovery-work-exhausted";
                    return Ok(spent);
                }
                let discovered = crate::source_root_parameter::discover_fraction(
                    point,
                    max_denominator,
                    max_candidates_per_axis,
                    (max_work - spent.exact_work).min(cad_predicates::MAX_WORK),
                )?;
                spent.exact_work += discovered.work_used;
                spent.root_checks += discovered.candidate_checks;
                fractions[side][end] = discovered.parameters;
            }
        }
    }
    if spent.exact_work == max_work {
        spent.reason = "projective-root-discovery-work-exhausted";
        return Ok(spent);
    }
    let fractions = fractions
        .iter()
        .flatten()
        .any(Option::is_some)
        .then_some(fractions);
    let mut report = qualify_impl(
        world,
        fragments,
        ranges,
        weights,
        planes,
        candidates,
        fractions,
        max_work - spent.exact_work,
        max_driver,
    )?;
    report.exact_work += spent.exact_work;
    report.root_checks += spent.root_checks;
    Ok(report)
}
fn qualify_impl(
    world: &Curve,
    fragments: [&Fragment; 2],
    ranges: [[[f64; 2]; 2]; 2],
    weights: [[f64; 2]; 2],
    planes: [Option<Plane>; 2],
    candidates: Candidates,
    fraction_candidates: Option<FractionCandidates>,
    max_work: u64,
    max_driver: usize,
) -> Result<Report> {
    let mut report = initial(max_work, max_driver)?;
    let mut uses = Vec::new();
    for i in 0..2 {
        if report.exact_work == max_work {
            return Ok(report);
        }
        let r = source_projective_use::qualify(
            world,
            fragments[i],
            ranges[i],
            weights[i],
            max_work - report.exact_work,
        )?;
        report.exact_work += r.exact_work;
        let Some(u) = r.mapped else {
            report.reason = r.reason;
            return Ok(report);
        };
        uses.push(u);
    }
    finish(
        [uses.remove(0), uses.remove(0)],
        planes,
        candidates,
        fraction_candidates,
        report,
        max_work,
        max_driver,
    )
}
/// Restore each original fragment/root under a bounded mapping allowance,
/// then repeat composition, orientation and shared-endpoint qualification.
#[cfg(feature = "codec")]
pub fn restore(
    value: Value,
    max_mapping_cells_per_use: usize,
    max_work: u64,
    max_driver: usize,
) -> Result<Report> {
    let mut report = initial(max_work, max_driver)?;
    if value["version"].as_u64() != Some(1) || !(1..=100000).contains(&max_mapping_cells_per_use) {
        return Err(invalid());
    }
    let definitions = value["uses"]
        .as_array()
        .filter(|a| a.len() == 2)
        .ok_or_else(invalid)?;
    let planes =
        <[Option<Plane>; 2]>::from_value(value["planes"].clone()).map_err(|_| invalid())?;
    let candidates = Candidates::from_value(value["candidates"].clone()).map_err(|_| invalid())?;
    let fraction_candidates = if value["fractionCandidates"].is_null() {
        None
    } else {
        Some(
            FractionCandidates::from_value(value["fractionCandidates"].clone())
                .map_err(|_| invalid())?,
        )
    };
    let mut uses = Vec::new();
    for definition in definitions {
        if report.exact_work == max_work {
            return Ok(report);
        }
        let r = source_projective_use::restore(
            definition.clone(),
            max_mapping_cells_per_use,
            max_work - report.exact_work,
        )?;
        report.exact_work += r.exact_work;
        let Some(u) = r.mapped else {
            report.reason = r.reason;
            return Ok(report);
        };
        uses.push(u);
    }
    finish(
        [uses.remove(0), uses.remove(0)],
        planes,
        candidates,
        fraction_candidates,
        report,
        max_work,
        max_driver,
    )
}

#[cfg(test)]
mod fraction_tests {
    use super::*;
    use crate::source_boundary_fragment::{Endpoint, Role};
    use nurbs_core::surface::Surface;
    #[test]
    fn nonlinear_rational_root_candidates_admit_shared_edge_and_replay() {
        let surface = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let a = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0., 0.], vec![0.5, 0.], vec![1., 1.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let mut b = a.clone();
        b.weights = vec![1., 2., 4.];
        let world = Curve {
            control_points: a
                .control_points
                .iter()
                .map(|p| vec![p[0], p[1], 0.])
                .collect(),
            ..a.clone()
        };
        let cutter = Curve::from_polyline(vec![vec![0., 0.], vec![3., 1.]]).unwrap();
        let ra = crate::source_contact_point::qualify(
            &surface,
            &a,
            &cutter,
            [[0.3, 0.36], [0.10, 0.12]],
            10000,
        )
        .unwrap();
        let rb = crate::source_contact_point::qualify(
            &surface,
            &b,
            &cutter,
            [[0.18, 0.22], [0.10, 0.12]],
            10000,
        )
        .unwrap();
        let fa = Fragment::new(
            &surface,
            &a,
            Endpoint::Parameter(0.),
            Endpoint::Crossing {
                point: ra.point.expect(ra.reason),
                role: Role::Boundary,
            },
        )
        .unwrap();
        let fb = Fragment::new(
            &surface,
            &b,
            Endpoint::Crossing {
                point: rb.point.expect(rb.reason),
                role: Role::Boundary,
            },
            Endpoint::Parameter(0.),
        )
        .unwrap();
        let ranges = [[[0., 1.], [1., 1.]]; 2];
        let weights = [[1., 1.], [1., 2.]];
        assert!(
            qualify(
                &world,
                [&fa, &fb],
                ranges,
                weights,
                [None; 2],
                [[None; 2]; 2],
                100000000,
                10000
            )
            .unwrap()
            .edge
            .is_none()
        );
        let candidates = [
            [None, Some([[1., 3.], [1., 9.]])],
            [Some([[1., 5.], [1., 9.]]), None],
        ];
        let report = qualify_with_fraction_candidates(
            &world,
            [&fa, &fb],
            ranges,
            weights,
            [None; 2],
            [[None; 2]; 2],
            candidates,
            100000000,
            10000,
        )
        .unwrap();
        let exact_work = report.exact_work;
        assert_eq!(report.driver_cells, 0);
        assert_eq!(report.root_checks, 2);
        let edge = report.edge.expect(report.reason);
        let definition = edge.definition();
        let discovered = qualify_with_discovered_fraction_candidates(
            &world,
            [&fa, &fb],
            ranges,
            weights,
            [None; 2],
            [[None; 2]; 2],
            128,
            16,
            100000000,
            10000,
        )
        .unwrap();
        let discovery_work = discovered.exact_work;
        assert!(discovery_work > exact_work && discovery_work <= 100000000);
        assert_eq!(
            discovered.edge.expect(discovered.reason).definition(),
            definition
        );
        assert!(
            qualify_with_discovered_fraction_candidates(
                &world,
                [&fa, &fb],
                ranges,
                weights,
                [None; 2],
                [[None; 2]; 2],
                128,
                16,
                discovery_work - 1,
                10000
            )
            .unwrap()
            .edge
            .is_none()
        );

        let bytes = value_codec::encode_binary(&definition).unwrap();
        let replay = restore(
            value_codec::decode_binary(&bytes).unwrap(),
            10000,
            100000000,
            10000,
        )
        .unwrap();
        let native = edge
            .requalify(crate::source_shared_edge::ReplayLimits {
                mapping_cells_per_use: 10000,
                exact_work: 100000000,
                driver_cells: 10000,
            })
            .unwrap();
        assert_eq!(
            (
                native.exact_work,
                native.root_checks,
                native.driver_cells,
                native.reason
            ),
            (
                replay.exact_work,
                replay.root_checks,
                replay.driver_cells,
                replay.reason
            )
        );
        assert_eq!(native.edge.unwrap().definition(), definition);
        let replay = replay.edge.expect(replay.reason);
        assert_eq!(replay.definition(), definition);
        for source in [&edge, &replay] {
            let proof = crate::source_seam_tangency::qualify(
                source,
                crate::source_seam_tangency::Limits {
                    max_sine_squared: 1e-10,
                    cells: 1000,
                    curve_spans: 2000,
                    normal_spans: 2000,
                },
            )
            .unwrap();
            assert!(proof.seam.is_some(), "{}", proof.reason);
            assert_eq!(source.uses()[0].fragment().definition(), fa.definition());
            assert_eq!(source.uses()[1].fragment().definition(), fb.definition());
        }
        let restriction = crate::source_edge_restriction::Restriction::from_edge(&edge);
        let bytes = value_codec::encode_binary(&restriction.definition()).unwrap();
        let restored_restriction = crate::source_edge_restriction::restore_projective(
            value_codec::decode_binary(&bytes).unwrap(),
            crate::source_shared_edge_restore::Limits {
                mapping_cells_per_use: 10000,
                exact_work: 100000000,
                driver_cells: 10000,
            },
        )
        .unwrap()
        .unwrap();
        assert_eq!(restored_restriction.definition(), restriction.definition());
        assert_eq!(
            restored_restriction.parameter_bounds().unwrap(),
            restriction.parameter_bounds().unwrap()
        );
        assert_eq!(
            restored_restriction.endpoint_boxes(100).unwrap(),
            restriction.endpoint_boxes(100).unwrap()
        );
        assert_eq!(
            restored_restriction.display_segments(16).unwrap(),
            restriction.display_segments(16).unwrap()
        );
        assert!(
            qualify_with_fraction_candidates(
                &world,
                [&fa, &fb],
                ranges,
                weights,
                [None; 2],
                [[None; 2]; 2],
                candidates,
                exact_work - 1,
                10000
            )
            .unwrap()
            .edge
            .is_none()
        );
        let mut wrong = definition.clone();
        wrong["fractionCandidates"][1][0] = json!([[10., 49.], [1., 9.]]);
        assert!(
            restore(wrong, 10000, 100000000, 10000)
                .unwrap()
                .edge
                .is_none()
        );
        let mut wrong = definition.clone();
        wrong["uses"][1]["weights"] = json!([1., 1.]);
        assert!(
            restore(wrong, 10000, 100000000, 10000)
                .unwrap()
                .edge
                .is_none()
        );
        let mut missing = definition;
        missing["fractionCandidates"] = Value::Null;
        assert!(
            restore(missing, 10000, 100000000, 10000)
                .unwrap()
                .edge
                .is_none()
        );
    }
}

#[path = "source_projective_edge/replay.rs"]
mod replay;
