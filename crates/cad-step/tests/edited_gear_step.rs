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

#[test]
fn shared_step_points_reduce_spinner_size_and_preserve_authored_geometry() {
    let spec = GearSpec { teeth:32, module:1.5, clearance:0.1, bore:5., height:5.,
        helix_angle_deg:-35., herringbone:true, ..GearSpec::default() };
    let (loops,_) = profile_with_report(&spec).unwrap();
    let packed = LosslessSectorPattern::encode(&loops[0],6).unwrap();
    let rebuilt = (0..32).map(|i| packed.sector(i).unwrap()).flatten().collect();
    let restored_loops = vec![rebuilt,loops[1].clone()];
    let (_,profile) = brep_core::gear::certify_grouped_profile_embedding(
        &restored_loops,32,6,0,1e-7,ProfileEmbeddingLimits::default()).unwrap();
    let rebuilt_body = gear_from_verified_profile(&profile,&spec).unwrap();
    let original = brep_core::gear::gear(&spec).unwrap();
    let (baseline,..) = cad_step::export_step_v9(&original).unwrap();
    let (expanded,..) = cad_step::export_step_v9(&rebuilt_body).unwrap();
    assert_eq!(baseline.as_bytes(),expanded.as_bytes());
    let (compact,..) = cad_step::export_step_v9_shared_points(&rebuilt_body).unwrap();
    assert!(compact.len() < expanded.len());
    let (a,..) = cad_step::import_step_v9(&expanded).unwrap();
    let (b,..) = cad_step::import_step_v9(&compact).unwrap();
    assert_eq!(a.edges.len(),b.edges.len());
    for (left,right) in a.edges.iter().zip(&b.edges) {
        assert_eq!(left.curve.control_points,right.curve.control_points);
        assert_eq!(left.curve.weights,right.curve.weights);
        assert_eq!(left.curve.knots,right.curve.knots);
        assert_eq!(left.vertices,right.vertices);
    }
    assert_eq!(a.faces.len(),b.faces.len());
    for (left,right) in a.faces.iter().zip(&b.faces) {
        assert_eq!(left.surface.control_points,right.surface.control_points);
        assert_eq!(left.surface.weights,right.surface.weights);
        assert_eq!(left.surface.knots_u,right.surface.knots_u);
        assert_eq!(left.surface.knots_v,right.surface.knots_v);
    }
    assert_eq!(b.bodies.len(),1);
    assert_eq!(b.validate().unwrap().boundary_edge_count,0);
    eprintln!("spinner STEP bytes: original {}, reconstructed {}, shared {}",baseline.len(),expanded.len(),compact.len());
}

#[test]
fn compact_gear_step_keeps_face_assignments_and_product_material() {
    use cad_step::step_interchange_v3::*;
    let model = brep_core::gear::gear(&GearSpec { teeth: 8, ..GearSpec::default() }).unwrap();
    let mut meta = StepSemanticMetadata::empty(&model);
    meta.xde.products.insert(42, StepProductMetadata { entity:42, id:"gear8".into(),
        name:"Шестерня".into(), material:Some("Сталь".into()), ..Default::default() });
    meta.xde.body_products[0] = Some(42);
    meta.presentation.body_products[0] = Some(42);
    let marked = model.faces.len()-1;
    let style = StepPresentationStyle { colour:Some(StepColour::new(0.2,0.4,0.8).unwrap()),
        layers:vec!["Контроль".into()], ..Default::default() };
    meta.presentation.face_styles[marked] = Some(style.clone());
    let (normal,..) = export_step_v9_with_metadata(&model,&meta).unwrap();
    let (small,..) = export_step_v9_with_metadata_shared_points(&model,&meta).unwrap();
    assert!(small.len() < normal.len());
    let (baseline,_,_, baseline_meta) = import_step_v9_with_metadata(&normal).unwrap();
    let (restored,_,_, read) = import_step_v9_with_metadata(&small).unwrap();
    let product = &read.xde.products[&read.xde.body_products[0].unwrap()];
    assert_eq!(product.name,"Шестерня");
    assert_eq!(product.material.as_deref(),Some("Сталь"));
    assert_eq!(baseline_meta.presentation.face_styles,read.presentation.face_styles);
    assert_eq!(read.presentation.face_style(marked,0).unwrap().colour,style.colour);
    assert_eq!(read.presentation.face_style(marked,0).unwrap().layers,style.layers);
    assert_eq!(baseline.faces[marked].surface.control_points,restored.faces[marked].surface.control_points);
    assert_eq!(restored.validate().unwrap().boundary_edge_count,0);
    eprintln!("attributed gear STEP bytes: ordinary {} compact {}",normal.len(),small.len());
}
