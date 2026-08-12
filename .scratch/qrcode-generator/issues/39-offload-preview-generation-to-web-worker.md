# 39 — Offload preview generation to a Web Worker

**What to build:** Run QR encoding and SVG/PNG preview generation in a
browser-owned Web Worker so expensive preview work cannot block input, focus,
or paint on the main thread.

**Blocked by:** none.

**Type:** task

**Priority:** performance and responsiveness

- [ ] Measure the current main-thread behavior with a dense, mixed-mode
  Adaptive payload that reaches a large QR version, and retain a reproducible
  Chromium result that demonstrates the blocking work this change addresses.
- [ ] Add a `qr-web` worker boundary that runs `evaluate_preview`—encoding,
  render-model construction, SVG rendering, and PNG rendering—outside the main
  thread. Keep all Worker, messaging, and browser lifecycle APIs in `qr-web`;
  preserve `qr-web -> qr-render -> qr-core` dependency direction.
- [ ] Define explicit request/result messages carrying the workflow revision,
  profile, logo choice, exact payload, diagnostics, SVG bytes, PNG bytes, and
  typed failures. Preserve every payload byte without normalization, logging,
  storage, metadata inclusion, or transmission outside the local worker.
- [ ] Bundle and initialize the worker without external resources or any new
  production network request. Keep generation functional offline and preserve
  the existing content-security policy.
- [ ] Reuse one lifecycle-owned worker, terminate it when the application is
  disposed, release transferred buffers and object URLs, and recover from
  worker startup/message failures without panic or an indefinitely pending UI.
- [ ] Preserve the 250 ms debounce and revision contract: newer input makes
  older results ineligible to update the preview or downloads, pending work
  keeps exports disabled, and only the latest successful revision becomes
  visible.
- [ ] Prove in desktop Chromium that input/focus/paint work can proceed while a
  worst-case preview is running in the worker. Use a deterministic
  responsiveness assertion instead of a machine-speed wall-clock threshold,
  and cover rapid typing, stale/out-of-order results, worker failure, disposal,
  and repeated generation without unbounded retained memory.
- [ ] Prove that worker-produced SVG and PNG bytes exactly match the existing
  deterministic native/WASM artifact contract for representative unbranded,
  branded, UTF-8, mixed-mode, and maximum-version requests; keep download
  filenames, MIME types, diagnostics, dimensions, and accessible behavior
  unchanged.
- [ ] Record the accepted worker/bootstrap/message boundary in
  `docs/DEVELOPMENT_PLAN.md`, update web/WASM test documentation if the harness
  changes, and run `pnpm run verify:web`. Add a specialized artifact/decoder
  check only if output bytes or the artifact pipeline change.

## Context

`schedule_preview` currently debounces for 250 ms and then calls
`evaluate_preview` synchronously on the browser main thread. That call performs
version-aware encoding, builds the render model, and eagerly generates both SVG
and PNG before the Leptos state update. Debouncing reduces how often this work
runs but does not prevent a large request from delaying UI events and paint.

The existing `Revision` check already prevents stale completed work from
replacing newer state. The worker protocol should carry that revision across
the asynchronous boundary rather than duplicate QR logic in JavaScript or move
browser APIs into `qr-core` or `qr-render`.
