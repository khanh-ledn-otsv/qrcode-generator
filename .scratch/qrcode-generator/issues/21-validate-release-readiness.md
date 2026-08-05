# 21 — Validate release readiness

**What to build:** Produce complete automated and physical evidence that the release candidate is safe, private, accessible, and usable.

**Blocked by:** 20 — Harden all approved output combinations.

**Prerequisite:** The owner has named the supported browsers, devices, scanners, printers, materials, and placement environments.

**Status:** ready-for-agent

- [ ] A local production build and network inspection show no runtime payload or logo requests.
- [ ] A clean build with pinned tools records reproducible application and artifact hashes plus compressed WASM size.
- [ ] Supported desktop/mobile browsers pass critical paths, downloads, privacy inspection, and accessibility checks without retry-hidden correctness failures.
- [ ] Named camera, scanner, screen, printer, material, and placement tests include print samples at 25 mm and 30 mm.
- [ ] User guidance explains SVG-first export, physical-size guidance, transparent/logo cautions, and environment-specific validation.
- [ ] The release runbook links every applicable acceptance criterion to automated results, manual evidence, or an explicitly signed-off exception.
