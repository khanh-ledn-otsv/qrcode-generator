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
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };

    use super::{PreviewRequest, PreviewWorkerError, WorkerResponse};
    use crate::worker_protocol::WorkerRequest;
    use js_sys::{Reflect, Uint8Array};
    use wasm_bindgen::{JsCast, JsValue, closure::Closure};
    use web_sys::{ErrorEvent, MessageEvent, Worker};

    const WORKER_LOADER: &str = "qr-preview-worker_loader.js";
    pub struct PreviewWorker {
        worker: Rc<RefCell<Worker>>,
        failed: Rc<Cell<bool>>,
        ready: Rc<Cell<bool>>,
        pending_message: Rc<RefCell<Option<String>>>,
        lifecycle_timer: Rc<RefCell<crate::debounce::DebounceTimer>>,
        on_failure: Rc<RefCell<Box<dyn FnMut(Option<crate::workflow::Revision>)>>>,
        latest_revision: Rc<Cell<Option<crate::workflow::Revision>>>,
        _on_message: Closure<dyn FnMut(MessageEvent)>,
        _on_error: Closure<dyn FnMut(ErrorEvent)>,
    }

    impl PreviewWorker {
        pub fn new(
            on_result: impl FnMut(WorkerResponse) + 'static,
            on_failure: impl FnMut(Option<crate::workflow::Revision>) + 'static,
        ) -> Result<Self, PreviewWorkerError> {
            let worker = Worker::new(WORKER_LOADER).map_err(|_| PreviewWorkerError::Startup)?;
            let active_worker = Rc::new(RefCell::new(worker));
            let on_result: Rc<RefCell<Box<dyn FnMut(WorkerResponse)>>> =
                Rc::new(RefCell::new(Box::new(on_result)));
            let on_failure: Rc<RefCell<Box<dyn FnMut(Option<crate::workflow::Revision>)>>> =
                Rc::new(RefCell::new(Box::new(on_failure)));
            let failed = Rc::new(Cell::new(false));
            let ready = Rc::new(Cell::new(false));
            let pending_message = Rc::new(RefCell::new(None::<String>));
            let lifecycle_timer = Rc::new(RefCell::new(crate::debounce::DebounceTimer::default()));
            let latest_revision = Rc::new(Cell::new(None));
            let message_failure = Rc::clone(&on_failure);
            let message_result = Rc::clone(&on_result);
            let message_failed = Rc::clone(&failed);
            let message_ready = Rc::clone(&ready);
            let queued_message = Rc::clone(&pending_message);
            let message_worker = Rc::clone(&active_worker);
            let message_timer = Rc::clone(&lifecycle_timer);
            let message_latest_revision = Rc::clone(&latest_revision);
            let on_message = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
                let data = event.data();
                if Reflect::has(&data, &"workerReady".into()).unwrap_or(false) {
                    message_ready.set(true);
                    if let Some(message) = queued_message.borrow_mut().take()
                        && message_worker
                            .borrow()
                            .post_message(&JsValue::from_str(&message))
                            .is_err()
                    {
                        message_failed.set(true);
                        (message_failure.borrow_mut())(message_latest_revision.get());
                    }
                    return;
                }
                if Reflect::has(&data, &"startedRevision".into()).unwrap_or(false)
                    || Reflect::has(&data, &"releasedRevision".into()).unwrap_or(false)
                {
                    return;
                }
                let message_revision = Reflect::get(&data, &"revision".into())
                    .ok()
                    .and_then(|value| value.as_f64())
                    .and_then(crate::workflow::Revision::from_wire_number);
                let metadata = Reflect::get(&data, &"metadata".into())
                    .ok()
                    .and_then(|value| value.as_string());
                let png = Reflect::get(&data, &"png".into())
                    .ok()
                    .and_then(|value| value.dyn_into::<Uint8Array>().ok())
                    .map(|value| value.to_vec());
                let (Some(metadata), Some(png)) = (metadata, png) else {
                    message_failed.set(true);
                    (message_failure.borrow_mut())(message_revision);
                    return;
                };
                match WorkerResponse::from_message_parts(&metadata, png) {
                    Ok(response) => {
                        if message_latest_revision.get() == Some(response.revision()) {
                            message_timer.borrow_mut().cancel();
                        }
                        (message_result.borrow_mut())(response);
                    }
                    Err(_) => {
                        let revision = message_revision
                            .or_else(|| WorkerResponse::revision_from_json(&metadata));
                        if revision.is_some() && message_latest_revision.get() == revision {
                            message_timer.borrow_mut().cancel();
                        }
                        message_failed.set(true);
                        (message_failure.borrow_mut())(revision);
                    }
                }
            });
            active_worker
                .borrow()
                .set_onmessage(Some(on_message.as_ref().unchecked_ref()));

            let worker_failure = Rc::clone(&on_failure);
            let worker_failed = Rc::clone(&failed);
            let worker_latest_revision = Rc::clone(&latest_revision);
            let error_active_worker = Rc::clone(&active_worker);
            let on_error = Closure::<dyn FnMut(ErrorEvent)>::new(move |event: ErrorEvent| {
                event.prevent_default();
                let is_active = event.current_target().is_some_and(|target| {
                    js_sys::Object::is(target.as_ref(), error_active_worker.borrow().as_ref())
                });
                if !is_active {
                    return;
                }
                worker_failed.set(true);
                (worker_failure.borrow_mut())(worker_latest_revision.get());
            });
            active_worker
                .borrow()
                .set_onerror(Some(on_error.as_ref().unchecked_ref()));

            Ok(Self {
                worker: active_worker,
                failed,
                ready,
                pending_message,
                lifecycle_timer,
                on_failure,
                latest_revision,
                _on_message: on_message,
                _on_error: on_error,
            })
        }

        pub fn dispatch(&self, request: &PreviewRequest) -> Result<(), PreviewWorkerError> {
            if self.failed.replace(false) {
                let replacement =
                    Worker::new(WORKER_LOADER).map_err(|_| PreviewWorkerError::Startup)?;
                self.ready.set(false);
                replacement.set_onmessage(Some(self._on_message.as_ref().unchecked_ref()));
                replacement.set_onerror(Some(self._on_error.as_ref().unchecked_ref()));
                let previous = self.worker.replace(replacement);
                previous.set_onmessage(None);
                previous.set_onerror(None);
                previous.terminate();
            }
            let message = WorkerRequest::from_preview(request)
                .to_json()
                .map_err(|_| PreviewWorkerError::Message)?;
            let revision = request.revision();
            self.latest_revision.set(Some(revision));
            self.lifecycle_timer.borrow_mut().cancel();
            let timer_failed = Rc::clone(&self.failed);
            let timer_failure = Rc::clone(&self.on_failure);
            let handle = leptos::prelude::set_timeout_with_handle(
                move || {
                    timer_failed.set(true);
                    (timer_failure.borrow_mut())(Some(revision));
                },
                std::time::Duration::from_secs(30),
            )
            .map_err(|_| PreviewWorkerError::Message)?;
            self.lifecycle_timer.borrow_mut().replace(handle);
            if !self.ready.get() {
                self.pending_message.replace(Some(message));
                return Ok(());
            }
            self.worker
                .borrow()
                .post_message(&JsValue::from_str(&message))
                .map_err(|_| {
                    self.failed.set(true);
                    PreviewWorkerError::Message
                })
        }
    }

    impl Drop for PreviewWorker {
        fn drop(&mut self) {
            let worker = self.worker.borrow();
            self.pending_message.borrow_mut().take();
            self.lifecycle_timer.borrow_mut().cancel();
            worker.set_onmessage(None);
            worker.set_onerror(None);
            worker.terminate();
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
        _on_failure: impl FnMut(Option<crate::workflow::Revision>) + 'static,
    ) -> Result<Self, PreviewWorkerError> {
        Err(PreviewWorkerError::Startup)
    }

    pub fn dispatch(&self, _request: &PreviewRequest) -> Result<(), PreviewWorkerError> {
        Err(PreviewWorkerError::Message)
    }
}
