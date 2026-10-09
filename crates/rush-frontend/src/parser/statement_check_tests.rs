
    use super::*;

    fn first_value(source: &str) -> J {
        parse(source).unwrap()[0].node["value"].clone()
    }

    fn kinds(items: &J) -> Vec<&str> {
        items
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item["kind"].as_str().unwrap_or("binding"))
            .collect()
    }

    #[test]
    fn checks_preserve_order_and_expressions_in_brace_functions() {
        let function = first_value(
            "check = fn value: f64 -> f64 {\n\
             assert value > 0\n\
             doubled = value * 2\n\
             validate doubled.atLeast(1)\n\
             assert(doubled >= value)\n\
             ret doubled\n\
             }",
        );
        assert_eq!(
            kinds(&function["items"]),
            ["assert", "binding", "validate", "assert"]
        );
        assert_eq!(function["items"][0]["left"]["kind"], "binary");
        assert_eq!(function["items"][0]["left"]["value"], ">");
        assert_eq!(function["items"][2]["left"]["kind"], "pipe");
        assert_eq!(function["items"][3]["left"]["value"], ">=");
        assert_eq!(function["left"]["value"], "doubled");
    }

    #[test]
    fn checks_work_in_layout_and_void_functions() {
        let statements = parse(
            "fn checked value: f64 -> f64\n  assert(value > 0)\n  ret value\n\
             fn checkOnly value: f64\n  validate value.atLeast(0)\n  assert value >= 0\n\
             output = 1",
        )
        .unwrap();
        assert_eq!(statements.len(), 3);
        assert_eq!(kinds(&statements[0].node["value"]["items"]), ["assert"]);
        let void_function = &statements[1].node["value"];
        assert_eq!(void_function["voidResult"], true);
        assert_eq!(kinds(&void_function["items"]), ["validate", "assert"]);
        assert_eq!(statements[2].node["name"], "output");
    }

    #[test]
    fn checks_preserve_foreach_clause_order() {
        let sequence = first_value(
            "values = foreach value in [1, 2]\n  where value > 0\n  assert value.atLeast(1)\n  doubled = value * 2\n  validate(doubled >= value)\n  yield doubled",
        );
        assert_eq!(
            kinds(&sequence["items"]),
            ["for", "where", "assert", "let", "validate"]
        );
        assert_eq!(sequence["items"][2]["left"]["kind"], "pipe");
        assert_eq!(sequence["items"][4]["left"]["kind"], "binary");
        assert_eq!(sequence["left"]["value"], "doubled");
    }

    #[test]
    fn match_blocks_can_start_with_either_check_keyword() {
        let expression = first_value(
            "value = match 1\n  1 =>\n    assert 1 > 0\n    local = 2\n    validate local.atLeast(1)\n    ret local\n  _ =>\n    validate(1 > 0)\n    ret 0",
        );
        let first = &expression["items"][0]["result"];
        assert_eq!(first["kind"], "match_block");
        assert_eq!(kinds(&first["items"]), ["assert", "binding", "validate"]);
        assert_eq!(first["items"][1]["name"], "local");
        assert_eq!(first["items"][0]["left"]["kind"], "binary");
        assert_eq!(
            kinds(&expression["items"][1]["result"]["items"]),
            ["validate"]
        );
    }

    #[test]
    fn top_level_check_ast_keeps_value_field() {
        let statements = parse("assert(1 > 0)\nvalidate 2 >= 1").unwrap();
        assert_eq!(statements[0].node["kind"], "assert");
        assert_eq!(statements[0].node["value"]["kind"], "binary");
        assert!(statements[0].node.get("left").is_none());
        assert_eq!(statements[1].node["kind"], "validate");
        assert_eq!(statements[1].node["value"]["value"], ">=");
    }

    #[test]
    fn checks_follow_block_indentation_and_statement_limits() {
        for source in [
            "fn checked value: f64 -> f64\n  assert value > 0\n    validate value > 0\n  ret value",
            "values = foreach value in [1]\n  assert value > 0\n    validate value > 0\n  yield value",
            "value = match 1\n  _ =>\n    assert 1 > 0\n      validate 1 > 0\n    ret 1",
            "check = fn value: f64 -> f64 { assert value > 0 ret value }",
        ] {
            assert!(parse(source).is_err(), "Unexpectedly accepted {source}");
        }
        for source in [
            format!(
                "check = fn value: f64 -> f64 {{\n{}ret value\n}}",
                "assert value > 0\n".repeat(65)
            ),
            format!(
                "fn checked value: f64 -> f64\n{}  ret value",
                "  assert value > 0\n".repeat(65)
            ),
            format!(
                "values = foreach value in [1]\n{}  yield value",
                "  assert value > 0\n".repeat(64)
            ),
            format!(
                "value = match 1\n  _ =>\n{}    ret 1",
                "    assert 1 > 0\n".repeat(65)
            ),
        ] {
            let error = parse(&source)
                .err()
                .expect("Statement budget must apply to checks");
            assert!(error.message.contains("At most 64"), "{error}");
        }
    }
