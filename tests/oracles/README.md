# Development-only QR oracles

These tools never link into a production crate. Install the two generators in an
isolated virtual environment with hashes enforced:

```sh
python3 -m venv .scratch/oracle-venv
.scratch/oracle-venv/bin/pip install --require-hashes -r tests/oracles/requirements.txt
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
.scratch/oracle-venv/bin/python tests/support/generate_fixtures.py \
  --fixture synthetic-v01-m-mask0-byte-001 \
  --fixture synthetic-v02-q-mask3-byte-002 \
  --check
.scratch/oracle-venv/bin/python tests/support/generate_fixtures.py \
  --fixture synthetic-v01-m-mask0-byte-001 \
  --fixture synthetic-v02-q-mask3-byte-002 \
  --write
cargo run -p fixture-tool -- diff HEAD
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
