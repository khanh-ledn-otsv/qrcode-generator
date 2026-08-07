# 21 — Validate release readiness

**What to build:** Produce complete automated and physical evidence that the release candidate is safe, private, accessible, and usable.

**Blocked by:** 20 — Harden all approved output combinations.

**Prerequisite:** The owner has named the supported browsers, devices, scanners, printers, materials, and placement environments.

**Status:** claimed

- [ ] A local production build and network inspection show no runtime payload or logo requests.
- [ ] A clean build with pinned tools records reproducible application and artifact hashes plus compressed WASM size.
- [ ] Supported desktop/mobile browsers pass critical paths, downloads, privacy inspection, and accessibility checks without retry-hidden correctness failures.
- [ ] Named camera, scanner, screen, printer, material, and placement tests include print samples at 25 mm and 30 mm.
- [x] User guidance explains SVG-first export, physical-size guidance, transparent/logo cautions, and environment-specific validation.
- [x] The release runbook links every applicable acceptance criterion to automated results, manual evidence, or an explicitly signed-off exception.

## Comments

- 2026-08-07: Added the clean-build evidence collector, strict readiness validator,
  complete criterion map, and manual evidence template. Final evidence and sign-off
  remain blocked because the prerequisite owner-supplied browser, device, scanner,
  printer, material, placement, and physical-test names/results are not present in
  the repository. The validator deliberately rejects placeholders and unsigned
  omissions.
