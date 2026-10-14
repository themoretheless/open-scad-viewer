fn main() -> nurbs_core::Result<()> {
    use nurbs_core::{
        curve_differential::{self as differential, Side, Status},
        paths,
    };
    let curve = paths::bezier(
        vec![
            vec![0., 0., 0.],
            vec![1., 0., 0.],
            vec![2., 1., 0.],
            vec![3., 3., 1.],
        ],
        None,
    )?;
    let r = differential::at(&curve, 0.5, Side::Automatic)?;
    assert_eq!(r.curvature_status, Status::Available);
    assert_eq!(r.torsion_status, Status::Available);
    println!("cubic curvature {:?}, torsion {:?}", r.curvature, r.torsion);
    let circle = nurbs_core::primitives::circle([0.; 3], [0., 0., 1.], 4.)?;
    let r = differential::at(&circle, 0.25, Side::Right)?;
    assert_eq!(r.curvature_status, Status::Available);
    println!(
        "circle curvature {:?}; one-sided binormal {:?}",
        r.curvature, r.binormal
    );
    Ok(())
}
