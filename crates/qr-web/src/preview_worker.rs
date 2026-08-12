//! Lifecycle ownership for the dedicated preview Web Worker.

use crate::worker_protocol::WorkerResponse;
use crate::workflow::PreviewRequest;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PreviewWorkerError {
    Startup,
    Message,
}

#[cfg(target_arch = "wasm32")]
mod browser {
    use std::{cell::RefCell, rc::Rc};

    use super::{PreviewRequest, PreviewWorkerError, WorkerResponse};
    use crate::worker_protocol::WorkerRequest;
    use js_sys::{Reflect, Uint8Array};
    use wasm_bindgen::{JsCast, JsValue, closure::Closure};
    use web_sys::{ErrorEvent, MessageEvent, Worker};

    const WORKER_LOADER: &str = "qr-preview-worker_loader.js";

    pub struct PreviewWorker {
        worker: Worker,
        _on_message: Closure<dyn FnMut(MessageEvent)>,
        _on_error: Closure<dyn FnMut(ErrorEvent)>,
    }

    impl PreviewWorker {
        pub fn new(
            mut on_result: impl FnMut(WorkerResponse) + 'static,
            on_failure: impl FnMut() + 'static,
        ) -> Result<Self, PreviewWorkerError> {
            let worker = Worker::new(WORKER_LOADER).map_err(|_| PreviewWorkerError::Startup)?;
            let on_failure: Rc<RefCell<Box<dyn FnMut()>>> =
                Rc::new(RefCell::new(Box::new(on_failure)));
            let message_failure = Rc::clone(&on_failure);
            let on_message = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
                let data = event.data();
                let metadata = Reflect::get(&data, &"metadata".into())
                    .ok()
                    .and_then(|value| value.as_string());
                let png = Reflect::get(&data, &"png".into())
                    .ok()
                    .and_then(|value| value.dyn_into::<Uint8Array>().ok())
                    .map(|value| value.to_vec());
                let (Some(metadata), Some(png)) = (metadata, png) else {
                    (message_failure.borrow_mut())();
                    return;
                };
                match WorkerResponse::from_message_parts(&metadata, png) {
                    Ok(response) => on_result(response),
                    Err(_) => (message_failure.borrow_mut())(),
                }
            });
            worker.set_onmessage(Some(on_message.as_ref().unchecked_ref()));

            let worker_failure = Rc::clone(&on_failure);
            let on_error = Closure::<dyn FnMut(ErrorEvent)>::new(move |event: ErrorEvent| {
                event.prevent_default();
                (worker_failure.borrow_mut())();
            });
            worker.set_onerror(Some(on_error.as_ref().unchecked_ref()));

            Ok(Self {
                worker,
                _on_message: on_message,
                _on_error: on_error,
            })
        }

        pub fn dispatch(&self, request: &PreviewRequest) -> Result<(), PreviewWorkerError> {
            let message = WorkerRequest::from_preview(request)
                .to_json()
                .map_err(|_| PreviewWorkerError::Message)?;
            self.worker
                .post_message(&JsValue::from_str(&message))
                .map_err(|_| PreviewWorkerError::Message)
        }
    }

    impl Drop for PreviewWorker {
        fn drop(&mut self) {
            self.worker.set_onmessage(None);
            self.worker.set_onerror(None);
            self.worker.terminate();
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub use browser::PreviewWorker;

#[cfg(not(target_arch = "wasm32"))]
pub struct PreviewWorker;

#[cfg(not(target_arch = "wasm32"))]
impl PreviewWorker {
    pub fn new(
        _on_result: impl FnMut(WorkerResponse) + 'static,
        _on_failure: impl FnMut() + 'static,
    ) -> Result<Self, PreviewWorkerError> {
        Err(PreviewWorkerError::Startup)
    }

    pub fn dispatch(&self, _request: &PreviewRequest) -> Result<(), PreviewWorkerError> {
        Err(PreviewWorkerError::Message)
    }
}
