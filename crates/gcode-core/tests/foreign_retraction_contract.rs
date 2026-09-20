use gcode_core::parse_foreign;

#[test]
fn unmodeled_firmware_commands_do_not_rebase_programmed_absolute_e() {
    let start = "G90\nM82\nG1 X0 Y0 Z0 F600\nG1 X10 E5\n";
    let control = parse_foreign(&format!("{start}G1 X20 E6\n")).unwrap();
    for commands in ["G10\n", "G10\nG11\n", "G10 P0 S210 R210\n"] {
        let preview = parse_foreign(&format!("{start}{commands}G1 X20 E6\n")).unwrap();
        assert_eq!(preview, control, "{commands}");
        assert_eq!(preview.extrusion_mm, 6.0);
    }
}

#[test]
fn unmodeled_retraction_does_not_change_relative_or_volumetric_e_units() {
    for mode in ["M83\n", "M83\nM200 D2\n"] {
        let start = format!("G1 X0 Y0 Z0 F600\n{mode}G1 X10 E5\n");
        let control = parse_foreign(&format!("{start}G1 X20 E1\n")).unwrap();
        let preview = parse_foreign(&format!("{start}G10\nG1 X20 E1\n")).unwrap();
        assert_eq!(preview, control);
    }
}
