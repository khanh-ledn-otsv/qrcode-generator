//! Development-only QR fixture provenance and independent decoder support.

#![forbid(unsafe_code)]

use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureManifest {
    schema_version: u8,
    decoder: DecoderProvenance,
    fixtures: Vec<Fixture>,
    #[serde(default)]
    algorithm_fixtures: Vec<AlgorithmFixture>,
}

impl FixtureManifest {
    pub fn load_and_verify(path: impl AsRef<Path>) -> Result<Self, VerificationError> {
        let path = path.as_ref();
        let bytes = fs::read(path).map_err(|error| {
            VerificationError::new(format!("could not read {}: {error}", path.display()))
        })?;
        let manifest: Self = serde_json::from_slice(&bytes).map_err(|error| {
            VerificationError::new(format!("invalid manifest {}: {error}", path.display()))
        })?;
        let root = path
            .parent()
            .ok_or_else(|| VerificationError::new("manifest path must have a parent directory"))?;
        manifest.verify(root)?;
        Ok(manifest)
    }

    #[must_use]
    pub fn fixtures(&self) -> &[Fixture] {
        &self.fixtures
    }

    #[must_use]
    pub fn algorithm_fixtures(&self) -> &[AlgorithmFixture] {
        &self.algorithm_fixtures
    }

    #[must_use]
    pub fn decoder(&self) -> &DecoderProvenance {
        &self.decoder
    }

    pub fn fixture(&self, id: &str) -> Result<&Fixture, VerificationError> {
        self.fixtures
            .iter()
            .find(|fixture| fixture.id == id)
            .ok_or_else(|| VerificationError::new(format!("unknown fixture id {id}")))
    }

    fn verify(&self, root: &Path) -> Result<(), VerificationError> {
        if self.schema_version != 1 {
            return Err(VerificationError::new(format!(
                "unsupported fixture schema version {}",
                self.schema_version
            )));
        }
        self.decoder.verify()?;
        if self.fixtures.is_empty() {
            return Err(VerificationError::new(
                "fixture manifest must contain at least one fixture",
            ));
        }

        let mut ids = HashSet::new();
        for fixture in &self.fixtures {
            if !ids.insert(fixture.id.as_str()) {
                return Err(VerificationError::new(format!(
                    "duplicate fixture id {}",
                    fixture.id
                )));
            }
            fixture.verify(root)?;
        }
        for fixture in &self.algorithm_fixtures {
            if !ids.insert(fixture.id.as_str()) {
                return Err(VerificationError::new(format!(
                    "duplicate fixture id {}",
                    fixture.id
                )));
            }
            fixture.verify(root)?;
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlgorithmFixture {
    id: String,
    kind: AlgorithmKind,
    synthetic: bool,
    artifact_file: PathBuf,
    artifact_sha256: String,
    standard_topic: String,
    scope: String,
    generation_command: String,
    sources: Vec<AlgorithmSource>,
    local_verification: Vec<String>,
    verification: VerificationState,
}

impl AlgorithmFixture {
    fn verify(&self, root: &Path) -> Result<(), VerificationError> {
        let policy = self.kind.policy();

        require_nonempty("algorithm fixture id", &self.id)?;
        if !self.synthetic {
            return Err(VerificationError::new(format!(
                "{} fixture {} must explicitly declare synthetic data",
                policy.label, self.id
            )));
        }
        require_nonempty("algorithm standard topic", &self.standard_topic)?;
        require_nonempty("algorithm fixture scope", &self.scope)?;
        if self.generation_command != policy.command {
            return Err(VerificationError::new(format!(
                "{} fixture {} has an unpinned generation command",
                policy.label, self.id
            )));
        }
        verify_hash(
            root,
            &self.artifact_file,
            &self.artifact_sha256,
            policy.label,
            &self.id,
        )?;
        if self.sources.len() != 2 {
            return Err(VerificationError::new(format!(
                "{} fixture {} requires two independent generators",
                policy.label, self.id
            )));
        }
        let mut oracles = HashSet::new();
        for source in &self.sources {
            source.verify(&self.id, &self.artifact_sha256, policy)?;
            if !oracles.insert(source.oracle) {
                return Err(VerificationError::new(format!(
                    "{} fixture {} does not identify two independent generators",
                    policy.label, self.id
                )));
            }
        }
        if self.local_verification.is_empty() {
            return Err(VerificationError::new(format!(
                "{} fixture {} requires local invariant or reference coverage",
                policy.label, self.id
            )));
        }
        for evidence in &self.local_verification {
            require_nonempty("local algorithm verification", evidence)?;
        }
        self.verification.verify(&self.id)
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AlgorithmSource {
    oracle: Oracle,
    tool: String,
    version: String,
    executed_source_url: String,
    executed_symbols: Vec<String>,
    evidence_source_url: String,
    evidence_symbols: Vec<String>,
    #[serde(default)]
    supporting_source_urls: Vec<String>,
    #[serde(default)]
    supporting_symbols: Vec<String>,
    command: String,
    observed_artifact_sha256: String,
}

impl AlgorithmSource {
    fn verify(
        &self,
        fixture_id: &str,
        artifact_sha256: &str,
        policy: AlgorithmPolicy,
    ) -> Result<(), VerificationError> {
        let pinned = self.oracle.provenance();
        let expected = policy.source(self.oracle);
        if self.tool != pinned.tool
            || self.version != pinned.version
            || self.executed_source_url != expected.executed_source_url
            || self.executed_symbols != expected.executed_symbols
            || self.evidence_source_url != expected.evidence_source_url
            || self.evidence_symbols != expected.evidence_symbols
            || self.supporting_source_urls != expected.supporting_source_urls
            || self.supporting_symbols != expected.supporting_symbols
            || self.command != policy.command
        {
            return Err(VerificationError::new(format!(
                "{} fixture {fixture_id} source {} does not match pinned provenance",
                policy.label, pinned.cli_name
            )));
        }
        if self.executed_symbols.is_empty() || self.evidence_symbols.is_empty() {
            return Err(VerificationError::new(format!(
                "{} fixture {fixture_id} source {} requires exact symbols",
                policy.label, pinned.cli_name
            )));
        }
        for symbol in &self.executed_symbols {
            require_nonempty("algorithm executed source symbol", symbol)?;
        }
        for symbol in &self.evidence_symbols {
            require_nonempty("algorithm evidence source symbol", symbol)?;
        }
        for source_url in &self.supporting_source_urls {
            require_nonempty("algorithm supporting source URL", source_url)?;
        }
        for symbol in &self.supporting_symbols {
            require_nonempty("algorithm supporting source symbol", symbol)?;
        }
        verify_sha256_text(
            &self.observed_artifact_sha256,
            "source algorithm artifact",
            fixture_id,
        )?;
        if self.observed_artifact_sha256 != artifact_sha256 {
            return Err(VerificationError::new(format!(
                "{} fixture {fixture_id} source {} disagrees with the accepted artifact",
                policy.label, pinned.cli_name
            )));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum AlgorithmKind {
    ReedSolomon,
    CodewordInterleaving,
}

impl AlgorithmKind {
    const fn policy(self) -> AlgorithmPolicy {
        match self {
            Self::ReedSolomon => AlgorithmPolicy {
                label: "Reed–Solomon",
                command: "uv run --project tests/oracles --locked python tests/support/verify_reed_solomon.py --check",
                nayuki_executed_symbols: &[
                    "_reed_solomon_compute_divisor",
                    "_reed_solomon_compute_remainder",
                    "_reed_solomon_multiply",
                ],
                nayuki_evidence_symbols: &[
                    "reed_solomon_compute_divisor",
                    "reed_solomon_compute_remainder",
                    "reed_solomon_multiply",
                ],
                python_source_url: "https://github.com/lincolnloop/python-qrcode/blob/v8.2/qrcode/base.py",
                python_symbols: &["Polynomial", "gexp", "glog"],
                python_supporting_source_urls: &[],
                python_supporting_symbols: &[],
            },
            Self::CodewordInterleaving => AlgorithmPolicy {
                label: "codeword-interleaving",
                command: "uv run --project tests/oracles --locked python tests/support/verify_interleaved_codewords.py --check",
                nayuki_executed_symbols: &["_add_ecc_and_interleave"],
                nayuki_evidence_symbols: &["add_ecc_and_interleave"],
                python_source_url: "https://github.com/lincolnloop/python-qrcode/blob/v8.2/qrcode/util.py",
                python_symbols: &["create_bytes"],
                python_supporting_source_urls: &[
                    "https://github.com/lincolnloop/python-qrcode/blob/v8.2/qrcode/base.py",
                ],
                python_supporting_symbols: &["rs_blocks", "Polynomial", "gexp"],
            },
        }
    }
}

#[derive(Clone, Copy)]
struct AlgorithmPolicy {
    label: &'static str,
    command: &'static str,
    nayuki_executed_symbols: &'static [&'static str],
    nayuki_evidence_symbols: &'static [&'static str],
    python_source_url: &'static str,
    python_symbols: &'static [&'static str],
    python_supporting_source_urls: &'static [&'static str],
    python_supporting_symbols: &'static [&'static str],
}

impl AlgorithmPolicy {
    const fn source(self, oracle: Oracle) -> AlgorithmSourcePolicy {
        match oracle {
            Oracle::Nayuki => AlgorithmSourcePolicy {
                executed_source_url: "https://github.com/nayuki/QR-Code-generator/blob/v1.8.0/python/qrcodegen.py",
                executed_symbols: self.nayuki_executed_symbols,
                evidence_source_url: "https://github.com/nayuki/QR-Code-generator/blob/v1.8.0/rust/src/lib.rs",
                evidence_symbols: self.nayuki_evidence_symbols,
                supporting_source_urls: &[],
                supporting_symbols: &[],
            },
            Oracle::PythonQrcode => AlgorithmSourcePolicy {
                executed_source_url: self.python_source_url,
                executed_symbols: self.python_symbols,
                evidence_source_url: self.python_source_url,
                evidence_symbols: self.python_symbols,
                supporting_source_urls: self.python_supporting_source_urls,
                supporting_symbols: self.python_supporting_symbols,
            },
        }
    }
}

#[derive(Clone, Copy)]
struct AlgorithmSourcePolicy {
    executed_source_url: &'static str,
    executed_symbols: &'static [&'static str],
    evidence_source_url: &'static str,
    evidence_symbols: &'static [&'static str],
    supporting_source_urls: &'static [&'static str],
    supporting_symbols: &'static [&'static str],
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecoderProvenance {
    tool: String,
    version: String,
    source_url: String,
    source_commit: String,
    checkout_command: String,
    build_command: String,
}

impl DecoderProvenance {
    fn verify(&self) -> Result<(), VerificationError> {
        require_nonempty("decoder tool", &self.tool)?;
        require_nonempty("decoder version", &self.version)?;
        require_nonempty("decoder source URL", &self.source_url)?;
        verify_commit(&self.source_commit)?;
        require_nonempty("decoder checkout command", &self.checkout_command)?;
        require_nonempty("decoder build command", &self.build_command)
    }

    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    #[must_use]
    pub fn source_commit(&self) -> &str {
        &self.source_commit
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fixture {
    id: String,
    synthetic: bool,
    payload_file: PathBuf,
    payload_sha256: String,
    encoding: String,
    eci_assignment: Option<EciAssignment>,
    mode: Mode,
    version: QrVersion,
    ecc: ErrorCorrection,
    mask: u8,
    expected_matrix_file: PathBuf,
    expected_matrix_sha256: String,
    sources: Vec<GeneratorProvenance>,
    verification: VerificationState,
}

impl Fixture {
    fn verify(&self, root: &Path) -> Result<(), VerificationError> {
        require_nonempty("fixture id", &self.id)?;
        if !self.synthetic {
            return Err(VerificationError::new(format!(
                "fixture {} must explicitly declare a synthetic payload",
                self.id
            )));
        }
        if self.mask > 7 {
            return Err(VerificationError::new(format!(
                "fixture {} has invalid mask {}",
                self.id, self.mask
            )));
        }
        require_nonempty("encoding", &self.encoding)?;
        if self.eci_assignment.is_some() && (self.mode != Mode::Byte || self.encoding != "utf-8") {
            return Err(VerificationError::new(format!(
                "fixture {} may use ECI only with UTF-8 byte mode",
                self.id
            )));
        }
        self.verification.verify(&self.id)?;

        verify_hash(
            root,
            &self.payload_file,
            &self.payload_sha256,
            "payload",
            &self.id,
        )?;
        let matrix = verify_hash(
            root,
            &self.expected_matrix_file,
            &self.expected_matrix_sha256,
            "matrix",
            &self.id,
        )?;
        verify_matrix(&self.id, self.version.number(), &matrix)?;

        if self.sources.len() != 2 {
            return Err(VerificationError::new(format!(
                "fixture {} requires two independent generators",
                self.id
            )));
        }
        let mut oracles = HashSet::new();
        for source in &self.sources {
            source.verify(&self.id)?;
            if !oracles.insert(source.oracle) {
                return Err(VerificationError::new(format!(
                    "fixture {} does not identify two independent generators",
                    self.id
                )));
            }
            if source.matrix_sha256 != self.expected_matrix_sha256 {
                return Err(VerificationError::new(format!(
                    "fixture {} has oracle matrix disagreement for {}",
                    self.id, source.tool
                )));
            }
        }
        Ok(())
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn decode_expectation(
        &self,
        fixture_root: impl AsRef<Path>,
    ) -> Result<DecodeExpectation, VerificationError> {
        let payload_path = safe_join(fixture_root.as_ref(), &self.payload_file)?;
        let payload = fs::read(&payload_path).map_err(|error| {
            VerificationError::new(format!(
                "could not read fixture payload {}: {error}",
                payload_path.display()
            ))
        })?;
        Ok(DecodeExpectation {
            payload,
            version: self.version,
            ecc: self.ecc,
            eci_assignment: self.eci_assignment,
        })
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
enum Mode {
    Numeric,
    Alphanumeric,
    Byte,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum ErrorCorrection {
    L,
    M,
    Q,
    H,
}

impl ErrorCorrection {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::L => "L",
            Self::M => "M",
            Self::Q => "Q",
            Self::H => "H",
        }
    }
}

impl fmt::Display for ErrorCorrection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(try_from = "u8")]
pub struct QrVersion(u8);

impl QrVersion {
    pub const fn new(number: u8) -> Result<Self, &'static str> {
        if number >= 1 && number <= 40 {
            Ok(Self(number))
        } else {
            Err("QR version must be between 1 and 40")
        }
    }

    #[must_use]
    pub const fn number(self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for QrVersion {
    type Error = &'static str;

    fn try_from(number: u8) -> Result<Self, Self::Error> {
        Self::new(number)
    }
}

impl fmt::Display for QrVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(try_from = "u32")]
pub enum EciAssignment {
    Utf8,
}

impl TryFrom<u32> for EciAssignment {
    type Error = &'static str;

    fn try_from(assignment: u32) -> Result<Self, Self::Error> {
        if assignment == 26 {
            Ok(Self::Utf8)
        } else {
            Err("only UTF-8 ECI assignment 26 is supported")
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GeneratorProvenance {
    oracle: Oracle,
    tool: String,
    implementation: String,
    version: String,
    command: String,
    matrix_sha256: String,
}

impl GeneratorProvenance {
    fn verify(&self, fixture_id: &str) -> Result<(), VerificationError> {
        require_nonempty("source tool", &self.tool)?;
        require_nonempty("source implementation", &self.implementation)?;
        require_nonempty("source version", &self.version)?;
        require_nonempty("source generation command", &self.command)?;
        verify_sha256_text(&self.matrix_sha256, "source matrix", fixture_id)?;
        let pinned = self.oracle.provenance();
        let expected_command = format!(
            "uv run --project tests/oracles --locked python tests/support/generate_fixtures.py --fixture {fixture_id} --oracle {}",
            pinned.cli_name
        );
        if self.tool != pinned.tool
            || self.implementation != pinned.implementation
            || self.version != pinned.version
            || self.command != expected_command
        {
            return Err(VerificationError::new(format!(
                "fixture {fixture_id} source {} does not match pinned provenance",
                pinned.cli_name
            )));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq)]
enum Oracle {
    #[serde(rename = "nayuki")]
    Nayuki,
    #[serde(rename = "python-qrcode")]
    PythonQrcode,
}

impl Oracle {
    const fn provenance(self) -> OracleProvenance {
        match self {
            Self::Nayuki => OracleProvenance {
                cli_name: "nayuki",
                tool: "Nayuki QR Code Generator",
                implementation: "nayuki-qrcodegen-python",
                version: "1.8.0",
            },
            Self::PythonQrcode => OracleProvenance {
                cli_name: "python-qrcode",
                tool: "python-qrcode",
                implementation: "python-qrcode",
                version: "8.2",
            },
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct OracleProvenance {
    cli_name: &'static str,
    tool: &'static str,
    implementation: &'static str,
    version: &'static str,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct VerificationState {
    state: AcceptanceState,
    reviewer: String,
    verified_at: String,
    notes: String,
}

impl VerificationState {
    fn verify(&self, fixture_id: &str) -> Result<(), VerificationError> {
        if self.state != AcceptanceState::Accepted {
            return Err(VerificationError::new(format!(
                "fixture {fixture_id} is not independently accepted"
            )));
        }
        require_nonempty("verification reviewer", &self.reviewer)?;
        require_nonempty("verification date", &self.verified_at)?;
        require_nonempty("verification notes", &self.notes)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
enum AcceptanceState {
    Accepted,
    Pending,
    Rejected,
}

fn require_nonempty(field: &str, value: &str) -> Result<(), VerificationError> {
    if value.trim().is_empty() {
        return Err(VerificationError::new(format!("{field} must not be empty")));
    }
    Ok(())
}

fn verify_hash(
    root: &Path,
    relative_path: &Path,
    expected: &str,
    artifact: &str,
    fixture_id: &str,
) -> Result<Vec<u8>, VerificationError> {
    verify_sha256_text(expected, artifact, fixture_id)?;
    let path = safe_join(root, relative_path)?;
    let bytes = fs::read(&path).map_err(|error| {
        VerificationError::new(format!(
            "could not read {artifact} for fixture {fixture_id} at {}: {error}",
            path.display()
        ))
    })?;
    let actual = sha256_hex(&bytes);
    if actual != expected {
        return Err(VerificationError::new(format!(
            "fixture {fixture_id} {artifact} SHA-256 mismatch: expected {expected}, got {actual}"
        )));
    }
    Ok(bytes)
}

fn safe_join(root: &Path, relative_path: &Path) -> Result<PathBuf, VerificationError> {
    if relative_path.is_absolute()
        || relative_path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(VerificationError::new(format!(
            "fixture path must be a simple relative path: {}",
            relative_path.display()
        )));
    }
    Ok(root.join(relative_path))
}

fn verify_sha256_text(
    hash: &str,
    artifact: &str,
    fixture_id: &str,
) -> Result<(), VerificationError> {
    if hash.len() != 64 || !hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(VerificationError::new(format!(
            "fixture {fixture_id} has invalid {artifact} SHA-256"
        )));
    }
    Ok(())
}

fn verify_commit(commit: &str) -> Result<(), VerificationError> {
    if commit.len() != 40
        || !commit
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(VerificationError::new(
            "decoder source commit must be a full lowercase SHA-1",
        ));
    }
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        use fmt::Write as _;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

fn verify_matrix(fixture_id: &str, version: u8, bytes: &[u8]) -> Result<(), VerificationError> {
    let matrix = std::str::from_utf8(bytes).map_err(|error| {
        VerificationError::new(format!("fixture {fixture_id} matrix is not UTF-8: {error}"))
    })?;
    let size = 17_usize + usize::from(version) * 4;
    let rows: Vec<&str> = matrix.lines().collect();
    let dimensions_are_valid = rows.len() == size
        && rows
            .iter()
            .all(|row| row.len() == size && row.bytes().all(|byte| matches!(byte, b'0' | b'1')));
    if !dimensions_are_valid {
        return Err(VerificationError::new(format!(
            "fixture {fixture_id} matrix must contain {size} rows of {size} modules"
        )));
    }
    Ok(())
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerificationError(String);

impl VerificationError {
    fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for VerificationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for VerificationError {}

#[derive(Debug)]
pub struct DecodeExpectation {
    pub payload: Vec<u8>,
    pub version: QrVersion,
    pub ecc: ErrorCorrection,
    pub eci_assignment: Option<EciAssignment>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct DecodeObservation {
    pub version: QrVersion,
    pub ecc: ErrorCorrection,
    pub has_eci: bool,
}

#[derive(Debug)]
pub struct ZxingDecoder {
    executable: PathBuf,
    expected_version: String,
    source_checkout: PathBuf,
    expected_source_commit: String,
}

impl ZxingDecoder {
    pub fn new(
        executable: impl Into<PathBuf>,
        expected_version: impl Into<String>,
        source_checkout: impl Into<PathBuf>,
        expected_source_commit: impl Into<String>,
    ) -> Self {
        Self {
            executable: executable.into(),
            expected_version: expected_version.into(),
            source_checkout: source_checkout.into(),
            expected_source_commit: expected_source_commit.into(),
        }
    }

    pub fn inspect_and_compare(
        &self,
        artifact: impl AsRef<Path>,
        expected: &DecodeExpectation,
    ) -> Result<DecodeObservation, VerificationError> {
        self.verify_source_checkout()?;
        self.verify_version()?;
        let artifact = artifact.as_ref();
        let metadata = self.run([
            "-formats",
            "QRCode",
            "-single",
            "-mode",
            "ECI",
            artifact
                .to_str()
                .ok_or_else(|| VerificationError::new("artifact path is not valid UTF-8"))?,
        ])?;
        let raw_bytes = self.run_bytes([
            "-formats",
            "QRCode",
            "-single",
            "-bytes",
            artifact
                .to_str()
                .ok_or_else(|| VerificationError::new("artifact path is not valid UTF-8"))?,
        ])?;
        if raw_bytes != expected.payload {
            return Err(VerificationError::new(
                "decoded bytes do not match the expected payload",
            ));
        }

        let format = metadata_value(&metadata, "Format")?;
        if format != "QRCode" {
            return Err(VerificationError::new(format!(
                "decoded format is {format}, expected QRCode"
            )));
        }
        let version_number = metadata_value(&metadata, "Version")?
            .parse::<u8>()
            .map_err(|error| VerificationError::new(format!("invalid decoded version: {error}")))?;
        let version = QrVersion::new(version_number).map_err(VerificationError::new)?;
        let ecc = match metadata_value(&metadata, "ECLevel")? {
            "L" => ErrorCorrection::L,
            "M" => ErrorCorrection::M,
            "Q" => ErrorCorrection::Q,
            "H" => ErrorCorrection::H,
            value => {
                return Err(VerificationError::new(format!(
                    "invalid decoded ECC level {value}"
                )));
            }
        };
        let has_eci = match metadata_value(&metadata, "HasECI")? {
            "0" | "false" => false,
            "1" | "true" => true,
            value => {
                return Err(VerificationError::new(format!(
                    "invalid decoded HasECI value {value}"
                )));
            }
        };
        if version != expected.version || ecc != expected.ecc {
            return Err(VerificationError::new(format!(
                "decoded metadata mismatch: expected version {} ECC {}, got version {version} ECC {ecc}",
                expected.version, expected.ecc
            )));
        }
        if has_eci != expected.eci_assignment.is_some() {
            return Err(VerificationError::new(
                "decoded ECI state does not match the fixture metadata",
            ));
        }

        Ok(DecodeObservation {
            version,
            ecc,
            has_eci,
        })
    }

    fn verify_version(&self) -> Result<(), VerificationError> {
        let output = self.run(["-version"])?;
        let expected = format!("ZXingReader version {}", self.expected_version);
        if output.trim() != expected {
            return Err(VerificationError::new(format!(
                "expected ZXingReader version {}, got {}",
                self.expected_version,
                output.trim()
            )));
        }
        Ok(())
    }

    fn verify_source_checkout(&self) -> Result<(), VerificationError> {
        let source_checkout = self.source_checkout.canonicalize().map_err(|error| {
            VerificationError::new(format!(
                "could not resolve ZXing-C++ source checkout {}: {error}",
                self.source_checkout.display()
            ))
        })?;
        let executable = self.executable.canonicalize().map_err(|error| {
            VerificationError::new(format!(
                "could not resolve ZXingReader executable {}: {error}",
                self.executable.display()
            ))
        })?;
        if !executable.starts_with(&source_checkout) {
            return Err(VerificationError::new(
                "ZXingReader executable must be built inside the pinned source checkout",
            ));
        }
        let output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&source_checkout)
            .output()
            .map_err(|error| {
                VerificationError::new(format!(
                    "could not inspect ZXing-C++ source checkout {}: {error}",
                    self.source_checkout.display()
                ))
            })?;
        if !output.status.success() {
            return Err(VerificationError::new(
                "ZXing-C++ source checkout is not a readable git repository",
            ));
        }
        let actual = String::from_utf8(output.stdout)
            .map_err(|error| VerificationError::new(format!("invalid git output: {error}")))?;
        if actual.trim() != self.expected_source_commit {
            return Err(VerificationError::new(format!(
                "expected ZXing-C++ source commit {}, got {}",
                self.expected_source_commit,
                actual.trim()
            )));
        }
        for cached in [false, true] {
            let mut command = Command::new("git");
            command.arg("diff");
            if cached {
                command.arg("--cached");
            }
            let status = command
                .arg("--quiet")
                .current_dir(&source_checkout)
                .status()
                .map_err(|error| {
                    VerificationError::new(format!("could not inspect ZXing-C++ source: {error}"))
                })?;
            if !status.success() {
                return Err(VerificationError::new(
                    "ZXing-C++ source checkout has tracked modifications",
                ));
            }
        }
        Ok(())
    }

    fn run<const N: usize>(&self, arguments: [&str; N]) -> Result<String, VerificationError> {
        let bytes = self.run_bytes(arguments)?;
        String::from_utf8(bytes).map_err(|error| {
            VerificationError::new(format!("ZXingReader output was not UTF-8: {error}"))
        })
    }

    fn run_bytes<const N: usize>(
        &self,
        arguments: [&str; N],
    ) -> Result<Vec<u8>, VerificationError> {
        let output = Command::new(&self.executable)
            .args(arguments)
            .output()
            .map_err(|error| {
                VerificationError::new(format!(
                    "could not run {}: {error}",
                    self.executable.display()
                ))
            })?;
        if !output.status.success() {
            return Err(VerificationError::new(format!(
                "ZXingReader failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        Ok(output.stdout)
    }
}

fn metadata_value<'a>(metadata: &'a str, key: &str) -> Result<&'a str, VerificationError> {
    metadata
        .lines()
        .find_map(|line| {
            line.split_once(':')
                .filter(|(candidate, _)| candidate.trim() == key)
                .map(|(_, value)| value.trim())
        })
        .ok_or_else(|| VerificationError::new(format!("ZXingReader did not expose {key}")))
}
