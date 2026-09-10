//! One disposable process per render: crashes and deadlines stay outside the listener.
use super::{render_html_to_pdf, RenderOptions, MAX_BODY_BYTES};
use std::io::{Read, Write};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

const MAX_PDF_BYTES: usize = 32 * 1024 * 1024;
const RENDER_TIMEOUT: Duration = Duration::from_secs(10);

pub fn run() -> Result<(), String> {
    let mut input = Vec::new();
    std::io::stdin()
        .take(MAX_BODY_BYTES as u64 + 1)
        .read_to_end(&mut input)
        .map_err(|err| err.to_string())?;
    if input.len() > MAX_BODY_BYTES {
        return Err("input exceeds limit".into());
    }
    let html = String::from_utf8(input).map_err(|err| err.to_string())?;
    let options = RenderOptions {
        allow_local_assets: false,
        ..RenderOptions::default()
    };
    let pdf = render_html_to_pdf(&html, &options).map_err(|err| err.to_string())?;
    if pdf.len() > MAX_PDF_BYTES {
        return Err("PDF exceeds limit".into());
    }
    std::io::stdout()
        .write_all(&pdf)
        .map_err(|err| err.to_string())
}

fn wait_bounded(child: &mut Child, deadline: Instant) -> Result<ExitStatus, u16> {
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(10));
            }
            result => {
                // Always reap, including timeout and supervision errors.
                let _ = child.kill();
                let _ = child.wait();
                return Err(if result.is_err() { 500 } else { 504 });
            }
        }
    }
}

pub fn render(html: String) -> Result<Vec<u8>, u16> {
    let executable = std::env::current_exe().map_err(|_| 500u16)?;
    let deadline = Instant::now() + RENDER_TIMEOUT;
    let mut child = Command::new(executable)
        .arg("--render-worker")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| 500u16)?;
    let Some(mut stdin) = child.stdin.take() else {
        let _ = child.kill();
        let _ = child.wait();
        return Err(500);
    };
    let Some(stdout) = child.stdout.take() else {
        let _ = child.kill();
        let _ = child.wait();
        return Err(500);
    };
    std::thread::scope(|scope| {
        // Drain stdout concurrently with input to avoid pipe-buffer deadlocks.
        let writer = scope.spawn(move || stdin.write_all(html.as_bytes()));
        let reader = scope.spawn(move || {
            let mut pdf = Vec::new();
            stdout
                .take(MAX_PDF_BYTES as u64 + 1)
                .read_to_end(&mut pdf)?;
            Ok::<_, std::io::Error>(pdf)
        });
        let status = wait_bounded(&mut child, deadline);
        let written = writer.join();
        let output = reader.join();
        let status = status?;
        if !status.success() {
            return Err(422);
        }
        written.map_err(|_| 500u16)?.map_err(|_| 500u16)?;
        let pdf = output.map_err(|_| 500u16)?.map_err(|_| 500u16)?;
        if pdf.len() > MAX_PDF_BYTES || !pdf.starts_with(b"%PDF-") {
            return Err(422);
        }
        Ok(pdf)
    })
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn kills_and_reaps_expired_worker() {
        let mut child = Command::new("sleep").arg("30").spawn().unwrap();
        assert_eq!(wait_bounded(&mut child, Instant::now()), Err(504));
        assert!(child.try_wait().unwrap().is_some());
    }

    #[test]
    fn observes_worker_failure_without_killing_listener() {
        let mut child = Command::new("false").spawn().unwrap();
        let status = wait_bounded(&mut child, Instant::now() + Duration::from_secs(2)).unwrap();
        assert!(!status.success());
    }
}
