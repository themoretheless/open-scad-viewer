use super::*;

pub fn pnp_with_options(
    template: &Camera,
    pairs: &[(V3, [f64; 2])],
    options: &GeometryOptions,
    report: &mut RobustReport,
) -> Option<Camera> {
    *report = RobustReport::default();
    if pairs.len() < 8
        || ![template.focal, template.cx, template.cy]
            .iter()
            .all(|v| v.is_finite())
        || template.focal <= 0.
        || pairs
            .iter()
            .any(|(p, q)| p.iter().chain(q).any(|v| !v.is_finite()))
    {
        return None;
    }
    let mut rng = Rng::new();
    let mut guess = template.clone();
    refine(&mut guess, pairs);
    let mut best: Vec<_> = pairs
        .iter()
        .copied()
        .filter(|&(p, q)| {
            guess
                .project(p)
                .is_some_and(|uv| (uv[0] - q[0]).powi(2) + (uv[1] - q[1]).powi(2) < 16.)
        })
        .collect();
    let mut result = Some(guess);
    let maximum = if options.adaptive_ransac { 384 } else { 192 };
    let mut budget = maximum;
    let mut sample: Vec<(V3, [f64; 2])> = Vec::with_capacity(6);
    let mut good: Vec<(V3, [f64; 2])> = Vec::with_capacity(pairs.len());
    let mut accepted: Vec<(V3, [f64; 2])> = Vec::with_capacity(pairs.len());
    for iteration in 0..maximum {
        if iteration >= budget {
            break;
        }
        report.iterations += 1;
        sample.clear();
        sample.extend(rng.subset(pairs.len(), 6).iter().map(|&i| pairs[i]));
        if options.reject_degenerate {
            let mean = scale(
                sample.iter().fold([0.; 3], |sum, (p, _)| add(sum, *p)),
                1. / sample.len() as f64,
            );
            let mut covariance = [[0.; 3]; 3];
            for (point, _) in &sample {
                let d = sub(*point, mean);
                for i in 0..3 {
                    for j in 0..3 {
                        covariance[i][j] += d[i] * d[j];
                    }
                }
            }
            let trace = covariance[0][0] + covariance[1][1] + covariance[2][2];
            if trace < 1e-12 || det(covariance) < 1e-8 * trace.powi(3) {
                report.degenerate_samples += 1;
                continue;
            }
        }
        let Some(mut c) = dlt(template, &sample) else {
            continue;
        };
        report.hypotheses += 1;
        refine(&mut c, &sample);
        good.clear();
        good.extend(pairs.iter().copied().filter(|&(p, q)| {
            c.project(p)
                .is_some_and(|uv| (uv[0] - q[0]).powi(2) + (uv[1] - q[1]).powi(2) < 16.)
        }));
        if good.len() > best.len() {
            if options.local_optimization {
                for _ in 0..2 {
                    let mut next = c.clone();
                    refine(&mut next, &good);
                    report.local_refits += 1;
                    accepted.clear();
                    accepted.extend(pairs.iter().copied().filter(|&(p, q)| {
                        next.project(p)
                            .is_some_and(|uv| (uv[0] - q[0]).powi(2) + (uv[1] - q[1]).powi(2) < 16.)
                    }));
                    if accepted.len() < good.len() {
                        break;
                    }
                    c = next;
                    std::mem::swap(&mut good, &mut accepted);
                }
            }
            std::mem::swap(&mut best, &mut good);
            result = Some(c);
            if options.adaptive_ransac {
                budget = adaptive_budget(best.len(), pairs.len(), 6, maximum).max(24);
            }
        }
        if !options.adaptive_ransac && best.len() * 10 > pairs.len() * 9 {
            break;
        }
    }
    if best.len() < 8.max(pairs.len() / 5) {
        return None;
    }
    let mut c = result?;
    refine(&mut c, &best);
    if options.local_optimization {
        let accepted = pairs
            .iter()
            .filter(|&&(p, q)| {
                c.project(p)
                    .is_some_and(|uv| (uv[0] - q[0]).powi(2) + (uv[1] - q[1]).powi(2) < 16.)
            })
            .count();
        if accepted < 8.max(pairs.len() / 5) {
            return None;
        }
        report.inliers = accepted;
    } else {
        report.inliers = best.len();
    }
    Some(c)
}
