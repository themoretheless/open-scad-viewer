use brep_core::gear::{GearSpec, ProfileEmbeddingLimits, gear_from_verified_profile,
    profile_with_report, replace_verified_gear_sector};
use nurbs_core::involute::LosslessSectorPattern;

#[test]
fn step_keeps_the_edited_flank_of_the_full_spinner_sun() {
    let spec = GearSpec { teeth:32, module:1.5, clearance:0.1, bore:5., height:5.,
        helix_angle_deg:-35., herringbone:true, ..GearSpec::default() };
    let (loops,_) = profile_with_report(&spec).unwrap();
    let mut packed = LosslessSectorPattern::encode(&loops[0],6).unwrap();
    let mut tooth = packed.sector(31).unwrap();
    let flank = tooth.iter().position(|c| c.degree > 2).unwrap();
    let old = tooth[flank].control_points[1][0];
    tooth[flank].control_points[1][0] += 1e-8;
    let expected = tooth[flank].control_points[1].clone();
    let profile = replace_verified_gear_sector(&mut packed,31,&tooth,false,Some(&loops[1]),
        1e-7,ProfileEmbeddingLimits::default()).unwrap();
    let model = gear_from_verified_profile(&profile,&spec).unwrap();
    let (step,..) = cad_step::export_step_v9(&model).unwrap();
    let (restored,..) = cad_step::import_step_v9(&step).unwrap();
    assert_eq!(restored.bodies.len(),1);
    assert_eq!(restored.validate().unwrap().boundary_edge_count,0);
    let has_edit = restored.edges.iter().any(|edge| edge.curve.control_points.iter().any(|p|
        p.len()==3 && p[2].abs()<1e-12 && (p[0]-expected[0]).abs()<1e-12 &&
        (p[1]-expected[1]).abs()<1e-12 && (p[0]-old).abs()>9e-9));
    assert!(has_edit);
    eprintln!("edited spinner sun STEP: {} bytes, {} faces",step.len(),restored.faces.len());
}
