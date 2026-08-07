use std::error::Error;

use qr_core::tables::ErrorCorrection;
use qr_core::{EncodeRequest, EncodedQr, encode};
use qr_render::{
    APPROVED_BACKGROUNDS, APPROVED_DATA_MODULE_STYLES, APPROVED_FOREGROUNDS, Background,
    DataModuleStyle, Foreground, OutputProfile, OutputSafety, RenderError, RenderOptions,
    SUPPORTED_PROFILES,
};

#[derive(Clone, Copy)]
pub struct ApprovedStyleTuple {
    pub profile_index: usize,
    pub foreground_index: usize,
    pub background_index: usize,
    pub style_index: usize,
    pub profile: OutputProfile,
    pub foreground: Foreground,
    pub background: Background,
    pub style: DataModuleStyle,
}

impl ApprovedStyleTuple {
    pub fn options(&self) -> Result<RenderOptions, RenderError> {
        RenderOptions::approved_with_data_style(
            self.profile,
            self.foreground,
            self.background,
            self.style,
        )
    }

    pub const fn expected_safety(&self) -> OutputSafety {
        match self.background {
            Background::Opaque(_) => OutputSafety::Safe,
            Background::Transparent => OutputSafety::Caution,
        }
    }

    pub fn label(&self) -> String {
        format!(
            "{}/{}/{}/{}",
            self.profile_index, self.foreground_index, self.background_index, self.style_index,
        )
    }
}

pub fn approved_style_tuples() -> Vec<ApprovedStyleTuple> {
    let mut tuples = Vec::new();
    for (profile_index, profile) in SUPPORTED_PROFILES.into_iter().enumerate() {
        for (foreground_index, foreground) in APPROVED_FOREGROUNDS.into_iter().enumerate() {
            for (background_index, background) in APPROVED_BACKGROUNDS.into_iter().enumerate() {
                for (style_index, style) in APPROVED_DATA_MODULE_STYLES.into_iter().enumerate() {
                    tuples.push(ApprovedStyleTuple {
                        profile_index,
                        foreground_index,
                        background_index,
                        style_index,
                        profile,
                        foreground,
                        background,
                        style,
                    });
                }
            }
        }
    }
    tuples
}

pub struct DecodeCase {
    pub label: String,
    pub text: String,
    pub eci_assignment: Option<u32>,
    pub expected_version: Option<u8>,
}

pub struct PreparedDecodeCase {
    pub label: String,
    pub payload: Vec<u8>,
    pub eci_assignment: Option<u32>,
    pub encoded: EncodedQr,
    pub options: RenderOptions,
}

pub fn approved_decode_cases() -> Result<Vec<PreparedDecodeCase>, Box<dyn Error>> {
    let mut prepared = Vec::new();
    for tuple in approved_style_tuples() {
        for case in required_payload_cases(tuple.profile) {
            prepared.push(prepare_decode_case(tuple, case)?);
        }
    }

    for (style_index, style) in APPROVED_DATA_MODULE_STYLES.into_iter().enumerate() {
        for version in [1, 2, 5, 6, 7, 8, 12, 13] {
            let (profile_index, profile) = SUPPORTED_PROFILES
                .into_iter()
                .enumerate()
                .find(|(_, profile)| version <= profile.maximum_version().number())
                .ok_or("transition version has no supporting profile")?;
            let tuple = ApprovedStyleTuple {
                profile_index,
                foreground_index: 0,
                background_index: 0,
                style_index,
                profile,
                foreground: Foreground::Brand,
                background: APPROVED_BACKGROUNDS[0],
                style,
            };
            prepared.push(prepare_decode_case(
                tuple,
                DecodeCase {
                    label: format!("transition-version-{version}"),
                    text: "a".repeat(crate::versions::first_byte_length(version)),
                    eci_assignment: None,
                    expected_version: Some(version),
                },
            )?);
        }
    }
    Ok(prepared)
}

fn prepare_decode_case(
    tuple: ApprovedStyleTuple,
    case: DecodeCase,
) -> Result<PreparedDecodeCase, Box<dyn Error>> {
    let encoded = encode(EncodeRequest {
        text: &case.text,
        ecc: ErrorCorrection::Medium,
        max_version: tuple.profile.maximum_version(),
    })?;
    if case
        .expected_version
        .is_some_and(|version| encoded.version().number() != version)
    {
        return Err(format!(
            "approved tuple {} case {} selected version {} instead of {}",
            tuple.label(),
            case.label,
            encoded.version().number(),
            case.expected_version.unwrap_or_default(),
        )
        .into());
    }
    let options = tuple.options()?;
    if options.safety() != tuple.expected_safety() {
        return Err(format!(
            "approved tuple {} case {} recorded the wrong safety classification",
            tuple.label(),
            case.label,
        )
        .into());
    }
    Ok(PreparedDecodeCase {
        label: format!("{}-{}", tuple.label().replace('/', "-"), case.label),
        payload: case.text.into_bytes(),
        eci_assignment: case.eci_assignment,
        encoded,
        options,
    })
}

pub fn required_payload_cases(profile: OutputProfile) -> Vec<DecodeCase> {
    let dense_prefix = "https://example.test/";
    let dense_version = profile.maximum_version().number().min(12);
    let dense_length = crate::versions::first_byte_length(dense_version);
    let dense_url = format!(
        "{dense_prefix}{}",
        "a".repeat(dense_length - dense_prefix.len())
    );
    vec![
        DecodeCase {
            label: "short-url".to_owned(),
            text: "https://example.test/a".to_owned(),
            eci_assignment: None,
            expected_version: None,
        },
        DecodeCase {
            label: "dense-url".to_owned(),
            text: dense_url,
            eci_assignment: None,
            expected_version: Some(dense_version),
        },
        DecodeCase {
            label: "numeric".to_owned(),
            text: if profile.maximum_version().number() == 13 {
                "1".repeat(160)
            } else {
                "1234567890".to_owned()
            },
            eci_assignment: None,
            expected_version: None,
        },
        DecodeCase {
            label: "alphanumeric".to_owned(),
            text: if profile.maximum_version().number() == 13 {
                "A".repeat(100)
            } else {
                "APPROVED 123".to_owned()
            },
            eci_assignment: None,
            expected_version: None,
        },
        DecodeCase {
            label: "ascii-byte".to_owned(),
            text: if profile.maximum_version().number() == 13 {
                "a".repeat(70)
            } else {
                "lowercase-ascii".to_owned()
            },
            eci_assignment: None,
            expected_version: None,
        },
        DecodeCase {
            label: "utf8-eci26".to_owned(),
            text: if profile.maximum_version().number() == 13 {
                "é".repeat(30)
            } else {
                "café".to_owned()
            },
            eci_assignment: Some(26),
            expected_version: None,
        },
    ]
}
