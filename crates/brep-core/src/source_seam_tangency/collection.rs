//! Complete ordinary-edge coverage on one privately admitted original body.
//! This composes tolerance G1/G2 seam proofs, not face interiors or pole limits.
use super::{Carrier, CurvatureSeam, Limits, Seam};
use crate::source_volume::Body;
use nurbs_core::{Error, Result};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Class {
    /// Explicit authored sharp join; positional ownership comes from the body.
    C0,
    /// Require tangent planes within the common angular allowance everywhere.
    TangentPlanes,
    /// Require normal AND signed curvature tensor agreement under one coherent
    /// orientation on this complete original carrier. Tolerances are explicit.
    Curvature {
        normal_distance: f64,
        curvature_distance: f64,
    },
}

/// Cannot be assembled from caller reports, partial coverage or another body.
pub struct Collection<'a, E: Carrier> {
    body: &'a Body<E>,
    classes: Vec<Class>,
    seams: Vec<(usize, Seam<E>)>,
    curvature_seams: Vec<(usize, CurvatureSeam<E>)>,
    polar_seams: Vec<(usize, super::PolarSeam<E>)>,
    polar_curvature_seams: Vec<(usize, super::PolarCurvatureSeam<E>)>,
    max_sine_squared: f64,
}
impl<E: Carrier> Collection<'_, E> {
    pub fn body(&self) -> &Body<E> {
        self.body
    }
    /// Complete carriers including collapsed-endpoint limit planes.
    pub fn polar_seams(&self) -> &[(usize, super::PolarSeam<E>)] {
        &self.polar_seams
    }
    pub fn polar_curvature_seams(&self) -> &[(usize, super::PolarCurvatureSeam<E>)] {
        &self.polar_curvature_seams
    }
    pub fn classes(&self) -> &[Class] {
        &self.classes
    }
    pub fn seams(&self) -> &[(usize, Seam<E>)] {
        &self.seams
    }
    pub fn curvature_seams(&self) -> &[(usize, CurvatureSeam<E>)] {
        &self.curvature_seams
    }
    /// Angular allowance for TangentPlanes entries; Curvature entries retain
    /// their own explicit normal-distance and curvature-distance allowances.
    pub fn max_sine_squared(&self) -> f64 {
        self.max_sine_squared
    }
    /// Ordinary shared edges only. This does not assert global surface G1/G2.
    pub fn all_edges_tangent(&self) -> bool {
        self.classes.iter().all(|c| *c != Class::C0)
    }
}
impl<E: Carrier> Collection<'_, E> {
    /// All ordinary edges at their declared curvature tolerances; not face G2.
    pub fn all_edges_curvature(&self) -> bool {
        self.classes
            .iter()
            .all(|c| matches!(c, Class::Curvature { .. }))
    }
}
pub struct CollectionReport<'a, E: Carrier> {
    pub certificate: Option<Collection<'a, E>>,
    pub cells: usize,
    pub curve_spans: usize,
    /// Shared source-surface work, including normal and curvature knot cells.
    pub normal_spans: usize,
    pub polar_coefficients: usize,
    pub polar_shape_products: u64,
    pub polar_exact_work: u64,
    pub regular_exact_work: u64,
    pub polar_shape_coefficients: usize,
    pub completed_edges: usize,
    pub uncertain_edge: Option<usize>,
    pub reason: &'static str,
}

struct Work<E: Carrier> {
    seams: Vec<(usize, Seam<E>)>,
    curvature_seams: Vec<(usize, CurvatureSeam<E>)>,
    polar_seams: Vec<(usize, super::PolarSeam<E>)>,
    polar_curvature_seams: Vec<(usize, super::PolarCurvatureSeam<E>)>,
    cells: usize,
    curve_spans: usize,
    normal_spans: usize,
    polar_coefficients: usize,
    polar_shape_products: u64,
    polar_exact_work: u64,
    regular_exact_work: u64,
    polar_shape_coefficients: usize,
    completed_edges: usize,
    uncertain_edge: Option<usize>,
    reason: &'static str,
}
pub(super) fn validate_classes(classes: &[Class]) -> Result<()> {
    for class in classes {
        if let Class::Curvature {
            normal_distance,
            curvature_distance,
        } = class
        {
            if !normal_distance.is_finite()
                || !(0. ..1.).contains(normal_distance)
                || !curvature_distance.is_finite()
                || *curvature_distance < 0.
            {
                return Err(Error::new(
                    "BREP_SOURCE_SEAM_COLLECTION",
                    "Choose finite original curvature tolerances",
                ));
            }
        }
    }
    Ok(())
}
fn inspect_edges<E: Carrier>(edges: &[E], classes: &[Class], limits: Limits) -> Result<Work<E>> {
    inspect_with_poles(edges, classes, limits, None, None)
}
type PolarContext<'p, 's, E> = (
    &'p crate::source_pole_tangency::BodyPlaneLimits<'s, E>,
    &'p [[crate::source_shell_incidence::Address; 2]],
    usize,
    Option<(u64, usize, Option<u64>)>,
);
fn inspect_with_poles<E: Carrier>(
    edges: &[E],
    classes: &[Class],
    limits: Limits,
    polar: Option<PolarContext<'_, '_, E>>,
    regular_exact: Option<(&[usize], u64)>,
) -> Result<Work<E>> {
    super::validate_limits(&limits)?;
    if edges.is_empty() || edges.len() > 100_000 || classes.len() != edges.len() {
        return Err(Error::new(
            "BREP_SOURCE_SEAM_COLLECTION",
            "Classify every original edge exactly once",
        ));
    }
    if let Some((axes, max)) = regular_exact {
        if axes.len() != edges.len()
            || axes.iter().any(|a| *a > 2)
            || !(1..=100000000).contains(&max)
        {
            return Err(Error::new(
                "BREP_SOURCE_EXACT_CURVATURE",
                "Use complete source projection proposals and bounded shared exact work",
            ));
        }
    }
    validate_classes(classes)?;
    let mut out = Work {
        curvature_seams: Vec::new(),
        polar_seams: Vec::new(),
        polar_curvature_seams: Vec::new(),
        seams: Vec::new(),
        cells: 0,
        curve_spans: 0,
        normal_spans: 0,
        polar_coefficients: 0,
        polar_shape_products: 0,
        polar_exact_work: 0,
        regular_exact_work: 0,
        polar_shape_coefficients: 0,
        completed_edges: 0,
        uncertain_edge: None,
        reason: "source-seam-collection-qualified",
    };
    for (index, (edge, class)) in edges.iter().zip(classes).enumerate() {
        if *class == Class::C0 {
            out.completed_edges += 1;
            continue;
        }
        out.uncertain_edge = Some(index);
        if out.cells == limits.cells
            || out.curve_spans == limits.curve_spans
            || out.normal_spans == limits.normal_spans
        {
            out.reason = "source-seam-collection-work-limit";
            return Ok(out);
        }
        if *class == Class::TangentPlanes {
            if let Some((atlas, uses, budget, _)) = polar {
                let proof = |side: usize| {
                    atlas
                        .poles()
                        .iter()
                        .find(|(a, _)| a.face == uses[index][side].face)
                        .map(|(_, p)| p.limit())
                };
                if let (Some(a), Some(b)) = (proof(0), proof(1)) {
                    if out.polar_coefficients == budget {
                        out.reason = "source-polar-seam-collection-work-limit";
                        return Ok(out);
                    }
                    let r = super::qualify_polar_tangency(
                        edge,
                        [a, b],
                        Limits {
                            max_sine_squared: limits.max_sine_squared,
                            cells: limits.cells - out.cells,
                            curve_spans: limits.curve_spans - out.curve_spans,
                            normal_spans: limits.normal_spans - out.normal_spans,
                        },
                        budget - out.polar_coefficients,
                    )?;
                    out.cells += r.cells;
                    out.curve_spans += r.curve_spans;
                    out.polar_coefficients += r.normal_coefficients;
                    let Some(seam) = r.seam else {
                        out.reason = r.reason;
                        return Ok(out);
                    };
                    out.polar_seams.push((index, seam));
                    out.completed_edges += 1;
                    out.uncertain_edge = None;
                    continue;
                }
            }
        }
        if let Class::Curvature {
            normal_distance,
            curvature_distance,
        } = *class
        {
            if let Some((atlas, uses, _, Some((products, coefficients, exact)))) = polar {
                let proof = |side: usize| {
                    atlas
                        .poles()
                        .iter()
                        .find(|(a, _)| a.face == uses[index][side].face)
                        .map(|(_, p)| p.limit())
                };
                if let (Some(a), Some(b)) = (proof(0), proof(1)) {
                    if out.polar_shape_products == products
                        || out.polar_shape_coefficients == coefficients
                    {
                        out.reason = "source-polar-curvature-collection-work-limit";
                        return Ok(out);
                    }
                    let l = super::CurvatureLimits {
                        normal_distance,
                        curvature_distance,
                        cells: limits.cells - out.cells,
                        curve_spans: limits.curve_spans - out.curve_spans,
                        shape_cells: limits.normal_spans - out.normal_spans,
                    };
                    let r = if let Some(exact) = exact {
                        if out.polar_exact_work + out.regular_exact_work == exact {
                            out.reason = "exact-polar-collection-work-limit";
                            return Ok(out);
                        }
                        super::qualify_polar_curvature_exact(
                            edge,
                            [a, b],
                            l,
                            products - out.polar_shape_products,
                            coefficients - out.polar_shape_coefficients,
                            exact - out.polar_exact_work - out.regular_exact_work,
                        )?
                    } else {
                        super::qualify_polar_curvature(
                            edge,
                            [a, b],
                            l,
                            products - out.polar_shape_products,
                            coefficients - out.polar_shape_coefficients,
                        )?
                    };
                    out.polar_exact_work += r.exact_work;
                    out.cells += r.cells;
                    out.curve_spans += r.curve_spans;
                    out.normal_spans += r.regular_queries;
                    out.polar_shape_products += r.products;
                    out.polar_shape_coefficients += r.coefficients;
                    let Some(seam) = r.seam else {
                        out.reason = r.reason;
                        return Ok(out);
                    };
                    out.polar_curvature_seams.push((index, seam));
                    out.completed_edges += 1;
                    out.uncertain_edge = None;
                    continue;
                }
            }
            if let Some((axes, max)) = regular_exact {
                if out.regular_exact_work + out.polar_exact_work == max {
                    out.reason = "exact-curvature-collection-work-limit";
                    return Ok(out);
                }
                let r = super::qualify_curvature_exact(
                    edge,
                    super::CurvatureLimits {
                        normal_distance,
                        curvature_distance,
                        cells: limits.cells - out.cells,
                        curve_spans: limits.curve_spans - out.curve_spans,
                        shape_cells: limits.normal_spans - out.normal_spans,
                    },
                    axes[index],
                    max - out.regular_exact_work - out.polar_exact_work,
                )?;
                out.regular_exact_work += r.exact_work;
                out.cells += r.cells;
                out.curve_spans += r.curve_spans;
                out.normal_spans += r.shape_cells;
                let Some(seam) = r.seam else {
                    out.reason = r.reason;
                    return Ok(out);
                };
                out.curvature_seams.push((index, seam));
                out.completed_edges += 1;
                out.uncertain_edge = None;
                continue;
            }
            let report = super::qualify_curvature(
                edge,
                super::CurvatureLimits {
                    normal_distance,
                    curvature_distance,
                    cells: limits.cells - out.cells,
                    curve_spans: limits.curve_spans - out.curve_spans,
                    shape_cells: limits.normal_spans - out.normal_spans,
                },
            )?;
            out.cells += report.cells;
            out.curve_spans += report.curve_spans;
            out.normal_spans += report.shape_cells;
            let Some(seam) = report.seam else {
                out.reason = report.reason;
                return Ok(out);
            };
            out.curvature_seams.push((index, seam));
            out.completed_edges += 1;
            out.uncertain_edge = None;
            continue;
        }
        let report = super::qualify(
            edge,
            Limits {
                max_sine_squared: limits.max_sine_squared,
                cells: limits.cells - out.cells,
                curve_spans: limits.curve_spans - out.curve_spans,
                normal_spans: limits.normal_spans - out.normal_spans,
            },
        )?;
        out.cells += report.cells;
        out.curve_spans += report.curve_spans;
        out.normal_spans += report.normal_spans;
        let Some(seam) = report.seam else {
            out.reason = report.reason;
            return Ok(out);
        };
        out.seams.push((index, seam));
        out.completed_edges += 1;
        out.uncertain_edge = None;
    }
    Ok(out)
}

/// Recompute every requested seam from this body's unchanged original carriers.
/// All edges must be classified; no positive partial certificate is returned.
/// Collapsed pole strata require their own common tangent-limit proof and remain
/// an explicit refusal here, even if every ordinary edge is authored C0.
pub fn qualify_collection<'a, E: Carrier>(
    body: &'a Body<E>,
    classes: &[Class],
    limits: Limits,
) -> Result<CollectionReport<'a, E>> {
    let shell = body.geometry().shell();
    // Validate before a pole refusal too: malformed requests are never proofs.
    super::validate_limits(&limits)?;
    if classes.len() != shell.edges().len() || classes.is_empty() || classes.len() > 100_000 {
        return Err(Error::new(
            "BREP_SOURCE_SEAM_COLLECTION",
            "Classify every original edge exactly once",
        ));
    }
    validate_classes(classes)?;
    if !shell.poles().is_empty() {
        return Ok(CollectionReport {
            certificate: None,
            cells: 0,
            curve_spans: 0,
            normal_spans: 0,
            polar_coefficients: 0,
            polar_shape_products: 0,
            polar_exact_work: 0,
            regular_exact_work: 0,
            polar_shape_coefficients: 0,
            completed_edges: 0,
            uncertain_edge: None,
            reason: "source-seam-collection-pole-limits-unproven",
        });
    }
    qualify_owned_edges(body, classes, limits)
}

// Only the sibling complete-boundary factory may compose this with freshly
// proved pole strata. Public ordinary collection admission still refuses poles.
pub(super) fn qualify_owned_edges<'a, E: Carrier>(
    body: &'a Body<E>,
    classes: &[Class],
    limits: Limits,
) -> Result<CollectionReport<'a, E>> {
    qualify_owned_edges_impl(body, classes, limits, None, None)
}
pub(super) fn qualify_owned_edges_with_poles<'a, E: Carrier>(
    body: &'a Body<E>,
    classes: &[Class],
    limits: Limits,
    planes: &crate::source_pole_tangency::BodyPlaneLimits<'_, E>,
    coefficients: usize,
) -> Result<CollectionReport<'a, E>> {
    if !std::ptr::eq(planes.body(), body) {
        return Err(Error::new(
            "BREP_SOURCE_BOUNDARY_SMOOTHNESS",
            "Use this original body's pole atlas",
        ));
    }
    qualify_owned_edges_impl(
        body,
        classes,
        limits,
        Some((planes, body.geometry().shell().uses(), coefficients, None)),
        None,
    )
}
pub(super) fn qualify_owned_edges_with_polar_curvature<'a, E: Carrier>(
    body: &'a Body<E>,
    classes: &[Class],
    limits: Limits,
    atlas: &crate::source_pole_tangency::PoleCurvatureAtlas<'_, E>,
    budgets: super::PolarBudgets,
    exact: Option<u64>,
    regular_exact: Option<(&[usize], u64)>,
) -> Result<CollectionReport<'a, E>> {
    let planes = atlas.graphs().charts().transverse().atlas().planes();
    if !std::ptr::eq(planes.body(), body) {
        return Err(Error::new(
            "BREP_SOURCE_BOUNDARY_SMOOTHNESS",
            "Use this original body's curvature atlas",
        ));
    }
    qualify_owned_edges_impl(
        body,
        classes,
        limits,
        Some((
            planes,
            body.geometry().shell().uses(),
            budgets.normal_coefficients,
            Some((budgets.shape_products, budgets.shape_coefficients, exact)),
        )),
        regular_exact,
    )
}
pub(super) fn qualify_owned_edges_exact_regular<'a, E: Carrier>(
    body: &'a Body<E>,
    classes: &[Class],
    limits: Limits,
    axes: &[usize],
    max: u64,
) -> Result<CollectionReport<'a, E>> {
    qualify_owned_edges_impl(body, classes, limits, None, Some((axes, max)))
}
fn qualify_owned_edges_impl<'a, E: Carrier>(
    body: &'a Body<E>,
    classes: &[Class],
    limits: Limits,
    polar: Option<PolarContext<'_, '_, E>>,
    regular_exact: Option<(&[usize], u64)>,
) -> Result<CollectionReport<'a, E>> {
    let shell = body.geometry().shell();
    let max_sine_squared = limits.max_sine_squared;
    let work = if polar.is_some() || regular_exact.is_some() {
        inspect_with_poles(shell.edges(), classes, limits, polar, regular_exact)?
    } else {
        inspect_edges(shell.edges(), classes, limits)?
    };
    let complete = work.completed_edges == classes.len() && work.uncertain_edge.is_none();
    let certificate = complete.then(|| Collection {
        body,
        classes: classes.to_vec(),
        max_sine_squared,
        seams: work.seams,
        curvature_seams: work.curvature_seams,
        polar_seams: work.polar_seams,
        polar_curvature_seams: work.polar_curvature_seams,
    });
    Ok(CollectionReport {
        certificate,
        cells: work.cells,
        curve_spans: work.curve_spans,
        normal_spans: work.normal_spans,
        polar_coefficients: work.polar_coefficients,
        polar_shape_products: work.polar_shape_products,
        polar_exact_work: work.polar_exact_work,
        regular_exact_work: work.regular_exact_work,
        polar_shape_coefficients: work.polar_shape_coefficients,
        completed_edges: work.completed_edges,
        uncertain_edge: work.uncertain_edge,
        reason: work.reason,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source_seam_tangency::tests::{limits, shared};
    #[test]
    fn complete_curved_seams_share_every_budget_without_reset() {
        let edges = [shared(false), shared(false)];
        let classes = [Class::TangentPlanes; 2];
        let once = inspect_edges(&edges[..1], &classes[..1], limits()).unwrap();
        assert_eq!(once.completed_edges, 1);
        let full = inspect_edges(&edges, &classes, limits()).unwrap();
        assert_eq!(full.completed_edges, 2);
        assert_eq!(full.seams.len(), 2);
        assert_eq!(full.cells, 2 * once.cells);
        for dimension in 0..3 {
            let mut short = limits();
            match dimension {
                0 => short.cells = once.cells,
                1 => short.curve_spans = once.curve_spans,
                _ => short.normal_spans = once.normal_spans,
            }
            let r = inspect_edges(&edges, &classes, short).unwrap();
            assert_eq!(r.completed_edges, 1);
            assert_eq!(r.uncertain_edge, Some(1));
            assert_eq!(r.reason, "source-seam-collection-work-limit");
        }
    }
    #[test]
    fn sharp_join_is_explicit_and_never_silently_downgraded() {
        let edges = [shared(false), shared(true)];
        let r = inspect_edges(&edges, &[Class::TangentPlanes; 2], limits()).unwrap();
        assert_eq!(r.completed_edges, 1);
        assert_eq!(r.uncertain_edge, Some(1));
        let mixed = inspect_edges(&edges, &[Class::TangentPlanes, Class::C0], limits()).unwrap();
        assert_eq!(mixed.completed_edges, 2);
        assert_eq!(mixed.seams.len(), 1);
        assert_eq!(mixed.seams[0].0, 0);
        assert!(inspect_edges(&edges, &[Class::C0], limits()).is_err());
        let mut invalid = limits();
        invalid.cells = 0;
        assert!(inspect_edges(&edges, &[Class::C0; 2], invalid).is_err());
    }
}
