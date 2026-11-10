import re,sys
C=sys.argv[1]; B=f'{C}/brep-core/src'
def sub(path,a,b,count=1):
    s=open(path).read(); assert s.count(a)>=1,(path,a[:70]); open(path,'w').write(s.replace(a,b,count))
SI=f'{B}/source_shell_incidence/tests/projective_qualification.rs'
sub(SI,"""        let (step, _, _) = crate::export_step_v6(&transferred).unwrap();
        let (imported, _, _) = crate::import_step_v6(&step).unwrap();
        crate::source_body_model::assert_step_identity(&transferred, &imported);
""","""        #[cfg(feature = "codec")]
        crate::exchange_fixtures::assert_matches(
            "projective_tetrahedron",
            value_codec::json!({"model": value_codec::to_value(&transferred).unwrap()}),
        );
""")
s=open(SI).read(); s=s.replace('crate::source_exchange_step::prepare(','crate::source_exchange_plan::prepare('); open(SI,'w').write(s)
sub(SI,'        assert!(candidate.text.contains("TRIMMED_CURVE"));\n','')
sub(SI,"""        if let Some(path) = std::env::var_os("CAD_PROJECTIVE_ROOT_STEP_OUTPUT") {
            std::fs::write(path, &candidate.text).unwrap();
        }
""","")
sub(SI,"""            if let Ok(path) = std::env::var("CAD_SOURCE_CURVED_STEP_OUTPUT") {
                std::fs::write(path, step.text).unwrap();
            }
""","")
SS=f'{B}/source_support_shell/tests/projective_qualification.rs'
sub(SS,"""        let (step, _, _) = crate::export_step_v6(&transferred).unwrap();
        if let Some(path) = std::env::var_os("CAD_PROJECTIVE_CAPPED_STEP_OUTPUT") {
            std::fs::write(path, &step).unwrap();
        }
        let (imported, _, _) = crate::import_step_v6(&step).unwrap();
        crate::source_body_model::assert_step_identity(&transferred, &imported);
        let body_definition = body.definition().unwrap();
""","""        let body_definition = body.definition().unwrap();
        // STEP export of `transferred` and of the source body: cad-step `tests/kernel_fixtures.rs`.
        crate::exchange_fixtures::assert_matches(
            "projective_capped_end_caps",
            value_codec::json!({
                "model": value_codec::to_value(&transferred).unwrap(),
                "body": value_codec::to_value(&body_definition).unwrap(),
            }),
        );
""")
s=open(SS).read(); s=s.replace('crate::source_exchange_step::prepare(','crate::source_exchange_plan::prepare('); open(SS,'w').write(s)
sub(SS,"""        if let Some(path) = std::env::var_os("CAD_PROJECTIVE_CAPPED_SOURCE_STEP_OUTPUT") {
            std::fs::write(path, &exchange.text).unwrap();
        }
""","")
print('ok')
