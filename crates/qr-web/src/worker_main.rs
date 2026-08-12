#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{JsCast, closure::Closure};

#[cfg(target_arch = "wasm32")]
fn main() {
    use js_sys::{Array, Object, Reflect, Uint8Array};
    use qr_web::worker_protocol::WorkerResponse;
    use web_sys::{DedicatedWorkerGlobalScope, MessageEvent};

    let scope: DedicatedWorkerGlobalScope = js_sys::global().unchecked_into();
    let response_scope = scope.clone();
    let on_message = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
        let Some(request) = event.data().as_string() else {
            notify_delivery_failure(&response_scope, None);
            return;
        };
        let Some(revision) = qr_web::worker_protocol::WorkerRequest::revision_from_json(&request)
        else {
            notify_delivery_failure(&response_scope, None);
            return;
        };
        if !notify_revision(&response_scope, "startedRevision", revision) {
            return;
        }
        let response = WorkerResponse::evaluate_json(&request).ok();
        let Some(response) = response else {
            notify_delivery_failure(&response_scope, Some(revision));
            return;
        };
        let Ok((metadata, png)) = response.into_message_parts() else {
            notify_delivery_failure(&response_scope, Some(revision));
            return;
        };

        let message = Object::new();
        let png = Uint8Array::from(png.as_slice());
        if Reflect::set(&message, &"revision".into(), &(revision as f64).into()).is_err()
            || Reflect::set(&message, &"completedAt".into(), &js_sys::Date::now().into()).is_err()
            || Reflect::set(&message, &"metadata".into(), &metadata.into()).is_err()
            || Reflect::set(&message, &"png".into(), &png).is_err()
        {
            notify_delivery_failure(&response_scope, Some(revision));
            return;
        }
        let transfer = Array::new();
        transfer.push(&png.buffer());
        if response_scope
            .post_message_with_transfer(&message, &transfer)
            .is_err()
        {
            notify_delivery_failure(&response_scope, Some(revision));
        } else if png.byte_length() == 0 {
            _ = notify_revision(&response_scope, "releasedRevision", revision);
        }
    });
    scope.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
    _ = notify_revision(&scope, "workerReady", 1);
    on_message.forget();

    fn notify_delivery_failure(scope: &DedicatedWorkerGlobalScope, revision: Option<u64>) {
        let message = Object::new();
        if let Some(revision) = revision {
            _ = Reflect::set(&message, &"revision".into(), &(revision as f64).into());
        }
        if scope.post_message(&message).is_err() {
            scope.close();
        }
    }

    fn notify_revision(scope: &DedicatedWorkerGlobalScope, key: &str, revision: u64) -> bool {
        let message = Object::new();
        Reflect::set(&message, &key.into(), &(revision as f64).into()).is_ok()
            && scope.post_message(&message).is_ok()
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {}
