# Production readiness

The renderer remains experimental. Passing unit tests does not establish fidelity
for arbitrary HTML and CSS. Validate each production template against independently
generated reference PDFs, including every page and extracted content.

## HTTP adapter

The adapter binds to loopback by default and accepts raw UTF-8 HTML through
`POST /render` with exactly one Content-Length header. Header names are
case-insensitive. Headers are limited to 16 KiB and bodies to 2 MiB. Truncated
requests, invalid UTF-8, duplicate lengths, transfer encoding and Expect are
rejected. Reading has a one-second inactivity timeout and a ten-second overall
budget checked between reads; a blocking read can extend that budget by up to
one second. Writes have a ten-second inactivity timeout.

The adapter has two handlers and a queue of eight connections. Excess connections
receive HTTP 503. Each render runs in a disposable child
process using the same binary. The parent kills and reaps a job after ten seconds
(returning HTTP 504), rejects failed workers with HTTP 422, and accepts at most
32 MiB of PDF output. Output is drained concurrently with input. These limits do
not bound the worker's internal memory, recursion, decoded image size or page
count; PDF size is checked after serialization in the worker. The timeout option
in RenderOptions still applies only to limited scripting for direct library use.
HTTP workers disable document-selected local assets: linked stylesheets are
rejected and local images (including CSS backgrounds) are omitted. Inline CSS and
data URI images remain available. The library and CLI retain local asset access
by default; library callers handling untrusted input must set
`allow_local_assets: false`. Built-in font discovery still reads system fonts.
Do not expose this process directly to untrusted clients.

## Linux deployment envelope

`deploy/htmlpdf.service` is a systemd deployment template for a release binary
installed at `/usr/local/bin/htmlpdf-server`. It binds to loopback, uses a dynamic
unprivileged user, makes the filesystem read-only, hides home directories, and
sets an aggregate 1 GiB memory ceiling with swap disabled and a two-core CPU
quota. These limits cover the listener and all children together, not each job;
memory exhaustion can terminate the service and interrupt other requests.
The unit restarts on failure and kills the entire process group on stop. It has
not been deployed or load-tested here. Install system fonts and test the unit on
the target host before exposure through an authenticated reverse proxy.

## Outstanding release gates

- Isolate render jobs with enforced CPU, memory, wall-clock and filesystem limits.
- Add bounded scheduling, overload responses and graceful shutdown to the service.
- Verify font availability and reproducibility in the deployment image.
- Resolve known layout gaps: inline-block, positioning, fragmentation and nested
  formatting contexts need broader behavioral coverage.
- Run all-page visual and content comparisons against representative workloads.
- Exercise malformed and adversarial documents, process termination, restart and
  concurrency under load.

CI runs formatting, workspace tests and a release build using the lockfile. The
browser comparison script now fails a configured threshold when page counts differ
or any compared page exceeds it. Use `--pages all` without a page cap for release
validation. The comparison browser is a development reference, not a renderer
runtime dependency.
