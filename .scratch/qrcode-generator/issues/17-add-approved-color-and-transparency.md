# 17 — Add approved color, contrast, and transparency behavior

**What to build:** Let users select only owner-approved foreground/background combinations and transparent output, with measurable safety classification and realistic surface cautions.

**Blocked by:** 16 — Complete diagnostics, downloads, accessibility, and privacy.

**Prerequisite:** The owner has approved the launch preset list, measurable contrast rule, and whether transparency ships in release one.

**Status:** ready-for-agent

- [ ] The safe black-on-white preset remains the default and the approved brand color is available only when it satisfies the approved contrast rule and decode suite.
- [ ] Unsafe opaque contrast is invalid and disables export with an associated explanation.
- [ ] Transparency is never the default and is classified as a caution because effective placement contrast is unknown.
- [ ] Transparent previews cover documented white, light-gray, dark, and patterned surfaces without modifying encoded modules.
- [ ] SVG and PNG backgrounds, quiet zones, and surplus padding consistently implement opaque or zero-alpha behavior.
- [ ] Every selectable color/background/profile tuple appears in generated structural, deterministic, and independent-decode tests.
