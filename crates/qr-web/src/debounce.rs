//! Lifecycle-safe browser timeouts for preview debouncing.

use std::error::Error;
use std::fmt;
use std::time::Duration;

#[cfg(target_arch = "wasm32")]
use js_sys::{Function, Reflect};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{JsCast, JsValue, closure::Closure};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TimerError;

impl fmt::Display for TimerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("the browser could not schedule preview work")
    }
}

impl Error for TimerError {}

#[cfg(target_arch = "wasm32")]
#[derive(Debug)]
pub struct BrowserTimeout {
    identifier: JsValue,
    callback: Closure<dyn FnMut()>,
}

#[cfg(target_arch = "wasm32")]
impl BrowserTimeout {
    pub fn new(duration: Duration, callback: impl FnOnce() + 'static) -> Result<Self, TimerError> {
        let milliseconds = i32::try_from(duration.as_millis()).map_err(|_| TimerError)?;
        let mut callback = Some(callback);
        let callback = Closure::new(move || {
            if let Some(callback) = callback.take() {
                callback();
            }
        });
        let global = js_sys::global();
        let set_timeout = Reflect::get(&global, &JsValue::from_str("setTimeout"))
            .map_err(|_| TimerError)?
            .dyn_into::<Function>()
            .map_err(|_| TimerError)?;
        let identifier = set_timeout
            .call2(
                &global,
                callback.as_ref(),
                &JsValue::from_f64(f64::from(milliseconds)),
            )
            .map_err(|_| TimerError)?;
        Ok(Self {
            identifier,
            callback,
        })
    }

    fn clear(&self) -> Result<(), TimerError> {
        let global = js_sys::global();
        let clear_timeout = Reflect::get(&global, &JsValue::from_str("clearTimeout"))
            .map_err(|_| TimerError)?
            .dyn_into::<Function>()
            .map_err(|_| TimerError)?;
        clear_timeout
            .call1(&global, &self.identifier)
            .map(|_| ())
            .map_err(|_| TimerError)
    }
}

#[cfg(target_arch = "wasm32")]
impl Drop for BrowserTimeout {
    fn drop(&mut self) {
        _ = self.clear();
        let _keep_callback_alive = &self.callback;
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
pub struct BrowserTimeout;

#[cfg(not(target_arch = "wasm32"))]
impl BrowserTimeout {
    pub fn new(
        _duration: Duration,
        _callback: impl FnOnce() + 'static,
    ) -> Result<Self, TimerError> {
        Err(TimerError)
    }
}

#[derive(Debug, Default)]
pub struct DebounceTimer {
    handle: Option<BrowserTimeout>,
}

impl DebounceTimer {
    pub fn replace(&mut self, handle: BrowserTimeout) {
        self.cancel();
        self.handle = Some(handle);
    }

    pub fn cancel(&mut self) {
        self.handle = None;
    }
}
