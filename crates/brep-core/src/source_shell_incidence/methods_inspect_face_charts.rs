//! source shell incidence operations: inspect_face_charts.
use super::*;
impl<E> Shell<E> {
/// Prove whole original charts injective, hence any retained source subset.
    /// Does not classify contacts between different faces or certify a volume.
    pub fn inspect_face_charts(
        &self,
        max_spans: usize,
        max_linear_cells: usize,
    ) -> Result<ChartReport> {
        if !(1..=100000).contains(&max_spans) || max_linear_cells > 100000 {
            return Err(error("Choose bounded source chart work"));
        }
        let mut out = ChartReport {
            all_injective: true,
            spans: 0,
            linear_cells: 0,
            faces: Vec::new(),
        };
        for (face, wires) in self.faces.iter().enumerate() {
            let surface = wires[0].edges()[0].surface();
            let contraction = if out.spans < max_spans {
                Some(nurbs_core::surface_injectivity::certify_contraction(
                    surface,
                    max_spans - out.spans,
                )?)
            } else {
                None
            };
            out.spans += contraction.as_ref().map_or(0, |r| r.spans);
            let linear = if !contraction.as_ref().is_some_and(|r| r.proven)
                && out.linear_cells < max_linear_cells
            {
                Some(nurbs_core::surface_linear_monotonicity::inspect_candidate(
                    surface,
                    max_linear_cells - out.linear_cells,
                )?)
            } else {
                None
            };
            out.linear_cells += linear.as_ref().map_or(0, |r| r.cells);
            let mut ruled_quotient = None;
            let mut polar_quotient = None;
            let quotient = if !contraction.as_ref().is_some_and(|r| r.proven)
                && !linear.as_ref().is_some_and(|r| r.certified)
                && max_spans - out.spans >= 16
            {
                let u = [
                    surface.knots_u[surface.degree_u],
                    surface.knots_u[surface.control_points.len()],
                ];
                let v = [
                    surface.knots_v[surface.degree_v],
                    surface.knots_v[surface.control_points[0].len()],
                ];
                let mut ends = Vec::new();
                for (address, pole) in &self.poles {
                    if address.face != face {
                        continue;
                    }
                    let source = pole.source();
                    let curve = source.curve();
                    let bounds = source.parameter_bounds();
                    let domain = curve.domain();
                    let full = bounds.iter().any(|b| *b == [domain[0]; 2])
                        && bounds.iter().any(|b| *b == [domain[1]; 2]);
                    if !full || curve.degree != 1 || curve.control_points.len() != 2 {
                        continue;
                    }
                    let a = &curve.control_points[0];
                    let b = &curve.control_points[1];
                    if a[0] == b[0]
                        && ((a[1] == v[0] && b[1] == v[1]) || (a[1] == v[1] && b[1] == v[0]))
                    {
                        if let Some(end) = u.iter().position(|x| *x == a[0]) {
                            ends.push(end);
                        }
                    }
                }
                if ends.len() == 1 {
                    let proof = nurbs_core::surface_quotient_injectivity::certify_source_frame(
                        surface,
                        ends[0],
                        16,
                        max_spans - out.spans,
                    )?;
                    out.spans += proof.cells;
                    if !proof.proven && surface.degree_u == 1 && max_spans - out.spans >= 16 {
                        let ruled =
                            nurbs_core::surface_quotient_injectivity::certify_ruled_source_frame(
                                surface,
                                ends[0],
                                16,
                                max_spans - out.spans,
                            )?;
                        out.spans += ruled.cells;
                        ruled_quotient = Some(ruled);
                    }
                    if !proof.proven && surface.degree_u >= 2 && max_spans - out.spans >= 256 {
                        let polar =
                            nurbs_core::surface_quotient_injectivity::certify_polar_source_frame(
                                surface,
                                ends[0],
                                16,
                                max_spans - out.spans,
                            )?;
                        out.spans += polar.cells;
                        polar_quotient = Some(polar);
                    }
                    Some(proof)
                } else {
                    None
                }
            } else {
                None
            };
            let result = ChartFace {
                face,
                contraction,
                linear,
                quotient,
                ruled_quotient,
                polar_quotient,
            };
            out.all_injective &= result.injectivity_proven();
            out.faces.push(result);
        }
        Ok(out)
    }
}
