use gcode_core::parse_foreign;

#[test]
fn override_changes_time_not_geometry_or_extrusion() {
    let base = "G1 X0 Y0 Z0 F600\nM83\nG1 X10 E1\n";
    let normal = parse_foreign(base).unwrap();
    let slowed = parse_foreign(&format!("M220 S50\n{base}")).unwrap();
    assert_eq!(slowed.estimated_time_s, 2.0);
    assert_eq!(normal.estimated_time_s, 1.0);
    assert_eq!(slowed.bounds, normal.bounds);
    assert_eq!(slowed.extrusion_mm, normal.extrusion_mm);
    assert_eq!(slowed.print_distance_mm, normal.print_distance_mm);
    assert_eq!(slowed.moves.last().unwrap().feedrate_mm_s, 5.0);
}

#[test]
fn override_is_not_compounded_and_backup_restore_order_is_explicit() {
    let result = parse_foreign("G1 X0 Y0 Z0 F600\nM220 S50\nM220 B S200\nG1 X10\nM220 R B\nG1 X20\nM220 R\nG1 X30\nM220 S100\nM220\nG1 X40\n").unwrap();
    let speeds: Vec<_> = result.moves.iter().map(|m| m.feedrate_mm_s).collect();
    assert_eq!(speeds, [10.0, 20.0, 5.0, 20.0, 10.0]);
    assert_eq!(result.estimated_time_s, 4.0);
}

#[test]
fn override_applies_to_new_feedrates_inches_and_arcs() {
    let result =
        parse_foreign("M220S50\nG20\nG1X0Y0Z0F60\nG1X1F120\nG21\nG3X0Y25.4I-25.4J0\n").unwrap();
    assert_eq!(result.moves[1].feedrate_mm_s, 25.4);
    assert!(result.moves[2..].iter().all(|m| m.feedrate_mm_s == 25.4));
    assert!((result.estimated_time_s - result.travel_distance_mm / 25.4).abs() < 1e-12);
}

#[test]
fn invalid_or_unrepresentable_override_refuses_and_unknown_feedrate_stays_unknown() {
    for command in [
        "M220 S0",
        "M220 S-1",
        "M220 S1.5",
        "M220 S32768",
        "M220 S",
        "M220 B1",
        "M220 X2",
    ] {
        assert!(
            parse_foreign(&format!("G1 X0 Y0 Z0 F600\n{command}\nG1 X1\n")).is_err(),
            "{command}"
        );
    }
    assert!(parse_foreign("G1 X0 Y0 Z0 F60000000\nM220 S200\nG1 X1\n").is_err());
    let unknown = parse_foreign("M220 S50\nG1 X0 Y0 Z0\nG1 X1\n").unwrap();
    assert_eq!(unknown.estimated_time_s, 0.0);
    assert!(unknown.moves.iter().all(|m| m.feedrate_mm_s == 0.0));
}
