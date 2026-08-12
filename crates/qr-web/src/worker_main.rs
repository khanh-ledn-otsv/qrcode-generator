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
        let parts = event
            .data()
            .as_string()
            .and_then(|request| WorkerResponse::evaluate_json(&request).ok())
            .and_then(|response| response.into_message_parts().ok());
        let Some((metadata, png)) = parts else {
            let _ = response_scope.post_message(&Object::new());
            return;
        };

        let message = Object::new();
        let png = Uint8Array::from(png.as_slice());
        let _ = Reflect::set(&message, &"metadata".into(), &metadata.into());
        let _ = Reflect::set(&message, &"png".into(), &png);
        let transfer = Array::new();
        transfer.push(&png.buffer());
        let _ = response_scope.post_message_with_transfer(&message, &transfer);
    });
    scope.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
    on_message.forget();
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {}
