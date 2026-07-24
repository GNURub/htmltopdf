use htmlpdf_core::{render_html_to_pdf, RenderOptions};
use std::env;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

fn main() {
    if let Err(err) = run() {
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
    let mut buffer = Vec::new();
    let mut temp = [0u8; 4096];
    let Ok(read) = stream.read(&mut temp) else {
        return;
    };
    buffer.extend_from_slice(&temp[..read]);

    let request = String::from_utf8_lossy(&buffer);
    let Some(header_end) = request.find("\r\n\r\n") else {
        write_response(&mut stream, 400, "text/plain", b"bad request");
        return;
    };
    let headers = &request[..header_end];
    if !headers.starts_with("POST /render ") {
        write_response(
            &mut stream,
            404,
            "text/plain",
            b"use POST /render with raw HTML body",
        );
        return;
    }

    let content_length = headers
        .lines()
        .find_map(|line| line.strip_prefix("Content-Length:"))
        .and_then(|value| value.trim().parse::<usize>().ok())
        .unwrap_or(0);
    let body_start = header_end + 4;
    while buffer.len() < body_start + content_length {
        let Ok(read) = stream.read(&mut temp) else {
            break;
        };
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&temp[..read]);
    }

    let body_end = (body_start + content_length).min(buffer.len());
    let html = String::from_utf8_lossy(&buffer[body_start..body_end]);
    match render_html_to_pdf(&html, &RenderOptions::default()) {
        Ok(pdf) => write_response(&mut stream, 200, "application/pdf", &pdf),
        Err(err) => write_response(&mut stream, 422, "text/plain", err.to_string().as_bytes()),
    }
}

fn write_response(stream: &mut TcpStream, status: u16, content_type: &str, body: &[u8]) {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        422 => "Unprocessable Entity",
        _ => "Internal Server Error",
    };
    let header = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(header.as_bytes());
    let _ = stream.write_all(body);
}
