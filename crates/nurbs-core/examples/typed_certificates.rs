use nurbs_core::{
    Result,
    curve::Curve,
    foundation::{
        certificates::SurfaceRegularityClass, certify_curve_report, certify_surface_report,
    },
    surface::Surface,
};

fn main() -> Result<()> {
    let curve = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]])?;
    let certificate = certify_curve_report(&curve, None)?;
    assert_eq!(certificate.spans.len(), 1);
    assert!(certificate.spans[0].regularity.certified());
    assert!(certificate.spans[0].denominator[0] > 0.);
    let surface = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        ],
        weights: vec![vec![1., 1.], vec![1., 1.]],
        periodic_u: false,
        periodic_v: false,
    };
    let certificate = certify_surface_report(&surface, None)?;
    assert_eq!(certificate.cells.len(), 1);
    assert_eq!(
        certificate.cells[0].normal_regularity.classification,
        SurfaceRegularityClass::PlanarRegular
    );
    assert!(certificate.singularity_localization.complete());
    println!("Regular curve span and planar surface cell certified directly in Rust");
    Ok(())
}
