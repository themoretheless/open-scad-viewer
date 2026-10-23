//! Proposal solve for radii authored at two world-coordinate stations.
//! Samples guide construction; original contact and whole-skin proof remain required.
use super::*;
pub struct StationRadiusReport {
    pub fillet: FilletReport,
    pub normalized_law: RadiusLaw,
    pub radius_field: [f64; 4],
    /// Outward representation error at the two authored coordinate stations.
    pub authored_endpoint_error_upper_mm: f64,
    pub iterations: usize,
    pub station_residual_estimate_mm: f64,
    pub converged: bool,
}
/// Fit the normalized tracing law to r(s0)=r0, r(s1)=r1 for one world coordinate.
/// At most eight skin proposals share one reference trace with the supplied
/// marching options.
/// This is a proposal solver, not a certificate; nonlinear/reversing traces may
/// remain unconverged. The authored world field is never fitted to the result.
pub fn linear_at_stations(
    a: &Surface,
    b: &Surface,
    coordinate: usize,
    stations: [f64; 2],
    radii: [f64; 2],
    options: &FilletOptions,
) -> Result<StationRadiusReport> {
    check(
        coordinate < 3
            && stations.iter().all(|x| x.is_finite())
            && stations[0] < stations[1]
            && radii.iter().all(|r| r.is_finite() && *r > 0.),
        "Need finite ordered stations and positive endpoint radii",
    )?;
    let slope = (radii[1] - radii[0]) / (stations[1] - stations[0]);
    let offset = radii[0] - slope * stations[0];
    check(
        slope.is_finite() && offset.is_finite(),
        "Station radius field overflows",
    )?;
    let mut field = [0.; 4];
    field[coordinate] = slope;
    field[3] = offset;
    use crate::interval_eval::Interval as I;
    let mut endpoint_error = 0_f64;
    for i in 0..2 {
        let e = I::point(slope)
            .mul(I::point(stations[i]))?
            .add(I::point(offset))?
            .sub(I::point(radii[i]))?;
        endpoint_error = endpoint_error.max(e.lo.abs().max(e.hi.abs()));
    }
    let desired = |p: [f64; 3]| slope * p[coordinate] + offset;
    let mut law = RadiusLaw::Linear {
        start: radii[0],
        end: radii[1],
    };
    // Keep one reference trace. Retracing after each endpoint update makes
    // adaptive endpoint clipping change the parameter domain being solved.
    let (marcher, sa, sb) = prepare(a, b, options)?;
    let (raw, seed, closed) = trace_spine(
        &marcher,
        radii[0] * 0.5 + radii[1] * 0.5,
        options.max_march_steps,
    )?;
    check(
        seed.is_some(),
        "Station radius reference trace could not be built",
    )?;
    for iteration in 1..=8 {
        let rmax = match law {
            RadiusLaw::Linear { start, end } => start.max(end),
            _ => unreachable!(),
        };
        let fillet = build_report(
            &marcher,
            &raw,
            &law,
            options,
            reachability_warnings(a, b, sa, sb, rmax),
            closed,
            SectionKind::Arc,
        )?;
        let residual = fillet
            .spine
            .iter()
            .map(|s| (s.radius - desired(s.center)).abs())
            .fold(0_f64, f64::max);
        let converged = fillet.failure.is_none()
            && !fillet.spine.is_empty()
            && residual <= options.tolerance * 0.1;
        if converged || fillet.failure.is_some() || iteration == 8 {
            return Ok(StationRadiusReport {
                fillet,
                normalized_law: law,
                radius_field: field,
                authored_endpoint_error_upper_mm: endpoint_error,
                iterations: iteration,
                station_residual_estimate_mm: residual,
                converged,
            });
        }
        let start = desired(fillet.spine.first().unwrap().center);
        let end = desired(fillet.spine.last().unwrap().center);
        check(
            start.is_finite() && end.is_finite() && start > 0. && end > 0.,
            "Station law extrapolates to nonpositive traced radii",
        )?;
        law = RadiusLaw::Linear { start, end };
    }
    unreachable!("bounded proposal loop returns on its final iteration")
}
