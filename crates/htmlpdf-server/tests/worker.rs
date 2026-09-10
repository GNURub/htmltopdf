use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn binary_worker_returns_pdf_and_rejects_invalid_utf8() {
    for (input, valid) in [
        (b"<p>Worker integration</p>".as_slice(), true),
        (b"\xff".as_slice(), false),
    ] {
        let mut child = Command::new(env!("CARGO_BIN_EXE_htmlpdf-server"))
            .arg("--render-worker")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(input).unwrap();
        let output = child.wait_with_output().unwrap();
        assert_eq!(output.status.success(), valid);
        if valid {
            assert!(output.stdout.starts_with(b"%PDF-"));
            assert!(output.stdout.ends_with(b"%%EOF\n"));
        } else {
            assert!(output.stdout.is_empty());
        }
    }
}
