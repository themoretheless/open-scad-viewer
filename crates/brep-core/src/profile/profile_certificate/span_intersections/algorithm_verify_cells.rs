//! span intersections algorithm: verify_cells.
use super::*;
const MAX_PAIRS: usize = 4096;
/// Outward-rounded global parameter domain of one source cell.
fn span_domain(cell: &Cell) -> Result<[f64; 2]> {
    Ok([
        outward(&cell.domain[0], false)?,
        outward(&cell.domain[1], true)?,
    ])
}
/// Maps a local parameter interval of one cell to its outward-rounded global interval.
fn global(cell: &Cell, local: &[Q; 2]) -> Result<[f64; 2]> {
    let at = |t: &Q| &cell.domain[0] + (&cell.domain[1] - &cell.domain[0]) * t;
    Ok([
        outward(&at(&local[0]), false)?,
        outward(&at(&local[1]), true)?,
    ])
}
/// One certified span pair together with its two source cells.
struct PairCheck<'a> {
    c: &'a SpanIntersectionCertificate,
    p: &'a SpanPairCertificate,
    a: &'a Cell,
    b: &'a Cell,
    spans: [usize; 2],
    diagonal: bool,
}
impl PairCheck<'_> {
    fn kind(&self) -> &str {
        self.p.kind.as_str()
    }
    fn axis(&self) -> usize {
        self.p.axis as usize
    }
    fn width(&self) -> Q {
        width(self.a, self.b, self.c.maximum_parameter_width)
    }
    /// A proof kind may only carry the shear, relation and factors it uses.
    fn reject_extraneous_claims(&self) -> Result<()> {
        let (p, kind) = (self.p, self.kind());
        if !matches!(kind, "events" | "locus") && p.shear != 0
            || !matches!(kind, "overlap" | "locus") && !p.relation.is_empty()
            || !matches!(kind, "events" | "stationary" | "locus") && !p.factors.is_empty()
        {
            return Err(refused("Extraneous multi-span pair proof claims"));
        }
        Ok(())
    }
    /// Verifies the pair's proof by kind and returns its local events. Loci
    /// and overlaps are recorded in `out` as they are proved.
    fn events(&self, out: &mut VerifiedSpanIntersections) -> Result<Vec<engine::Event>> {
        match self.kind() {
            "locus" | "whole-square" => self.locus(out),
            "separate" => self.separate(),
            "stationary" => self.stationary(),
            "range" => self.range(),
            "overlap" => self.overlap(out),
            "events" => engine::verify(
                &self.a.net,
                &self.b.net,
                self.diagonal,
                &self.width(),
                self.p.shear,
                &decode_factors(&self.p.factors)?,
            ),
            _ => Err(refused("Unknown multi-span pair proof kind")),
        }
    }
    fn locus(&self, out: &mut VerifiedSpanIntersections) -> Result<Vec<engine::Event>> {
        let Self { c, p, a, b, .. } = *self;
        if c.version < 2 {
            return Err(refused(
                "Algebraic parameter loci require span certificate version >=2",
            ));
        }
        let (h, residual) = arithmetic::bivariate::component_system(&a.net, &b.net, self.diagonal)?
            .ok_or_else(|| refused("False common parameter locus"))?;
        let whole = h.is_empty();
        if whole != (p.kind == "whole-square") || (!whole && decode_locus(&p.relation)? != h) {
            return Err(refused("Incomplete or false common parameter component"));
        }
        out.algebraic_loci.push(VerifiedParameterLocus {
            first_span: self.spans[0],
            second_span: self.spans[1],
            first_domain: span_domain(a)?,
            second_domain: span_domain(b)?,
            whole_square: whole,
            diagonal_excluded: c.self_pair,
            relation: p.relation.clone(),
        });
        if whole {
            if self.diagonal {
                out.stationary_spans.push(span_domain(a)?)
            }
            return Ok(Vec::new());
        }
        let w = self.width();
        let mut events = engine::verify_system(
            &residual,
            self.diagonal,
            &w,
            p.shear,
            &decode_factors(&p.factors)?,
        )?;
        if self.diagonal {
            let stationary = engine::stationary(&a.net, &w)?;
            events.extend(engine::verify_stationary(&a.net, &w, &stationary)?);
        }
        Ok(events)
    }
    fn separate(&self) -> Result<Vec<engine::Event>> {
        if !separated(self.a, self.b, self.axis()) {
            return Err(refused("False exact source hull separation"));
        }
        Ok(Vec::new())
    }
    fn stationary(&self) -> Result<Vec<engine::Event>> {
        if !self.diagonal || !arithmetic::monotone(&self.a.net, self.axis())? {
            return Err(refused("False diagonal injectivity claim"));
        }
        engine::verify_stationary(
            &self.a.net,
            &self.width(),
            &decode_factors(&self.p.factors)?,
        )
    }
    fn range(&self) -> Result<Vec<engine::Event>> {
        if self.diagonal {
            return Err(refused("Self diagonal needs a stationary census"));
        }
        let (lo, hi, events) = range(self.a, self.b, self.axis())?;
        if lo < hi {
            return Err(refused("False separated image ranges"));
        }
        Ok(events)
    }
    fn overlap(&self, out: &mut VerifiedSpanIntersections) -> Result<Vec<engine::Event>> {
        let Self { p, a, b, .. } = *self;
        if self.diagonal {
            return Err(refused("A self diagonal cannot become a common component"));
        }
        let (lo, hi, _) = range(a, b, self.axis())?;
        let h = nonlinear::decode(&p.relation)?;
        if lo >= hi
            || !arithmetic::bivariate::verify_partial_relation(&a.net, &b.net, self.axis(), &h)?
        {
            return Err(refused(
                "Common curve component or its image coverage is false",
            ));
        }
        out.overlaps.push(VerifiedCurveOverlap {
            first_span: self.spans[0],
            second_span: self.spans[1],
            first_domain: span_domain(a)?,
            second_domain: span_domain(b)?,
            axis: p.axis,
            image_range: [ratio(&lo), ratio(&hi)],
            relation: p.relation.clone(),
        });
        Ok(Vec::new())
    }
    /// Version 3 certificates carry a branch graph for every common
    /// component; any other pair must carry none.
    fn branch_graph(&self, out: &mut VerifiedSpanIntersections) -> Result<()> {
        let Self { c, p, a, b, .. } = *self;
        if !(matches!(self.kind(), "overlap" | "locus" | "whole-square") && c.version == 3) {
            if p.branches.is_some() {
                return Err(refused("Extraneous parameter branch graph"));
            }
            return Ok(());
        }
        let h = if p.kind == "whole-square" {
            Vec::new()
        } else {
            decode_locus(&p.relation)?
        };
        let graph =
            branches::decompose(&h, c.self_pair.then(|| local_diagonal(a, b)), &self.width())?;
        if p.branches.as_ref() != Some(&graph) {
            return Err(refused("False or incomplete parameter branch graph"));
        }
        out.branch_decompositions.push(VerifiedBranchGraph {
            first_span: self.spans[0],
            second_span: self.spans[1],
            decomposition: graph,
        });
        Ok(())
    }
    /// Records events in global parameters. An event at the end of a span
    /// belongs to the next span, and a self pair's diagonal point is no event.
    fn emit(
        &self,
        events: Vec<engine::Event>,
        span_counts: [usize; 2],
        out: &mut VerifiedSpanIntersections,
    ) -> Result<()> {
        let end = [Q::one(), Q::one()];
        for e in events {
            if e.singular {
                out.singular.push(global(self.a, &e.first)?);
                continue;
            }
            if (e.first == end && self.spans[0] + 1 < span_counts[0])
                || (e.second == end && self.spans[1] + 1 < span_counts[1])
            {
                continue;
            }
            let first = global(self.a, &e.first)?;
            let second = global(self.b, &e.second)?;
            if self.c.self_pair && first[0] == first[1] && first == second {
                continue;
            }
            out.events.push([first, second]);
        }
        Ok(())
    }
}
pub fn verify_cells(
    x: &[Cell],
    y: &[Cell],
    c: &SpanIntersectionCertificate,
) -> Result<VerifiedSpanIntersections> {
    if c.pairs.len() > MAX_PAIRS {
        return Err(refused("Translated span pair census exceeds allowance"));
    }
    let mut n = 0;
    let mut out = VerifiedSpanIntersections {
        events: Vec::new(),
        singular: Vec::new(),
        overlaps: Vec::new(),
        algebraic_loci: Vec::new(),
        stationary_spans: Vec::new(),
        branch_decompositions: Vec::new(),
    };
    for i in 0..x.len() {
        for j in 0..y.len() {
            if c.self_pair && i > j {
                continue;
            }
            let p = c
                .pairs
                .get(n)
                .ok_or_else(|| refused("Multi-span intersection pair is missing"))?;
            n += 1;
            if p.first != i || p.second != j || p.axis > 1 {
                return Err(refused(
                    "Multi-span intersection pairs are reordered or false",
                ));
            }
            let pair = PairCheck {
                c,
                p,
                a: &x[i],
                b: &y[j],
                spans: [i, j],
                diagonal: c.self_pair && i == j,
            };
            pair.reject_extraneous_claims()?;
            let events = pair.events(&mut out)?;
            pair.branch_graph(&mut out)?;
            pair.emit(events, [x.len(), y.len()], &mut out)?;
        }
    }
    if n != c.pairs.len() {
        return Err(refused("Multi-span intersection census has extra pairs"));
    }
    out.events.sort_by(|a, b| {
        a[0][0]
            .total_cmp(&b[0][0])
            .then(a[1][0].total_cmp(&b[1][0]))
    });
    out.events.dedup();
    out.singular.sort_by(|a, b| a[0].total_cmp(&b[0]));
    out.singular.dedup();
    Ok(out)
}
