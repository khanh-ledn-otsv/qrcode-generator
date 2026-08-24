# 40 — Add black foreground and matching logo theme

**What to build:** Add black as an approved QR foreground option alongside the
existing ONE magenta treatment. A black QR code must use a black ONE logo, while
the magenta QR code continues to use the magenta ONE logo.

**Blocked by:** none.

**Type:** task

**Status:** open

- [ ] Expose exactly two approved foreground themes: ONE magenta and black. Do
  not add arbitrary color picking, gradients, transparency, or external assets.
- [ ] Keep the opaque white background, four-module quiet zone, Rounded ONE data
  modules, square finders, blank separators, and deterministic SVG/PNG bytes.
- [ ] Render the logo in the same foreground color as the QR modules: magenta QR
  uses a magenta logo; black QR uses a black logo. The logo source shape,
  aspect ratio, knockout, and placement rules remain owned by the logo geometry
  policy.
- [ ] Keep theme selection in the web/render presentation layer without changing
  payload bytes, selected mode, ECI, ECC, version, mask, or encoded modules.
- [ ] Validate contrast for both approved themes against the white background in
  preview, SVG export, PNG export, and independent decoder inputs.
- [ ] Update diagnostics, labels, approved-output matrix rows, deterministic
  artifact hashes, browser coverage, and documentation so both themes have one
  explicit contract.
- [ ] Run the routine covering gate for the changed surface and the required
  artifact/decoder evidence for the new black SVG and PNG rows.

## Product intent

The product needs one conservative visual alternative for contexts where the
magenta brand treatment is not acceptable or where black print output is
required. Black is a first-class approved theme, not a user-authored styling
system.

## Implementation constraints

Preserve exact user input and keep all processing in the browser. Do not log,
rewrite, transmit, shorten, or normalize payloads. Rendering must not alter the
encoded QR matrix; it may only change the approved foreground and matching logo
paint used to draw that matrix.

## Comments
