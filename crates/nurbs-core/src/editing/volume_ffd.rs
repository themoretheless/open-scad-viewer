//! Volume-preserving free-form deformation (item 859).
//!
//! The user moves controls of an [`FfdLattice`]; the deformation is applied to
//! the control points of every patch of a closed shell, as
//! [`FfdLattice::apply_to_surface`] does. The volume change is then removed
//! with a one-parameter correction of the lattice: every control is moved by
//! `mu · (rest - centre)`, where `rest` is its identity (Greville) position and
//! `centre` the lattice box centre. Because the lattice reproduces affine maps,
//! this adds `mu · (x - centre)` to the deformation of every point, a uniform
//! inflation in the undeformed frame that keeps the user's shape edit.
//!
//! `mu` is found by a secant iteration on the certified enclosed volume of
//! [`crate::analysis::surface_volume`]. The reported `volume_change_upper` is
//! a certified bound on `|V(result) - V(input)|` for the returned surfaces
//! (the control-point FFD result, not the exact composition `F∘S`).
use crate::analysis::surface_volume::{VolumeReport, enclosed_volume};
use crate::editing::deform::FfdLattice;
use crate::{Result, check, numeric, surface::Surface};

#[derive(Clone, Debug)]
pub struct VolumePreservingFfd {
    pub surfaces: Vec<Surface>,
    /// The corrected lattice actually applied.
    pub lattice: FfdLattice,
    /// Inflation coefficient `mu` of the correction.
    pub correction: f64,
    pub original: VolumeReport,
    pub deformed: VolumeReport,
    /// Certified `|V(result) - V(input)|` upper bound.
    pub volume_change_upper: f64,
    pub within_tolerance: bool,
    pub iterations: usize,
}

fn corrected(lattice: &FfdLattice, rest: &FfdLattice, mu: f64) -> Result<FfdLattice> {
    let (min, max) = lattice.bounds();
    let centre: [f64; 3] = std::array::from_fn(|k| min[k] + (max[k] - min[k]) / 2.);
    let mut out = lattice.clone();
    let [a, b, c] = lattice.counts();
    for slot in 0..a * b * c {
        let (p, r) = (lattice.control(slot)?, rest.control(slot)?);
        out.set_control(slot, std::array::from_fn(|k| p[k] + mu * (r[k] - centre[k])))?;
    }
    Ok(out)
}

fn measure(
    surfaces: &[Surface],
    lattice: &FfdLattice,
    tolerance: f64,
    max_cells: usize,
) -> Result<(Vec<Surface>, VolumeReport)> {
    let deformed = surfaces.iter().map(|s| lattice.apply_to_surface(s)).collect::<Result<Vec<_>>>()?;
    let volume = enclosed_volume(&deformed, tolerance, max_cells)?;
    Ok((deformed, volume))
}

/// Apply `lattice` to the closed shell `surfaces` and correct the result so
/// its enclosed volume matches the input within `tolerance` (cubic units).
/// `max_cells` is the per-measurement budget of [`enclosed_volume`].
pub fn deform_preserving_volume(
    surfaces: &[Surface],
    lattice: &FfdLattice,
    tolerance: f64,
    max_cells: usize,
    max_iterations: usize,
) -> Result<VolumePreservingFfd> {
    check(tolerance.is_finite() && tolerance > 0., "Volume-preserving FFD tolerance must be positive")?;
    check((1..=64).contains(&max_iterations), "Volume-preserving FFD iterations must be 1..64")?;
    let (min, max) = lattice.bounds();
    let degrees = lattice.degrees();
    let counts = lattice.counts();
    let rest = FfdLattice::new(min, max, degrees, std::array::from_fn(|k| counts[k] - degrees[k]))?;
    let measure_tolerance = tolerance / 4.;
    let original = enclosed_volume(surfaces, measure_tolerance, max_cells)?;
    check(original.within_tolerance, "Input volume could not be certified within the budget")?;
    let target = original.value;
    check(
        original.bounds[0] > 0. || original.bounds[1] < 0.,
        "Input shell encloses no certified nonzero volume",
    )?;
    let finish = |mu: f64, iterations: usize, (surfaces, deformed): (Vec<Surface>, VolumeReport), lattice| {
        let volume_change_upper = ((deformed.value - target).abs() + deformed.error_upper + original.error_upper).next_up();
        VolumePreservingFfd {
            surfaces,
            lattice,
            correction: mu,
            original: original.clone(),
            within_tolerance: deformed.within_tolerance && volume_change_upper <= tolerance,
            deformed,
            volume_change_upper,
            iterations,
        }
    };
    let mut a = (0., measure(surfaces, lattice, measure_tolerance, max_cells)?);
    let ra = a.1.1.value - target;
    let done = |r: &VolumeReport| r.within_tolerance && (r.value - target).abs() <= tolerance / 2.;
    if done(&a.1.1) {
        return Ok(finish(0., 0, a.1, lattice.clone()));
    }
    let ratio = target / a.1.1.value;
    numeric(ratio.is_finite() && ratio > 0., "Deformed shell flipped or collapsed its volume")?;
    // Exact if the edit were a pure scaling about the centre.
    let mut b_mu = ratio.cbrt() - 1.;
    let mut ra = ra;
    for iteration in 1..=max_iterations {
        let lb = corrected(lattice, &rest, b_mu)?;
        let b = measure(surfaces, &lb, measure_tolerance, max_cells)?;
        let rb = b.1.value - target;
        if done(&b.1) || iteration == max_iterations {
            return Ok(finish(b_mu, iteration, b, lb));
        }
        let slope = (rb - ra) / (b_mu - a.0);
        numeric(slope.is_finite() && slope != 0., "Volume correction stalled")?;
        let next = b_mu - rb / slope;
        a = (b_mu, b);
        ra = rb;
        b_mu = next;
    }
    unreachable!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::surface_volume::test_cuboid;

    #[test]
    fn pulling_a_face_keeps_the_cuboid_volume() {
        let shell = test_cuboid(1., 1., 1.);
        let mut lattice = FfdLattice::new([-0.25; 3], [1.25; 3], [2, 2, 2], [1, 1, 1]).unwrap();
        // Pull the top-centre control upward: a dome on the +z face.
        let slot = lattice.control_index(1, 1, 2).unwrap();
        let p = lattice.control(slot).unwrap();
        lattice.set_control(slot, [p[0], p[1], p[2] + 0.6]).unwrap();
        let plain: Vec<_> = shell.iter().map(|s| lattice.apply_to_surface(s).unwrap()).collect();
        let grown = enclosed_volume(&plain, 1e-4, 20_000).unwrap();
        assert!(grown.within_tolerance && grown.bounds[0] > 1.04, "the edit must change the volume: {grown:?}");

        let tolerance = 1e-3;
        let r = deform_preserving_volume(&shell, &lattice, tolerance, 20_000, 16).unwrap();
        assert!(r.within_tolerance, "{r:?}");
        assert!(r.volume_change_upper <= tolerance);
        assert!(r.deformed.bounds[0] <= 1. + tolerance && 1. - tolerance <= r.deformed.bounds[1]);
        assert!(r.correction < 0.);
        // The shape edit survives: the top face centre is still raised.
        let top = r.surfaces[1].evaluate(0.5, 0.5).unwrap().point;
        let corner = r.surfaces[1].evaluate(0., 0.).unwrap().point;
        assert!(top[2] > corner[2] + 0.02, "{top:?} {corner:?}");
    }

    #[test]
    fn identity_lattice_needs_no_correction() {
        let shell = test_cuboid(1., 2., 1.);
        let lattice = FfdLattice::new([-0.5; 3], [2.5; 3], [2, 2, 2], [2, 1, 1]).unwrap();
        let r = deform_preserving_volume(&shell, &lattice, 1e-6, 5_000, 4).unwrap();
        assert!(r.within_tolerance && r.correction == 0. && r.iterations == 0, "{r:?}");
        assert!(deform_preserving_volume(&shell, &lattice, 0., 5_000, 4).is_err());
    }
}
