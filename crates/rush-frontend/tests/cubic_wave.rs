#[test]
fn cubic_wave_requires_its_authored_period_count() {
    let source = include_str!("../../../examples/rush/cubic-wave-surface.r");
    assert!(rush_frontend::compile(source).is_ok());
    assert!(rush_frontend::compile(&source.replace(",periods:2", "")).is_err());
}
