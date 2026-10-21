//! Fiber certificate: preserves the original arithmetic and proof budgets.
use super::*;

pub(super) fn certify(
    source: &FiberSources,
    domain: [[f64; 2]; 4],
    axis: FiberAxis,
    limits: FiberLimits,
) -> Result<FiberReport> {
    check(
        (1..=100000).contains(&limits.max_rectangles)
            && limits
                .preconditioner
                .as_ref()
                .is_none_or(|r| r.iter().flatten().all(|v| v.is_finite())),
        "Fiber proof requires finite preconditioning and bounded positive rectangle work",
    )?;
    let free = axis.index();
    let surfaces = [source.first(), source.second()];
    for k in 0..4 {
        let s = surfaces[k / 2];
        let (knots, d, n) = if k % 2 == 0 {
            (&s.knots_u, s.degree_u, s.control_points.len())
        } else {
            (&s.knots_v, s.degree_v, s.control_points[0].len())
        };
        check(
            domain[k].iter().all(|v| v.is_finite())
                && domain[k][0] >= knots[d]
                && domain[k][1] <= knots[n]
                && (if k == free {
                    domain[k][0] <= domain[k][1]
                } else {
                    domain[k][0] < domain[k][1]
                }),
            "Fiber box must cover noncollapsed unknowns inside the original surface domains",
        )?;
    }
    let mut out = FiberReport {
        status: FiberStatus::Unresolved,
        reason: "source-bounds-unproven",
        rectangles: 0,
        residual_box: None,
        root_image: None,
        contraction_upper: None,
        operator: None,
        support_regularity_proven: [false; 2],
    };
    // Closed original-domain claims require the actual evaluator's natural
    // endpoint span to be nonempty; an interior one-sided limit is insufficient.
    for k in 0..4 {
        let s = surfaces[k / 2];
        let (knots, p, n) = if k % 2 == 0 {
            (&s.knots_u, s.degree_u, s.control_points.len())
        } else {
            (&s.knots_v, s.degree_v, s.control_points[0].len())
        };
        if (domain[k][0] == knots[p] && knots[p] == knots[p + 1])
            || (domain[k][1] == knots[n] && knots[n - 1] == knots[n])
        {
            out.reason = "original-endpoint-basis-unproven";
            return Ok(out);
        }
    }
    for k in 0..4 {
        let s = surfaces[k / 2];
        let (periodic, upper) = if k % 2 == 0 {
            (s.periodic_u, s.knots_u[s.control_points.len()])
        } else {
            (s.periodic_v, s.knots_v[s.control_points[0].len()])
        };
        if periodic
            && domain[k][1] == upper
            && crate::periodic_trim::supporting_seam_identity(s, k % 2).is_none()
        {
            out.reason = "periodic-upper-seam-unproven";
            return Ok(out);
        }
    }
    let mut budget = Budget {
        max: limits.max_rectangles,
        used: 0,
    };
    let run = (|| -> Result<()> {
        let Some((a, ja)) = evaluate(surfaces[0], [domain[0], domain[1]], true, &mut budget)?
        else {
            out.reason = "rectangle-budget";
            return Ok(());
        };
        let Some((b, jb)) = evaluate(surfaces[1], [domain[2], domain[3]], true, &mut budget)?
        else {
            out.reason = "rectangle-budget";
            return Ok(());
        };
        let residual = [a[0].sub(b[0])?, a[1].sub(b[1])?, a[2].sub(b[2])?];
        out.residual_box = Some(residual.map(pair));
        if residual.iter().any(|v| v.lo > 0. || v.hi < 0.) {
            out.status = FiberStatus::Excluded;
            out.reason = "original-box-separation";
            return Ok(());
        }
        let mut unknown = [0_usize; 3];
        let mut u_idx = 0;
        for k in 0..4 {
            if k != free {
                unknown[u_idx] = k;
                u_idx += 1;
            }
        }
        out.support_regularity_proven = [regular(ja), regular(jb)];
        let mut centers = [0.; 3];
        let mut scales = [0.; 3];
        let mut center_domain = domain;
        for i in 0..3 {
            let d = domain[unknown[i]];
            let c = 0.5 * d[0] + 0.5 * d[1];
            if c <= d[0] || c >= d[1] {
                out.reason = "unknown-center-unrepresentable";
                return Ok(());
            }
            centers[i] = c;
            scales[i] = (c - d[0]).max(d[1] - c);
            center_domain[unknown[i]] = [c; 2];
        }
        let mut j = [[I::point(0.); 3]; 3];
        for k in 0..3 {
            for i in 0..3 {
                let a = unknown[i];
                j[k][i] = if a < 2 {
                    ja[k][a]
                } else {
                    I::point(0.).sub(jb[k][a - 2])?
                };
            }
        }
        let midpoint = j.map(|row| row.map(|v| 0.5 * v.lo + 0.5 * v.hi));
        let Some(r) = limits.preconditioner.or_else(|| inverse(midpoint)) else {
            out.reason = "preconditioner-unavailable";
            return Ok(());
        };
        let Some((a, _)) = evaluate(
            surfaces[0],
            [center_domain[0], center_domain[1]],
            false,
            &mut budget,
        )?
        else {
            out.reason = "rectangle-budget";
            return Ok(());
        };
        let Some((b, _)) = evaluate(
            surfaces[1],
            [center_domain[2], center_domain[3]],
            false,
            &mut budget,
        )?
        else {
            out.reason = "rectangle-budget";
            return Ok(());
        };
        let f = [a[0].sub(b[0])?, a[1].sub(b[1])?, a[2].sub(b[2])?];
        let mut image = [I::point(0.); 3];
        let mut magnitudes = [[0.; 3]; 3];
        let mut q = 0f64;
        for i in 0..3 {
            let mut base = I::point(centers[i]);
            for k in 0..3 {
                base = base.sub(I::point(r[i][k]).mul(f[k])?)?;
            }
            let mut row = I::point(0.);
            for c in 0..3 {
                let mut v = I::point(if i == c { 1. } else { 0. });
                for k in 0..3 {
                    v = v.sub(I::point(r[i][k]).mul(j[k][c])?)?;
                }
                let magnitude = v.lo.abs().max(v.hi.abs());
                magnitudes[i][c] = magnitude;
                row = row
                    .add(I::point(magnitude).mul(I::point(scales[c]).div(I::point(scales[i]))?)?)?;
                let delta = I::new(domain[unknown[c]][0], domain[unknown[c]][1])?
                    .sub(I::point(centers[c]))?;
                base = base.add(v.mul(delta)?)?;
            }
            q = q.max(row.hi);
            image[i] = base;
        }
        if q >= 1. {
            let (new_scales, new_q) = rebalance_norm(magnitudes, scales, q)?;
            scales = new_scales;
            q = new_q;
        }
        // Keep the existing inexpensive enclosure when it qualifies. For
        // rotated correlated coordinates, a fresh original-net R*F enclosure
        // can avoid artificial cancellation loss in R*[DF]. Both constructions
        // implement the SAME fixed preconditioner; no geometry/tolerance edit.
        let included = |image: &[I; 3]| {
            (0..3)
                .all(|i| domain[unknown[i]][0] < image[i].lo && image[i].hi < domain[unknown[i]][1])
        };
        if (q >= 1. || !included(&image)) && budget.used < budget.max {
            let origin = &surfaces[0].control_points[0][0];
            let Some((_, pa)) = evaluate_projected(
                surfaces[0],
                [domain[0], domain[1]],
                r,
                origin,
                true,
                &mut budget,
            )?
            else {
                out.reason = "rectangle-budget";
                return Ok(());
            };
            let Some((_, pb)) = evaluate_projected(
                surfaces[1],
                [domain[2], domain[3]],
                r,
                origin,
                true,
                &mut budget,
            )?
            else {
                out.reason = "rectangle-budget";
                return Ok(());
            };
            let Some((ca, _)) = evaluate_projected(
                surfaces[0],
                [center_domain[0], center_domain[1]],
                r,
                origin,
                false,
                &mut budget,
            )?
            else {
                out.reason = "rectangle-budget";
                return Ok(());
            };
            let Some((cb, _)) = evaluate_projected(
                surfaces[1],
                [center_domain[2], center_domain[3]],
                r,
                origin,
                false,
                &mut budget,
            )?
            else {
                out.reason = "rectangle-budget";
                return Ok(());
            };
            let mut candidate = [I::point(0.); 3];
            let mut contraction = 0_f64;
            let mut candidate_magnitudes = [[0.; 3]; 3];
            for i in 0..3 {
                let mut base = I::point(centers[i]).sub(ca[i].sub(cb[i])?)?;
                let mut row = I::point(0.);
                for c in 0..3 {
                    let a = unknown[c];
                    let j = if a < 2 {
                        pa[i][a]
                    } else {
                        I::point(0.).sub(pb[i][a - 2])?
                    };
                    let v = I::point(if i == c { 1. } else { 0. }).sub(j)?;
                    let magnitude = v.lo.abs().max(v.hi.abs());
                    candidate_magnitudes[i][c] = magnitude;
                    row = row.add(
                        I::point(magnitude).mul(I::point(scales[c]).div(I::point(scales[i]))?)?,
                    )?;
                    base = base.add(
                        v.mul(I::new(domain[a][0], domain[a][1])?.sub(I::point(centers[c]))?)?,
                    )?;
                }
                candidate[i] = base;
                contraction = contraction.max(row.hi);
            }
            let (candidate_scales, contraction) = if contraction >= 1. {
                rebalance_norm(candidate_magnitudes, scales, contraction)?
            } else {
                (scales, contraction)
            };
            // Both images enclose the SAME fixed-R Krawczyk map on the
            // original box. Their intersection is a valid map enclosure even
            // before strict inclusion succeeds; retaining it lets contraction
            // refine an inconclusive box without inventing a sampled seed.
            let combined = [0, 1, 2].map(|i| {
                (
                    image[i].lo.max(candidate[i].lo),
                    image[i].hi.min(candidate[i].hi),
                )
            });
            if combined.iter().all(|&(lo, hi)| lo <= hi) {
                for i in 0..3 {
                    image[i] = I::new(combined[i].0, combined[i].1)?;
                }
            }
            if contraction < q {
                q = contraction;
                scales = candidate_scales;
                magnitudes = candidate_magnitudes;
            }
        }
        if q < 1. {
            // A nearly constant unknown has a tiny box radius. Reprove a norm
            // that avoids artificial physical residual amplification; keeping
            // a better verified world-error bound does not change inclusion.
            let size = scales.iter().copied().fold(0_f64, f64::max);
            let equal = [size; 3];
            let equal_q = norm_upper(magnitudes, equal)?;
            let (balanced, balanced_q) = rebalance_norm(magnitudes, equal, equal_q)?;
            if balanced_q < 1.
                && world_gain(r, ja, &unknown, free, balanced, balanced_q)?
                    < world_gain(r, ja, &unknown, free, scales, q)?
            {
                scales = balanced;
                q = balanced_q;
            }
        }
        out.root_image = Some(image.map(pair));
        out.contraction_upper = Some(q);
        if (0..3)
            .any(|i| image[i].hi < domain[unknown[i]][0] || image[i].lo > domain[unknown[i]][1])
        {
            out.status = FiberStatus::Excluded;
            out.reason = "original-krawczyk-exclusion";
            return Ok(());
        }
        if q >= 1. {
            out.reason = "contraction-unproven";
            return Ok(());
        }
        if !(0..3)
            .all(|i| domain[unknown[i]][0] < image[i].lo && image[i].hi < domain[unknown[i]][1])
        {
            out.reason = "uniform-inclusion-unproven";
            return Ok(());
        }
        out.operator = Some(FiberOperator {
            preconditioner: r,
            scales,
            first_jacobian: ja.map(|row| row.map(pair)),
        });
        out.status = FiberStatus::UniformUniqueRoot;
        out.reason = "uniform-original-krawczyk-fiber";
        Ok(())
    })();
    out.rectangles = budget.used;
    if run.is_err() {
        out.status = FiberStatus::Unresolved;
        out.reason = "source-operator-unavailable";
    }
    Ok(out)
}
