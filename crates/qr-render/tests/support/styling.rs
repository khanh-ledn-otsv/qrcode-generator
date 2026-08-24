use std::error::Error;

use qr_core::tables::ErrorCorrection;
use qr_core::{EncodeRequest, EncodedQr, encode};
use qr_render::{
    APPROVED_FOREGROUND_THEMES, APPROVED_LOGO_STYLES, BRANDED_LOGO_VERSION, ForegroundTheme,
    LogoPlacement, LogoStyle, OutputProfile, OutputSafety, RenderError, RenderModel, RenderOptions,
    SUPPORTED_PROFILES,
};
use sha2::{Digest, Sha256};

#[derive(Clone, Copy)]
pub struct ApprovedStyleTuple {
    pub profile_index: usize,
    pub logo_index: usize,
    pub foreground_index: usize,
    pub profile: OutputProfile,
    pub logo: LogoStyle,
    pub foreground: ForegroundTheme,
}

impl ApprovedStyleTuple {
    pub fn options(self) -> Result<RenderOptions, RenderError> {
        RenderOptions::safe(self.profile)?
            .with_logo(self.logo)?
            .with_foreground_theme(self.foreground)
    }

    pub const fn expected_safety(self) -> OutputSafety {
        match self.logo {
            LogoStyle::Bundled => OutputSafety::Caution,
            LogoStyle::None => OutputSafety::Safe,
        }
    }

    pub const fn ecc(self) -> ErrorCorrection {
        match self.logo {
            LogoStyle::None => ErrorCorrection::Medium,
            LogoStyle::Bundled => ErrorCorrection::High,
        }
    }

    pub fn label(self) -> String {
        format!(
            "{}/{}/{}",
            self.profile_index, self.logo_index, self.foreground_index
        )
    }
}

pub fn approved_style_tuples() -> Vec<ApprovedStyleTuple> {
    let mut tuples = Vec::new();
    for (profile_index, profile) in SUPPORTED_PROFILES.into_iter().enumerate() {
        for (logo_index, logo) in APPROVED_LOGO_STYLES.into_iter().enumerate() {
            for (foreground_index, foreground) in APPROVED_FOREGROUND_THEMES.into_iter().enumerate()
            {
                tuples.push(ApprovedStyleTuple {
                    profile_index,
                    logo_index,
                    foreground_index,
                    profile,
                    logo,
                    foreground,
                });
            }
        }
    }
    tuples
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PayloadClass {
    ShortUrl,
    DenseUrl,
    Numeric,
    Alphanumeric,
    AsciiByte,
    Utf8Eci26,
}

impl PayloadClass {
    pub const fn label(self) -> &'static str {
        match self {
            Self::ShortUrl => "short-url",
            Self::DenseUrl => "dense-url",
            Self::Numeric => "numeric",
            Self::Alphanumeric => "alphanumeric",
            Self::AsciiByte => "ascii-byte",
            Self::Utf8Eci26 => "utf8-eci26",
        }
    }
}

pub const REQUIRED_PAYLOAD_CLASSES: [PayloadClass; 6] = [
    PayloadClass::ShortUrl,
    PayloadClass::DenseUrl,
    PayloadClass::Numeric,
    PayloadClass::Alphanumeric,
    PayloadClass::AsciiByte,
    PayloadClass::Utf8Eci26,
];

pub struct DecodeCase {
    pub kind: MatrixCaseKind,
    pub class: PayloadClass,
    pub label: String,
    pub text: String,
    pub eci_assignment: Option<u32>,
    pub expected_version: Option<u8>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MatrixCaseKind {
    RequiredPayload,
    VersionCoverage,
}

impl MatrixCaseKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::RequiredPayload => "required-payload",
            Self::VersionCoverage => "version-coverage",
        }
    }
}

pub struct PreparedDecodeCase {
    pub label: String,
    pub case_label: String,
    pub tuple: ApprovedStyleTuple,
    pub payload_class: PayloadClass,
    pub case_kind: MatrixCaseKind,
    pub payload: Vec<u8>,
    pub eci_assignment: Option<u32>,
    pub ecc: ErrorCorrection,
    pub encoded: EncodedQr,
    pub options: RenderOptions,
    pub logo_placement: Option<LogoPlacement>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CombinationOutcome {
    Renderable { safety: OutputSafety },
    ExpectedInvalid { error: RenderError },
}

impl CombinationOutcome {
    pub const fn is_renderable(self) -> bool {
        matches!(self, Self::Renderable { .. })
    }

    pub const fn is_expected_invalid(self) -> bool {
        matches!(self, Self::ExpectedInvalid { .. })
    }

    pub const fn safety(self) -> Option<OutputSafety> {
        match self {
            Self::Renderable { safety } => Some(safety),
            Self::ExpectedInvalid { .. } => None,
        }
    }

    pub const fn expected_error(self) -> Option<RenderError> {
        match self {
            Self::Renderable { .. } => None,
            Self::ExpectedInvalid { error } => Some(error),
        }
    }
}

pub struct ApprovedCombinationRecord {
    pub tuple: ApprovedStyleTuple,
    pub payload_class: PayloadClass,
    pub case_kind: MatrixCaseKind,
    pub case_label: String,
    pub version: Option<u8>,
    pub logo_placement: Option<LogoPlacement>,
    pub outcome: CombinationOutcome,
}

impl ApprovedCombinationRecord {
    pub fn label(&self) -> String {
        format!(
            "{}-{}",
            self.tuple.label().replace('/', "-"),
            self.case_label
        )
    }
}

pub fn decoded_evidence(
    case: &PreparedDecodeCase,
    format: &'static str,
    artifact_sha256: String,
    decoder_input_sha256: String,
) -> serde_json::Value {
    let metadata = EvidenceMetadata {
        id: &case.label,
        case_label: &case.case_label,
        tuple: case.tuple,
        payload_class: case.payload_class,
        case_kind: case.case_kind,
        version: Some(case.encoded.version().number()),
        safety: Some(case.options.safety()),
        logo_placement: case.logo_placement,
    };
    evidence_row(
        metadata,
        serde_json::json!({
            "format": format,
            "outcome": "decoded",
            "sha256": artifact_sha256,
            "decoder_input_sha256": decoder_input_sha256,
        }),
    )
}

pub fn invalid_evidence(
    record: &ApprovedCombinationRecord,
    format: &'static str,
    error: RenderError,
) -> serde_json::Value {
    let id = record.label();
    let metadata = EvidenceMetadata {
        id: &id,
        case_label: &record.case_label,
        tuple: record.tuple,
        payload_class: record.payload_class,
        case_kind: record.case_kind,
        version: record.version,
        safety: None,
        logo_placement: None,
    };
    evidence_row(
        metadata,
        serde_json::json!({
            "format": format,
            "outcome": "expected-invalid",
            "error": error.to_string(),
            "sha256": null,
            "decoder_input_sha256": null,
        }),
    )
}

struct EvidenceMetadata<'a> {
    id: &'a str,
    case_label: &'a str,
    tuple: ApprovedStyleTuple,
    payload_class: PayloadClass,
    case_kind: MatrixCaseKind,
    version: Option<u8>,
    safety: Option<OutputSafety>,
    logo_placement: Option<LogoPlacement>,
}

fn evidence_row(metadata: EvidenceMetadata<'_>, artifact: serde_json::Value) -> serde_json::Value {
    let EvidenceMetadata {
        id,
        case_label,
        tuple,
        payload_class,
        case_kind,
        version,
        safety,
        logo_placement,
    } = metadata;
    let profile = tuple.profile;
    let selected_version = version.and_then(|number| qr_core::Version::new(number).ok());
    let svg_dimensions = selected_version
        .and_then(|selected| profile.svg_dimensions_for(selected).ok())
        .unwrap_or_else(|| profile.svg_dimensions());
    let png_dimensions = selected_version
        .and_then(|selected| profile.png_dimensions_for(selected).ok())
        .unwrap_or_else(|| profile.png_dimensions());
    serde_json::json!({
        "id": id,
        "case_kind": case_kind.label(),
        "case_label": case_label,
        "profile_index": tuple.profile_index,
        "profile": format!("{:?}", profile.id()),
        "logo_state_index": tuple.logo_index,
        "foreground_index": tuple.foreground_index,
        "foreground": format!("{:?}", tuple.foreground),
        "payload_class": payload_class.label(),
        "ecc": format!("{:?}", tuple.ecc()),
        "version": version,
        "matrix_modules": version.map(|number| 17 + u16::from(number) * 4),
        "svg_dimensions": [svg_dimensions.width().get(), svg_dimensions.height().get()],
        "png_dimensions": [png_dimensions.width().get(), png_dimensions.height().get()],
        "safety": safety.map(safety_label),
        "logo_geometry": logo_placement.map(logo_geometry),
        "artifact": artifact,
    })
}

fn logo_geometry(placement: LogoPlacement) -> serde_json::Value {
    let source = placement.source_bounds();
    let knockout = placement.knockout_bounds();
    serde_json::json!({
        "source_ten_thousandths": [
            source.left_ten_thousandths(),
            source.top_ten_thousandths(),
            source.width_ten_thousandths(),
            source.height_ten_thousandths(),
        ],
        "knockout_modules": [
            knockout.left().get(),
            knockout.top().get(),
            knockout.width().get(),
            knockout.height().get(),
        ],
        "protected_clearance_modules": placement.protected_clearance(),
        "obscured_data_modules": placement.obscured_data_modules(),
        "obscured_remainder_modules": placement.obscured_remainder_modules(),
    })
}

const fn safety_label(safety: OutputSafety) -> &'static str {
    match safety {
        OutputSafety::Safe => "safe",
        OutputSafety::Caution => "caution",
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub fn approved_combination_records() -> Result<Vec<ApprovedCombinationRecord>, Box<dyn Error>> {
    let mut records = Vec::new();
    for tuple in approved_style_tuples() {
        for case in matrix_cases(tuple)? {
            let payload_class = case.class;
            let case_kind = case.kind;
            let case_label = case.label.clone();
            let requested_version = case.expected_version;
            let (outcome, version, logo_placement) = match prepare_decode_case(tuple, case) {
                Ok(prepared) => (
                    CombinationOutcome::Renderable {
                        safety: tuple.expected_safety(),
                    },
                    Some(prepared.encoded.version().number()),
                    prepared.logo_placement,
                ),
                Err(error) if matches!(error, RenderError::UnsafeLogoGeometry) => (
                    CombinationOutcome::ExpectedInvalid { error },
                    requested_version,
                    None,
                ),
                Err(error) => return Err(error.into()),
            };
            records.push(ApprovedCombinationRecord {
                tuple,
                payload_class,
                case_kind,
                case_label,
                version,
                logo_placement,
                outcome,
            });
        }
    }
    Ok(records)
}

pub fn approved_decode_cases() -> Result<Vec<PreparedDecodeCase>, Box<dyn Error>> {
    let mut prepared = Vec::new();
    for tuple in approved_style_tuples() {
        for case in matrix_cases(tuple)? {
            match prepare_decode_case(tuple, case) {
                Ok(case) => prepared.push(case),
                Err(RenderError::UnsafeLogoGeometry) => {}
                Err(error) => return Err(error.into()),
            }
        }
    }

    Ok(prepared)
}

pub fn representative_decode_cases() -> Result<Vec<PreparedDecodeCase>, Box<dyn Error>> {
    let mut prepared = Vec::new();
    for tuple in approved_style_tuples() {
        prepared.push(prepare_decode_case(tuple, short_url_case())?);
    }
    Ok(prepared)
}

fn matrix_cases(tuple: ApprovedStyleTuple) -> Result<Vec<DecodeCase>, Box<dyn Error>> {
    let mut cases = required_payload_cases(tuple.profile, tuple.ecc())?;
    for version in 1..=tuple.profile.maximum_version().number() {
        cases.push(DecodeCase {
            kind: MatrixCaseKind::VersionCoverage,
            class: PayloadClass::AsciiByte,
            label: format!("version-v{version}"),
            text: "a".repeat(crate::versions::first_byte_length_at_ecc(
                version,
                tuple.ecc(),
            )),
            eci_assignment: None,
            expected_version: Some(version),
        });
    }
    Ok(cases)
}

fn prepare_decode_case(
    tuple: ApprovedStyleTuple,
    case: DecodeCase,
) -> Result<PreparedDecodeCase, RenderError> {
    let payload_class = case.class;
    let case_kind = case.kind;
    let case_label = case.label.clone();
    let options = tuple.options()?;
    let request = if let Some(version) = case.expected_version {
        let version =
            qr_core::Version::try_from(version).map_err(|_| RenderError::RenderFailure)?;
        EncodeRequest::with_version_range(&case.text, tuple.ecc(), version, version)
    } else if tuple.logo == LogoStyle::Bundled
        && tuple.profile.maximum_version() >= BRANDED_LOGO_VERSION
    {
        EncodeRequest::with_version_range(
            &case.text,
            tuple.ecc(),
            BRANDED_LOGO_VERSION,
            tuple.profile.maximum_version(),
        )
    } else {
        EncodeRequest::first_fit(&case.text, tuple.ecc(), tuple.profile.maximum_version())
    };
    let encoded = encode(request).map_err(|_| RenderError::RenderFailure)?;
    if case
        .expected_version
        .is_some_and(|version| encoded.version().number() != version)
    {
        return Err(RenderError::RenderFailure);
    }
    if options.safety() != tuple.expected_safety() {
        return Err(RenderError::RenderFailure);
    }
    let model = RenderModel::new(&encoded, options)?;
    let logo_placement = model.logo_placement();
    Ok(PreparedDecodeCase {
        label: format!("{}-{}", tuple.label().replace('/', "-"), case.label),
        case_label,
        tuple,
        payload_class,
        case_kind,
        payload: case.text.into_bytes(),
        eci_assignment: case.eci_assignment,
        ecc: tuple.ecc(),
        encoded,
        options,
        logo_placement,
    })
}

pub fn required_payload_cases(
    profile: OutputProfile,
    ecc: ErrorCorrection,
) -> Result<Vec<DecodeCase>, Box<dyn Error>> {
    let dense_url = dense_url_at_profile_ceiling(profile, ecc)?;
    Ok(vec![
        short_url_case(),
        DecodeCase {
            kind: MatrixCaseKind::RequiredPayload,
            class: PayloadClass::DenseUrl,
            label: PayloadClass::DenseUrl.label().to_owned(),
            text: dense_url,
            eci_assignment: None,
            expected_version: Some(profile.maximum_version().number()),
        },
        DecodeCase {
            kind: MatrixCaseKind::RequiredPayload,
            class: PayloadClass::Numeric,
            label: PayloadClass::Numeric.label().to_owned(),
            text: "12345678901234567890".to_owned(),
            eci_assignment: None,
            expected_version: None,
        },
        DecodeCase {
            kind: MatrixCaseKind::RequiredPayload,
            class: PayloadClass::Alphanumeric,
            label: PayloadClass::Alphanumeric.label().to_owned(),
            text: "APPROVED OUTPUT 123".to_owned(),
            eci_assignment: None,
            expected_version: None,
        },
        DecodeCase {
            kind: MatrixCaseKind::RequiredPayload,
            class: PayloadClass::AsciiByte,
            label: PayloadClass::AsciiByte.label().to_owned(),
            text: "lowercase-ascii-output".to_owned(),
            eci_assignment: None,
            expected_version: None,
        },
        DecodeCase {
            kind: MatrixCaseKind::RequiredPayload,
            class: PayloadClass::Utf8Eci26,
            label: PayloadClass::Utf8Eci26.label().to_owned(),
            text: "café output".to_owned(),
            eci_assignment: Some(26),
            expected_version: None,
        },
    ])
}

fn short_url_case() -> DecodeCase {
    DecodeCase {
        kind: MatrixCaseKind::RequiredPayload,
        class: PayloadClass::ShortUrl,
        label: PayloadClass::ShortUrl.label().to_owned(),
        text: "https://example.test/a".to_owned(),
        eci_assignment: None,
        expected_version: None,
    }
}

fn dense_url_at_profile_ceiling(
    profile: OutputProfile,
    ecc: ErrorCorrection,
) -> Result<String, Box<dyn Error>> {
    let prefix = "https://example.test/";
    let maximum_suffix = 4_096 - prefix.len();
    let mut first_rejected = maximum_suffix + 1;
    let mut first_unknown = 0;
    while first_unknown < first_rejected {
        let suffix_length = first_unknown + (first_rejected - first_unknown) / 2;
        let candidate = format!("{prefix}{}", "a".repeat(suffix_length));
        if encode(EncodeRequest::first_fit(
            &candidate,
            ecc,
            profile.maximum_version(),
        ))
        .is_ok()
        {
            first_unknown = suffix_length + 1;
        } else {
            first_rejected = suffix_length;
        }
    }
    let fitting_suffix = first_unknown
        .checked_sub(1)
        .ok_or("profile cannot fit the dense URL prefix")?;
    let text = format!("{prefix}{}", "a".repeat(fitting_suffix));
    let encoded = encode(EncodeRequest::first_fit(
        &text,
        ecc,
        profile.maximum_version(),
    ))?;
    if encoded.version() != profile.maximum_version() {
        return Err("dense URL did not select the profile ceiling version".into());
    }
    Ok(text)
}
