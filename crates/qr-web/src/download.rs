//! Browser download boundary for deterministic QR artifacts.

use std::error::Error;
use std::fmt;

use js_sys::{Array, Uint8Array};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{Blob, BlobPropertyBag, HtmlAnchorElement, Url};

use crate::workflow::DownloadArtifact;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DownloadError {
    BlobCreation,
    ObjectUrlCreation,
    BrowserUnavailable,
    DocumentUnavailable,
    DownloadLinkCreation,
    DomOperation,
    ObjectUrlRevocation,
}

impl fmt::Display for DownloadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::BlobCreation => "the browser could not create the download file",
            Self::ObjectUrlCreation => "the browser could not prepare the download URL",
            Self::BrowserUnavailable => "the browser window is unavailable",
            Self::DocumentUnavailable => "the browser document is unavailable",
            Self::DownloadLinkCreation => "the browser could not create the download control",
            Self::DomOperation => "the browser could not start the download",
            Self::ObjectUrlRevocation => "the browser could not release the download URL",
        })
    }
}

impl Error for DownloadError {}

pub fn create_blob(artifact: DownloadArtifact<'_>) -> Result<Blob, DownloadError> {
    create_blob_with(artifact, Blob::new_with_u8_array_sequence_and_options)
}

fn create_blob_with(
    artifact: DownloadArtifact<'_>,
    create: impl FnOnce(&JsValue, &BlobPropertyBag) -> Result<Blob, JsValue>,
) -> Result<Blob, DownloadError> {
    let parts = Array::new();
    let bytes = Uint8Array::from(artifact.bytes());
    parts.push(bytes.as_ref());
    let options = BlobPropertyBag::new();
    options.set_type(artifact.mime_type());
    create(parts.as_ref(), &options).map_err(|_| DownloadError::BlobCreation)
}

#[derive(Debug)]
pub struct ObjectUrl {
    value: Option<String>,
}

impl ObjectUrl {
    pub fn new(blob: &Blob) -> Result<Self, DownloadError> {
        let value =
            Url::create_object_url_with_blob(blob).map_err(|_| DownloadError::ObjectUrlCreation)?;
        Ok(Self { value: Some(value) })
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.value.as_deref().unwrap_or("")
    }

    pub fn revoke(mut self) -> Result<(), DownloadError> {
        let Some(value) = self.value.take() else {
            return Ok(());
        };
        Url::revoke_object_url(&value).map_err(|_| DownloadError::ObjectUrlRevocation)
    }
}

impl Drop for ObjectUrl {
    fn drop(&mut self) {
        if let Some(value) = self.value.take() {
            _ = Url::revoke_object_url(&value);
        }
    }
}

pub fn trigger_download(artifact: DownloadArtifact<'_>) -> Result<(), DownloadError> {
    let blob = create_blob(artifact)?;
    let object_url = ObjectUrl::new(&blob)?;
    let window = web_sys::window().ok_or(DownloadError::BrowserUnavailable)?;
    let document = window
        .document()
        .ok_or(DownloadError::DocumentUnavailable)?;
    let body = document.body().ok_or(DownloadError::DocumentUnavailable)?;
    let anchor = document
        .create_element("a")
        .map_err(|_| DownloadError::DownloadLinkCreation)?
        .dyn_into::<HtmlAnchorElement>()
        .map_err(|_| DownloadError::DownloadLinkCreation)?;
    anchor.set_href(object_url.as_str());
    anchor.set_download(artifact.filename());
    anchor.set_hidden(true);
    body.append_child(&anchor)
        .map_err(|_| DownloadError::DomOperation)?;
    anchor.click();
    body.remove_child(&anchor)
        .map_err(|_| DownloadError::DomOperation)?;
    object_url.revoke()
}

#[cfg(all(test, target_arch = "wasm32"))]
mod tests {
    use qr_render::ProfileId;
    use wasm_bindgen::JsValue;
    use wasm_bindgen_test::wasm_bindgen_test;

    use super::{DownloadError, create_blob_with};
    use crate::workflow::{ArtifactKind, WorkflowState, evaluate_preview};

    #[wasm_bindgen_test]
    fn blob_constructor_errors_become_typed_failures_without_panicking() {
        let mut state = WorkflowState::new(ProfileId::Inline);
        let request = state
            .set_payload("blob error".to_owned())
            .expect("revision is available");
        assert!(state.complete_preview(request.revision(), evaluate_preview(&request)));
        let artifact = state
            .preview()
            .expect("valid payload has artifacts")
            .artifact(ArtifactKind::Png);

        let error = create_blob_with(artifact, |_, _| Err(JsValue::from_str("synthetic failure")))
            .expect_err("synthetic constructor failure is mapped");
        assert_eq!(error, DownloadError::BlobCreation);
    }
}
