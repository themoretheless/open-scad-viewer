use gcode_core::{Flavor, WaitPolicy, analyze_firmware};
#[test]
fn tool_targets_waits_and_retract_do_not_alias() {
    let s = analyze_firmware(
        "M104 T1 S210\nT0\nM109 R180\nM207 S0.8\nG10\nG10\nG11",
        Flavor::Marlin,
    )
    .unwrap();
    assert_eq!(s.active_tool, 0);
    assert_eq!(
        s.tools.iter().find(|t| t.id == 1).unwrap().target_c,
        Some(210.0)
    );
    assert_eq!(s.events[1].wait, WaitPolicy::HeatingOrCooling);
    assert_eq!(s.events[2].retract_offset_mm, Some(0.8));
    assert_eq!(s.events[3].retract_offset_mm, Some(0.8));
    assert_eq!(s.events[4].retract_offset_mm, Some(0.0));
}
#[test]
fn reprap_temperature_is_not_retraction_and_unknown_targets_stay_unknown() {
    let s = analyze_firmware("G10 P1 S210 R150\nM109 T0\nG10", Flavor::RepRapFirmware).unwrap();
    assert_eq!(s.tools[0].standby_c, Some(150.0));
    assert_eq!(s.tools[0].retracted, None);
    assert_eq!(s.events[1].target_c, None);
    assert_eq!(s.unverified_lines, vec![3]);
}
#[test]
fn refuses_invalid_and_marks_unverified_extensions() {
    assert!(analyze_firmware("M104 S-1", Flavor::Marlin).is_err());
    assert!(analyze_firmware("M104 T1.5 S200", Flavor::Marlin).is_err());
    let s = analyze_firmware("M109 S200 B250\nG10 P0 S210", Flavor::Marlin).unwrap();
    assert_eq!(s.unverified_lines, vec![1, 2]);
    assert!(s.events.is_empty());
}
#[test]
fn unknown_temperature_units_clear_targets_until_explicit_celsius() {
    let state =
        analyze_firmware("M104 S210\nM149 F\nM109 R180\nM149 C\nM109", Flavor::Marlin).unwrap();
    assert_eq!(state.tools[0].target_c, None);
    assert_eq!(state.unverified_lines, vec![2, 3]);
    assert_eq!(state.events.last().unwrap().target_c, None);
}
#[test]
fn event_output_is_bounded_separately_from_input() {
    let code = "M104 S200\n".repeat(8193);
    assert!(analyze_firmware(&code, Flavor::Marlin).is_err());
}
