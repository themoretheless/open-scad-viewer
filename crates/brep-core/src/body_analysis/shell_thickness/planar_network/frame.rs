//! Exact common original plane, coherent normal orientation and continuous
//! positive-rational control-hull bound on rounded normal-offset translation.
use super::*;
use num_rational::BigRational as Q;
use num_traits::{ToPrimitive, Zero};
pub(in crate::body_analysis::shell_thickness) struct Offset {
    pub patches: Vec<Surface>,
    pub error_upper: f64,
    pub unit_normal: [I; 3],
}
fn q(v: f64) -> Q {
    Q::from_float(v).unwrap()
}
fn enclosure(v: &Q) -> Result<I> {
    let x = v
        .to_f64()
        .filter(|v| v.is_finite())
        .ok_or_else(|| refused("Original plane scalar overflow"))?;
    let r = q(x);
    I::new(
        if r > *v { x.next_down() } else { x },
        if r < *v { x.next_up() } else { x },
    )
}
fn cross(a: &[Q; 3], b: &[Q; 3]) -> [Q; 3] {
    std::array::from_fn(|k| &a[(k + 1) % 3] * &b[(k + 2) % 3] - &a[(k + 2) % 3] * &b[(k + 1) % 3])
}
fn dot(a: &[Q; 3], b: &[Q; 3]) -> Q {
    (0..3).map(|k| &a[k] * &b[k]).sum()
}
fn delta(a: &[f64], b: &[f64]) -> [Q; 3] {
    std::array::from_fn(|k| q(a[k]) - q(b[k]))
}
fn normal(s: &Surface) -> [Q; 3] {
    cross(
        &delta(&s.control_points[1][0], &s.control_points[0][0]),
        &delta(&s.control_points[0][1], &s.control_points[0][0]),
    )
}
fn spend(cells: &mut usize, max: usize) -> Result<()> {
    if *cells == max {
        return Err(refused("Original planar offset proof allowance exhausted"));
    }
    *cells += 1;
    Ok(())
}
pub(in crate::body_analysis::shell_thickness) fn offset(
    sources: &[Surface],
    reversed: &[bool],
    thickness: f64,
    tolerance: f64,
    max: usize,
    cells: &mut usize,
) -> Result<Offset> {
    let mut n = normal(&sources[0]);
    if reversed[0] {
        for v in &mut n {
            *v = -v.clone();
        }
    }
    let scale = n
        .iter()
        .map(|v| {
            if v < &Q::zero() {
                -v.clone()
            } else {
                v.clone()
            }
        })
        .max()
        .unwrap();
    if scale.is_zero() {
        return Err(refused("Original source corner has no oriented plane"));
    }
    let origin = &sources[0].control_points[0][0];
    for (s, &reversed) in sources.iter().zip(reversed) {
        spend(cells, max)?;
        let sign = dot(&n, &normal(s));
        if (!reversed && sign <= Q::zero()) || (reversed && sign >= Q::zero()) {
            return Err(refused(
                "Original patch normals are not coherently oriented",
            ));
        }
        for p in s.control_points.iter().flatten() {
            spend(cells, max)?;
            if !dot(&n, &delta(p, origin)).is_zero() {
                return Err(refused(
                    "Original source controls do not share one exact plane",
                ));
            }
        }
    }
    let scaled = n.map(|v| v / &scale);
    let mut ni = [I::point(0.); 3];
    let mut sum = I::point(0.);
    let mut proposed = [0.; 3];
    for k in 0..3 {
        ni[k] = enclosure(&scaled[k])?;
        sum = sum.add(ni[k].mul(ni[k])?)?;
        proposed[k] = scaled[k].to_f64().unwrap();
    }
    if sum.lo <= 0. {
        return Err(refused("Original plane norm remains unproven"));
    }
    // sqrt only proposes scalar endpoints. Exact rational square comparisons
    // certify the enclosing norm, independently of library sqrt accuracy.
    let mut lo = sum.lo.sqrt();
    let mut hi = sum.hi.sqrt();
    for _ in 0..4 {
        if &q(lo) * &q(lo) > q(sum.lo) {
            lo = lo.next_down();
        } else {
            break;
        }
    }
    for _ in 0..4 {
        if &q(hi) * &q(hi) < q(sum.hi) {
            hi = hi.next_up();
        } else {
            break;
        }
    }
    if !lo.is_finite()
        || !hi.is_finite()
        || lo <= 0.
        || &q(lo) * &q(lo) > q(sum.lo)
        || &q(hi) * &q(hi) < q(sum.hi)
    {
        return Err(refused("Original plane norm enclosure remains unproven"));
    }
    let length = I::new(lo, hi)?;
    let estimate = proposed.iter().map(|x| x * x).sum::<f64>().sqrt();
    let mut ideal = [I::point(0.); 3];
    for k in 0..3 {
        ideal[k] = ni[k].div(length)?.mul(I::point(thickness))?;
        proposed[k] = proposed[k] / estimate * thickness;
    }
    let mut patches = sources.to_vec();
    let mut maximum = [0f64; 3];
    for (source, target) in sources.iter().zip(&mut patches) {
        for (old, new) in source
            .control_points
            .iter()
            .flatten()
            .zip(target.control_points.iter_mut().flatten())
        {
            spend(cells, max)?;
            for k in 0..3 {
                new[k] = old[k] + proposed[k];
                if !new[k].is_finite() || new[k].abs() > 1e6 {
                    return Err(refused("Actual planar offset coordinates exceed bounds"));
                }
                let actual = enclosure(&(q(new[k]) - q(old[k])))?;
                let e = actual.sub(ideal[k])?;
                maximum[k] = maximum[k].max(e.lo.abs().max(e.hi.abs()));
            }
        }
    }
    let mut error = I::point(0.);
    for e in maximum {
        error = error.add(I::point(e))?;
    }
    if error.hi > tolerance {
        return Err(refused(
            "Whole original normal-offset error exceeds tolerance",
        ));
    }
    if error.hi >= thickness.abs() {
        return Err(refused(
            "Positive original/offset cap separation remains unproven",
        ));
    }
    Ok(Offset {
        patches,
        error_upper: error.hi,
        unit_normal: [ni[0].div(length)?, ni[1].div(length)?, ni[2].div(length)?],
    })
}
