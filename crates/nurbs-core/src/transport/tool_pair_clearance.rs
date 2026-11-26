use super::*;
use crate::analysis::curve_measure::{
    measure_scheduled_tools_clearance, partition_scheduled_tools_clearance, FirstScheduledToolContact,
    LastScheduledToolContact, ScheduledTool, TimedMotionSegment,
};
pub(super) const OPS: &[&str] = &["minimum_tool_pair_gap","tool_pair_intervals","first_tool_pair_contact","last_tool_pair_contact",
    "tool_contact_duration","longest_tool_contact","peak_tool_pair_count","simultaneous_tool_duration","simultaneous_tool_windows","peak_tool_contacts","tool_pairs_at_time"];

pub(super) fn dispatch(v: &Value, op: &str) -> Result<Value> {
    let curves: Vec<crate::curve::Curve> = field(v,"curves")?;
    let windows: Vec<[f64;2]> = field(v,"windows")?;
    let durations: Vec<f64> = field(v,"durations")?;
    let starts: Vec<f64> = field(v,"start_times")?;
    let radii: Vec<f64> = field(v,"radii_mm")?;
    let count = curves.len();
    check((2..=32).contains(&count) && [windows.len(),durations.len(),starts.len(),radii.len()].iter().all(|&n| n==count),
        "Each scheduled tool requires a window, duration, start and radius")?;
    let tools=curves.iter().enumerate().map(|(i,curve)| ScheduledTool {
        segment:TimedMotionSegment {curve, interval:windows[i],duration:durations[i]},
        start_time:starts[i],radius:radii[i]
    }).collect::<Vec<_>>();
    let budget=field(v,"max_cells")?;
    if op=="minimum_tool_pair_gap" {
        let r=measure_scheduled_tools_clearance(&tools, field(v,"gap_tolerance_mm")?, budget)?;
        return Ok(json!({"gap":r.minimum.gap_bounds,"witnessTime":r.minimum.witness_time_bounds,"tools":r.witness_tools,
            "stopReason":format!("{:?}",r.minimum.stop_reason),"workUsed":r.minimum.work_cells}));
    }
    let r=partition_scheduled_tools_clearance(&tools, field(v,"progress_tolerance")?, budget)?;
    let mut report=match op {
        "tool_pair_intervals"=>json!({"cells":r.cells.iter().map(|c|json!({"tools":c.tools,"time":c.clearance.time_endpoint_bounds,
            "gap":c.clearance.gap_bounds,"status":format!("{:?}",c.clearance.status)})).collect::<Vec<_>>()}),
        "first_tool_pair_contact"=>match r.first_contact(){
            FirstScheduledToolContact::Clear=>json!({"kind":"Clear"}),
            FirstScheduledToolContact::Possible{earliest_time}=>json!({"kind":"Possible","earliestTime":earliest_time}),
            FirstScheduledToolContact::Confirmed{time_bounds,witness_tools}=>json!({"kind":"Confirmed","time":time_bounds,"tools":witness_tools}),
        },
        "last_tool_pair_contact"=>match r.last_contact(){
            LastScheduledToolContact::Clear=>json!({"kind":"Clear"}),
            LastScheduledToolContact::Possible{latest_time}=>json!({"kind":"Possible","latestTime":latest_time}),
            LastScheduledToolContact::Confirmed{time_bounds,witness_tools}=>json!({"kind":"Confirmed","time":time_bounds,"tools":witness_tools}),
        },
        "tool_contact_duration"=>json!({"duration":r.contact_duration_bounds()?}),
        "longest_tool_contact"=>json!({"duration":r.longest_contact_duration_bounds()?}),
        "peak_tool_pair_count"=>json!({"count":r.peak_contact_pair_count_bounds()}),
        "simultaneous_tool_duration"=>json!({"duration":r.simultaneous_contact_duration_bounds(field(v,"minimum_pairs")?)?}),
        "simultaneous_tool_windows"=>{
            let w=r.simultaneous_contact_windows(field(v,"minimum_pairs")?)?;
            json!({"confirmed":w.confirmed,"possible":w.possible})
        }
        "peak_tool_contacts"=>{
            let p=r.peak_contact_report();
            json!({"count":p.pair_count_bounds,"witness":p.witness.map(|w|json!({"time":w.time,"tools":w.tools}))})
        }
        "tool_pairs_at_time"=>{
            let p=r.contact_pairs_at_time(field(v,"time")?)?;
            json!({"confirmed":p.confirmed,"possible":p.possible})
        }
        _=>return Err(input("Unknown scheduled tool pair operation")),
    };
    report["workUsed"]=json!(r.work_cells);
    Ok(report)
}
