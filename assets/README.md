# Bundled assets

## Agent metadata

- **Purpose:** provenance and handling contract for production-bundled assets.
- **Read when:** adding, replacing, optimizing, or changing placement of an
  asset.
- **Authority:** asset origin/licensing and immutable-source expectations.
- **Verification:** asset changes cross the shared artifact contract; use the
  full routine gate and any branding/decoder specialized trigger selected by
  `AGENTS.md`.

`RGB-one-lettermark-magenta.svg` is the project-owned ONE brand lettermark supplied directly by the project owner on 2026-08-07 and approved for use in this product. The supplied file was sanitized to explicit local magenta geometry with no scripts, events, text, external references, metadata, embedded resources, CSS, or invisible layers. The original `0 0 1000 602` view box remains in the asset for provenance. QR artifacts render the unchanged geometry through the reviewed `180 180 640 240` presentation box, with QR-specific clear space supplied separately by the module-aligned knockout. The renderer may deterministically recolor the sanitized local geometry to another approved QR foreground theme, currently black, without introducing a second asset or runtime request.

The asset is compile-time embedded and has no runtime request path. The supplied white variant is not bundled into generated QR artifacts. Replacing or editing the magenta asset requires recorded license/provenance, sanitization, and rerunning the complete structural, deterministic, geometry, and independent-decode logo suite before release.

`../crates/qr-web/public/images/one-logotype-white.png` is the project-owned
white ONE logotype supplied directly by the project owner on 2026-08-24. It is
bundled locally for the web page header only and is never embedded in
generated QR artifacts. It introduces no runtime third-party request.
Replacing or editing it requires an updated provenance record and web
visual/privacy verification; it does not trigger QR artifact decoder evidence
unless QR rendering also changes.

`../crates/qr-web/public/images/accessibility-icon.png`,
`../crates/qr-web/public/images/scan-icon.png`,
`../crates/qr-web/public/images/qr-sample.png`, and
`../crates/qr-web/public/images/qr-anatomy-diagram.png` are project-owned
illustrations supplied directly by the project owner on 2026-08-24 for the web
"Usage" tab (principles and QR anatomy explainer). They are bundled locally,
introduce no runtime third-party request, and are never embedded in generated
QR artifacts. Replacing or editing them requires an updated provenance record
and web visual/privacy verification; they do not trigger QR artifact decoder
evidence.
