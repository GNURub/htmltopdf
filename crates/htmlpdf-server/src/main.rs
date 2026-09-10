use htmlpdf_core::{render_html_to_pdf, RenderOptions};
use std::env;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::{Duration, Instant};
mod worker;

const MAX_HEADER_BYTES: usize = 16 * 1024;
const MAX_BODY_BYTES: usize = 2 * 1024 * 1024;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

fn read_request<R: Read>(reader: &mut R) -> Result<String, u16> {
    let mut buffer = Vec::new();
    let mut chunk = [0; 4096];
    let deadline = Instant::now() + REQUEST_TIMEOUT;
    let header_end = loop {
        if Instant::now() >= deadline {
            return Err(408);
        }
        if let Some(end) = buffer.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
            if end + 4 > MAX_HEADER_BYTES {
                return Err(431);
            }
            break end;
        }
        if buffer.len() >= MAX_HEADER_BYTES {
            return Err(431);
        }
        let count = reader.read(&mut chunk).map_err(|_| 408u16)?;
        if count == 0 {
            return Err(400);
        }
        buffer.extend_from_slice(&chunk[..count]);
    };
    let headers = std::str::from_utf8(&buffer[..header_end]).map_err(|_| 400u16)?;
    let mut lines = headers.split("\r\n");
    let request = lines.next().ok_or(400u16)?;
    let parts: Vec<_> = request.split_whitespace().collect();
    if parts.len() != 3 || !matches!(parts[2], "HTTP/1.0" | "HTTP/1.1") {
        return Err(400);
    }
    if parts[0] != "POST" || parts[1] != "/render" {
        return Err(404);
    }
    let mut length = None;
    for line in lines {
        let (name, value) = line.split_once(':').ok_or(400u16)?;
        if name.is_empty()
            || name
                .bytes()
                .any(|byte| !byte.is_ascii_alphanumeric() && byte != b'-')
        {
            return Err(400);
        }
        if name.eq_ignore_ascii_case("transfer-encoding") {
            return Err(400);
        }
        if name.eq_ignore_ascii_case("expect") {
            return Err(417);
        }
        if name.eq_ignore_ascii_case("content-length") {
            let value = value.trim();
            if length.is_some()
                || value.is_empty()
                || !value.bytes().all(|byte| byte.is_ascii_digit())
            {
                return Err(400);
            }
            let parsed = value.parse::<usize>().map_err(|_| 413u16)?;
            if parsed > MAX_BODY_BYTES {
                return Err(413);
            }
            length = Some(parsed);
        }
    }
    let length = length.ok_or(411u16)?;
    let body_start = header_end + 4;
    let body_end = body_start + length;
    while buffer.len() < body_end {
        if Instant::now() >= deadline {
            return Err(408);
        }
        let remaining = (body_end - buffer.len()).min(chunk.len());
        let count = reader.read(&mut chunk[..remaining]).map_err(|_| 408u16)?;
        if count == 0 {
            return Err(400);
        }
        buffer.extend_from_slice(&chunk[..count]);
    }
    String::from_utf8(buffer[body_start..body_end].to_vec()).map_err(|_| 400u16)
}

fn main() {
    let result = if env::args().nth(1).as_deref() == Some("--render-worker") {
        worker::run()
    } else {
        run()
    };
    if let Err(err) = result {
        eprintln!("htmlpdf-server: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:4000".to_string());
    let listener = TcpListener::bind(&addr).map_err(|err| format!("bind {addr}: {err}"))?;
    println!("htmlpdf-server listening on http://{addr}");
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => handle_client(stream),
            Err(err) => eprintln!("connection error: {err}"),
        }
    }
    Ok(())
}

fn handle_client(mut stream: TcpStream) {
    if stream
        .set_read_timeout(Some(Duration::from_secs(1)))
        .is_err()
        || stream.set_write_timeout(Some(REQUEST_TIMEOUT)).is_err()
    {
        return;
    }
    let html = match read_request(&mut stream) {
        Ok(html) => html,
        Err(status) => {
            write_response(&mut stream, status, "text/plain", b"invalid render request");
            return;
        }
    };
    match worker::render(html) {
        Ok(pdf) => write_response(&mut stream, 200, "application/pdf", &pdf),
        Err(status) => write_response(&mut stream, status, "text/plain", b"render job failed"),
    }
}

fn write_response(stream: &mut TcpStream, status: u16, content_type: &str, body: &[u8]) {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        408 => "Request Timeout",
        411 => "Length Required",
        413 => "Content Too Large",
        417 => "Expectation Failed",
        422 => "Unprocessable Entity",
        431 => "Request Header Fields Too Large",
        504 => "Gateway Timeout",
        _ => "Internal Server Error",
    };
    let header = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(header.as_bytes());
    let _ = stream.write_all(body);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    struct Fragmented(Cursor<Vec<u8>>);
    impl Read for Fragmented {
        fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
            let length = bytes.len().min(1);
            self.0.read(&mut bytes[..length])
        }
    }

    #[test]
    fn accepts_fragmented_headers_and_utf8_body() {
        let body = "<p>España</p>";
        let request = format!(
            "POST /render HTTP/1.1\r\ncontent-length: {}\r\n\r\n{body}",
            body.len()
        );
        assert_eq!(
            read_request(&mut Fragmented(Cursor::new(request.into_bytes()))),
            Ok(body.to_string())
        );
    }

    #[test]
    fn rejects_invalid_framing_before_rendering() {
        for (request, status) in [
            ("POST /render HTTP/1.1\r\n\r\n", 411),
            (
                "POST /render HTTP/1.1\r\nContent-Length: 20\r\n\r\nshort",
                400,
            ),
            (
                "POST /render HTTP/1.1\r\nContent-Length: 0\r\ncontent-length: 0\r\n\r\n",
                400,
            ),
            (
                "POST /render HTTP/1.1\r\nTransfer-Encoding: chunked\r\nContent-Length: 0\r\n\r\n",
                400,
            ),
            ("POST /render HTTP/1.1\r\nContent-Length: +1\r\n\r\nx", 400),
            (
                "POST /render HTTP/1.1\r\nContent-Length: 9999999999999999999999999\r\n\r\n",
                413,
            ),
        ] {
            assert_eq!(
                read_request(&mut request.as_bytes()),
                Err(status),
                "{request:?}"
            );
        }
    }

    #[test]
    fn enforces_input_limits() {
        let request = format!(
            "POST /render HTTP/1.1\r\nContent-Length: {}\r\n\r\n",
            MAX_BODY_BYTES + 1
        );
        assert_eq!(read_request(&mut request.as_bytes()), Err(413));
        let request = format!(
            "POST /render HTTP/1.1\r\nX-Padding: {}",
            "x".repeat(MAX_HEADER_BYTES)
        );
        assert_eq!(read_request(&mut request.as_bytes()), Err(431));
        let request = b"POST /render HTTP/1.1\r\nContent-Length: 1\r\n\r\n\xff";
        assert_eq!(read_request(&mut &request[..]), Err(400));
    }
}
