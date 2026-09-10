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

The adapter is still single-threaded. Each render runs in a disposable child
process using the same binary. The parent kills and reaps a job after ten seconds
(returning HTTP 504), rejects failed workers with HTTP 422, and accepts at most
32 MiB of PDF output. Output is drained concurrently with input. These limits do
not bound the worker's internal memory, recursion, decoded image size or page
count; PDF size is checked after serialization in the worker. The timeout option
in RenderOptions still applies only to limited scripting for direct library use.
Local asset loading is supported and must be treated as filesystem access by the
input document. Do not expose this process directly to untrusted clients.

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
