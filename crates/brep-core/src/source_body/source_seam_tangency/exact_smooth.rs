//! Complete exact G1/G2 coverage at the body's unchanged declared classes.
use super::{
    Carrier, Class, Collection, CurvatureLimits, CurvatureSeam, ExactTangencySeam, Limits,
    qualify_curvature_exact, qualify_tangency_exact,
};
use nurbs_core::{Error, Result};
pub struct ExactSmoothCollection<'c, 'b, E: Carrier> {
    original: &'c Collection<'b, E>,
    tangent: Vec<(usize, ExactTangencySeam<E>)>,
    curvature: Vec<(usize, CurvatureSeam<E>)>,
    polar_tangent: Vec<usize>,
    polar_curvature: Vec<usize>,
}
impl<'c, 'b, E: Carrier> ExactSmoothCollection<'c, 'b, E> {
    pub fn original(&self) -> &'c Collection<'b, E> {
        self.original
    }
    pub fn tangent_seams(&self) -> &[(usize, ExactTangencySeam<E>)] {
        &self.tangent
    }
    pub fn curvature_seams(&self) -> &[(usize, CurvatureSeam<E>)] {
        &self.curvature
    }
    pub fn polar_tangent_edges(&self) -> &[usize] {
        &self.polar_tangent
    }
    pub fn polar_curvature_edges(&self) -> &[usize] {
        &self.polar_curvature
    }
    pub fn all_edges_exact_tangent(&self) -> bool {
        self.original.all_edges_tangent()
    }
    pub fn all_edges_exact_curvature(&self) -> bool {
        self.original.all_edges_curvature()
    }
}
pub struct ExactSmoothLimits {
    pub incidence_steps: usize,
    pub exact_work: u64,
    pub cells: usize,
    pub curve_spans: usize,
    /// Shared original normal/shape visits; no fresh allowance per seam/class.
    pub source_spans: usize,
}
pub struct ExactSmoothReport<'c, 'b, E: Carrier> {
    pub certificate: Option<ExactSmoothCollection<'c, 'b, E>>,
    pub steps: usize,
    pub exact_work: u64,
    pub cells: usize,
    pub curve_spans: usize,
    pub source_spans: usize,
    pub completed_smooth_edges: usize,
    pub uncertain_edge: Option<usize>,
    pub reason: &'static str,
}
/// One immutable same-body collection supplies positional ownership, embedded
/// geometry, declared sharpness and native full polar normal coverage. Every
/// G1 request gets a fresh original gradient identity, every regular G2 request
/// gets a fresh original gradient/Hessian identity plus native regular shapes.
/// Polar G2 may reuse ONLY that collection's already exact token; tolerance-only
/// polar curvature refuses. C0 stays C0 and G1 is never promoted to G2. A failed
/// or exhausted prefix never publishes a certificate. Pole graph neighborhoods,
/// face-interior smoothness and ideal-law binding remain separate obligations.
pub fn qualify_exact_smooth_collection<'c, 'b, E: Carrier>(
    original: &'c Collection<'b, E>,
    axes: &[usize],
    l: ExactSmoothLimits,
) -> Result<ExactSmoothReport<'c, 'b, E>> {
    let classes = original.classes();
    if axes.len() != classes.len()
        || axes.iter().any(|&a| a > 2)
        || !(1..=1000000).contains(&l.incidence_steps)
        || !(1..=100000000).contains(&l.exact_work)
        || [l.cells, l.curve_spans, l.source_spans]
            .iter()
            .any(|n| !(1..=100000).contains(n))
    {
        return Err(Error::new(
            "BREP_SOURCE_EXACT_SMOOTH",
            "Choose original edge axes and bounded aggregate smoothness work",
        ));
    }
    let mut out = ExactSmoothReport {
        certificate: None,
        steps: 0,
        exact_work: 0,
        cells: 0,
        curve_spans: 0,
        source_spans: 0,
        completed_smooth_edges: 0,
        uncertain_edge: None,
        reason: "exact-smooth-collection-work-limit",
    };
    let mut polar = std::collections::BTreeMap::new();
    for (i, _) in original.polar_seams() {
        if out.steps == l.incidence_steps {
            return Ok(out);
        }
        out.steps += 1;
        if polar.insert(*i, None).is_some() {
            out.reason = "exact-smooth-duplicate-polar-carrier";
            return Ok(out);
        }
    }
    for (i, p) in original.polar_curvature_seams() {
        if out.steps == l.incidence_steps {
            return Ok(out);
        }
        out.steps += 1;
        if polar.insert(*i, Some(p.is_exact())).is_some() {
            out.reason = "exact-smooth-duplicate-polar-carrier";
            return Ok(out);
        }
    }
    let edges = original.body().geometry().shell().edges();
    let mut tangent = Vec::new();
    let mut curvature = Vec::new();
    let mut polar_tangent = Vec::new();
    let mut polar_curvature = Vec::new();
    for (i, class) in classes.iter().enumerate() {
        if out.steps == l.incidence_steps {
            return Ok(out);
        }
        out.steps += 1;
        if *class == Class::C0 {
            continue;
        }
        out.uncertain_edge = Some(i);
        if let Some(exact) = polar.get(&i) {
            if *class == Class::TangentPlanes {
                if out.exact_work == l.exact_work {
                    return Ok(out);
                }
                let (equal, work, reason) = super::exact_tangency::identity(
                    &edges[i],
                    axes[i],
                    l.exact_work - out.exact_work,
                )?;
                out.exact_work += work;
                if !equal {
                    out.reason = reason;
                    return Ok(out);
                }
                polar_tangent.push(i);
            } else if *exact == Some(true) {
                polar_curvature.push(i);
            } else {
                out.reason = "exact-smooth-polar-g2-unproven";
                return Ok(out);
            }
            out.completed_smooth_edges += 1;
            out.uncertain_edge = None;
            continue;
        }
        if out.exact_work == l.exact_work
            || out.cells == l.cells
            || out.curve_spans == l.curve_spans
            || out.source_spans == l.source_spans
        {
            return Ok(out);
        }
        if *class == Class::TangentPlanes {
            let r = qualify_tangency_exact(
                &edges[i],
                Limits {
                    max_sine_squared: 0.,
                    cells: l.cells - out.cells,
                    curve_spans: l.curve_spans - out.curve_spans,
                    normal_spans: l.source_spans - out.source_spans,
                },
                axes[i],
                l.exact_work - out.exact_work,
            )?;
            out.exact_work += r.exact_work;
            out.cells += r.cells;
            out.curve_spans += r.curve_spans;
            out.source_spans += r.normal_spans;
            let Some(seam) = r.seam else {
                out.reason = r.reason;
                return Ok(out);
            };
            tangent.push((i, seam));
        } else {
            let r = qualify_curvature_exact(
                &edges[i],
                CurvatureLimits {
                    normal_distance: 0.,
                    curvature_distance: 0.,
                    cells: l.cells - out.cells,
                    curve_spans: l.curve_spans - out.curve_spans,
                    shape_cells: l.source_spans - out.source_spans,
                },
                axes[i],
                l.exact_work - out.exact_work,
            )?;
            out.exact_work += r.exact_work;
            out.cells += r.cells;
            out.curve_spans += r.curve_spans;
            out.source_spans += r.shape_cells;
            let Some(seam) = r.seam else {
                out.reason = r.reason;
                return Ok(out);
            };
            curvature.push((i, seam));
        }
        out.completed_smooth_edges += 1;
        out.uncertain_edge = None;
    }
    out.certificate = Some(ExactSmoothCollection {
        original,
        tangent,
        curvature,
        polar_tangent,
        polar_curvature,
    });
    out.reason = "exact-original-mixed-g1-g2-collection-qualified";
    Ok(out)
}
