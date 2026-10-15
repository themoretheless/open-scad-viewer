use super::*;

pub(super) fn certify_periodic_surface(surface: &Surface) -> Result<Option<StepRegularityEvidence>> {
    let nu = surface.control_points.len();
    let nv = surface.control_points.first().map_or(0, Vec::len);
    if nu != 9
        || surface.degree_u != 2
        || !matches!((nv, surface.degree_v), (2, 1) | (5, 2) | (9, 2))
    {
        return Ok(None);
    }
    if surface
        .weights
        .iter()
        .flatten()
        .any(|weight| !weight.is_finite() || *weight <= 0.)
    {
        return Err(refuse(
            "STEP /8 rational denominator is not positive on the whole parameter domain",
        ));
    }
    if surface.control_points[0] != surface.control_points[nu - 1]
        || surface.weights[0] != surface.weights[nu - 1]
    {
        return Err(refuse(
            "STEP /8 lifted U seam lacks exact homogeneous control-net identity",
        ));
    }
    let du = [
        surface.knots_u[surface.degree_u],
        surface.knots_u[surface.knots_u.len() - surface.degree_u - 1],
    ];
    let dv = [
        surface.knots_v[surface.degree_v],
        surface.knots_v[surface.knots_v.len() - surface.degree_v - 1],
    ];
    let denominator_lower_bound = surface
        .weights
        .iter()
        .flatten()
        .copied()
        .fold(f64::INFINITY, f64::min);
    let radius_at = |column: usize| {
        surface.control_points[0][column]
            .iter()
            .zip(&surface.control_points[4][column])
            .map(|(left, right)| (left - right) * (left - right))
            .sum::<f64>()
            .sqrt()
            / 2.
    };
    let radii = if nv == 2 {
        vec![radius_at(0), radius_at(1)]
    } else {
        (0..nv).step_by(2).map(radius_at).collect::<Vec<_>>()
    };
    let positive_min = radii
        .iter()
        .copied()
        .filter(|radius| *radius > 1e-12)
        .fold(f64::INFINITY, f64::min);
    let positive_max = radii.iter().copied().fold(0., f64::max);
    let (
        carrier,
        collapsed_boundaries,
        regular_open_domain,
        jacobian_lower_bound,
        lifted_v,
        identity,
    ) = match nv {
        2 => {
            let collapsed = if constant_boundary(surface, 0) {
                vec!["v-min"]
            } else if constant_boundary(surface, 1) {
                vec!["v-max"]
            } else {
                vec![]
            };
            let carrier = if collapsed.is_empty() && ((radii[0] - radii[1]).abs() <= 1e-12) {
                "cylinder"
            } else {
                "cone"
            };
            (
                carrier,
                collapsed,
                !radii.iter().all(|radius| *radius > 1e-12),
                if radii.iter().all(|radius| *radius > 1e-12) {
                    positive_min
                } else {
                    0.
                },
                0.,
                "positive rational quadratic circle × affine generator; J=(affine radius)·J0",
            )
        }
        5 => {
            if !constant_boundary(surface, 0) || !constant_boundary(surface, nv - 1) {
                return Err(refuse(
                    "STEP /8 sphere requires exact collapsed V boundary control nets",
                ));
            }
            (
                "sphere",
                vec!["v-min", "v-max"],
                true,
                0.,
                0.,
                "positive rational circle × semicircle; J=R² cos(v)·J0 with only endpoint zeros",
            )
        }
        9 => {
            if surface
                .control_points
                .iter()
                .map(|row| &row[0])
                .ne(surface.control_points.iter().map(|row| &row[nv - 1]))
                || surface
                    .weights
                    .iter()
                    .map(|row| row[0])
                    .ne(surface.weights.iter().map(|row| row[nv - 1]))
            {
                return Err(refuse(
                    "STEP /8 lifted V seam lacks exact homogeneous control-net identity",
                ));
            }
            if !(positive_min > 1e-12 && positive_max > positive_min) {
                return Err(refuse(
                    "STEP /8 torus requires exact positive major-minus-minor radius",
                ));
            }
            (
                "torus",
                vec![],
                false,
                positive_min,
                dv[1] - dv[0],
                "positive rational circle product; J=r(R+r cos(v))·J0 and R-r>0",
            )
        }
        _ => unreachable!(),
    };
    Ok(Some(StepRegularityEvidence {
        carrier,
        parameter_u: du,
        parameter_v: dv,
        lifted_periods: [du[1] - du[0], lifted_v],
        denominator_lower_bound,
        jacobian_lower_bound,
        collapsed_boundaries,
        regular_open_domain,
        identity,
    }))
}
pub(super) fn certify_step_v8_topology(model: &Model) -> Result<StepV8Certificate> {
    model.validate()?;
    let mut seen = BTreeSet::new();
    let mut regularity = Vec::new();
    for face in &model.faces {
        if !seen.insert(format!("{:?}", face.surface)) {
            continue;
        }
        if let Some(evidence) = certify_periodic_surface(&face.surface)? {
            regularity.push(evidence)
        }
    }
    for evidence in &regularity {
        for boundary in &evidence.collapsed_boundaries {
            let boundary_value = if *boundary == "v-min" {
                evidence.parameter_v[0]
            } else {
                evidence.parameter_v[1]
            };
            let mut intervals = Vec::<[f64; 2]>::new();
            for face in &model.faces {
                if !certify_periodic_surface(&face.surface)?
                    .is_some_and(|candidate| candidate.carrier == evidence.carrier)
                {
                    continue;
                }
                for loop_id in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
                    for coedge in &model.loops[loop_id].coedges {
                        if !model.edges[coedge.edge].degenerate {
                            continue;
                        }
                        let domain = coedge.pcurve.domain();
                        let a = coedge.pcurve.evaluate(domain[0])?.point;
                        let b = coedge.pcurve.evaluate(domain[1])?.point;
                        if (a[1] - boundary_value).abs() <= 1e-12
                            && (b[1] - boundary_value).abs() <= 1e-12
                        {
                            intervals.push([a[0].min(b[0]), a[0].max(b[0])])
                        }
                    }
                }
            }
            intervals.sort_by(|left, right| left[0].total_cmp(&right[0]));
            let mut covered = evidence.parameter_u[0];
            for interval in intervals {
                if interval[0] > covered + 1e-12 {
                    break;
                }
                covered = covered.max(interval[1]);
            }
            if covered < evidence.parameter_u[1] - 1e-12 {
                return Err(refuse(
                    "STEP /8 pole topology does not cover the complete collapsed parameter interval",
                ));
            }
        }
    }
    Ok(StepV8Certificate {
        capability: STEP_INTERCHANGE_V8_CAPABILITY,
        complete: true,
        regularity,
        sense_layers: [
            "seam/pole/hole",
            "EDGE_CURVE.same_sense",
            "ORIENTED_EDGE.orientation",
            "FACE_BOUND.orientation",
            "ADVANCED_FACE.same_sense",
            "CLOSED_SHELL face use",
            "ORIENTED_CLOSED_SHELL.orientation",
        ],
        coupled_sense_cases: 128,
        notes: vec![
            "whole_domain_homogeneous_interval_certificate",
            "collapsed_boundaries_bound_to_pole_topology",
            "lifted_periods_bound_to_seam_topology",
            "exhaustive_coupled_sense_matrix",
        ],
    })
}
pub(crate) fn refs_text(v: &[usize]) -> String {
    v.iter()
        .map(|x| format!("#{x}"))
        .collect::<Vec<_>>()
        .join(",")
}
pub(super) fn knot_form(knots: &[f64]) -> (Vec<usize>, Vec<f64>) {
    let mut m = Vec::new();
    let mut k = Vec::new();
    for &x in knots {
        if k.last()
            .is_some_and(|v: &f64| (*v - x).abs() <= f64::EPSILON)
        {
            *m.last_mut().unwrap() += 1;
        } else {
            k.push(x);
            m.push(1);
        }
    }
    (m, k)
}
pub(super) fn emit_curve(w: &mut Writer, c: &Curve) -> usize {
    let cps: Vec<_> = c.control_points.iter().map(|p| w.point(p)).collect();
    let (m, k) = knot_form(&c.knots);
    w.emit(format!("B_SPLINE_CURVE_WITH_KNOTS('',{},({}),.UNSPECIFIED.,.F.,.{closed}.,.F.,({}),({}),({}),.UNSPECIFIED.)",
        c.degree, refs_text(&cps), floats(&c.weights), m.iter().map(ToString::to_string).collect::<Vec<_>>().join(","), floats(&k),
        closed = if c.periodic { "T" } else { "F" }))
}
pub(super) fn emit_surface(w: &mut Writer, s: &Surface) -> usize {
    let grid = s
        .control_points
        .iter()
        .map(|row| {
            let ids: Vec<_> = row.iter().map(|p| w.point(p)).collect();
            format!("({})", refs_text(&ids))
        })
        .collect::<Vec<_>>()
        .join(",");
    let (mu, ku) = knot_form(&s.knots_u);
    let (mv, kv) = knot_form(&s.knots_v);
    w.emit(format!("B_SPLINE_SURFACE_WITH_KNOTS('',{},{},({grid}),.UNSPECIFIED.,.F.,.{pu}.,.{pv}.,.F.,.F.,({}),({}),({}),({}),({}),.UNSPECIFIED.)",
        s.degree_u, s.degree_v, floats(&s.weights.iter().flatten().copied().collect::<Vec<_>>()),
        mu.iter().map(ToString::to_string).collect::<Vec<_>>().join(","),
        mv.iter().map(ToString::to_string).collect::<Vec<_>>().join(","), floats(&ku), floats(&kv),
        pu=if s.periodic_u {"T"} else {"F"}, pv=if s.periodic_v {"T"} else {"F"}))
}
pub(super) fn all_unit_weights(mut weights: impl Iterator<Item = f64>) -> bool {
    weights.all(|weight| (weight - 1.).abs() <= f64::EPSILON)
}
pub(super) fn knot_form_exchange(knots: &[f64], exact: bool) -> (Vec<usize>, Vec<f64>) {
    if !exact {
        return knot_form(knots);
    }
    let mut counts = Vec::new();
    let mut values = Vec::new();
    for &knot in knots {
        if values.last() == Some(&knot) {
            *counts.last_mut().unwrap() += 1;
        } else {
            values.push(knot);
            counts.push(1);
        }
    }
    (counts, values)
}
pub(super) fn emit_curve_v5(w: &mut Writer, c: &Curve) -> usize {
    emit_curve_exchange(w, c, false)
}
pub(crate) fn emit_curve_source(w: &mut Writer, c: &Curve) -> usize {
    emit_curve_exchange(w, c, true)
}
pub(super) fn emit_curve_exchange(w: &mut Writer, c: &Curve, exact: bool) -> usize {
    let cps: Vec<_> = c.control_points.iter().map(|p| w.point(p)).collect();
    let (m, k) = knot_form_exchange(&c.knots, exact);
    let mult = m
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let closed = if c.periodic { "T" } else { "F" };
    let unit = if exact { c.weights.iter().all(|&v| v == 1.) }
        else { all_unit_weights(c.weights.iter().copied()) };
    if unit {
        w.emit(format!("B_SPLINE_CURVE_WITH_KNOTS('',{},({}),.UNSPECIFIED.,.{closed}.,.F.,({mult}),({}),.UNSPECIFIED.)",
            c.degree,refs_text(&cps),floats(&k)))
    } else {
        w.emit(format!("(BOUNDED_CURVE()B_SPLINE_CURVE({},({}),.UNSPECIFIED.,.{closed}.,.F.)B_SPLINE_CURVE_WITH_KNOTS(({mult}),({}),.UNSPECIFIED.)CURVE()GEOMETRIC_REPRESENTATION_ITEM()RATIONAL_B_SPLINE_CURVE(({}))REPRESENTATION_ITEM(''))",
            c.degree,refs_text(&cps),floats(&k),floats(&c.weights)))
    }
}
pub(super) fn emit_surface_v5(w: &mut Writer, s: &Surface) -> usize {
    emit_surface_exchange(w, s, false)
}
pub(crate) fn emit_surface_source(w: &mut Writer, s: &Surface) -> usize {
    emit_surface_exchange(w, s, true)
}
pub(super) fn emit_surface_exchange(w: &mut Writer, s: &Surface, exact: bool) -> usize {
    let grid = s
        .control_points
        .iter()
        .map(|row| {
            let ids: Vec<_> = row.iter().map(|p| w.point(p)).collect();
            format!("({})", refs_text(&ids))
        })
        .collect::<Vec<_>>()
        .join(",");
    let weights = s
        .weights
        .iter()
        .map(|row| format!("({})", floats(row)))
        .collect::<Vec<_>>()
        .join(",");
    let (mu, ku) = knot_form_exchange(&s.knots_u, exact);
    let (mv, kv) = knot_form_exchange(&s.knots_v, exact);
    let mu = mu
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let mv = mv
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let pu = if s.periodic_u { "T" } else { "F" };
    let pv = if s.periodic_v { "T" } else { "F" };
    let unit = if exact { s.weights.iter().flatten().all(|&v| v == 1.) }
        else { all_unit_weights(s.weights.iter().flatten().copied()) };
    if unit {
        w.emit(format!("B_SPLINE_SURFACE_WITH_KNOTS('',{},{},({grid}),.UNSPECIFIED.,.{pu}.,.{pv}.,.F.,({mu}),({mv}),({}),({}),.UNSPECIFIED.)",
            s.degree_u,s.degree_v,floats(&ku),floats(&kv)))
    } else {
        w.emit(format!("(BOUNDED_SURFACE()B_SPLINE_SURFACE({},{},({grid}),.UNSPECIFIED.,.{pu}.,.{pv}.,.F.)B_SPLINE_SURFACE_WITH_KNOTS(({mu}),({mv}),({}),({}),.UNSPECIFIED.)GEOMETRIC_REPRESENTATION_ITEM()RATIONAL_B_SPLINE_SURFACE(({weights}))REPRESENTATION_ITEM('')SURFACE())",
            s.degree_u,s.degree_v,floats(&ku),floats(&kv)))
    }
}
