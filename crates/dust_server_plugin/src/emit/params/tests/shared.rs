use super::*;

#[test]
fn emits_passthrough_and_shared_body_code_exactly() {
    let request = param("request", TypeIr::named("Request"));
    let parts = param("parts", TypeIr::named("RequestParts"));
    let title = param("title", TypeIr::string());
    let priority = param("priority", TypeIr::generic("Option", vec![TypeIr::int()]));
    let upload_value = TypeIr::named("UploadedFile");
    let upload = param(
        "upload",
        TypeIr::generic(
            "Result",
            vec![upload_value.clone(), TypeIr::named("Rejection")],
        ),
    );
    let mut emitter = ParamEmitter::default();
    let mut lines = Vec::new();
    emitter.push(
        &mut lines,
        0,
        &plan(
            &request,
            &request.ty,
            ExtractionWrapper::Required,
            Extraction::Request,
        ),
    );
    emitter.push(
        &mut lines,
        1,
        &plan(
            &parts,
            &parts.ty,
            ExtractionWrapper::Required,
            Extraction::RequestParts,
        ),
    );
    emitter.push(
        &mut lines,
        2,
        &plan(
            &title,
            &title.ty,
            ExtractionWrapper::Required,
            Extraction::Field("title".to_owned()),
        ),
    );
    emitter.push(
        &mut lines,
        3,
        &plan(
            &priority,
            &priority.ty.args()[0],
            ExtractionWrapper::Optional,
            Extraction::Field("priority".to_owned()),
        ),
    );

    assert_eq!(
        lines.join("\n"),
        r#"  final _dustValue0 = request;

  final _dustValue1 = RequestParts.of(request);

  final Result<FormMap, Rejection> _dustBodyResult =
      await const FormExtractable().extract(request);
  if (_dustBodyResult case Err(:final error)) return error.intoResponse();
  final _dustBody = (_dustBodyResult as Ok<FormMap, Rejection>).value;

  final Result<String, Rejection> _dustResult2 =
      _dustBody.field<String>('title');
  if (_dustResult2 case Err(:final error)) return error.intoResponse();
  final _dustValue2 = (_dustResult2 as Ok<String, Rejection>).value;

  final Result<Option<int>, Rejection> _dustResult3 =
      optionalExtractionResult<int>(_dustBody.field<int>('priority'));
  if (_dustResult3 case Err(:final error)) return error.intoResponse();
  final _dustValue3 = (_dustResult3 as Ok<Option<int>, Rejection>).value;
"#
    );

    let mut emitter = ParamEmitter::default();
    let mut lines = Vec::new();
    emitter.push(
        &mut lines,
        0,
        &plan(
            &upload,
            &upload_value,
            ExtractionWrapper::Fallible,
            Extraction::Part {
                name: "file".to_owned(),
                file: true,
            },
        ),
    );
    assert_eq!(
        lines.join("\n"),
        r#"  final Result<MultipartForm, Rejection> _dustBodyResult =
      await const MultipartExtractable().extract(request);
  if (_dustBodyResult case Err(:final error)) return error.intoResponse();
  final _dustBody = (_dustBodyResult as Ok<MultipartForm, Rejection>).value;

  final Result<Result<UploadedFile, Rejection>, Rejection> _dustResult0 =
      fallibleExtractionResult<UploadedFile>(_dustBody.file('file'));
  if (_dustResult0 case Err(:final error)) return error.intoResponse();
  final _dustValue0 =
      (_dustResult0 as Ok<Result<UploadedFile, Rejection>, Rejection>).value;
"#
    );

    let title = param("title", TypeIr::string());
    let mut emitter = ParamEmitter::default();
    let mut lines = Vec::new();
    emitter.push(
        &mut lines,
        0,
        &plan(
            &title,
            &title.ty,
            ExtractionWrapper::Required,
            Extraction::Part {
                name: "title".to_owned(),
                file: false,
            },
        ),
    );
    assert_eq!(
        lines.join("\n"),
        r#"  final Result<MultipartForm, Rejection> _dustBodyResult =
      await const MultipartExtractable().extract(request);
  if (_dustBodyResult case Err(:final error)) return error.intoResponse();
  final _dustBody = (_dustBodyResult as Ok<MultipartForm, Rejection>).value;

  final Result<String, Rejection> _dustResult0 =
      _dustBody.field<String>('title');
  if (_dustResult0 case Err(:final error)) return error.intoResponse();
  final _dustValue0 = (_dustResult0 as Ok<String, Rejection>).value;
"#
    );
}
