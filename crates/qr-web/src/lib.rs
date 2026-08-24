//! Browser-independent state and browser UI for the QR workflow.

#![forbid(unsafe_code)]

pub mod debounce;
pub mod preview_worker;
pub mod url_payload;
pub mod workflow;
pub use workflow::worker_protocol;

pub mod download;

mod textarea;
