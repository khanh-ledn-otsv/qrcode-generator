use std::error::Error;

use qr_core::tables::ErrorCorrection;
use qr_core::{EncodeRequest, EncodedQr, encode};
use qr_render::{
    APPROVED_BACKGROUNDS, APPROVED_FINDERS, APPROVED_FOREGROUNDS, APPROVED_LOGO_STYLES,
    APPROVED_MODULE_STYLES, Background, FinderStyle, Foreground, LogoStyle, ModuleStyle,
    OutputProfile, OutputSafety, RenderError, RenderModel, RenderOptions, SUPPORTED_PROFILES,
};

#[derive(Clone, Copy)]
pub struct ApprovedStyleTuple {
    pub profile_index: usize,
    pub foreground_index: usize,
    pub background_index: usize,
    pub function_style_index: usize,
    pub finder_index: usize,
    pub logo_index: usize,
    pub profile: OutputProfile,
    pub foreground: Foreground,
    pub background: Background,
    pub function_style: ModuleStyle,
    pub finder: FinderStyle,
    pub logo: LogoStyle,
}

impl ApprovedStyleTuple {
    pub fn options(self) -> Result<RenderOptions, RenderError> {
        let options = RenderOptions::approved(self.profile, self.foreground, self.background)?
            .with_logo(self.logo)?;
        if options.module_style() != self.function_style || options.finder_style() != self.finder {
            return Err(RenderError::RenderFailure);
        }
        Ok(options)
    }

    pub const fn expected_safety(self) -> Option<OutputSafety> {
        match (self.logo, self.background) {
            (LogoStyle::Bundled, Background::Transparent) => None,
            (LogoStyle::Bundled, Background::Opaque(_))
            | (LogoStyle::None, Background::Transparent) => Some(OutputSafety::Caution),
            (LogoStyle::None, Background::Opaque(_)) => Some(OutputSafety::Safe),
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
            "{}/{}/{}/{}/{}/{}",
            self.profile_index,
            self.foreground_index,
            self.background_index,
            self.function_style_index,
            self.finder_index,
            self.logo_index,
        )
    }
}

pub fn approved_style_tuples() -> Vec<ApprovedStyleTuple> {
    let mut tuples = Vec::new();
    for (profile_index, profile) in SUPPORTED_PROFILES.into_iter().enumerate() {
        for (foreground_index, foreground) in APPROVED_FOREGROUNDS.into_iter().enumerate() {
            for (background_index, background) in APPROVED_BACKGROUNDS.into_iter().enumerate() {
                for (function_style_index, function_style) in
                    APPROVED_MODULE_STYLES.into_iter().enumerate()
                {
                    for (finder_index, finder) in APPROVED_FINDERS.into_iter().enumerate() {
                        for (logo_index, logo) in APPROVED_LOGO_STYLES.into_iter().enumerate() {
                            tuples.push(ApprovedStyleTuple {
                                profile_index,
                                foreground_index,
                                background_index,
                                function_style_index,
                                finder_index,
                                logo_index,
                                profile,
                                foreground,
                                background,
                                function_style,
                                finder,
                                logo,
                            });
                        }
                    }
                }
            }
        }
    }
    tuples
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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
    pub class: PayloadClass,
    pub label: String,
    pub text: String,
    pub eci_assignment: Option<u32>,
    pub expected_version: Option<u8>,
}

pub struct PreparedDecodeCase {
    pub label: String,
    pub tuple: ApprovedStyleTuple,
    pub payload_class: PayloadClass,
    pub payload: Vec<u8>,
    pub eci_assignment: Option<u32>,
    pub ecc: ErrorCorrection,
    pub encoded: EncodedQr,
    pub options: RenderOptions,
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
    pub outcome: CombinationOutcome,
}

impl ApprovedCombinationRecord {
    pub fn label(&self) -> String {
        format!(
            "{}-{}",
            self.tuple.label().replace('/', "-"),
            self.payload_class.label()
        )
    }
}

pub fn approved_combination_records() -> Result<Vec<ApprovedCombinationRecord>, Box<dyn Error>> {
    let mut records = Vec::new();
    for tuple in approved_style_tuples() {
        for case in required_payload_cases(tuple.profile, tuple.ecc())? {
            let payload_class = case.class;
            let outcome = match prepare_decode_case(tuple, case) {
                Ok(_) => CombinationOutcome::Renderable {
                    safety: tuple
                        .expected_safety()
                        .ok_or("renderable tuple has no safety classification")?,
                },
                Err(error)
                    if matches!(
                        error,
                        RenderError::LogoRequiresOpaqueWhite | RenderError::UnsafeLogoGeometry
                    ) =>
                {
                    CombinationOutcome::ExpectedInvalid { error }
                }
                Err(error) => return Err(error.into()),
            };
            records.push(ApprovedCombinationRecord {
                tuple,
                payload_class,
                outcome,
            });
        }
    }
    Ok(records)
}

pub fn approved_decode_cases() -> Result<Vec<PreparedDecodeCase>, Box<dyn Error>> {
    let mut prepared = Vec::new();
    for tuple in approved_style_tuples() {
        for case in required_payload_cases(tuple.profile, tuple.ecc())? {
            match prepare_decode_case(tuple, case) {
                Ok(case) => prepared.push(case),
                Err(RenderError::LogoRequiresOpaqueWhite | RenderError::UnsafeLogoGeometry) => {}
                Err(error) => return Err(error.into()),
            }
        }
    }

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
            function_style_index: 0,
            finder_index: 0,
            logo_index: 0,
            profile,
            foreground: Foreground::Brand,
            background: APPROVED_BACKGROUNDS[0],
            function_style: APPROVED_MODULE_STYLES[0],
            finder: APPROVED_FINDERS[0],
            logo: LogoStyle::None,
        };
        prepared.push(prepare_decode_case(
            tuple,
            DecodeCase {
                class: PayloadClass::AsciiByte,
                label: format!("transition-version-{version}"),
                text: "a".repeat(crate::versions::first_byte_length(version)),
                eci_assignment: None,
                expected_version: Some(version),
            },
        )?);
    }
    Ok(prepared)
}

fn prepare_decode_case(
    tuple: ApprovedStyleTuple,
    case: DecodeCase,
) -> Result<PreparedDecodeCase, RenderError> {
    let payload_class = case.class;
    let options = tuple.options()?;
    let encoded = encode(EncodeRequest {
        text: &case.text,
        ecc: tuple.ecc(),
        max_version: tuple.profile.maximum_version(),
    })
    .map_err(|_| RenderError::RenderFailure)?;
    if case
        .expected_version
        .is_some_and(|version| encoded.version().number() != version)
    {
        return Err(RenderError::RenderFailure);
    }
    if options.safety() != tuple.expected_safety().ok_or(RenderError::RenderFailure)? {
        return Err(RenderError::RenderFailure);
    }
    RenderModel::new(&encoded, options)?;
    Ok(PreparedDecodeCase {
        label: format!("{}-{}", tuple.label().replace('/', "-"), case.label),
        tuple,
        payload_class,
        payload: case.text.into_bytes(),
        eci_assignment: case.eci_assignment,
        ecc: tuple.ecc(),
        encoded,
        options,
    })
}

pub fn required_payload_cases(
    profile: OutputProfile,
    ecc: ErrorCorrection,
) -> Result<Vec<DecodeCase>, Box<dyn Error>> {
    let dense_url = dense_url_at_profile_ceiling(profile, ecc)?;
    Ok(vec![
        DecodeCase {
            class: PayloadClass::ShortUrl,
            label: PayloadClass::ShortUrl.label().to_owned(),
            text: "https://example.test/a".to_owned(),
            eci_assignment: None,
            expected_version: None,
        },
        DecodeCase {
            class: PayloadClass::DenseUrl,
            label: PayloadClass::DenseUrl.label().to_owned(),
            text: dense_url,
            eci_assignment: None,
            expected_version: Some(profile.maximum_version().number()),
        },
        DecodeCase {
            class: PayloadClass::Numeric,
            label: PayloadClass::Numeric.label().to_owned(),
            text: "12345678901234567890".to_owned(),
            eci_assignment: None,
            expected_version: None,
        },
        DecodeCase {
            class: PayloadClass::Alphanumeric,
            label: PayloadClass::Alphanumeric.label().to_owned(),
            text: "APPROVED OUTPUT 123".to_owned(),
            eci_assignment: None,
            expected_version: None,
        },
        DecodeCase {
            class: PayloadClass::AsciiByte,
            label: PayloadClass::AsciiByte.label().to_owned(),
            text: "lowercase-ascii-output".to_owned(),
            eci_assignment: None,
            expected_version: None,
        },
        DecodeCase {
            class: PayloadClass::Utf8Eci26,
            label: PayloadClass::Utf8Eci26.label().to_owned(),
            text: "café output".to_owned(),
            eci_assignment: Some(26),
            expected_version: None,
        },
    ])
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
        if encode(EncodeRequest {
            text: &candidate,
            ecc,
            max_version: profile.maximum_version(),
        })
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
    let encoded = encode(EncodeRequest {
        text: &text,
        ecc,
        max_version: profile.maximum_version(),
    })?;
    if encoded.version() != profile.maximum_version() {
        return Err("dense URL did not select the profile ceiling version".into());
    }
    Ok(text)
}
