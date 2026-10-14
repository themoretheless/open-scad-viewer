use nurbs_core::{
    Result, paths, primitives,
    progressive_miter::{Options, Sweep},
};
fn main() -> Result<()> {
    let profiles = [primitives::line([0., 0.1, 0.2], [0., 0.2, 0.3])?];
    let points = [
        [0., 0., 0.],
        [10., 0., 0.],
        [10., 10., 4.],
        [0., 10., 1.],
        [0., 5., -2.],
    ];
    let scale = paths::bezier(
        vec![vec![1., 0., 0.], vec![1.5, 0., 0.], vec![1., 0., 0.]],
        None,
    )?;
    let twist = paths::bezier(vec![vec![0.; 3], vec![std::f64::consts::TAU, 0., 0.]], None)?;
    let sweep = Sweep::new(
        &profiles,
        &points,
        &scale,
        &twist,
        Options {
            normal: [0., 0., 1.],
            closed: true,
            miter_limit: 4.,
            initial_steps: 1,
            max_steps: 64,
            max_deviation: 1e-3,
        },
    )?;
    let mut accepted = false;
    for level in sweep {
        let level = level?;
        let r = &level.report;
        println!(
            "steps={} sections={} sampled_deviation={} continuous_error_upper={} phase_resolved={} holonomy_radians={} accepted={}",
            r.steps,
            r.sections,
            r.sampled_control_deviation,
            r.continuous_error_upper,
            r.phase_resolved,
            r.holonomy_correction,
            r.accepted
        );
        if r.accepted {
            assert_eq!(level.sections.first(), level.sections.last());
            accepted = true;
        }
    }
    assert!(accepted, "The continuous interpolation estimate budget was not reached");
    Ok(())
}
