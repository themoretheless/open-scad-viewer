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

#[test]
fn named_assembly_writes_one_product_for_repeated_parts() {
    use cad_step::{export_named_step_assembly, import_step_shared_assembly, StepAssemblyOccurrence, StepSharedAssembly};
    let body = brep_core::gear::gear(&GearSpec {teeth:4, ..GearSpec::default()}).unwrap();
    let pin = brep_core::cuboid([0.;3],[1.,2.,3.]).unwrap();
    let placed = |definition, x| StepAssemblyOccurrence { definition, transform:
        [[1.,0.,0.,x],[0.,1.,0.,0.],[0.,0.,1.,0.],[0.,0.,0.,1.]] };
    let assembly = StepSharedAssembly::from_placed_solids(vec![body,pin],
        vec![placed(0,0.),placed(1,5.),placed(1,10.)]).unwrap();
    let text = export_named_step_assembly(&assembly,"Spinner A",&["Sun 'A'","Штифт"]).unwrap();
    let encoded = cad_step::step_interchange_v3::step_xde::encode_step_string("Штифт");
    for row in ["PRODUCT('model','Sun ''A'''".to_owned(),format!("PRODUCT('gear-1','{encoded}'"),
        "PRODUCT('planet-set','Spinner A'".to_owned()] {
        assert_eq!(text.matches(&row).count(),1);
    }
    let restored = import_step_shared_assembly(&text).unwrap();
    assert_eq!(restored.definitions().len(),2);
    assert_eq!(restored.occurrences().len(),3);
    assert_eq!(restored.occurrences()[1].definition,restored.occurrences()[2].definition);
    assert_ne!(restored.occurrences()[1].transform,restored.occurrences()[2].transform);
    assert!(export_named_step_assembly(&assembly,"Spinner A",&["Sun"]).is_err());
}

#[test]
fn imports_product_names_by_representation_after_step_rows_are_reordered() {
    use cad_step::{export_named_step_assembly, import_named_step_assembly, StepAssemblyOccurrence, StepSharedAssembly};
    let placed = |definition, x| StepAssemblyOccurrence { definition, transform:
        [[1.,0.,0.,x],[0.,1.,0.,0.],[0.,0.,1.,0.],[0.,0.,0.,1.]] };
    let parts = vec![brep_core::cuboid([0.;3],[1.,2.,3.]).unwrap(),brep_core::cuboid([0.;3],[3.,2.,1.]).unwrap()];
    let assembly = StepSharedAssembly::from_placed_solids(parts, vec![placed(1,8.),placed(0,3.),placed(1,12.)]).unwrap();
    let original = export_named_step_assembly(&assembly,"Сборка",&["Pin 'A'","Колесо"]).unwrap();
    let (header, data) = original.split_once("DATA;\n").unwrap();
    let (rows, tail) = data.split_once("ENDSEC;").unwrap();
    let reordered = format!("{header}DATA;\n{}\nENDSEC;{tail}",rows.lines().rev().collect::<Vec<_>>().join("\n"));
    let read = import_named_step_assembly(&reordered).unwrap();
    assert_eq!(read.name,"Сборка");
    for occurrence in read.assembly.occurrences() {
        assert_eq!(read.definition_names[occurrence.definition], if occurrence.transform[0][3] == 3. { "Pin 'A'" } else { "Колесо" });
    }
}

#[test]
fn named_parts_list_counts_coincident_instances_and_separates_same_name_parts() {
    use cad_step::{export_named_step_assembly, import_named_step_assembly, StepAssemblyOccurrence, StepSharedAssembly};
    let placement = |definition,x| StepAssemblyOccurrence { definition,transform:
        [[1.,0.,0.,x],[0.,1.,0.,0.],[0.,0.,1.,0.],[0.,0.,0.,1.]] };
    let assembly = StepSharedAssembly::from_placed_solids(vec![
        brep_core::cuboid([0.;3],[1.,2.,3.]).unwrap(),brep_core::cuboid([0.;3],[4.,5.,6.]).unwrap()],
        vec![placement(1,8.),placement(0,2.),placement(1,8.),placement(1,10.)]).unwrap();
    let step = export_named_step_assembly(&assembly,"Узел",&["Вставка","Вставка"]).unwrap();
    let mut read = import_named_step_assembly(&step).unwrap();
    let rows = read.parts_list().unwrap();
    assert_eq!(rows.len(),2);
    let repeated = read.assembly.occurrences().iter().find(|o| o.transform[0][3]==8.).unwrap().definition;
    for row in rows {
        assert_eq!(row.name,"Вставка");
        assert_eq!(row.quantity,if row.definition==repeated {3} else {1});
    }
    read.definition_names.pop();
    assert!(read.parts_list().is_err());
}

#[test]
fn selected_parts_export_keeps_names_and_all_original_placements() {
    use cad_step::{import_named_step_assembly, NamedStepAssembly, StepAssemblyOccurrence, StepSharedAssembly};
    let parts = (1..=3).map(|w| brep_core::cuboid([0.;3],[w as f64,2.,3.]).unwrap()).collect();
    let placed = |definition, x| StepAssemblyOccurrence { definition,transform:
        [[1.,0.,0.,x],[0.,1.,0.,0.],[0.,0.,1.,0.],[0.,0.,0.,1.]] };
    let source = NamedStepAssembly { assembly:StepSharedAssembly::from_placed_solids(parts,
        vec![placed(2,20.),placed(1,10.),placed(0,5.),placed(2,25.)]).unwrap(),
        name:"Full".into(),definition_names:vec!["Штифт".into(),"Штифт".into(),"Gear".into()] };
    let step = source.export_selected_parts(&[0,2],"Subset").unwrap();
    let sub = import_named_step_assembly(&step).unwrap();
    assert_eq!(sub.name,"Subset");
    assert_eq!(sub.definition_names.len(),2);
    let mut observed: Vec<_> = sub.assembly.occurrences().iter().map(|o|
        (o.transform[0][3] as i32,sub.definition_names[o.definition].as_str())).collect();
    observed.sort();
    assert_eq!(observed,[(5,"Штифт"),(20,"Gear"),(25,"Gear")]);
    for invalid in [&[][..], &[0,0], &[3]] { assert!(source.export_selected_parts(invalid,"Subset").is_err()); }
}

#[test]
fn named_variant_keeps_part_labels_geometry_and_quantities_aligned() {
    use cad_step::{NamedStepAssembly,StepSharedAssembly,StepAssemblyOccurrence,import_named_step_assembly};
    let block = |w| brep_core::cuboid([0.;3],[w,2.,3.]).unwrap();
    let placements = [0.,5.,10.].map(|x| StepAssemblyOccurrence {definition:0,
        transform:[[1.,0.,0.,x],[0.,1.,0.,0.],[0.,0.,1.,0.],[0.,0.,0.,1.]]});
    let mut source = NamedStepAssembly {name:"Set".into(),definition_names:vec!["Pin".into()],
        assembly:StepSharedAssembly::from_placed_solids(vec![block(1.)],placements.to_vec()).unwrap()};
    let variant = source.replace_named_variant(&[0,2],block(2.),"Wide".into()).unwrap();
    assert_eq!(source.replace_named_variant(&[2,0],block(3.),"Final".into()).unwrap(),variant);
    assert!(source.replace_named_variant(&[0,1],block(4.),"Invalid".into()).is_err());
    let restored = import_named_step_assembly(&source.export_selected_parts(&[0,variant],"Set").unwrap()).unwrap();
    for row in restored.parts_list().unwrap() {
        let wide = row.name == "Final";
        assert!(wide || row.name == "Pin");
        assert_eq!(row.quantity,if wide {2} else {1});
        let width = restored.assembly.definitions()[row.definition].vertices.iter()
            .map(|v| v.point[0]).fold(0.,f64::max);
        assert_eq!(width,if wide {3.} else {1.});
    }
}

#[test]
fn append_named_assembly_composes_placements_and_keeps_distinct_part_rows() {
    use cad_step::{NamedStepAssembly,StepSharedAssembly,StepAssemblyOccurrence,import_named_step_assembly};
    let shift = |x| [[1.,0.,0.,x],[0.,1.,0.,0.],[0.,0.,1.,0.],[0.,0.,0.,1.]];
    let group = |width, xs:&[f64]| NamedStepAssembly {name:"Узел".into(),definition_names:vec!["Вставка".into()],
        assembly:StepSharedAssembly::from_placed_solids(vec![brep_core::cuboid([0.;3],[width,2.,3.]).unwrap()],
            xs.iter().map(|&x| StepAssemblyOccurrence {definition:0,transform:shift(x)}).collect()).unwrap()};
    let mut parent = group(1.,&[0.]);
    let rotation = [[0.,-1.,0.,10.],[1.,0.,0.,20.],[0.,0.,1.,0.],[0.,0.,0.,1.]];
    assert_eq!(parent.append_named_assembly(group(4.,&[2.,5.]),rotation).unwrap(),1..2);
    let mut invalid = rotation;
    invalid[0][0] = f64::NAN;
    assert!(parent.append_named_assembly(group(6.,&[1.]),invalid).is_err());
    let restored = import_named_step_assembly(&parent.export_selected_parts(&[0,1],"Корпус").unwrap()).unwrap();
    let rows = restored.parts_list().unwrap();
    assert_eq!(rows.len(),2);
    for row in rows {
        assert_eq!(row.name,"Вставка");
        let width = restored.assembly.definitions()[row.definition].vertices.iter().map(|v| v.point[0]).fold(0.,f64::max);
        let locations:Vec<_> = restored.assembly.occurrences().iter().filter(|o| o.definition==row.definition)
            .map(|o| [o.transform[0][3],o.transform[1][3]]).collect();
        if width==1. { assert_eq!(locations,[[0.,0.]]); }
        else { assert_eq!(width,4.); assert_eq!(locations,[[10.,22.],[10.,25.]]); }
    }
}

#[test]
fn part_consolidation_requires_both_matching_shape_and_matching_label() {
    use cad_step::{NamedStepAssembly, StepSharedAssembly, StepAssemblyOccurrence, import_named_step_assembly};
    let block = |w| brep_core::cuboid([0.;3],[w,2.,3.]).unwrap();
    let definitions = vec![block(1.),block(2.),block(1.),block(1.)];
    let occurrences = (0..4).map(|i| StepAssemblyOccurrence { definition:i,transform:
        [[1.,0.,0.,i as f64],[0.,1.,0.,0.],[0.,0.,1.,0.],[0.,0.,0.,1.]] }).collect();
    let mut source = NamedStepAssembly {name:"Set".into(),definition_names:["A","A","B","A"].map(String::from).to_vec(),
        assembly:StepSharedAssembly::from_placed_solids(definitions,occurrences).unwrap()};
    assert_eq!(source.consolidate_named_parts().unwrap(),1);
    assert_eq!(source.consolidate_named_parts().unwrap(),0);
    let restored = import_named_step_assembly(&source.export_selected_parts(&[0,1,2],"Set").unwrap()).unwrap();
    assert_eq!(restored.assembly.definitions().len(),3);
    assert_eq!(restored.assembly.occurrences().len(),4);
    for part in restored.assembly.occurrences() {
        let x = part.transform[0][3];
        assert_eq!(restored.definition_names[part.definition],if x==2. {"B"} else {"A"});
        let width = restored.assembly.definitions()[part.definition].vertices.iter().map(|v|v.point[0]).fold(0.,f64::max);
        assert_eq!(width,if x==1. {2.} else {1.});
    }
    source.definition_names.clear();
    assert!(source.consolidate_named_parts().is_err());
}

#[test]
fn parts_delta_aggregates_duplicate_geometry_and_ignores_definition_order() {
    use cad_step::{NamedStepAssembly,StepSharedAssembly,StepAssemblyOccurrence};
    let revision = |widths:&[f64], placements:&[usize]| NamedStepAssembly {name:"Set".into(),
        definition_names:vec!["Pin".into();widths.len()],assembly:StepSharedAssembly::from_placed_solids(
            widths.iter().map(|&w| brep_core::cuboid([0.;3],[w,1.,1.]).unwrap()).collect(),
            placements.iter().enumerate().map(|(i,&definition)| StepAssemblyOccurrence {definition,transform:
                [[1.,0.,0.,i as f64],[0.,1.,0.,0.],[0.,0.,1.,0.],[0.,0.,0.,1.]]}).collect()).unwrap()};
    let before = revision(&[1.,2.,1.],&[0,1,2]);
    let reordered = revision(&[2.,1.],&[1,1,0]);
    assert!(before.parts_list_changes(&reordered).unwrap().is_empty());
    let after = revision(&[3.,1.],&[1,0,1,1]);
    let rows = before.parts_list_changes(&after).unwrap();
    assert_eq!(rows.iter().map(|r|(r.before,r.after)).collect::<Vec<_>>(),[(2,3),(1,0),(0,1)]);
    assert_eq!(rows[1].after_definition,None);
    assert_eq!(rows[2].before_definition,None);
    let mut broken = after;
    broken.definition_names.clear();
    assert!(before.parts_list_changes(&broken).is_err());
}

#[test]
fn grouped_placements_share_a_solid_across_two_groups_and_root() {
    use cad_step::{export_grouped_step_assembly, StepAssemblyOccurrence, StepSharedAssembly};
    let placements = (0..3).map(|i| StepAssemblyOccurrence {
        definition: 0,
        transform: [[1.,0.,0.,i as f64],[0.,1.,0.,0.],[0.,0.,1.,0.],[0.,0.,0.,1.]],
    }).collect();
    let scene = StepSharedAssembly::from_placed_solids(vec![brep_core::cuboid([0.;3],[1.;3]).unwrap()], placements).unwrap();
    let parents = [Some(1),None,Some(0)];
    let step = export_grouped_step_assembly(&scene, &["Rotor's", "Drive"], &parents).unwrap();
    assert_eq!(step.matches("=MANIFOLD_SOLID_BREP(").count(), 1);
    let usages: Vec<_> = step.lines().filter(|line| line.contains("=NEXT_ASSEMBLY_USAGE_OCCURRENCE(")).collect();
    assert_eq!(usages.len(), 5);
    let endpoints = |s: &str| -> (usize, usize) {
        let mut ids = s.split("',#").last().unwrap().split(",#");
        (ids.next().unwrap().parse().unwrap(), ids.next().unwrap().split(',').next().unwrap().parse().unwrap())
    };
    assert!(usages[0].contains("Rotor''s"));
    assert_eq!(endpoints(usages[2]).0, endpoints(usages[1]).1);
    assert_eq!(endpoints(usages[3]).0, endpoints(usages[0]).0);
    assert_eq!(endpoints(usages[4]).0, endpoints(usages[0]).1);
    assert_eq!(endpoints(usages[2]).1, endpoints(usages[4]).1);
    assert!(export_grouped_step_assembly(&scene, &["Drive"], &parents).is_err());
}

#[test]
fn shared_occurrences_keep_separate_surface_colours() {
    use cad_step::{StepAssemblyOccurrence, StepOccurrenceColour, StepSharedAssembly, export_styled_step_assembly};
    let placements = (0..2).map(|i| StepAssemblyOccurrence {
        definition: 0,
        transform: [[1.,0.,0.,5.*i as f64],[0.,1.,0.,0.],[0.,0.,1.,0.],[0.,0.,0.,1.]],
    }).collect();
    let scene = StepSharedAssembly::from_placed_solids(vec![brep_core::cuboid([0.;3],[1.,2.,3.]).unwrap()], placements).unwrap();
    let colours = [Some(StepOccurrenceColour { name: "Blue", rgb: [0.,0.,1.] }),
                   Some(StepOccurrenceColour { name: "Red", rgb: [1.,0.,0.] })];
    let text = export_styled_step_assembly(&scene, &[], &[None,None], &colours).unwrap();
    assert_eq!(text.matches("=MANIFOLD_SOLID_BREP(").count(), 1);
    assert_eq!(text.matches("=PRESENTATION_STYLE_BY_CONTEXT(").count(), 2);
    let reopened = cad_step::import_step_shared_assembly(&text).unwrap();
    assert_eq!(reopened.occurrences().len(), 2);
    assert_eq!(cad_step::import_step_occurrence_colours(&text).unwrap(), [Some([0.,0.,1.]),Some([1.,0.,0.])]);
    let contexts: Vec<_> = text.lines().filter(|line| line.contains("=PRESENTATION_STYLE_BY_CONTEXT(")).collect();
    let context_ref = |line: &str| line.rsplit_once(",#").unwrap().1.trim_end_matches(");").to_owned();
    let conflicting = contexts[1].replace(&format!(",#{});", context_ref(contexts[1])),
        &format!(",#{});", context_ref(contexts[0])));
    assert!(cad_step::import_step_occurrence_colours(&text.replace(contexts[1], &conflicting)).is_err());
    let grouped = export_styled_step_assembly(&scene, &["Drive"], &[None,Some(0)], &[None,Some(StepOccurrenceColour { name:"Amber",rgb:[1.,0.5,0.] })]).unwrap();
    let reopened = cad_step::import_step_shared_assembly(&grouped).unwrap();
    for (placement, colour) in reopened.occurrences().iter().zip(cad_step::import_step_occurrence_colours(&grouped).unwrap()) {
        assert_eq!(colour, (placement.transform[0][3] == 5.).then_some([1.,0.5,0.]));
    }
    if let Some(path) = std::env::var_os("CAD_STEP_OCCURRENCE_COLOUR_FIXTURE") {
        std::fs::write(path, text).unwrap();
    }
}

#[test]
fn exchanged_spinner_cylindrical_replacement_is_a_closed_single_body() {
    let source = brep_core::gear(&GearSpec { module:1.5, teeth:32, height:10., helix_angle_deg:-35.,
        herringbone:true, clearance:0.1, backlash:0.05, bore:42., ..Default::default() }).unwrap();
    let cutter = brep_core::cylinder(22., 10.).unwrap();
    let result = brep_core::boolean(&source, &cutter, "difference").unwrap();
    let (text,..) = cad_step::export_step_v9(&result).unwrap();
    // Each coedge keeps a representation on its own carrier. Identical 2D
    // splines may be referenced by several of these representations.
    let curve_refs: Vec<_> = text.lines().filter_map(|line| {
        let (_, tail) = line.split_once("=DEFINITIONAL_REPRESENTATION('',(#")?;
        tail.split_once("),#").map(|(curve, _)| curve)
    }).collect();
    assert_eq!(curve_refs.len(), result.loops.iter().map(|l| l.coedges.len()).sum::<usize>());
    let unique: std::collections::BTreeSet<_> = curve_refs.iter().collect();
    assert!(unique.len() * 2 < curve_refs.len());
    let (read,..) = cad_step::import_step_v9(&text).unwrap();
    assert_eq!(read.bodies.len(), 1);
    assert_eq!(read.validate().unwrap().boundary_edge_count, 0);
    assert_eq!(read.faces.len(), result.faces.len());
    if let Some(dir) = std::env::var_os("CAD_STEP_BORE_VOLUME_FIXTURE") {
        let dir = std::path::Path::new(&dir);
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join("before.step"), cad_step::export_step_v9(&source).unwrap().0).unwrap();
        std::fs::write(dir.join("after.step"), text).unwrap();
        let sleeve = brep_core::boolean(&source, &cutter, "intersection").unwrap();
        let (sleeve_step, ..) = cad_step::export_step_v9(&sleeve).unwrap();
        let (reopened, ..) = cad_step::import_step_v9(&sleeve_step).unwrap();
        assert_eq!(reopened.bodies.len(), 1);
        assert_eq!(reopened.validate().unwrap().boundary_edge_count, 0);
        std::fs::write(dir.join("sleeve.step"), sleeve_step).unwrap();
        let filled = brep_core::boolean(&source, &cutter, "union").unwrap();
        let (filled_step, ..) = cad_step::export_step_v9(&filled).unwrap();
        let (reopened, ..) = cad_step::import_step_v9(&filled_step).unwrap();
        assert_eq!(reopened.bodies.len(), 1);
        assert!(reopened.faces.iter().all(|f| f.holes.is_empty()));
        assert_eq!(reopened.validate().unwrap().boundary_edge_count, 0);
        std::fs::write(dir.join("filled.step"), filled_step).unwrap();
    }
}
