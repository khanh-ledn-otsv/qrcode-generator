# Development-only QR oracles

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

## Fixture workflow

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
