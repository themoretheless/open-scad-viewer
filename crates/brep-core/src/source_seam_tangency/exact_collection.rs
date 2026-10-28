//! Exact G2 on every curvature-class carrier of one immutable collection.
use super::{Carrier, Class, Collection, CurvatureLimits, CurvatureSeam, qualify_curvature_exact};
use nurbs_core::{Error, Result};
pub struct ExactCurvatureCollection<'c, 'b, E: Carrier> {
    original: &'c Collection<'b, E>,
    regular: Vec<(usize, CurvatureSeam<E>)>,
    polar: Vec<usize>,
}
impl<'c, 'b, E: Carrier> ExactCurvatureCollection<'c, 'b, E> {
    pub fn original(&self) -> &'c Collection<'b, E> {
        self.original
    }
    pub fn regular_seams(&self) -> &[(usize, CurvatureSeam<E>)] {
        &self.regular
    }
    /// Exact polar tokens remain privately owned by `original()`.
    pub fn polar_edges(&self) -> &[usize] {
        &self.polar
    }
    /// C0 and G1 requests are preserved, never promoted by this factory.
    pub fn all_edges_exact_curvature(&self) -> bool {
        self.original.all_edges_curvature()
    }
}
pub struct ExactCollectionLimits {
    pub incidence_steps: usize,
    pub exact_work: u64,
    pub cells: usize,
    pub curve_spans: usize,
    pub shape_cells: usize,
}
pub struct ExactCollectionReport<'c, 'b, E: Carrier> {
    pub certificate: Option<ExactCurvatureCollection<'c, 'b, E>>,
    pub steps: usize,
    pub exact_work: u64,
    pub cells: usize,
    pub curve_spans: usize,
    pub shape_cells: usize,
    pub completed_curvature_edges: usize,
    pub uncertain_edge: Option<usize>,
    pub reason: &'static str,
}
/// Fresh exact regular-carrier proofs share ONE aggregate ledger. An already
/// exact polar token may be reused only from this very private collection;
/// a tolerance-only polar token refuses. All incidence/axis inputs are bounded
/// and validated before any proof work. No caller report or partial prefix
/// can manufacture a certificate. This proves declared seam compatibility,
/// not face interiors, pole neighborhoods, ideal-law binding or whole-body C2.
pub fn qualify_exact_curvature_collection<'c, 'b, E: Carrier>(
    original: &'c Collection<'b, E>,
    axes: &[usize],
    limits: ExactCollectionLimits,
) -> Result<ExactCollectionReport<'c, 'b, E>> {
    let classes = original.classes();
    if axes.len() != classes.len()
        || axes.iter().any(|&a| a > 2)
        || !(1..=100000).contains(&limits.incidence_steps)
        || !(1..=100000000).contains(&limits.exact_work)
        || [limits.cells, limits.curve_spans, limits.shape_cells]
            .iter()
            .any(|n| !(1..=100000).contains(n))
    {
        return Err(Error::new(
            "BREP_SOURCE_EXACT_COLLECTION",
            "Choose one axis per original edge and bounded aggregate work",
        ));
    }
    let mut out = ExactCollectionReport {
        certificate: None,
        steps: 0,
        exact_work: 0,
        cells: 0,
        curve_spans: 0,
        shape_cells: 0,
        completed_curvature_edges: 0,
        uncertain_edge: None,
        reason: "exact-curvature-collection-work-limit",
    };
    let mut polar = std::collections::BTreeMap::new();
    for (i, p) in original.polar_curvature_seams() {
        if out.steps == limits.incidence_steps {
            return Ok(out);
        }
        out.steps += 1;
        if polar.insert(*i, p.is_exact()).is_some() {
            out.reason = "exact-collection-duplicate-polar-carrier";
            return Ok(out);
        }
    }
    let edges = original.body().geometry().shell().edges();
    let mut regular = Vec::new();
    let mut polar_edges = Vec::new();
    for (index, class) in classes.iter().enumerate() {
        if out.steps == limits.incidence_steps {
            return Ok(out);
        }
        out.steps += 1;
        let Class::Curvature { .. } = class else {
            continue;
        };
        out.uncertain_edge = Some(index);
        if let Some(exact) = polar.get(&index) {
            if !exact {
                out.reason = "exact-collection-polar-g2-unproven";
                return Ok(out);
            }
            polar_edges.push(index);
            out.completed_curvature_edges += 1;
            out.uncertain_edge = None;
            continue;
        }
        if out.exact_work == limits.exact_work
            || out.cells == limits.cells
            || out.curve_spans == limits.curve_spans
            || out.shape_cells == limits.shape_cells
        {
            return Ok(out);
        }
        let r = qualify_curvature_exact(
            &edges[index],
            CurvatureLimits {
                normal_distance: 0.,
                curvature_distance: 0.,
                cells: limits.cells - out.cells,
                curve_spans: limits.curve_spans - out.curve_spans,
                shape_cells: limits.shape_cells - out.shape_cells,
            },
            axes[index],
            limits.exact_work - out.exact_work,
        )?;
        out.exact_work += r.exact_work;
        out.cells += r.cells;
        out.curve_spans += r.curve_spans;
        out.shape_cells += r.shape_cells;
        let Some(seam) = r.seam else {
            out.reason = r.reason;
            return Ok(out);
        };
        regular.push((index, seam));
        out.completed_curvature_edges += 1;
        out.uncertain_edge = None;
    }
    out.certificate = Some(ExactCurvatureCollection {
        original,
        regular,
        polar: polar_edges,
    });
    out.reason = "exact-original-curvature-collection-qualified";
    Ok(out)
}
