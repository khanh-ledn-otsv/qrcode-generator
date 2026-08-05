# 02 — Define validated output profiles and canvas geometry

**What to build:** Provide four compiled output profiles whose QR version ceilings and fixed-canvas geometry can be validated independently of encoding and browser behavior.

**Blocked by:** 01 — Establish the offline workspace baseline.

**Status:** claimed

- [ ] Each supported profile has typed base dimensions, PNG dimensions exactly three times the base dimensions, and an explicit maximum QR version.
- [ ] Geometry includes four quiet modules per side and chooses the largest positive even integer module scale that fits the canvas.
- [ ] Outer padding is checked, symmetric, integral, and contains background only.
- [ ] Every version allowed by every profile is exercised, including scale transitions and the maximum-version minimum of six pixels per module.
- [ ] Invalid profiles, impossible dimensions, and arithmetic overflow return typed errors without panic.
