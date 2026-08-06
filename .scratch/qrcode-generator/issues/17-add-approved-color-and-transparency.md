# 17 — Add approved color, contrast, and transparency behavior

**What to build:** Let users select only owner-approved foreground/background combinations and transparent output, with measurable safety classification and realistic surface cautions.

**Blocked by:** 16 — Complete diagnostics, downloads, accessibility, and privacy.

**Status:** ready-for-agent

- [ ] The safe black-on-white preset remains the default; the accepted `#BD0F72`-on-white brand preset is selectable only when it satisfies the 4.5:1 opaque contrast rule and decode suite.
- [ ] Unsafe opaque contrast is invalid and disables export with an associated explanation.
- [ ] Optional transparency ships in release 1, is never the default, and is classified as a caution because effective placement contrast is unknown.
- [ ] Transparent previews cover documented white, light-gray, dark, and patterned surfaces without modifying encoded modules.
- [ ] SVG and PNG backgrounds, quiet zones, and surplus padding consistently implement opaque or zero-alpha behavior.
- [ ] Every selectable color/background/profile tuple appears in generated structural, deterministic, and independent-decode tests.
