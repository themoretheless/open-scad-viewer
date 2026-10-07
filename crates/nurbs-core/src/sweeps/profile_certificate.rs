//! Whole-domain deviation from the ideal Bishop-transported profile.
//!
//! Planar paths have an analytic Bishop frame. Spatial paths use the enclosing
//! set of all orthonormal frames, including any closing holonomy correction.
//! This can be deliberately loose. Neither bound proves surface regularity,
//! injectivity, seam smoothness or solid topology. Unfinished subdivision never
//! publishes a partial bound.
use super::progressive_miter::{scalar_certificate as scalar, vector_certificate as vector};
use crate::sweep_support::interval_vec3::{add, cross, div, dot_tight as dot, norm, scale, sub};
use crate::{Result, check, curve::Curve, distance_bounds::Interval as I, surface::Surface};
type V = [I; 3];

#[derive(Clone, Debug)]
pub struct Report {
    pub error_upper: Option<f64>,
    pub cells: usize,
    pub within_budget: bool,
    pub method: &'static str,
    pub reason: Option<&'static str>,
}
fn point(v: &[f64]) -> V {
    std::array::from_fn(|k| I::point(v[k]))
}
fn unit(v: V) -> Result<V> {
    div(v, norm(v)?)
}
fn from_bounds(v: [[f64; 2]; 3]) -> Result<V> {
    Ok([
        I::new(v[0][0], v[0][1])?,
        I::new(v[1][0], v[1][1])?,
        I::new(v[2][0], v[2][1])?,
    ])
}
fn initial_tangent(path: &Curve) -> Result<V> {
    let span = (path.degree..path.control_points.len())
        .find(|&i| path.knots[i] < path.knots[i + 1])
        .unwrap();
    let h = I::point(path.knots[span + 1]).sub(I::point(path.knots[span]))?;
    let c = crate::curve_distance::restricted_controls(
        path,
        span,
        I::new(path.knots[span], path.knots[span + 1])?,
    )?;
    let factor = I::point(path.degree as f64).div(h)?;
    let w1 = c[1][3].sub(c[0][3])?.mul(factor)?;
    let mut d = [I::point(0.); 3];
    for k in 0..3 {
        let relative = c[0][k].div(c[0][3])?;
        d[k] = c[1][k]
            .sub(c[0][k])?
            .mul(factor)?
            .sub(relative.mul(w1)?)?
            .div(c[0][3])?;
    }
    unit(d)
}
fn values(curve: &Curve, range: [f64; 2], remaining: usize, used: &mut usize) -> Result<Option<V>> {
    let r = vector::certify_values_traversal(curve, range, remaining, false)?;
    *used += r.cells;
    r.value.map(from_bounds).transpose()
}
// Restrict *original-span derivatives*, rather than differentiating already
// restricted controls. The latter divides coefficient rounding by the width
// of an ulp-sized overlap at a knot and can lose a perfectly regular tangent.
fn polynomial_hull(c: &[[I; 4]], lo: I, hi: I) -> Result<[I; 4]> {
    let degree = c.len() - 1;
    let mut result = [I::point(0.); 4];
    for i in 0..=degree {
        let mut d = c.to_vec();
        for step in 0..degree {
            let t = if step < degree - i { lo } else { hi };
            let one = I::point(1.).sub(t)?;
            for j in 0..degree - step {
                for k in 0..4 {
                    d[j][k] = d[j][k].mul(one)?.add(d[j + 1][k].mul(t)?)?;
                }
            }
        }
        if i == 0 {
            result = d[0];
        } else {
            for k in 0..4 {
                result[k] = I::new(result[k].lo.min(d[0][k].lo), result[k].hi.max(d[0][k].hi))?;
            }
        }
    }
    Ok(result)
}
fn curve_jets(
    c: &Curve,
    range: [f64; 2],
    remaining: usize,
    used: &mut usize,
) -> Result<Option<[V; 2]>> {
    let [a, b] = c.domain();
    let t = I::point(a)
        .add(
            I::point(b)
                .sub(I::point(a))?
                .mul(I::new(range[0], range[1])?)?,
        )?
        .intersect(a, b)?;
    let mut out: Option<[V; 2]> = None;
    for span in c.degree..c.control_points.len() {
        let ka = c.knots[span];
        let kb = c.knots[span + 1];
        if ka >= kb || t.hi < ka || t.lo > kb {
            continue;
        }
        if *used >= remaining {
            return Ok(None);
        }
        *used += 1;
        let width = I::point(kb).sub(I::point(ka))?;
        let raw = crate::curve_distance::restricted_controls(c, span, I::new(ka, kb)?)?;
        let controls: Vec<[I; 4]> = raw.iter().map(|v| [v[0], v[1], v[2], v[3]]).collect();
        let factor = I::point(c.degree as f64).div(width)?;
        let mut derivative = Vec::new();
        for pair in controls.windows(2) {
            let mut d = [I::point(0.); 4];
            for k in 0..4 {
                d[k] = pair[1][k].sub(pair[0][k])?.mul(factor)?;
            }
            derivative.push(d);
        }
        let lo = I::point(t.lo.max(ka))
            .sub(I::point(ka))?
            .div(width)?
            .intersect(0., 1.)?;
        let hi = I::point(t.hi.min(kb))
            .sub(I::point(ka))?
            .div(width)?
            .intersect(0., 1.)?;
        let h = polynomial_hull(&controls, lo, hi)?;
        let dh = polynomial_hull(&derivative, lo, hi)?;
        let mut second_controls = Vec::new();
        let factor2 = I::point(c.degree.saturating_sub(1) as f64).div(width)?;
        for pair in derivative.windows(2) {
            let mut d = [I::point(0.); 4];
            for k in 0..4 {
                d[k] = pair[1][k].sub(pair[0][k])?.mul(factor2)?;
            }
            second_controls.push(d);
        }
        if second_controls.is_empty() {
            second_controls.push([I::point(0.); 4]);
        }
        let ddh = polynomial_hull(&second_controls, lo, hi)?;
        let mut first = [I::point(0.); 3];
        let mut second = first;
        for k in 0..3 {
            first[k] = dh[k].sub(h[k].div(h[3])?.mul(dh[3])?)?.div(h[3])?;
            second[k] = ddh[k]
                .sub(first[k].mul(dh[3])?.mul(I::point(2.))?)?
                .sub(h[k].div(h[3])?.mul(ddh[3])?)?
                .div(h[3])?;
        }
        out = Some(if let Some(v) = out {
            let values = [first, second];
            std::array::from_fn(|j| {
                std::array::from_fn(|k| I {
                    lo: v[j][k].lo.min(values[j][k].lo),
                    hi: v[j][k].hi.max(values[j][k].hi),
                })
            })
        } else {
            [first, second]
        });
    }
    Ok(out)
}
fn retained_values(
    c: &Curve,
    range: [f64; 2],
    remaining: usize,
    used: &mut usize,
) -> Result<Option<V>> {
    let charge = (c.degree..c.control_points.len())
        .filter(|&i| {
            c.knots[i] < c.knots[i + 1] && c.knots[i] <= range[1] && c.knots[i + 1] >= range[0]
        })
        .count();
    if charge > remaining.saturating_sub(*used) {
        return Ok(None);
    }
    *used += charge;
    let value = crate::interval_eval::evaluate_interval(c, I::new(range[0], range[1])?)?;
    Ok(Some([value[0], value[1], value[2]]))
}
struct Reference<'a> {
    path: &'a Curve,
    scale: &'a Curve,
    plane: Option<V>,
    initial: V,
    initial_side: V,
    initial_tangent: V,
    coordinates: Vec<V>,
}
impl Reference<'_> {
    fn at(&self, range: [f64; 2], remaining: usize, used: &mut usize) -> Result<Option<Vec<V>>> {
        let Some(position) = values(self.path, range, remaining, used)? else {
            return Ok(None);
        };
        let scale_bounds =
            scalar::value_traversal(self.scale, range, remaining.saturating_sub(*used))?;
        // The value-only scalar API has no counter. Conservatively charge all
        // nonempty spans even when a range intersects only one of them.
        let charge = (self.scale.degree..self.scale.control_points.len())
            .filter(|&i| self.scale.knots[i] < self.scale.knots[i + 1])
            .count();
        if charge > remaining.saturating_sub(*used) {
            return Ok(None);
        }
        *used += charge;
        let Some(s) = scale_bounds else {
            return Ok(None);
        };
        let s = I::new(s[0], s[1])?;
        if s.lo <= 0. {
            return Ok(None);
        }
        // A point interval uses the original endpoint jet, not differences of
        // a zero-width restricted blossom.
        let tangent = if range == [0., 0.] {
            self.initial_tangent
        } else {
            let Some(jets) = curve_jets(self.path, range, remaining, used)? else {
                return Ok(None);
            };
            let Ok(t) = unit(jets[0]) else {
                return Ok(None);
            };
            t
        };
        let (normal, side) = if let Some(plane) = self.plane {
            let n0 = cross(plane, self.initial_tangent)?;
            let alpha = dot(self.initial, n0)?;
            let beta = dot(self.initial, plane)?;
            let normal = add(scale(cross(plane, tangent)?, alpha)?, scale(plane, beta)?)?;
            (normal, cross(tangent, normal)?)
        } else if range == [0., 0.] {
            (self.initial, self.initial_side)
        } else {
            // Every exact Bishop normal and binormal is a unit vector. This
            // includes any exact closing correction, without asserting its
            // angle or arc-length distribution has been computed accurately.
            ([I::new(-1., 1.)?; 3], [I::new(-1., 1.)?; 3])
        };
        self.coordinates
            .iter()
            .map(|q| {
                let offset = add(
                    add(scale(normal, q[0])?, scale(side, q[1])?)?,
                    scale(tangent, q[2])?,
                )?;
                add(position, scale(offset, s)?)
            })
            .collect::<Result<Vec<_>>>()
            .map(Some)
    }
    fn derivative(
        &self,
        range: [f64; 2],
        remaining: usize,
        used: &mut usize,
    ) -> Result<Option<Vec<V>>> {
        let Some(plane) = self.plane else {
            return Ok(None);
        };
        let Some(j) = curve_jets(self.path, range, remaining, used)? else {
            return Ok(None);
        };
        let domain = self.path.domain();
        let h = I::point(domain[1]).sub(I::point(domain[0]))?;
        let d1 = scale(j[0], h)?;
        let d2 = scale(j[1], h.mul(h)?)?;
        let speed = norm(d1)?;
        let t = div(d1, speed)?;
        let dt = sub(
            div(d2, speed)?,
            scale(d1, dot(d1, d2)?.div(speed.mul(speed)?.mul(speed)?)?)?,
        )?;
        let alpha = dot(self.initial, cross(plane, self.initial_tangent)?)?;
        let beta = dot(self.initial, plane)?;
        let normal = add(scale(cross(plane, t)?, alpha)?, scale(plane, beta)?)?;
        let dn = scale(cross(plane, dt)?, alpha)?;
        let side = cross(t, normal)?;
        let db = add(cross(dt, normal)?, cross(t, dn)?)?;
        let charge = (self.scale.degree..self.scale.control_points.len())
            .filter(|&i| self.scale.knots[i] < self.scale.knots[i + 1])
            .count();
        if charge > remaining.saturating_sub(*used) {
            return Ok(None);
        }
        let Some(s) = scalar::value_traversal(self.scale, range, charge)? else {
            return Ok(None);
        };
        *used += charge;
        let Some(sj) = curve_jets(self.scale, range, remaining, used)? else {
            return Ok(None);
        };
        let sd = self.scale.domain();
        let ds = sj[0][0].mul(I::point(sd[1]).sub(I::point(sd[0]))?)?;
        let s = I::new(s[0], s[1])?;
        self.coordinates
            .iter()
            .map(|q| {
                let offset = add(
                    add(scale(normal, q[0])?, scale(side, q[1])?)?,
                    scale(t, q[2])?,
                )?;
                let derivative = add(add(scale(dn, q[0])?, scale(db, q[1])?)?, scale(dt, q[2])?)?;
                add(d1, add(scale(offset, ds)?, scale(derivative, s)?)?)
            })
            .collect::<Result<Vec<_>>>()
            .map(Some)
    }
}
/// Certifies the supplied retained compatible loft against the ideal continuous
/// source sweep at matched (u,v), including arithmetic error. Positive profile
/// weights lift the control-wise bound to every u by convex combination.
pub fn certify(
    profile: &Curve,
    path: &Curve,
    law: &Curve,
    normal: [f64; 3],
    retained: &Surface,
    budget: f64,
    max_cells: usize,
) -> Result<Report> {
    profile.validate()?;
    path.validate()?;
    law.validate()?;
    retained.validate()?;
    check(
        budget.is_finite() && budget >= 0. && max_cells <= 100000,
        "Invalid continuous profile certificate budget",
    )?;
    let count = retained.control_points[0].len();
    check(
        retained.degree_v >= 1
            && retained.degree_u == profile.degree
            && retained.knots_u == profile.knots
            && retained.control_points.len() == profile.control_points.len()
            && count >= 2
            && retained
                .weights
                .iter()
                .zip(&profile.weights)
                .all(|(row, weight)| row.iter().all(|w| w == weight))
            && retained.knots_v[retained.degree_v] == 0.
            && retained.knots_v[count] == 1.,
        "Certificate requires a normalized compatible profile loft",
    )?;
    let linear = retained.degree_v == 1
        && retained.knots_v[1..=count]
            .iter()
            .enumerate()
            .all(|(i, k)| *k == i as f64 / (count - 1) as f64);
    let retained_curves: Vec<_> = retained
        .control_points
        .iter()
        .map(|row| Curve {
            degree: retained.degree_v,
            knots: retained.knots_v.clone(),
            control_points: row.clone(),
            weights: vec![1.; count],
            periodic: false,
        })
        .collect();
    check(
        normal.iter().all(|x| x.is_finite())
            && profile.control_points[0].len() == 3
            && path.control_points[0].len() == 3
            && path.degree > 0
            && law
                .control_points
                .iter()
                .all(|p| p.len() == 3 && p[0] > 0. && p[1] == 0. && p[2] == 0.),
        "Profile certificate requires finite 3D inputs",
    )?;
    let mut plane = (0..3).find(|&k| {
        path.control_points
            .iter()
            .all(|p| p[k] == path.control_points[0][k])
    });
    if retained.periodic_v && plane.is_some() {
        let [a, b] = path.domain();
        let clamped = path.knots[..=path.degree].iter().all(|&k| k == a)
            && path.knots[path.control_points.len()..]
                .iter()
                .all(|&k| k == b);
        let p = &path.control_points[0];
        let incoming = &path.control_points[path.control_points.len() - 2];
        let outgoing = &path.control_points[1];
        let aligned = clamped
            && p == path.control_points.last().unwrap()
            && (0..3).any(|axis| {
                (0..3).all(|k| k == axis || incoming[k] == p[k] && outgoing[k] == p[k])
                    && (incoming[axis] < p[axis] && p[axis] < outgoing[axis]
                        || incoming[axis] > p[axis] && p[axis] > outgoing[axis])
            });
        // Without exact endpoint tangent agreement, zero planar holonomy is
        // not an admissible premise. The unit-frame envelope still includes
        // every possible closing correction.
        if !aligned {
            plane = None;
        }
    }
    let method = if plane.is_some() {
        "interval-planar-bishop-frame"
    } else {
        "interval-unit-bishop-frame-envelope"
    };
    let unresolved = |cells, reason| Report {
        error_upper: None,
        cells,
        within_budget: false,
        method,
        reason: Some(reason),
    };
    if max_cells == 0 {
        return Ok(unresolved(0, "cell-budget-exhausted"));
    }
    for curve in [law] {
        let [a, b] = curve.domain();
        if curve.knots.iter().any(|&k| {
            k > a && k < b && curve.knots.iter().filter(|&&v| v == k).count() > curve.degree
        }) {
            return Ok(unresolved(0, "continuous-scale-unproved"));
        }
    }
    if retained.knots_v.iter().any(|&k| {
        k > 0. && k < 1. && retained.knots_v.iter().filter(|&&v| v == k).count() > retained.degree_v
    }) {
        return Ok(unresolved(0, "retained-loft-continuity-unproved"));
    }
    for span in path.degree..path.control_points.len() - 1 {
        let k = path.knots[span + 1];
        if path.knots[span] < k
            && k < path.domain()[1]
            && path.knots.iter().filter(|&&v| v == k).count() >= path.degree
        {
            // A full-multiplicity Bezier join has these authored endpoint
            // controls. Equal transverse coordinates and an equal signed axis
            // direction prove tangent alignment without tolerance snapping.
            let p = &path.control_points[span];
            let a = &path.control_points[span - 1];
            let b = &path.control_points[span + 1];
            let aligned = path.knots.iter().filter(|&&v| v == k).count() == path.degree
                && (0..3).any(|axis| {
                    (0..3).all(|j| j == axis || (a[j] == p[j] && p[j] == b[j]))
                        && ((a[axis] < p[axis] && p[axis] < b[axis])
                            || (a[axis] > p[axis] && p[axis] > b[axis]))
                });
            if !aligned {
                return Ok(unresolved(0, "continuous-path-tangent-unproved"));
            }
        }
    }
    let Ok(t0) = initial_tangent(path) else {
        return Ok(unresolved(0, "initial-frame-unresolved"));
    };
    let Ok(n0) = unit(sub(point(&normal), scale(t0, dot(point(&normal), t0)?)?)?) else {
        return Ok(unresolved(0, "initial-frame-unresolved"));
    };
    let b0 = cross(t0, n0)?;
    let mut cells = 1;
    let mut used = 0;
    let start = values(path, [0., 0.], max_cells - cells, &mut used)?;
    cells += used;
    let Some(start) = start else {
        return Ok(unresolved(cells, "cell-budget-exhausted"));
    };
    let coordinates = profile
        .control_points
        .iter()
        .map(|p| {
            let q = sub(point(p), start)?;
            Ok([dot(q, n0)?, dot(q, b0)?, dot(q, t0)?])
        })
        .collect::<Result<Vec<_>>>()?;
    let reference = Reference {
        path,
        scale: law,
        plane: plane.map(|axis| std::array::from_fn(|k| I::point(if k == axis { 1. } else { 0. }))),
        initial: n0,
        initial_side: b0,
        initial_tangent: t0,
        coordinates,
    };
    // Equal-weight one-span lines and affine laws give an affine ideal sweep.
    // Its distance to each affine retained segment is convex, hence endpoints
    // suffice (interval widths between equal linear values must not dominate).
    let affine =
        |c: &Curve| c.degree == 1 && c.control_points.len() == 2 && c.weights[0] == c.weights[1];
    let affine_reference = linear && affine(path) && affine(law);
    let mut pending: Vec<_> = (retained.degree_v..count)
        .filter(|&i| retained.knots_v[i] < retained.knots_v[i + 1])
        .map(|i| {
            (
                i - retained.degree_v,
                retained.knots_v[i],
                retained.knots_v[i + 1],
                0usize,
            )
        })
        .collect();
    let mut upper = 0_f64;
    while let Some((segment, lo, hi, depth)) = pending.pop() {
        let mut used = 0;
        let taylor = reference.plane.is_some() && !affine_reference;
        let midpoint = lo + (hi - lo) / 2.;
        let range = if affine_reference {
            [lo, lo]
        } else if taylor {
            [midpoint, midpoint]
        } else {
            [lo, hi]
        };
        // Noninitial affine point jets are constant and use an enclosing full
        // span; the target point itself still uses the exact point parameter.
        let target = if affine_reference {
            affine_points(&reference, range[0], max_cells - cells, &mut used)
        } else {
            reference.at(range, max_cells - cells, &mut used)
        };
        let target = match target {
            Ok(value) => value,
            Err(_) => None,
        };
        cells += used;
        let Some(target) = target else {
            if cells >= max_cells || depth >= 24 {
                return Ok(unresolved(cells, "regular-frame-or-budget-unresolved"));
            }
            let mid = lo + (hi - lo) / 2.;
            pending.extend([(segment, lo, mid, depth + 1), (segment, mid, hi, depth + 1)]);
            continue;
        };
        let fraction = if linear {
            I::new(range[0], range[1])?
                .sub(I::point(retained.knots_v[segment + 1]))?
                .div(
                    I::point(retained.knots_v[segment + 2])
                        .sub(I::point(retained.knots_v[segment + 1]))?,
                )?
        } else {
            I::point(0.)
        };
        let mut derivatives = None;
        if taylor {
            let mut used = 0;
            let result = reference.derivative([lo, hi], max_cells - cells, &mut used);
            cells += used;
            derivatives = match result {
                Ok(r) => r,
                Err(_) => None,
            };
            if derivatives.is_none() {
                if depth >= 24 || cells >= max_cells {
                    return Ok(unresolved(cells, "continuous-derivative-unresolved"));
                }
                pending.extend([
                    (segment, lo, midpoint, depth + 1),
                    (segment, midpoint, hi, depth + 1),
                ]);
                continue;
            }
        }
        let mut bound = 0_f64;
        let mut actual_unresolved = false;
        for (index, ((row, curve), ideal)) in retained
            .control_points
            .iter()
            .zip(&retained_curves)
            .zip(target)
            .enumerate()
        {
            let actual = if linear {
                add(
                    point(&row[segment]),
                    scale(
                        sub(point(&row[segment + 1]), point(&row[segment]))?,
                        fraction,
                    )?,
                )?
            } else {
                let mut used = 0;
                let enclosure = retained_values(curve, range, max_cells - cells, &mut used)?;
                cells += used;
                let Some(value) = enclosure else {
                    actual_unresolved = true;
                    break;
                };
                value
            };
            let mut delta = sub(ideal, actual)?;
            if let Some(derivatives) = &derivatives {
                let actual_derivative = if linear {
                    div(
                        sub(point(&row[segment + 1]), point(&row[segment]))?,
                        I::point(retained.knots_v[segment + 2])
                            .sub(I::point(retained.knots_v[segment + 1]))?,
                    )?
                } else {
                    let mut used = 0;
                    let jets = curve_jets(curve, [lo, hi], max_cells - cells, &mut used)?;
                    cells += used;
                    let Some(jets) = jets else {
                        actual_unresolved = true;
                        break;
                    };
                    jets[0]
                };
                let dt = I::new(lo, hi)?.sub(I::point(midpoint))?;
                delta = add(
                    delta,
                    scale(sub(derivatives[index], actual_derivative)?, dt)?,
                )?;
            }
            bound = bound.max(norm(delta)?.hi);
        }
        if actual_unresolved {
            return Ok(unresolved(cells, "retained-surface-work-budget-exhausted"));
        }
        if affine_reference {
            // Also include the final endpoint of the last segment.
            if segment == count - 2 {
                let mut used = 0;
                let Some(end) = affine_points(&reference, 1., max_cells - cells, &mut used)? else {
                    return Ok(unresolved(cells + used, "cell-budget-exhausted"));
                };
                cells += used;
                for (row, ideal) in retained.control_points.iter().zip(end) {
                    bound = bound.max(norm(sub(ideal, point(row.last().unwrap()))?)?.hi);
                }
            }
            upper = upper.max(bound);
        } else if bound <= budget {
            upper = upper.max(bound);
        } else {
            if depth >= 24 || cells >= max_cells {
                return Ok(unresolved(cells, "deviation-or-work-budget-exceeded"));
            }
            let mid = lo + (hi - lo) / 2.;
            pending.extend([(segment, lo, mid, depth + 1), (segment, mid, hi, depth + 1)]);
        }
    }
    Ok(Report {
        error_upper: Some(upper),
        cells,
        within_budget: upper <= budget,
        method,
        reason: if upper <= budget {
            None
        } else {
            Some("continuous-deviation-budget-exceeded")
        },
    })
}
fn affine_points(
    r: &Reference<'_>,
    t: f64,
    remaining: usize,
    used: &mut usize,
) -> Result<Option<Vec<V>>> {
    let Some(position) = values(r.path, [t, t], remaining, used)? else {
        return Ok(None);
    };
    if remaining.saturating_sub(*used) < 1 {
        return Ok(None);
    }
    let s = scalar::value_traversal(r.scale, [t, t], 1)?;
    *used += 1;
    let Some(s) = s else {
        return Ok(None);
    };
    r.coordinates
        .iter()
        .map(|q| {
            let offset = add(
                add(scale(r.initial, q[0])?, scale(r.initial_side, q[1])?)?,
                scale(r.initial_tangent, q[2])?,
            )?;
            add(position, scale(offset, I::new(s[0], s[1])?)?)
        })
        .collect::<Result<Vec<_>>>()
        .map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sweeps::progressive_sweep::{
        Options, Orientation, Spacing, Sweep, constant_vector_law,
    };
    fn curve(p: &[[f64; 3]], w: &[f64], d: [f64; 2]) -> Curve {
        Curve {
            degree: p.len() - 1,
            knots: [vec![d[0]; p.len()], vec![d[1]; p.len()]].concat(),
            control_points: p.iter().map(|p| p.to_vec()).collect(),
            weights: w.to_vec(),
            periodic: false,
        }
    }
    fn retained(p: &Curve, path: &Curve, s: &Curve, n: [f64; 3], count: usize) -> Surface {
        let twist = constant_vector_law([0.; 3]).unwrap();
        let sweep = Sweep::new(
            p,
            path,
            s,
            &twist,
            Options {
                normal: n,
                orientation: Orientation::RotationMinimizing,
                spacing: Spacing::Parameter,
                initial_sections: count,
                max_sections: count,
                max_deviation: 1.,
            },
        )
        .unwrap();
        let mut result = crate::surface::loft(&sweep.sections_at(count).unwrap()).unwrap();
        for k in &mut result.knots_v {
            *k /= (count - 1) as f64;
        }
        result
    }
    #[test]
    fn affine_rational_profile_has_rounding_inclusive_continuous_bound() {
        let p = curve(
            &[[1., 2., 0.], [2., 3., 1.], [4., 1., 0.]],
            &[1., 0.25, 3.],
            [2., 7.],
        );
        let path = curve(&[[0., 0., 0.], [0., 0., 5.]], &[2., 2.], [-4., 9.]);
        let s = curve(&[[1., 0., 0.], [2., 0., 0.]], &[3., 3.], [10., 12.]);
        let surface = retained(&p, &path, &s, [1., 0., 0.], 6);
        let r = certify(&p, &path, &s, [1., 0., 0.], &surface, 1e-11, 1000).unwrap();
        assert!(r.within_budget, "{r:?}");
        assert!(r.error_upper.unwrap() > 0. && r.error_upper.unwrap() < 1e-11);
        for u in [2., 2.7, 4.2, 7.] {
            for v in [0., 0.13, 0.77, 1.] {
                let source = p.evaluate(u).unwrap().point;
                let actual = surface.evaluate(u, v).unwrap().point;
                let expected = [
                    (1. + v) * source[0],
                    (1. + v) * source[1],
                    5. * v + (1. + v) * source[2],
                ];
                let error = (0..3)
                    .map(|k| (actual[k] - expected[k]).powi(2))
                    .sum::<f64>()
                    .sqrt();
                assert!(error <= r.error_upper.unwrap());
            }
        }
    }
    #[test]
    fn budgets_and_changed_retained_geometry_never_publish_partial_bounds() {
        let p = curve(&[[1., 0., 0.], [2., 0., 0.]], &[1., 1.], [0., 1.]);
        let path = curve(&[[0., 0., 0.], [0., 0., 5.]], &[1., 1.], [0., 1.]);
        let s = constant_vector_law([1., 0., 0.]).unwrap();
        let surface = retained(&p, &path, &s, [1., 0., 0.], 5);
        for limit in [0, 1, 3] {
            let r = certify(&p, &path, &s, [1., 0., 0.], &surface, 1e-9, limit).unwrap();
            assert!(!r.within_budget && r.error_upper.is_none());
            assert!(r.cells <= limit);
        }
        let mut changed = surface;
        changed.control_points[0][2][0] += 1.;
        let r = certify(&p, &path, &s, [1., 0., 0.], &changed, 0.01, 1000).unwrap();
        assert!(!r.within_budget && r.error_upper.unwrap() >= 1.);
    }
    #[test]
    fn planar_curved_source_is_bounded_between_stations() {
        let p = curve(&[[1., 0., 0.], [2., 0., 0.]], &[1., 2.], [0., 1.]);
        let path = curve(
            &[[0., 0., 0.], [0., 0.08, 2.5], [0., 0., 5.]],
            &[1., 1., 1.],
            [-2., 3.],
        );
        let s = constant_vector_law([1., 0., 0.]).unwrap();
        let surface = retained(&p, &path, &s, [1., 0., 0.], 9);
        let r = certify(&p, &path, &s, [1., 0., 0.], &surface, 0.01, 10000).unwrap();
        assert!(r.within_budget, "{r:?}");
        for i in 0..=200 {
            let v = i as f64 / 200.;
            let e = path.evaluate(-2. + 5. * v).unwrap();
            let q = surface.evaluate(0.37, v).unwrap().point;
            // X is a constant exact Bishop normal; the weighted profile is
            // independent of path curvature, so this oracle needs no frame code.
            let x = p.evaluate(0.37).unwrap().point[0];
            let d = [q[0] - x, q[1] - e.point[1], q[2] - e.point[2]];
            assert!(d.iter().map(|x| x * x).sum::<f64>().sqrt() <= r.error_upper.unwrap());
        }
    }
    #[test]
    fn hidden_stationary_tangent_is_unresolved() {
        let p = curve(&[[1., 0., 0.], [2., 0., 0.]], &[1., 1.], [0., 1.]);
        let path = curve(
            &[[0., 0., 0.], [0., 0., 1.], [0., 0., -1.], [0., 0., 0.]],
            &[1.; 4],
            [0., 1.],
        );
        let s = constant_vector_law([1., 0., 0.]).unwrap();
        let mut surface = crate::surface::extrude(&p, [0., 0., 1.]).unwrap();
        surface.knots_v = vec![0., 0., 1., 1.];
        let r = certify(&p, &path, &s, [1., 0., 0.], &surface, 100., 100).unwrap();
        assert!(!r.within_budget && r.error_upper.is_none());
    }
}
