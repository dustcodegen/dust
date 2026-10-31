const HEADER_END: &[u8] = b"\r\n\r\n";

pub(crate) fn complete_request_len(bytes: &[u8]) -> Result<Option<usize>, ()> {
    let Some(header_index) = bytes
        .windows(HEADER_END.len())
        .position(|window| window == HEADER_END)
    else {
        return Ok(None);
    };
    let body_start = header_index + HEADER_END.len();
    let headers = std::str::from_utf8(&bytes[..header_index]).map_err(|_| ())?;
    let mut content_length = 0usize;
    for line in headers.split("\r\n").skip(1) {
        let Some((name, value)) = line.split_once(':') else {
            return Err(());
        };
        if name.trim().eq_ignore_ascii_case("content-length") {
            content_length = value.trim().parse().map_err(|_| ())?;
        }
    }
    let request_len = body_start.checked_add(content_length).ok_or(())?;
    Ok((bytes.len() >= request_len).then_some(request_len))
}

pub(crate) fn response_head(status: u16, body_len: usize) -> Vec<u8> {
    let reason = match status {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        400 => "Bad Request",
        404 => "Not Found",
        422 => "Unprocessable Entity",
        500 => "Internal Server Error",
        _ => "Response",
    };
    format!(
        "HTTP/1.1 {status} {reason}\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {body_len}\r\n\
         Connection: keep-alive\r\n\r\n"
    )
    .into_bytes()
}

#[cfg(test)]
mod tests {
    use super::{complete_request_len, response_head};

    #[test]
    fn finds_complete_body() {
        let request = b"POST / HTTP/1.1\r\nContent-Length: 4\r\n\r\ntestmore";
        assert_eq!(complete_request_len(request), Ok(Some(request.len() - 4)));
    }

    #[test]
    fn waits_for_body() {
        assert_eq!(
            complete_request_len(b"POST / HTTP/1.1\r\nContent-Length: 4\r\n\r\nte"),
            Ok(None)
        );
    }

    #[test]
    fn creates_keep_alive_response() {
        let response = String::from_utf8(response_head(201, 3)).unwrap();
        assert!(response.starts_with("HTTP/1.1 201 Created\r\n"));
        assert!(response.contains("Content-Length: 3\r\n"));
    }
}
