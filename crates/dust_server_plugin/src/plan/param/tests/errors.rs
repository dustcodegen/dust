use super::*;

#[test]
fn rejects_unnamed_scalar_missing_header_name_and_invalid_raw_query_type() {
    let cases = [
        param(
            "query",
            TypeIr::string(),
            vec![config("dust_server::Query", None)],
        ),
        param(
            "header",
            TypeIr::string(),
            vec![config("dust_server::Header", None)],
        ),
        param(
            "raw",
            TypeIr::string(),
            vec![config("dust_server::RawQuery", None)],
        ),
    ];
    let mut diagnostics = Vec::new();
    for (index, expected) in [(0, true), (1, false), (2, true)] {
        assert_eq!(
            parse_param(
                &cases[index],
                &BTreeSet::new(),
                BodyMedia::Json,
                &[],
                &[],
                &mut diagnostics,
            )
            .is_some(),
            expected
        );
    }

    assert_eq!(diagnostics.len(), 3);
    for expected in [
        "same-library model",
        "header name",
        "RawQuery requires String?",
    ] {
        assert!(
            diagnostics
                .iter()
                .any(|value| value.message.contains(expected)),
            "missing diagnostic: {expected}"
        );
    }
}

#[test]
fn rejects_an_unknown_server_extractor() {
    let param = param(
        "cookie",
        TypeIr::string(),
        vec![config("dust_server::Cookie", None)],
    );
    let mut diagnostics = Vec::new();

    assert!(
        parse_param(
            &param,
            &BTreeSet::new(),
            BodyMedia::Json,
            &[],
            &[],
            &mut diagnostics,
        )
        .is_none()
    );
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].message,
        "extractor is not supported by generated server handlers yet"
    );
}
