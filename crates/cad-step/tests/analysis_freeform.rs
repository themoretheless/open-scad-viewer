//! Freeform tessellation and mass bounds on the NURBS STEP solid fixtures.
use brep_core::analysis::{
    CertifiedInterval, FREEFORM_MASS_CAPABILITY, certified_freeform_mass_properties,
    certified_freeform_tessellation_deviation, certified_mass_properties,
    certified_tessellation_deviation,
};

fn contains(interval: CertifiedInterval, value: f64) {
    assert!(
        interval.lower <= value && value <= interval.upper,
        "{interval:?} excludes {value}"
    );
}
#[test]
fn freeform_tessellation_bounds_planar_cuboid_and_bump() {
    let planar = cad_step::freeform_cuboid_solid([0., 0., 0.], [2., 3., 4.]).unwrap();
    assert_eq!(
        certified_tessellation_deviation(&planar, 4)
            .unwrap_err()
            .code,
        "BREP_CERTIFIED_TESSELLATION_REFUSED"
    );
    let planar_dev = certified_freeform_tessellation_deviation(&planar, 1).unwrap();
    assert!(
        planar_dev <= 1e-12,
        "planar elevated cuboid should be zero-bound"
    );

    let bump = cad_step::freeform_cuboid_with_bump_face([0., 0., 0.], [2., 2., 2.]).unwrap();
    let coarse = certified_freeform_tessellation_deviation(&bump, 1).unwrap();
    let fine = certified_freeform_tessellation_deviation(&bump, 4).unwrap();
    assert!(coarse > 0., "bump needs a positive Bernstein bound");
    assert!(fine < coarse, "bound must tighten with subdivisions");
    assert!(fine <= coarse / 15., "1/n² scaling should dominate");

    let mut unequal = bump.clone();
    unequal.faces[1].surface.weights[1][1] = 2.;
    assert_eq!(
        certified_freeform_tessellation_deviation(&unequal, 4)
            .unwrap_err()
            .code,
        "BREP_CERTIFIED_TESSELLATION_REFUSED"
    );
}

#[test]
fn freeform_mass_encloses_planar_cuboid_and_refuses_bump() {
    let planar = cad_step::freeform_cuboid_solid([1., 2., 3.], [3., 6., 9.]).unwrap();
    assert_eq!(
        certified_mass_properties(&planar).unwrap_err().code,
        "BREP_CERTIFIED_MASS_REFUSED"
    );
    let mass = certified_freeform_mass_properties(&planar).unwrap();
    assert_eq!(mass.capability, FREEFORM_MASS_CAPABILITY);
    contains(mass.volume_mm3, 48.);
    contains(mass.surface_area_mm2, 88.);
    assert!(mass.audit.ok && mass.naming_complete);

    let bump = cad_step::freeform_cuboid_with_bump_face([0., 0., 0.], [2., 2., 2.]).unwrap();
    assert_eq!(
        certified_freeform_mass_properties(&bump).unwrap_err().code,
        "BREP_CERTIFIED_MASS_REFUSED"
    );
}
