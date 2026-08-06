//! Lifecycle ownership for cancellable Leptos preview debounce timers.

use leptos::prelude::TimeoutHandle;

#[derive(Debug, Default)]
pub struct DebounceTimer {
    handle: Option<TimeoutHandle>,
}

impl DebounceTimer {
    pub fn replace(&mut self, handle: TimeoutHandle) {
        self.cancel();
        self.handle = Some(handle);
    }

    pub fn cancel(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.clear();
        }
    }
}

impl Drop for DebounceTimer {
    fn drop(&mut self) {
        self.cancel();
    }
}
