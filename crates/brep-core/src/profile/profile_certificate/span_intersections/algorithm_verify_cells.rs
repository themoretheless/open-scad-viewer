//! span intersections algorithm: verify_cells.
use super::*;
pub fn verify_cells(x: &[Cell], y: &[Cell], c: &SpanIntersectionCertificate) -> Result<VerifiedSpanIntersections> {
    if c.pairs.len() > 4096 {
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
            let a = &x[i];
            let b = &y[j];
            let diagonal = c.self_pair && i == j;
            if !matches!(p.kind.as_str(), "events" | "locus") && p.shear != 0
                || !matches!(p.kind.as_str(), "overlap" | "locus") && !p.relation.is_empty()
                || !matches!(p.kind.as_str(), "events" | "stationary" | "locus")
                    && !p.factors.is_empty()
            {
                return Err(refused("Extraneous multi-span pair proof claims"));
            }
            let events = match p.kind.as_str() {
                "locus" | "whole-square" => {
                    if c.version < 2 {
                        return Err(refused(
                            "Algebraic parameter loci require span certificate version >=2",
                        ));
                    }
                    let (h, residual) =
                        arithmetic::bivariate::component_system(&a.net, &b.net, diagonal)?
                            .ok_or_else(|| refused("False common parameter locus"))?;
                    let whole = h.is_empty();
                    if whole != (p.kind == "whole-square")
                        || (!whole && decode_locus(&p.relation)? != h)
                    {
                        return Err(refused("Incomplete or false common parameter component"));
                    }
                    let domain = |cell: &Cell| -> Result<[f64; 2]> {
                        Ok([
                            outward(&cell.domain[0], false)?,
                            outward(&cell.domain[1], true)?,
                        ])
                    };
                    out.algebraic_loci.push(VerifiedParameterLocus {
                        first_span: i,
                        second_span: j,
                        first_domain: domain(a)?,
                        second_domain: domain(b)?,
                        whole_square: whole,
                        diagonal_excluded: c.self_pair,
                        relation: p.relation.clone(),
                    });
                    if whole && diagonal {
                        out.stationary_spans.push(domain(a)?)
                    }
                    if whole {
                        Vec::new()
                    } else {
                        let w = width(a, b, c.maximum_parameter_width);
                        let mut events = engine::verify_system(
                            &residual,
                            diagonal,
                            &w,
                            p.shear,
                            &decode_factors(&p.factors)?,
                        )?;
                        if diagonal {
                            let stationary = engine::stationary(&a.net, &w)?;
                            events.extend(engine::verify_stationary(&a.net, &w, &stationary)?);
                        }
                        events
                    }
                }
                "separate" => {
                    if !separated(a, b, p.axis as usize) {
                        return Err(refused("False exact source hull separation"));
                    }
                    Vec::new()
                }
                "stationary" => {
                    if !diagonal || !arithmetic::monotone(&a.net, p.axis as usize)? {
                        return Err(refused("False diagonal injectivity claim"));
                    }
                    engine::verify_stationary(
                        &a.net,
                        &width(a, b, c.maximum_parameter_width),
                        &decode_factors(&p.factors)?,
                    )?
                }
                "range" => {
                    if diagonal {
                        return Err(refused("Self diagonal needs a stationary census"));
                    }
                    let (lo, hi, events) = range(a, b, p.axis as usize)?;
                    if lo < hi {
                        return Err(refused("False separated image ranges"));
                    }
                    events
                }
                "overlap" => {
                    if diagonal {
                        return Err(refused("A self diagonal cannot become a common component"));
                    }
                    let (lo, hi, _) = range(a, b, p.axis as usize)?;
                    let h = nonlinear::decode(&p.relation)?;
                    if lo >= hi
                        || !arithmetic::bivariate::verify_partial_relation(
                            &a.net,
                            &b.net,
                            p.axis as usize,
                            &h,
                        )?
                    {
                        return Err(refused(
                            "Common curve component or its image coverage is false",
                        ));
                    }
                    let domain = |c: &Cell| {
                        [
                            outward(&c.domain[0], false).unwrap(),
                            outward(&c.domain[1], true).unwrap(),
                        ]
                    };
                    out.overlaps.push(VerifiedCurveOverlap {
                        first_span: i,
                        second_span: j,
                        first_domain: domain(a),
                        second_domain: domain(b),
                        axis: p.axis,
                        image_range: [ratio(&lo), ratio(&hi)],
                        relation: p.relation.clone(),
                    });
                    Vec::new()
                }
                "events" => engine::verify(
                    &a.net,
                    &b.net,
                    diagonal,
                    &width(a, b, c.maximum_parameter_width),
                    p.shear,
                    &decode_factors(&p.factors)?,
                )?,
                _ => return Err(refused("Unknown multi-span pair proof kind")),
            };
            if matches!(p.kind.as_str(), "overlap" | "locus" | "whole-square") && c.version == 3 {
                let h = if p.kind == "whole-square" {
                    Vec::new()
                } else {
                    decode_locus(&p.relation)?
                };
                let graph = branches::decompose(
                    &h,
                    c.self_pair.then(|| local_diagonal(a, b)),
                    &width(a, b, c.maximum_parameter_width),
                )?;
                if p.branches.as_ref() != Some(&graph) {
                    return Err(refused("False or incomplete parameter branch graph"));
                }
                out.branch_decompositions.push(VerifiedBranchGraph {
                    first_span: i,
                    second_span: j,
                    decomposition: graph,
                });
            } else if p.branches.is_some() {
                return Err(refused("Extraneous parameter branch graph"));
            }
            let global = |c: &Cell, p: &[Q; 2]| -> Result<[f64; 2]> {
                let f = |t: &Q| &c.domain[0] + (&c.domain[1] - &c.domain[0]) * t;
                Ok([outward(&f(&p[0]), false)?, outward(&f(&p[1]), true)?])
            };
            for e in events {
                if e.singular {
                    out.singular.push(global(a, &e.first)?);
                    continue;
                }
                if (e.first == [Q::one(), Q::one()] && i + 1 < x.len())
                    || (e.second == [Q::one(), Q::one()] && j + 1 < y.len())
                {
                    continue;
                }
                let first = global(a, &e.first)?;
                let second = global(b, &e.second)?;
                if c.self_pair && first[0] == first[1] && first == second {
                    continue;
                }
                out.events.push([first, second]);
            }
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
