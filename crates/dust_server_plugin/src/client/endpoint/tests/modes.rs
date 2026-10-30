use dust_ir::{FunctionIr, NameIr};

use super::*;

#[test]
fn endpoint_request_modes_and_function_spans_are_derived_from_facts() {
    for (binding, expected) in [
        ("part", GeneratedRequestMode::MultiPart),
        ("field", GeneratedRequestMode::FormUrlEncoded),
    ] {
        let function = FunctionIr {
            name: NameIr {
                source: "upload".to_owned(),
                short: "upload".to_owned(),
                prefix: None,
                span: span(),
            },
            return_type: TypeIr::string(),
            params: vec![param("wire", TypeIr::string())],
            annotations: Vec::new(),
            span: span(),
        };
        let extraction = if binding == "part" {
            Extraction::Part {
                name: "wire".to_owned(),
                file: false,
            }
        } else {
            Extraction::Field("wire".to_owned())
        };
        let handler = HandlerPlan {
            callable: Callable::Function(&function),
            method: "POST",
            path: "/upload".to_owned(),
            status: 200,
            params: vec![ParamPlan {
                param: &function.params[0],
                value_type: &function.params[0].ty,
                wrapper: ExtractionWrapper::Required,
                extraction,
                validated: false,
                serde_deserialized: false,
            }],
            middleware: Vec::new(),
            metadata: metadata(),
        };
        let endpoint = build_endpoint(
            &route_fact(vec![param_fact("wire", binding)]),
            &handler,
            Vec::new(),
        )
        .unwrap();
        assert_eq!(endpoint.request_mode, expected);
        assert_eq!(endpoint.method.span, function.span);
    }
}
