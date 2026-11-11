//! Native convex hull, regularity and singularity certificates.
use super::{
    Curve, MAX_CERTIFICATE_CELLS, Result, Surface, ToleranceContext, bernstein_product, box_of,
    check, context, fully_bezier_surface, interval, interval_contains_zero, interval_width,
    numeric, resource, surface_normal_bounds,
};
#[cfg(feature = "codec")]
mod serialization;
#[cfg(feature = "codec")]
pub use serialization::{certify_curve, certify_surface};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CurveRegularityClass {
    Regular,
    Singular,
    Unresolved,
}
pub enum CurveRegularity {
    Constant,
    Subdivision {
        depth: usize,
        children: Vec<CurveRegularity>,
    },
    Bounds {
        classification: CurveRegularityClass,
        derivative_numerator_bounds: Vec<[f64; 2]>,
    },
}
impl CurveRegularity {
    pub fn certified(&self) -> bool {
        matches!(
            self,
            Self::Subdivision { .. }
                | Self::Bounds {
                    classification: CurveRegularityClass::Regular,
                    ..
                }
        )
    }
}
pub struct CurveSpanCertificate {
    pub domain: [f64; 2],
    pub min: Vec<f64>,
    pub max: Vec<f64>,
    pub denominator: [f64; 2],
    pub regularity: CurveRegularity,
}
pub struct CurveCertificate {
    pub periodic: bool,
    pub period: Option<f64>,
    pub spans: Vec<CurveSpanCertificate>,
    pub tolerance: ToleranceContext,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceRegularityClass {
    Singular,
    SingularOrUnresolved,
    PlanarRegular,
    Regular,
    Unresolved,
}
pub struct SurfaceNormalRegularity {
    pub classification: SurfaceRegularityClass,
    pub normal_numerator_bounds: Vec<[f64; 2]>,
    pub corner_normals: Option<Vec<[f64; 3]>>,
}
pub struct SurfaceCellCertificate {
    pub domain_u: [f64; 2],
    pub domain_v: [f64; 2],
    pub min: Vec<f64>,
    pub max: Vec<f64>,
    pub denominator: [f64; 2],
    pub normal_regularity: SurfaceNormalRegularity,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NormalBoxClass {
    SingularPatch,
    IsolatedSingularPoint,
    Regular,
    Unresolved,
    ResourceExhausted,
    TrimFailed,
}
pub struct SingularRegion {
    pub parameter_box: [f64; 4],
    pub normal_numerator_bounds: Option<Vec<[f64; 2]>>,
    pub classification: Option<NormalBoxClass>,
    pub point: Option<[f64; 2]>,
}
pub struct SingularityLocalization {
    pub work_cells: usize,
    pub isolated_points: Vec<SingularRegion>,
    pub singular_curves: Vec<SingularRegion>,
    pub regular_complement: Vec<SingularRegion>,
    pub unresolved: Vec<SingularRegion>,
}
impl SingularityLocalization {
    pub fn complete(&self) -> bool {
        self.unresolved.is_empty()
    }
}
pub struct SurfaceCertificate {
    pub periodic_u: bool,
    pub periodic_v: bool,
    pub period_u: Option<f64>,
    pub period_v: Option<f64>,
    pub cells: Vec<SurfaceCellCertificate>,
    pub singularity_localization: SingularityLocalization,
    pub tolerance: ToleranceContext,
}

/// Per-Bezier-span positive denominator and conservative Euclidean hull bounds.
pub fn certify_curve_report(
    curve: &Curve,
    tolerance: Option<ToleranceContext>,
) -> Result<CurveCertificate> {
    curve.validate()?;
    let tolerance = context(tolerance);
    let segments = curve.decompose()?;
    check(
        segments.len() <= MAX_CERTIFICATE_CELLS,
        "Curve certificate exceeds the span resource limit",
    )?;
    let mut spans = Vec::with_capacity(segments.len());
    for segment in segments {
        let c = segment.definition();
        let (min, max) = box_of(&c.control_points);
        let denominator = interval(c.weights.iter().copied());
        numeric(
            denominator[0] > 0.,
            "Outward denominator lower bound is not positive",
        )?;
        let regularity = curve_span_regularity(c);
        spans.push(CurveSpanCertificate {
            domain: segment.domain(),
            min,
            max,
            denominator,
            regularity,
        });
    }
    Ok(CurveCertificate {
        periodic: curve.periodic,
        period: if curve.periodic {
            Some(curve.domain()[1] - curve.domain()[0])
        } else {
            None
        },
        spans,
        tolerance,
    })
}

fn curve_span_regularity(curve: &Curve) -> CurveRegularity {
    curve_span_regularity_depth(curve, 0)
}

fn curve_span_regularity_depth(curve: &Curve, depth: usize) -> CurveRegularity {
    let p = curve.degree;
    if p == 0 {
        return CurveRegularity::Constant;
    }
    let homogeneous: Vec<Vec<f64>> = curve
        .control_points
        .iter()
        .zip(&curve.weights)
        .map(|(point, weight)| {
            point
                .iter()
                .map(|coordinate| coordinate * weight)
                .chain(std::iter::once(*weight))
                .collect()
        })
        .collect();
    let derivative: Vec<Vec<f64>> = homogeneous
        .array_windows()
        .map(|[a, b]| a.iter().zip(b).map(|(x, y)| p as f64 * (y - x)).collect())
        .collect();
    // C' numerator is X'W-XW'. Bernstein products preserve convex hulls.
    let dimension = curve.control_points[0].len();
    let mut component_bounds = Vec::new();
    for axis in 0..dimension {
        let first = bernstein_product(
            &derivative
                .iter()
                .map(|value| value[axis])
                .collect::<Vec<_>>(),
            &homogeneous
                .iter()
                .map(|value| value[dimension])
                .collect::<Vec<_>>(),
        );
        let second = bernstein_product(
            &homogeneous
                .iter()
                .map(|value| value[axis])
                .collect::<Vec<_>>(),
            &derivative
                .iter()
                .map(|value| value[dimension])
                .collect::<Vec<_>>(),
        );
        let coefficients = first.into_iter().zip(second).map(|(a, b)| a - b);
        component_bounds.push(interval(coefficients.into_iter()));
    }
    let separated = component_bounds
        .iter()
        .any(|bound| bound[0] > 0. || bound[1] < 0.);
    let identically_zero = component_bounds.iter().all(|bound| {
        bound[0] <= 0.
            && bound[1] >= 0.
            && bound[0].abs() <= f64::from_bits(1)
            && bound[1].abs() <= f64::from_bits(1)
    });
    if !separated && !identically_zero && depth < 8 {
        let [a, b] = curve.domain();
        if let Ok(children) = curve.split((a + b) / 2.) {
            let certificates = children
                .iter()
                .map(|child| curve_span_regularity_depth(child, depth + 1))
                .collect::<Vec<_>>();
            if certificates
                .iter()
                .all(|certificate| certificate.certified())
            {
                return CurveRegularity::Subdivision {
                    depth: depth + 1,
                    children: certificates,
                };
            }
        }
    }
    CurveRegularity::Bounds {
        classification: if separated {
            CurveRegularityClass::Regular
        } else if identically_zero {
            CurveRegularityClass::Singular
        } else {
            CurveRegularityClass::Unresolved
        },
        derivative_numerator_bounds: component_bounds,
    }
}

/// Per nonempty knot rectangle, active control support supplies a conservative hull.
pub fn certify_surface_report(
    surface: &Surface,
    tolerance: Option<ToleranceContext>,
) -> Result<SurfaceCertificate> {
    surface.validate()?;
    let tolerance = context(tolerance);
    let bezier = fully_bezier_surface(surface)?;
    let nu = bezier.control_points.len();
    let nv = bezier.control_points[0].len();
    let u_spans: Vec<usize> = (bezier.degree_u..nu)
        .filter(|&i| bezier.knots_u[i] < bezier.knots_u[i + 1])
        .collect();
    let v_spans: Vec<usize> = (bezier.degree_v..nv)
        .filter(|&i| bezier.knots_v[i] < bezier.knots_v[i + 1])
        .collect();
    if u_spans.len().saturating_mul(v_spans.len()) > MAX_CERTIFICATE_CELLS {
        return Err(resource("Surface certificate exceeds 4096 span cells"));
    }
    let mut cells = Vec::new();
    for &iu in &u_spans {
        for &iv in &v_spans {
            let mut points = Vec::new();
            let mut weights = Vec::new();
            for u in iu - bezier.degree_u..=iu {
                for v in iv - bezier.degree_v..=iv {
                    points.push(bezier.control_points[u][v].clone());
                    weights.push(bezier.weights[u][v]);
                }
            }
            let (min, max) = box_of(&points);
            let denominator = interval(weights.into_iter());
            numeric(
                denominator[0] > 0.,
                "Outward denominator lower bound is not positive",
            )?;
            let regularity = certify_surface_cell(&bezier, iu, iv);
            cells.push(SurfaceCellCertificate {
                domain_u: [bezier.knots_u[iu], bezier.knots_u[iu + 1]],
                domain_v: [bezier.knots_v[iv], bezier.knots_v[iv + 1]],
                min,
                max,
                denominator,
                normal_regularity: regularity,
            });
        }
    }
    let localization =
        localize_singularities(&bezier, &cells, tolerance.parametric_bounds().floor)?;
    Ok(SurfaceCertificate {
        periodic_u: surface.periodic_u,
        periodic_v: surface.periodic_v,
        period_u: if surface.periodic_u {
            Some(surface.knots_u[nu] - surface.knots_u[surface.degree_u])
        } else {
            None
        },
        period_v: if surface.periodic_v {
            Some(surface.knots_v[nv] - surface.knots_v[surface.degree_v])
        } else {
            None
        },
        cells,
        singularity_localization: localization,
        tolerance,
    })
}

pub(super) fn certify_surface_cell(
    surface: &Surface,
    iu: usize,
    iv: usize,
) -> SurfaceNormalRegularity {
    let bounds = surface_normal_bounds(surface, iu, iv);
    let separated = bounds.iter().any(|bound| bound[0] > 0. || bound[1] < 0.);
    let zero = bounds.iter().all(|bound| bound[0] == 0. && bound[1] == 0.);
    let corners = [
        (surface.knots_u[iu], surface.knots_v[iv]),
        (surface.knots_u[iu + 1], surface.knots_v[iv]),
        (surface.knots_u[iu], surface.knots_v[iv + 1]),
        (surface.knots_u[iu + 1], surface.knots_v[iv + 1]),
    ];
    let evaluations: Vec<_> = corners
        .iter()
        .filter_map(|&(u, v)| surface.evaluate_validated(u, v).ok())
        .collect();
    if zero {
        return SurfaceNormalRegularity {
            classification: SurfaceRegularityClass::Singular,
            normal_numerator_bounds: bounds,
            corner_normals: None,
        };
    }
    if evaluations
        .iter()
        .any(|evaluation| evaluation.unit_normal().is_none())
    {
        return SurfaceNormalRegularity {
            classification: SurfaceRegularityClass::SingularOrUnresolved,
            normal_numerator_bounds: bounds,
            corner_normals: None,
        };
    }
    let normals: Vec<[f64; 3]> = evaluations
        .iter()
        .map(|evaluation| evaluation.unit_normal().unwrap())
        .collect();
    let reference = normals[0];
    let aligned = normals.iter().all(|normal| {
        normal
            .iter()
            .zip(reference)
            .map(|(a, b)| (a - b).abs())
            .fold(0., f64::max)
            <= 64. * f64::EPSILON
    });
    // Aligned corner normals do not prove planarity or exclude an interior fold.
    // Constant coordinates prove an axis-aligned plane; separation additionally
    // proves that its parameterization does not lose rank inside the cell.
    let axis_plane = (0..3).any(|axis| {
        let value = surface.control_points[iu - surface.degree_u][iv - surface.degree_v][axis];
        (iu - surface.degree_u..=iu).all(|u| {
            (iv - surface.degree_v..=iv).all(|v| surface.control_points[u][v][axis] == value)
        })
    });
    if aligned && separated && axis_plane {
        return SurfaceNormalRegularity {
            classification: SurfaceRegularityClass::PlanarRegular,
            normal_numerator_bounds: bounds,
            corner_normals: Some(normals),
        };
    }
    if separated {
        return SurfaceNormalRegularity {
            classification: SurfaceRegularityClass::Regular,
            normal_numerator_bounds: bounds,
            corner_normals: Some(normals),
        };
    }
    SurfaceNormalRegularity {
        classification: SurfaceRegularityClass::Unresolved,
        normal_numerator_bounds: bounds,
        corner_normals: Some(normals),
    }
}

pub(super) fn classify_normal_box(bounds: &[[f64; 2]]) -> NormalBoxClass {
    let zeros = bounds
        .iter()
        .filter(|b| interval_contains_zero(**b))
        .count();
    let separated = bounds.iter().any(|b| b[0] > 0. || b[1] < 0.);
    let exact = bounds.iter().all(|b| b[0] == 0. && b[1] == 0.);
    if exact {
        NormalBoxClass::SingularPatch
    } else if zeros == 3 && bounds.iter().all(|b| interval_width(*b) <= 2_f64.powi(-40)) {
        NormalBoxClass::IsolatedSingularPoint
    } else if separated {
        NormalBoxClass::Regular
    } else {
        NormalBoxClass::Unresolved
    }
}

fn localize_singularities(
    surface: &Surface,
    cells: &[SurfaceCellCertificate],
    floor: f64,
) -> Result<SingularityLocalization> {
    let mut pending = Vec::new();
    for cell in cells {
        if matches!(
            cell.normal_regularity.classification,
            SurfaceRegularityClass::Singular
                | SurfaceRegularityClass::SingularOrUnresolved
                | SurfaceRegularityClass::Unresolved
        ) {
            let domain_u = cell.domain_u;
            let domain_v = cell.domain_v;
            pending.push([domain_u[0], domain_u[1], domain_v[0], domain_v[1]]);
        }
    }
    let mut isolated_points = Vec::new();
    let mut singular_curves = Vec::new();
    let mut regular_complement = Vec::new();
    let mut unresolved = Vec::new();
    let mut work = 0_usize;
    while let Some(bounds) = pending.pop() {
        work += 1;
        if work > MAX_CERTIFICATE_CELLS {
            unresolved.push(SingularRegion {
                parameter_box: bounds,
                normal_numerator_bounds: None,
                classification: Some(NormalBoxClass::ResourceExhausted),
                point: None,
            });
            continue;
        }
        let patch = match surface.trim(bounds) {
            Ok(value) => value,
            Err(_) => {
                unresolved.push(SingularRegion {
                    parameter_box: bounds,
                    normal_numerator_bounds: None,
                    classification: Some(NormalBoxClass::TrimFailed),
                    point: None,
                });
                continue;
            }
        };
        // Evaluate normal numerator bounds on the single Bézier cell of the trimmed patch.
        let iu = patch.degree_u;
        let iv = patch.degree_v;
        let normal = surface_normal_bounds(&patch, iu, iv);
        let class = classify_normal_box(&normal);
        match class {
            NormalBoxClass::Regular => regular_complement.push(SingularRegion {
                parameter_box: bounds,
                normal_numerator_bounds: Some(normal),
                classification: None,
                point: None,
            }),
            NormalBoxClass::IsolatedSingularPoint => isolated_points.push(SingularRegion {
                parameter_box: bounds,
                point: Some([(bounds[0] + bounds[1]) * 0.5, (bounds[2] + bounds[3]) * 0.5]),
                normal_numerator_bounds: Some(normal),
                classification: Some(class),
            }),
            NormalBoxClass::SingularPatch => {
                if (bounds[1] - bounds[0]).max(bounds[3] - bounds[2]) <= floor.max(2_f64.powi(-36))
                {
                    singular_curves.push(SingularRegion {
                        parameter_box: bounds,
                        normal_numerator_bounds: Some(normal),
                        classification: Some(class),
                        point: None,
                    });
                } else {
                    let mid_u = (bounds[0] + bounds[1]) * 0.5;
                    let mid_v = (bounds[2] + bounds[3]) * 0.5;
                    pending.extend([
                        [bounds[0], mid_u, bounds[2], mid_v],
                        [mid_u, bounds[1], bounds[2], mid_v],
                        [bounds[0], mid_u, mid_v, bounds[3]],
                        [mid_u, bounds[1], mid_v, bounds[3]],
                    ]);
                }
            }
            _ if (bounds[1] - bounds[0]).max(bounds[3] - bounds[2])
                <= floor.max(2_f64.powi(-36)) =>
            {
                unresolved.push(SingularRegion {
                    parameter_box: bounds,
                    normal_numerator_bounds: Some(normal),
                    classification: Some(class),
                    point: None,
                })
            }
            _ => {
                let mid_u = (bounds[0] + bounds[1]) * 0.5;
                let mid_v = (bounds[2] + bounds[3]) * 0.5;
                pending.extend([
                    [bounds[0], mid_u, bounds[2], mid_v],
                    [mid_u, bounds[1], bounds[2], mid_v],
                    [bounds[0], mid_u, mid_v, bounds[3]],
                    [mid_u, bounds[1], mid_v, bounds[3]],
                ]);
            }
        }
    }
    Ok(SingularityLocalization {
        work_cells: work,
        isolated_points,
        singular_curves,
        regular_complement,
        unresolved,
    })
}
