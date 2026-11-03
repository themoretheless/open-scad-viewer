use super::*;
struct Work {
    native: usize,
    interval: usize,
    projection: usize,
}
struct Image {
    status: FiberStatus,
    image: Option<[[f64; 2]; 3]>,
}
fn original_image(curve: &OriginalSsRootCurve) -> [[f64; 2]; 3] {
    match &curve.carrier {
        Carrier::Native(f) => f.report().root_image.unwrap(),
        Carrier::Quotient(f) => f.report().root_image.unwrap(),
        Carrier::Deflated(f) => f.report().root_image.unwrap(),
        Carrier::Pole(f) => f.report().root_image.unwrap(),
    }
}
fn restriction(
    curve: &OriginalSsRootCurve,
    d: [[f64; 2]; 4],
    limits: OriginalSsCurveLimits,
    w: &mut Work,
) -> Result<Option<Image>> {
    let mut native = FiberLimits {
        max_rectangles: (limits.native_rectangles - w.native).min(100000),
        preconditioner: None,
    };
    let mut projection = limits.projection;
    projection.max_work -= w.projection;
    let mut algebraic = DeflatedSsFiberLimits {
        max_interval_operations: limits.interval_operations - w.interval,
        projection,
        preconditioner: None,
    };
    match &curve.carrier {
        Carrier::Native(f) => {
            if native.max_rectangles == 0 {
                return Ok(None);
            }
            native.preconditioner = Some(f.report().operator.as_ref().unwrap().preconditioner());
            let r = f.source().certify(d, f.free_axis(), native)?;
            w.native += r.report().rectangles;
            Ok(Some(Image {
                status: r.report().status,
                image: r.report().root_image,
            }))
        }
        Carrier::Quotient(f) => {
            if native.max_rectangles == 0 {
                return Ok(None);
            }
            native.preconditioner = Some(f.report().operator.as_ref().unwrap().preconditioner());
            let r = f.source().certify_quotient(d, f.free_axis(), native)?;
            w.native += r.report().rectangles;
            Ok(Some(Image {
                status: r.report().status,
                image: r.report().root_image,
            }))
        }
        Carrier::Deflated(f) => {
            if algebraic.max_interval_operations == 0 || projection.max_work == 0 {
                return Ok(None);
            }
            algebraic.preconditioner = f.report().preconditioner;
            let r = f.certify_restriction(d, algebraic)?;
            w.interval += r.interval_operations;
            w.projection += r.projection_work;
            Ok(Some(Image {
                status: r.status,
                image: r.root_image,
            }))
        }
        Carrier::Pole(f) => {
            if algebraic.max_interval_operations == 0 || projection.max_work == 0 {
                return Ok(None);
            }
            algebraic.preconditioner = f.report().preconditioner;
            let r = f.certify_restriction(d, algebraic)?;
            w.interval += r.interval_operations;
            w.projection += r.projection_work;
            Ok(Some(Image {
                status: r.status,
                image: r.root_image,
            }))
        }
    }
}
fn clip(mut domain: [[f64; 2]; 4], free: usize, image: [[f64; 2]; 3]) -> Result<[[f64; 2]; 4]> {
    let mut i = 0;
    for k in 0..4 {
        if k != free {
            domain[k] = [domain[k][0].max(image[i][0]), domain[k][1].min(image[i][1])];
            i += 1;
        }
    }
    check(
        domain.iter().all(|d| d[0] <= d[1]),
        "Original root enclosure became empty",
    )?;
    Ok(domain)
}
pub(super) fn enclose(
    curve: &OriginalSsRootCurve,
    parameter: [f64; 2],
    limits: OriginalSsCurveLimits,
) -> Result<OriginalSsCurveBounds> {
    check(
        limits.max_refinements <= 16
            && limits.native_rectangles <= 1000000
            && limits.interval_operations <= 1000000
            && (1..=100000).contains(&limits.world_rectangles)
            && (1..=10000000).contains(&limits.projection.max_work),
        "Invalid original root curve query allowance",
    )?;
    let free = curve.free_axis().index();
    let mut d = curve.coordinate_domain();
    check(
        parameter.iter().all(|x| x.is_finite())
            && d[free][0] <= parameter[0]
            && parameter[0] <= parameter[1]
            && parameter[1] <= d[free][1],
        "Original root curve query leaves its owned interval",
    )?;
    d[free] = parameter;
    let mut current = clip(d, free, original_image(curve))?;
    let mut work = Work {
        native: 0,
        interval: 0,
        projection: 0,
    };
    for _ in 0..limits.max_refinements {
        let Some(result) = restriction(curve, d, limits, &mut work)? else {
            break;
        };
        check(
            result.status != FiberStatus::Excluded,
            "Fresh exclusion contradicts the owned original root graph",
        )?;
        if result.status != FiberStatus::UniformUniqueRoot {
            break;
        }
        current = clip(d, free, result.image.unwrap())?;
        let mut next = d;
        let mut improved = false;
        for k in 0..4 {
            if k == free {
                continue;
            }
            let mid = current[k][0] * 0.5 + current[k][1] * 0.5;
            // Keep proposed boxes above the original basis's derivative
            // resolution while other coordinates still need contraction.
            // Only the fresh root IMAGE is returned, never this padding.
            let original = curve.coordinate_domain();
            let scale = match &curve.carrier {
                Carrier::Deflated(_) | Carrier::Pole(_) => f64::EPSILON.sqrt(),
                _ => {
                    let source = curve.source();
                    let surface = if k < 2 { source.first() } else { source.second() };
                    let (knots, p, n) = if k % 2 == 0 {
                        (&surface.knots_u, surface.degree_u, surface.control_points.len())
                    } else {
                        (&surface.knots_v, surface.degree_v, surface.control_points[0].len())
                    };
                    (knots[n] * 0.5 - knots[p] * 0.5) * (2. * f64::EPSILON.sqrt())
                }
            };
            let floor = scale.min((original[k][1] - original[k][0]) * 0.25);
            let pad = (0.1 * (current[k][1] - current[k][0]))
                .max(16. * (mid.next_up() - mid).abs()).max(floor);
            next[k] = [
                (current[k][0] - pad).max(d[k][0]),
                (current[k][1] + pad).min(d[k][1]),
            ];
            improved |= next[k][1] - next[k][0] < 0.8 * (d[k][1] - d[k][0]);
        }
        if !improved || (0..4).any(|k| k != free && next[k][0] >= next[k][1]) {
            break;
        }
        d = next;
    }
    let uv = frames::uv(curve, current)?;
    let (world, world_rectangles) = frames::world(curve, uv, limits.world_rectangles)?;
    Ok(OriginalSsCurveBounds {
        uv,
        world,
        native_rectangles: work.native,
        interval_operations: work.interval,
        projection_work: work.projection,
        world_rectangles,
    })
}
