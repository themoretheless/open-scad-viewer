//! Native original Body pole self-wall experiment; no global wall certificate.
use value_codec::{Value,Deserialize,json};
type Result<T> = std::result::Result<T,nurbs_core::Error>;
fn field<T:for<'de> Deserialize<'de>>(v:&Value,key:&str)->Result<T>{
 T::from_value(v[key].clone()).map_err(|_|nurbs_core::Error::new("EXPERIMENT_INPUT","Invalid saved restoration field"))
}
fn limits(v: &Value) -> Result<brep_core::source_body_restore::Limits> {
    let shell = &v["shell"];
    let regions = &shell["regions"];
    let region = &regions["region"];
    let embedding = &v["embedding"];
    let pairs = &embedding["pairs"];
    let volume = &v["volume"];
    Ok(brep_core::source_body_restore::Limits {
        shell: brep_core::source_shell_restore::Limits {
            exact_work: field(shell, "exactWork")?,
            driver_cells: field(shell, "driverCells")?,
            regions: brep_core::source_region_restore::Limits {
                region: brep_core::trimmed_face_recipe::Limits {
                    pairs: field(region, "pairs")?,
                    region_cells: field(region, "regionCells")?,
                    domain_cells: field(region, "domainCells")?,
                    agreement_cells: field(region, "agreementCells")?,
                },
                search_cells: field(regions, "searchCells")?,
                point_checks: field(regions, "pointChecks")?,
                mapping_cells: field(regions, "mappingCells")?,
                driver_cells: field(regions, "driverCells")?,
                membership_cells: field(regions, "membershipCells")?,
                controls: field(regions, "controls")?,
                winding_cells: field(regions, "windingCells")?,
                steps: field(regions, "steps")?,
            },
        },
        embedding: brep_core::source_shell_geometry::Limits {
            tolerance_uv: field(embedding, "toleranceUv")?,
            corners: field(embedding, "corners")?,
            spans: field(embedding, "spans")?,
            linear_cells: field(embedding, "linearCells")?,
            exact_work: field(embedding, "exactWork")?,
            driver_cells: field(embedding, "driverCells")?,
            pairs: brep_core::face_contacts::Limits {
                pairs: field(pairs, "pairs")?,
                cells: field(pairs, "cells")?,
                domain_cells: field(pairs, "domainCells")?,
                cells_per_pair: field(pairs, "cellsPerPair")?,
                domain_cells_per_pair: field(pairs, "domainCellsPerPair")?,
            },
        },
        volume: brep_core::source_volume::Limits {
            axis: field(volume, "axis")?,
            origin: field(volume, "origin")?,
            absolute_error: field(volume, "absoluteError")?,
            tolerance_uv: field(volume, "toleranceUv")?,
            cells: field(volume, "cells")?,
            spans: field(volume, "spans")?,
            domain_cells: field(volume, "domainCells")?,
        },
    })
}
fn main(){
 let args:Vec<_>=std::env::args().collect();
 let request:Value=value_codec::from_str(&std::fs::read_to_string(&args[1]).unwrap()).unwrap();
 let restored=brep_core::source_body_restore::restore(request["definition"].clone(),limits(&request["limits"]).unwrap()).unwrap();
 let body=restored.body().expect(restored.reason());
 let before=body.definition().unwrap();let mut faces=Vec::new();
 for face in [0,10]{
  let surface=body.geometry().shell().regions().unwrap()[face].loops()[0][0].surface();
  let mut prepared=None;
  for end in [false,true] {
   let normals=nurbs_core::factored_surface_normals::prepare(surface,end,100000000).unwrap();
   if normals.certificate.is_some(){
    let differential=nurbs_core::rational_pole_differential::prepare(surface,end,100000000).unwrap();
    if differential.certificate.is_some(){prepared=Some((normals,differential));break;}
   }
  }
  let Some((normals,differential))=prepared else {
   faces.push(json!({"face":face,"qualified":false,"reason":"pole-preparation-unproven"}));
   eprintln!("face={face} pole preparation unproven");continue;
  };
  let r=nurbs_core::surface_self_chord_coverage::qualify_with_rational_pole(surface,5.99,1e-6,
   nurbs_core::surface_self_chord_coverage::Limits{cells:4096,spans:100000},
   normals.certificate.as_ref().expect(normals.reason),differential.certificate.as_ref().expect(differential.reason)).unwrap();
  eprintln!("face={face} qualified={} cells={} spans={} pending={}",r.certificate.is_some(),r.cells,r.spans,r.pending);
  faces.push(json!({"face":face,"qualified":r.certificate.is_some(),"cells":r.cells,"spans":r.spans,"pending":r.pending,"reason":r.reason,"normalExactWork":normals.work,"differentialExactWork":differential.work,"unresolvedPairs":r.unresolved_pairs}));
 }
 let mut adapter=brep_core::source_wall_self_coverage::qualify(body,5.99,1e-6,
  brep_core::source_wall_self_coverage::Limits{plane_controls:1000000,cells:50000,spans:100000,cells_per_face:8192,spans_per_face:10000}).unwrap();
 let previous:Vec<_>=adapter.certificates().iter().map(|c|c.face()).collect();
 brep_core::source_wall_self_coverage::extend_with_pole_facts(body,5.99,1e-6,&mut adapter,
  brep_core::source_wall_self_coverage::Limits{plane_controls:1,cells:100000,spans:100000,cells_per_face:4096,spans_per_face:100000},100000000).unwrap();
 assert!(previous.iter().all(|face|adapter.certificates().iter().any(|c|c.face()==*face)));
 if args.get(3).is_some_and(|v|v=="refine") {
  let retained:Vec<_>=adapter.certificates().iter().map(|c|c.face()).collect();
  brep_core::source_wall_self_coverage::refine_unqualified(body,5.99,1e-6,&mut adapter,
   brep_core::source_wall_self_coverage::Limits{plane_controls:1,cells:100000,spans:100000,cells_per_face:100000,spans_per_face:100000}).unwrap();
  assert!(retained.iter().all(|face|adapter.certificates().iter().any(|c|c.face()==*face)));
  eprintln!("refinement cells={} spans={} qualified={}",adapter.refinement_cells,adapter.refinement_spans,adapter.certificates().len());
 }

 assert!(adapter.pole_exact_work<=100000000&&adapter.pole_cells<=100000&&adapter.pole_spans<=100000);
 eprintln!("adapter exact={} extra cells={} spans={} qualified={}",adapter.pole_exact_work,adapter.pole_cells,adapter.pole_spans,adapter.certificates().len());
 let adapter_report=json!({"refinementCells":adapter.refinement_cells,"refinementSpans":adapter.refinement_spans,"allSelfPairsQualified":adapter.all_self_pairs_qualified(),"poleExactWork":adapter.pole_exact_work,"cells":adapter.cells,"spans":adapter.spans,"poleCells":adapter.pole_cells,"poleSpans":adapter.pole_spans,
  "faces":adapter.faces.iter().map(|f|json!({"face":f.face,"qualified":f.qualified,"pending":f.pending,"reason":f.reason})).collect::<Vec<_>>()});
 assert_eq!(before,body.definition().unwrap());
 std::fs::write(&args[2],value_codec::to_string_pretty(&json!({"scope":"Native restored original Body selected pole faces, almost-normal self-chord coverage only","minimumMm":5.99,"maxSineSquared":1e-6,"definitionUnchanged":true,"adapter":adapter_report,"faces":faces})).unwrap()).unwrap();
}
