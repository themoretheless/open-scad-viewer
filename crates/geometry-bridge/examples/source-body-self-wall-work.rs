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
 let body=restored.body().expect(restored.reason());let before=body.definition().unwrap();let mut cases=Vec::new();
 let cells:usize=args.get(3).map(|v|v.parse().unwrap()).unwrap_or(4096);
 let spans:usize=args.get(4).map(|v|v.parse().unwrap()).unwrap_or(100000);
 assert!((1..=100000).contains(&cells)&&(1..=100000).contains(&spans));
 let selected:Vec<usize>=args.get(5).map(|v|v.split(',').map(|f|f.parse().unwrap()).collect()).unwrap_or(vec![5,16,17,20,21,24,25]);
 for face in selected{
  let surface=body.geometry().shell().regions().unwrap()[face].loops()[0][0].surface();
  let r=nurbs_core::surface_self_chord_coverage::qualify(surface,5.99,1e-6,
   nurbs_core::surface_self_chord_coverage::Limits{cells,spans}).unwrap();
  eprintln!("face={face} qualified={} cells={} spans={} pending={}",r.certificate.is_some(),r.cells,r.spans,r.pending);
  cases.push(json!({"face":face,"qualified":r.certificate.is_some(),"cells":r.cells,"spans":r.spans,"pending":r.pending,"reason":r.reason,"intrinsicExclusions":r.intrinsic_exclusions,"directionExclusions":r.direction_exclusions,"angularExclusions":r.angular_exclusions,"distanceExclusions":r.distance_exclusions,"coordinatePathExclusions":r.coordinate_path_exclusions,"localCoordinatePathExclusions":r.local_coordinate_path_exclusions,"localCoordinatePathRefusals":r.local_coordinate_path_refusals,"unresolvedPairs":r.unresolved_pairs}));
 }
 assert_eq!(before,body.definition().unwrap());
 std::fs::write(&args[2],value_codec::to_string_pretty(&json!({"scope":"Native original source-face self-chord budget experiment; no whole-wall or general self-intersection certificate","minimumMm":5.99,"maxSineSquared":1e-6,"definitionUnchanged":true,"limits":{"cells":cells,"spans":spans},"cases":cases})).unwrap()).unwrap();
}
