use super::*;
use crate::analysis::class_a as optics;

fn defects(items: &[optics::Defect]) -> Vec<Value> {
    items.iter().map(|d|json!({"kind":format!("{:?}",d.kind),"uv":d.uv,"value":d.value,"threshold":d.threshold})).collect()
}
fn lines(line:&optics::IsoLineReport)->Value{
    json!({"polylinesUv":line.polylines,"defects":defects(&line.defects)})
}
pub(super) fn inspect(v:&Value,op:&str)->Result<Value>{
    let surface=field(v,"surface")?;
    let grid=field(v,"grid")?;
    let limits=optics::Limits::default();
    let mut report=match op {
        "surface_highlight_lines"=>{
            let lights:Vec<[f64;3]>=field(v,"lights")?;
            let r=optics::highlight_lines(&surface,&lights,field(v,"view_direction")?,field(v,"level")?,grid,field(v,"kink_angle_rad")?,&limits)?;
            json!({"lights":r.iter().map(lines).collect::<Vec<_>>()})
        }
        "surface_reflection_lines"=>{
            let (r,jumps)=optics::reflection_lines(&surface,field(v,"light")?,field(v,"view_direction")?,
                field(v,"level")?,grid,field(v,"jump_threshold")?,&limits)?;
            json!({"lines":lines(&r),"jumps":defects(&jumps)})
        }
        "surface_isophote_analysis"=>{
            let levels:Vec<f64>=field(v,"cos_levels")?;
            check(levels.len()<=64,"Isophote analysis supports up to 64 levels per request")?;
            let r=optics::isophote_analysis(&surface,field(v,"light")?,&levels,grid,field(v,"uneven_tolerance")?,&limits)?;
            json!({"levels":r.levels.iter().map(lines).collect::<Vec<_>>(),"meanSpacing":r.mean_spacing,
                "spacingUnevenness":r.spacing_unevenness,"defects":defects(&r.defects)})
        }
        _=>{
            let r=optics::dent_detection(&surface,grid,field(v,"sigma_cells")?,field(v,"residual_per_mm2")?,&limits)?;
            json!({"maxResidualPerMm2":r.max_residual,"zones":r.zones.iter().map(|z|json!({"uv":z.uv,"amplitudePerMm2":z.amplitude})).collect::<Vec<_>>()})
        }
    };
    report["grid"]=json!(grid);
    report["sampled"]=json!(true);
    Ok(report)
}
