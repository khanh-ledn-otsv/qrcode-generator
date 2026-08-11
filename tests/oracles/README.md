# Development-only QR oracles

## Agent metadata

- **Purpose:** pinned oracle identities and explicit fixture verification/
  regeneration protocol.
- **Read when:** touching `tests/oracles`, fixture manifests/goldens, QR tables,
  oracle adapters, or decoder pins.
- **Authority:** oracle environment and fixture mutation workflow.
- **Default safe action:** select only the relevant `--check` verifier.
- **Mutation warning:** `--write` changes committed evidence and is allowed only
  when the task explicitly requires fixture regeneration. Review the complete
  diff; tests must never regenerate implicitly.

These tools never link into a production crate. The uv project and committed
lockfile pin the two generators and their artifact hashes. Create or synchronize
the isolated environment without changing the lockfile:

```sh
uv sync --project tests/oracles --locked
```

The accepted generator pair is Nayuki QR Code Generator 1.8.0 and
`python-qrcode` 8.2. Both expose an exact QR version, ECC level, mask, and data
mode. Segno 1.6.6 was evaluated first because it is the latest stable release,
but its byte-mode output disagreed with Nayuki when the pre-padding bit stream
was already byte-aligned: it inserted a zero codeword where Nayuki emitted the
required first `0xEC` pad codeword. The disputed output was rejected, and Segno
is not pinned as an accepted matrix oracle.

ZXing-C++ 3.0.2 at commit
`8dd1cf5c4fd6fb6211bb96713db926ac6f2cf825` is the independent raster decoder.
Run the exact checkout and build commands recorded in
`tests/fixtures/manifest.json`. The `fixture-tool decode` command rejects a
different or tracked-modified source checkout and verifies the binary version
before decoding. It then compares raw bytes and checks exposed QR version, ECC,
and ECI-presence metadata.

If `cmake` is not installed globally, the same build can be run with the
ephemeral executable supplied by `uv tool run cmake`; the verified decoder pin
is the ZXing source commit and reported reader version.

## Fixture workflow

Do not run every command in this file as a checklist. Select the verifier for
the changed fixture family; `AGENTS.md` determines the covering routine gate.

Normal tests only validate the strict manifest, hashes, matrix dimensions, and
recorded dual-oracle agreement:

```sh
cargo run -p fixture-tool -- verify tests/fixtures/manifest.json
```

Oracle execution is always explicit. `--check` regenerates in memory and fails
on either oracle disagreement or committed drift. `--write` is the only action
that changes goldens, and it deliberately changes verification state to
`pending`. Regeneration executes the two oracle identities declared by each
fixture and refreshes each source's pinned tool version, reproducible command,
and independently observed matrix hash:

```sh
uv run --project tests/oracles --locked python tests/support/generate_fixtures.py \
  --fixture synthetic-v01-m-mask0-byte-001 \
  --fixture synthetic-v02-q-mask3-byte-002 \
  --check
uv run --project tests/oracles --locked python tests/support/generate_fixtures.py \
  --fixture synthetic-v01-m-mask0-byte-001 \
  --fixture synthetic-v02-q-mask3-byte-002 \
  --write
cargo run -p fixture-tool -- diff HEAD
```

The QR capacity/version table fixture has its own explicit verifier. It checks
all 160 version/ECC rows against both pinned generators before comparing the
result with the committed CSV:

```sh
uv run --project tests/oracles --locked python \
  tests/support/verify_qr_tables.py --check
```

Use `--write` only when intentionally refreshing the table fixture, then review
the complete CSV diff before accepting it.

Reed–Solomon generator and remainder vectors are checked for every QR ECC
degree against Nayuki's `reed_solomon_*` functions and python-qrcode's
`Polynomial`/`gexp`/`glog` implementation. The fixture includes leading and
trailing zero blocks plus the maximum table-defined QR data block. Its accepted
provenance, artifact hash, source files/symbols, and local-reference coverage
are recorded as `qr-reed-solomon-vectors` in `tests/fixtures/manifest.json`:

```sh
uv run --project tests/oracles --locked python \
  tests/support/verify_reed_solomon.py --check
```

Block splitting and final interleaving fixtures cover one-group and two-group
short/long layouts. Their complete streams must agree between both pinned
encoders:

```sh
uv run --project tests/oracles --locked python \
  tests/support/verify_interleaved_codewords.py --check
```

Classified function-matrix fixtures cover all 40 versions with compact FNV-1a
fingerprints and Versions 1, 2, 7, and 40 with a readable
one-character-per-module map. The verifier derives module classifications from
instrumented calls into both pinned encoders and requires exact agreement.
Format and version regions are normalized to light reservations because their
BCH values are added by ticket 10:

```sh
uv run --project tests/oracles --locked python \
  tests/support/verify_function_matrices.py --check
```

Explicit data-placement fixtures cover Versions 1, 2, 7, and 40 under every
mask. The verifier instruments both encoders' placement routines, compares the
exact traversal and completed masked matrices, and preserves separate data and
remainder classifications after normalizing the already-accepted function
regions:

```sh
uv run --project tests/oracles --locked python \
  tests/support/verify_placement_matrices.py --check
```

Mask-selection evidence records both the owner-approved literal complete-window
Rule 3 score and Nayuki's differing run-history score for every candidate:

```sh
uv run --project tests/oracles --locked python \
  tests/support/verify_mask_selection.py --check
```

Composed encoder goldens cover Versions 1, 2, 6, 7, 9, 10, 26, 27 and 40,
all ECC levels and masks, every supported mode, UTF-8 ECI 26, and the
character-count version-band boundaries. Both pinned encoders must produce the
same completed matrix; the verifier manually frames ECI 26 through
python-qrcode's independent bit-buffer, RS-block, and matrix path because its
public API does not expose ECI segments:

```sh
uv run --project tests/oracles --locked python \
  tests/support/verify_encoder_goldens.py --check
```

Replay the seeded end-to-end decode suite after building the pinned reader. It
checks exact bytes, version and ECI presence across all ECC levels and masks:

```sh
cargo test -p qr-core --test independent_decode -- --ignored --nocapture
```

Replay the production SVG artifact suite through pinned `resvg` rasterization
and the same pinned ZXing-C++ reader:

```sh
cargo test -p qr-render --test independent_svg_decode -- --ignored --nocapture
```

Review the readable manifest and `0`/`1` matrix diff, then record the reviewer,
date, notes, and `accepted` state. `fixture-tool verify` rejects pending fixture
changes.

After building ZXingReader, inspect a production PNG against a fixture with:

```sh
cargo run -p fixture-tool -- decode \
  tests/fixtures/manifest.json synthetic-v01-m-mask0-byte-001 \
  path/to/production.png tests/oracles/zxing-cpp \
  tests/oracles/zxing-cpp/build/example/ZXingReader
```
