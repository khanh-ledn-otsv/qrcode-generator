//! Exact-base URL validation and deterministic query-parameter composition.

use std::collections::HashSet;

use url::{Url, form_urlencoded};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UrlPayloadError {
    InvalidBaseUrl,
    MissingParameterName,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct QueryParameter<'a> {
    pub name: &'a str,
    pub value: &'a str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CustomParameter {
    id: u64,
    name: String,
    value: String,
}

impl CustomParameter {
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UrlPayloadState {
    base_url: String,
    utm_enabled: bool,
    utm_source: String,
    utm_medium: String,
    utm_campaign: String,
    custom_parameters: Vec<CustomParameter>,
    next_parameter_id: u64,
}

impl Default for UrlPayloadState {
    fn default() -> Self {
        Self {
            base_url: String::new(),
            utm_enabled: true,
            utm_source: String::new(),
            utm_medium: String::new(),
            utm_campaign: String::new(),
            custom_parameters: Vec::new(),
            next_parameter_id: 0,
        }
    }
}

impl UrlPayloadState {
    #[must_use]
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn set_base_url(&mut self, value: String) {
        self.base_url = value;
    }

    #[must_use]
    pub const fn utm_enabled(&self) -> bool {
        self.utm_enabled
    }

    pub const fn set_utm_enabled(&mut self, enabled: bool) {
        self.utm_enabled = enabled;
    }

    #[must_use]
    pub fn utm_source(&self) -> &str {
        &self.utm_source
    }

    pub fn set_utm_source(&mut self, value: String) {
        self.utm_source = value;
    }

    #[must_use]
    pub fn utm_medium(&self) -> &str {
        &self.utm_medium
    }

    pub fn set_utm_medium(&mut self, value: String) {
        self.utm_medium = value;
    }

    #[must_use]
    pub fn utm_campaign(&self) -> &str {
        &self.utm_campaign
    }

    pub fn set_utm_campaign(&mut self, value: String) {
        self.utm_campaign = value;
    }

    #[must_use]
    pub fn custom_parameters(&self) -> &[CustomParameter] {
        &self.custom_parameters
    }

    pub fn add_custom_parameter(&mut self) -> Option<u64> {
        let id = self.next_parameter_id;
        self.next_parameter_id = self.next_parameter_id.checked_add(1)?;
        self.custom_parameters.push(CustomParameter {
            id,
            name: String::new(),
            value: String::new(),
        });
        Some(id)
    }

    pub fn set_custom_parameter_name(&mut self, id: u64, name: String) -> bool {
        let Some(parameter) = self
            .custom_parameters
            .iter_mut()
            .find(|parameter| parameter.id == id)
        else {
            return false;
        };
        parameter.name = name;
        true
    }

    pub fn set_custom_parameter_value(&mut self, id: u64, value: String) -> bool {
        let Some(parameter) = self
            .custom_parameters
            .iter_mut()
            .find(|parameter| parameter.id == id)
        else {
            return false;
        };
        parameter.value = value;
        true
    }

    pub fn remove_custom_parameter(&mut self, id: u64) -> bool {
        let original_length = self.custom_parameters.len();
        self.custom_parameters
            .retain(|parameter| parameter.id != id);
        self.custom_parameters.len() != original_length
    }

    pub fn compose(&self) -> Result<String, UrlPayloadError> {
        let mut parameters = Vec::with_capacity(3 + self.custom_parameters.len());
        if self.utm_enabled {
            parameters.extend([
                QueryParameter {
                    name: "utm_source",
                    value: &self.utm_source,
                },
                QueryParameter {
                    name: "utm_medium",
                    value: &self.utm_medium,
                },
                QueryParameter {
                    name: "utm_campaign",
                    value: &self.utm_campaign,
                },
            ]);
        }
        parameters.extend(
            self.custom_parameters
                .iter()
                .map(|parameter| QueryParameter {
                    name: &parameter.name,
                    value: &parameter.value,
                }),
        );
        compose_url(&self.base_url, parameters)
    }
}

pub fn compose_url<'a>(
    base_url: &str,
    parameters: impl IntoIterator<Item = QueryParameter<'a>>,
) -> Result<String, UrlPayloadError> {
    let parsed = Url::parse(base_url).map_err(|_| UrlPayloadError::InvalidBaseUrl)?;
    if !matches!(parsed.scheme(), "http" | "https")
        || parsed.host_str().is_none()
        || !has_explicit_authority(base_url, parsed.scheme())
    {
        return Err(UrlPayloadError::InvalidBaseUrl);
    }

    let mut known_names = parsed
        .query_pairs()
        .map(|(name, _)| name.into_owned())
        .collect::<HashSet<_>>();
    let mut additions = Vec::new();
    for parameter in parameters {
        if parameter.name.is_empty() {
            if parameter.value.is_empty() {
                continue;
            }
            return Err(UrlPayloadError::MissingParameterName);
        }
        if parameter.value.is_empty() || !known_names.insert(parameter.name.to_owned()) {
            continue;
        }
        additions.push(parameter);
    }

    if additions.is_empty() {
        return Ok(base_url.to_owned());
    }

    let (without_fragment, fragment) = base_url
        .split_once('#')
        .map_or((base_url, None), |(before, after)| (before, Some(after)));
    let mut result = without_fragment.to_owned();
    if !without_fragment.ends_with(['?', '&']) {
        result.push(if parsed.query().is_some() { '&' } else { '?' });
    }
    let mut serializer = form_urlencoded::Serializer::new(String::new());
    for parameter in additions {
        serializer.append_pair(parameter.name, parameter.value);
    }
    result.push_str(&serializer.finish());
    if let Some(fragment) = fragment {
        result.push('#');
        result.push_str(fragment);
    }
    Ok(result)
}

fn has_explicit_authority(base_url: &str, scheme: &str) -> bool {
    let Some(authority_and_rest) = base_url
        .strip_prefix(scheme)
        .and_then(|rest| rest.strip_prefix("://"))
    else {
        return false;
    };
    authority_and_rest
        .split(['/', '?', '#'])
        .next()
        .is_some_and(|authority| !authority.is_empty())
}

#[cfg(test)]
mod tests {
    use super::{QueryParameter, UrlPayloadError, UrlPayloadState, compose_url};

    #[test]
    fn accepts_http_urls_and_appends_before_the_exact_fragment() {
        assert_eq!(
            compose_url(
                "https://example.test/path?existing=1#section%202",
                [QueryParameter {
                    name: "utm_source",
                    value: "QR campaign",
                }],
            ),
            Ok(
                "https://example.test/path?existing=1&utm_source=QR+campaign#section%202"
                    .to_owned()
            )
        );
    }

    #[test]
    fn rejects_non_web_or_hostless_urls_without_trimming() {
        for value in [
            "mailto:team@example.test",
            "https:///path",
            "example.test/path",
            " https://example.test",
        ] {
            assert_eq!(compose_url(value, []), Err(UrlPayloadError::InvalidBaseUrl));
        }
    }

    #[test]
    fn existing_decoded_names_win_and_added_names_are_unique() {
        assert_eq!(
            compose_url(
                "https://example.test/?utm%5Fsource=existing",
                [
                    QueryParameter {
                        name: "utm_source",
                        value: "ignored",
                    },
                    QueryParameter {
                        name: "custom",
                        value: "first",
                    },
                    QueryParameter {
                        name: "custom",
                        value: "second",
                    },
                ],
            ),
            Ok("https://example.test/?utm%5Fsource=existing&custom=first".to_owned())
        );
    }

    #[test]
    fn blank_values_are_omitted_and_value_without_name_is_invalid() {
        assert_eq!(
            compose_url(
                "http://example.test",
                [QueryParameter {
                    name: "utm_medium",
                    value: "",
                }],
            ),
            Ok("http://example.test".to_owned())
        );
        assert_eq!(
            compose_url(
                "http://example.test",
                [QueryParameter {
                    name: "",
                    value: "orphan",
                }],
            ),
            Err(UrlPayloadError::MissingParameterName)
        );
    }

    #[test]
    fn preserves_bare_query_delimiters_and_encodes_reserved_characters() {
        assert_eq!(
            compose_url(
                "https://example.test/path?",
                [QueryParameter {
                    name: "next page",
                    value: "/home?a=1&b=2",
                }],
            ),
            Ok("https://example.test/path?next+page=%2Fhome%3Fa%3D1%26b%3D2".to_owned())
        );
    }

    #[test]
    fn owned_state_preserves_disabled_utm_values_and_custom_row_order() {
        let mut state = UrlPayloadState::default();
        state.set_base_url("https://example.test/path#part".to_owned());
        state.set_utm_source("newsletter".to_owned());
        state.set_utm_medium("email".to_owned());
        let first = state.add_custom_parameter().unwrap();
        let second = state.add_custom_parameter().unwrap();
        assert!(state.set_custom_parameter_name(first, "audience".to_owned()));
        assert!(state.set_custom_parameter_value(first, "staff".to_owned()));
        assert!(state.set_custom_parameter_name(second, "lang".to_owned()));
        assert!(state.set_custom_parameter_value(second, "en".to_owned()));
        assert_eq!(
            state.compose(),
            Ok("https://example.test/path?utm_source=newsletter&utm_medium=email&audience=staff&lang=en#part".to_owned())
        );

        state.set_utm_enabled(false);
        assert_eq!(
            state.compose(),
            Ok("https://example.test/path?audience=staff&lang=en#part".to_owned())
        );
        assert_eq!(state.utm_source(), "newsletter");
        assert!(state.remove_custom_parameter(first));
        assert_eq!(
            state.compose(),
            Ok("https://example.test/path?lang=en#part".to_owned())
        );
    }
}
