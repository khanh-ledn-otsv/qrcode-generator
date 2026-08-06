use std::time::Duration;

use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use leptos::web_sys::{ClipboardEvent, DragEvent, Event, HtmlTextAreaElement, InputEvent};
use qr_render::{ProfileId, SUPPORTED_PROFILES};
use qr_web::debounce::{BrowserTimeout, DebounceTimer};
use qr_web::download::trigger_download;
use qr_web::workflow::{
    ArtifactKind, PreviewRequest, WorkflowFailure, WorkflowState, ecc_label, evaluate_preview,
    mode_label, profile_presentation, textarea_display_utf16_length,
};

const PREVIEW_DEBOUNCE: Duration = Duration::from_millis(250);
type DebounceSignal = RwSignal<DebounceTimer, leptos::reactive::owner::LocalStorage>;

#[component]
fn App() -> impl IntoView {
    let state = RwSignal::new(WorkflowState::new(ProfileId::Content));
    let pending_timer = RwSignal::new_local(DebounceTimer::default());
    let pending_edit_start = RwSignal::new(None::<u32>);

    Owner::on_cleanup(move || {
        pending_timer.update(DebounceTimer::cancel);
    });

    let profile_options = SUPPORTED_PROFILES.map(|profile| {
        let profile_id = profile.id();
        let presentation = profile_presentation(profile_id);
        view! {
            <label class=move || profile_card_class(state.with(|value| value.profile_id() == profile_id))>
                <input
                    class="peer sr-only"
                    type="radio"
                    name="output-profile"
                    value=presentation.value()
                    prop:checked=move || state.with(|value| value.profile_id() == profile_id)
                    on:change=move |_| {
                        if let Some(Ok(request)) = state.try_update(|value| value.select_profile(profile_id)) {
                            schedule_preview(state, pending_timer, request);
                        }
                    }
                />
                <span class="block text-sm font-bold text-slate-950">{presentation.name()}</span>
                <span class="mt-1 block text-xs leading-5 text-slate-600">
                    {format!(
                        "{} px SVG · {} px PNG · up to V{}",
                        profile.svg_dimensions().width().get(),
                        profile.png_dimensions().width().get(),
                        profile.maximum_version().number(),
                    )}
                </span>
            </label>
        }
    });

    view! {
        <main class="min-h-screen bg-slate-100 px-4 py-8 sm:px-6 lg:px-8 lg:py-12">
            <div class="mx-auto max-w-7xl">
                <header class="mb-8 max-w-3xl">
                    <p class="text-brand text-sm font-bold uppercase tracking-[0.22em]">"Private by design"</p>
                    <h1 class="mt-3 text-4xl font-black tracking-tight text-slate-950 sm:text-5xl">
                        "Create a safe QR code"
                    </h1>
                    <p class="mt-4 text-base leading-7 text-slate-600 sm:text-lg">
                        "Your text stays in this browser. Choose an output profile and the QR code will refit automatically at error correction M."
                    </p>
                </header>

                <div class="grid items-start gap-6 lg:grid-cols-[minmax(0,1.08fr)_minmax(22rem,0.92fr)]">
                    <section class="rounded-3xl border border-slate-200 bg-white p-5 shadow-sm sm:p-8" aria-labelledby="payload-heading">
                        <div>
                            <h2 id="payload-heading" class="text-xl font-bold text-slate-950">"Payload"</h2>
                            <p class="mt-1 text-sm leading-6 text-slate-600">
                                "Text is encoded exactly as entered—spaces, line breaks, and Unicode included."
                            </p>
                        </div>

                        <label for="qr-payload" class="mt-6 block text-sm font-semibold text-slate-800">
                            "Text to encode"
                        </label>
                        <textarea
                            id="qr-payload"
                            class="focus:border-brand focus:ring-brand mt-2 min-h-44 w-full resize-y rounded-2xl border border-slate-300 bg-white px-4 py-3 font-mono text-sm leading-6 text-slate-950 shadow-inner outline-none transition focus:ring-2 focus:ring-offset-2"
                            placeholder="Paste or type text exactly as it should be encoded"
                            autocomplete="off"
                            autocapitalize="off"
                            spellcheck="false"
                            aria-describedby="payload-counts payload-validation payload-caution"
                            aria-invalid=move || state.with(|value| value.validation_message().is_some())
                            prop:value=move || state.with(WorkflowState::textarea_value)
                            on:dragstart=move |event: DragEvent| {
                                let target = event
                                    .target()
                                    .and_then(|target| target.dyn_into::<HtmlTextAreaElement>().ok());
                                let selection = target.as_ref().and_then(textarea_selection);
                                let transfer = event.data_transfer();
                                let raw = selection.map(|(start, end)| {
                                    state.with(|value| value.raw_text_for_display_range(start, end))
                                });
                                if let (Some(transfer), Some(Ok(raw))) = (transfer, raw)
                                    && transfer.set_data("text/plain", &raw).is_ok()
                                {
                                    return;
                                }
                                event.prevent_default();
                                pending_edit_start.set(None);
                                reject_raw_input_failure(state, pending_timer, target.as_ref());
                            }
                            on:beforeinput=move |event: InputEvent| {
                                let target = event
                                    .target()
                                    .and_then(|target| target.dyn_into::<HtmlTextAreaElement>().ok());
                                let selection = target.as_ref().and_then(textarea_selection);
                                if matches!(
                                    event.input_type().as_str(),
                                    "insertFromDrop" | "insertFromPaste"
                                ) {
                                    event.prevent_default();
                                    pending_edit_start.set(None);
                                    let raw = event
                                        .data_transfer()
                                        .and_then(|transfer| transfer.get_data("text/plain").ok());
                                    if let (Some(raw), Some(target), Some((start, end))) =
                                        (raw, target.as_ref(), selection)
                                    {
                                        apply_raw_textarea_insertion(
                                            state,
                                            pending_timer,
                                            target,
                                            start,
                                            end,
                                            &raw,
                                        );
                                    } else {
                                        reject_raw_input_failure(
                                            state,
                                            pending_timer,
                                            target.as_ref(),
                                        );
                                    }
                                    return;
                                }
                                let start = selection.map(|(start, _)| start);
                                pending_edit_start.set(start);
                            }
                            on:input=move |event: Event| {
                                let Some(target) = event
                                    .target()
                                    .and_then(|target| target.dyn_into::<HtmlTextAreaElement>().ok())
                                else {
                                    state.update(WorkflowState::reject_internal_failure);
                                    return;
                                };
                                let before_start = pending_edit_start.get_untracked();
                                pending_edit_start.set(None);
                                let after_start = target.selection_start().ok().flatten();
                                let Some(edit_start) = before_start
                                    .zip(after_start)
                                    .map(|(before, after)| before.min(after))
                                else {
                                    state.update(WorkflowState::reject_internal_failure);
                                    target.set_value(&state.with(WorkflowState::textarea_value));
                                    return;
                                };
                                match state.try_update(|value| {
                                    value.set_display_payload_at(target.value(), edit_start)
                                }) {
                                    Some(Ok(request)) => schedule_preview(state, pending_timer, request),
                                    Some(Err(_)) | None => {
                                        target.set_value(&state.with(WorkflowState::textarea_value));
                                    }
                                }
                            }
                            on:paste=move |event: ClipboardEvent| {
                                event.prevent_default();
                                pending_edit_start.set(None);
                                let target = event
                                    .target()
                                    .and_then(|target| target.dyn_into::<HtmlTextAreaElement>().ok());
                                let selection = target.as_ref().and_then(textarea_selection);
                                let pasted = event
                                    .clipboard_data()
                                    .and_then(|clipboard| clipboard.get_data("text/plain").ok());
                                if let (Some(pasted), Some(target), Some((start, end))) =
                                    (pasted, target.as_ref(), selection)
                                {
                                    apply_raw_textarea_insertion(
                                        state,
                                        pending_timer,
                                        target,
                                        start,
                                        end,
                                        &pasted,
                                    );
                                } else {
                                    reject_raw_input_failure(
                                        state,
                                        pending_timer,
                                        target.as_ref(),
                                    );
                                }
                            }
                        ></textarea>

                        <p id="payload-counts" class="mt-2 flex flex-wrap gap-x-4 gap-y-1 text-xs font-medium text-slate-500">
                            <span>{move || format!("{} characters", state.with(WorkflowState::character_count))}</span>
                            <span>{move || format!("{} UTF-8 bytes", state.with(WorkflowState::byte_count))}</span>
                        </p>
                        <p id="payload-validation" class="mt-3 min-h-6 text-sm font-semibold text-red-700" role="alert" aria-live="polite">
                            {move || state.with(WorkflowState::validation_message).unwrap_or_default()}
                        </p>
                        <p id="payload-caution" class="mt-2 min-h-6 text-sm font-semibold text-amber-800" role="status">
                            {move || state.with(|value| value.caution().map(|message| format!("Caution: {message}")).unwrap_or_default())}
                        </p>

                        <fieldset class="mt-8">
                            <legend class="text-sm font-semibold text-slate-800">"Output profile"</legend>
                            <div class="mt-3 grid gap-3 sm:grid-cols-2">{profile_options}</div>
                        </fieldset>
                    </section>

                    <div class="grid gap-6 lg:sticky lg:top-8">
                        <section class="rounded-3xl border border-slate-200 bg-white p-5 shadow-sm sm:p-8" aria-labelledby="preview-heading">
                            <div class="flex items-center justify-between gap-4">
                                <h2 id="preview-heading" class="text-xl font-bold text-slate-950">"Preview"</h2>
                                <span class="rounded-full bg-emerald-50 px-3 py-1 text-xs font-bold text-emerald-800 ring-1 ring-inset ring-emerald-200">
                                    {move || state.with(|value| value.preview().map(|preview| format!("ECC {}", ecc_label(preview.diagnostics().ecc()))).unwrap_or_else(|| "ECC —".to_owned()))}
                                </span>
                            </div>
                            <div
                                class="mt-5 grid aspect-square w-full place-items-center overflow-hidden rounded-2xl border border-slate-200 bg-slate-50 p-5"
                                role="img"
                                aria-label=move || state.with(|value| value.preview().map(|preview| preview.accessible_label()).unwrap_or_else(|| "QR code preview unavailable".to_owned()))
                                data-testid="qr-preview"
                            >
                                <div
                                    class:hidden=move || state.with(|value| value.preview().is_none())
                                    class="w-full [&>svg]:h-auto [&>svg]:w-full"
                                    aria-hidden="true"
                                    inner_html=move || state.with(|value| value.preview().map(|preview| preview.svg().to_owned()).unwrap_or_default())
                                ></div>
                                <p class:hidden=move || state.with(|value| value.preview().is_some()) class="max-w-xs text-center text-sm leading-6 text-slate-500">
                                    {move || if state.with(WorkflowState::is_pending) { "Updating preview…" } else { "Enter a valid payload to see the QR preview." }}
                                </p>
                            </div>
                        </section>

                        <section class="rounded-3xl border border-slate-200 bg-white p-5 shadow-sm sm:p-8" aria-labelledby="diagnostics-heading">
                            <h2 id="diagnostics-heading" class="text-xl font-bold text-slate-950">"Diagnostics"</h2>
                            <dl class="mt-5 grid grid-cols-2 gap-x-5 gap-y-4 text-sm">
                                <Diagnostic label="Mode" value=move || diagnostic_value(state, |details| mode_label(details.mode()).to_owned()) />
                                <Diagnostic label="ECC" value=move || diagnostic_value(state, |details| ecc_label(details.ecc()).to_owned()) />
                                <Diagnostic label="Mask" value=move || diagnostic_value(state, |details| details.mask().to_string()) />
                                <Diagnostic label="Version" value=move || diagnostic_value(state, |details| format!("V{} / V{} max", details.selected_version().number(), details.maximum_version().number())) />
                                <Diagnostic label="Data bits" value=move || diagnostic_value(state, |details| format!("{} / {}", details.used_data_bits(), details.available_data_bits())) />
                                <Diagnostic label="Data codewords" value=move || diagnostic_value(state, |details| details.data_codewords().to_string()) />
                                <Diagnostic label="Matrix" value=move || diagnostic_value(state, |details| format!("{} × {} modules", details.matrix_modules(), details.matrix_modules())) />
                                <Diagnostic label="Quiet zone" value=move || diagnostic_value(state, |details| format!("{} modules per side", details.quiet_zone_modules())) />
                                <Diagnostic label="PNG geometry" value=move || diagnostic_value(state, |details| format!("{} px/module · {} px symbol · {} px padding", details.module_scale(), details.rendered_symbol_side_pixels(), details.outer_padding_per_side())) />
                                <Diagnostic label="Output" value=move || diagnostic_value(state, |details| format!("{} px SVG · {} px PNG", details.svg_side_pixels(), details.png_side_pixels())) />
                            </dl>
                            <p class="mt-5 rounded-2xl bg-slate-100 px-4 py-3 text-xs font-medium leading-5 text-slate-600">
                                {move || diagnostic_value(state, |details| details.print_guidance().to_owned())}
                            </p>
                        </section>

                        <section class="rounded-3xl border border-slate-200 bg-white p-5 shadow-sm sm:p-8" aria-labelledby="exports-heading">
                            <h2 id="exports-heading" class="text-xl font-bold text-slate-950">"Download"</h2>
                            <p class="mt-1 text-sm leading-6 text-slate-600">
                                "Files use fixed private filenames and contain no payload metadata."
                            </p>
                            <div class="mt-5 grid gap-3 sm:grid-cols-2">
                                <button
                                    type="button"
                                    class="focus:ring-brand rounded-xl bg-slate-950 px-4 py-3 text-sm font-bold text-white transition hover:bg-slate-800 focus:outline-none focus:ring-2 focus:ring-offset-2 disabled:cursor-not-allowed disabled:bg-slate-300 disabled:text-slate-600"
                                    disabled=move || !state.with(WorkflowState::exports_enabled)
                                    aria-describedby="export-status"
                                    data-testid="download-svg"
                                    on:click=move |_| download_artifact(state, pending_timer, ArtifactKind::Svg)
                                >
                                    "Download SVG"
                                </button>
                                <button
                                    type="button"
                                    class="focus:ring-brand rounded-xl bg-slate-950 px-4 py-3 text-sm font-bold text-white transition hover:bg-slate-800 focus:outline-none focus:ring-2 focus:ring-offset-2 disabled:cursor-not-allowed disabled:bg-slate-300 disabled:text-slate-600"
                                    disabled=move || !state.with(WorkflowState::exports_enabled)
                                    aria-describedby="export-status"
                                    data-testid="download-png"
                                    on:click=move |_| download_artifact(state, pending_timer, ArtifactKind::Png)
                                >
                                    "Download PNG"
                                </button>
                            </div>
                            <p id="export-status" class="mt-3 min-h-6 text-sm font-medium text-slate-700" role="status">
                                {move || state.with(|value| value.export_disabled_reason().unwrap_or_else(|| "SVG and PNG downloads are ready.".to_owned()))}
                            </p>
                        </section>
                    </div>
                </div>
            </div>
        </main>
    }
}

#[component]
fn Diagnostic<F>(label: &'static str, value: F) -> impl IntoView
where
    F: Fn() -> String + Send + 'static,
{
    view! {
        <div>
            <dt class="text-xs font-semibold uppercase tracking-wide text-slate-500">{label}</dt>
            <dd class="mt-1 font-bold text-slate-900">{value}</dd>
        </div>
    }
}

fn schedule_preview(
    state: RwSignal<WorkflowState>,
    pending_timer: DebounceSignal,
    request: PreviewRequest,
) {
    pending_timer.update(DebounceTimer::cancel);
    let revision = request.revision();
    match BrowserTimeout::new(PREVIEW_DEBOUNCE, move || {
        let result = evaluate_preview(&request);
        state.update(|value| {
            _ = value.complete_preview(revision, result);
        });
    }) {
        Ok(handle) => pending_timer.update(|timer| timer.replace(handle)),
        Err(_) => state.update(|value| {
            _ = value.complete_preview(revision, Err(WorkflowFailure::Internal));
        }),
    }
}

fn apply_raw_textarea_insertion(
    state: RwSignal<WorkflowState>,
    pending_timer: DebounceSignal,
    target: &HtmlTextAreaElement,
    start: u32,
    end: u32,
    raw: &str,
) {
    let result = state.try_update(|value| value.replace_display_range(start, end, raw));
    target.set_value(&state.with(WorkflowState::textarea_value));
    if let Some(Ok(request)) = result {
        if let Some(inserted_length) = textarea_display_utf16_length(raw)
            && let Some(caret) = start.checked_add(inserted_length)
        {
            _ = target.set_selection_range(caret, caret);
        }
        schedule_preview(state, pending_timer, request);
    }
}

fn textarea_selection(target: &HtmlTextAreaElement) -> Option<(u32, u32)> {
    target
        .selection_start()
        .ok()
        .flatten()
        .zip(target.selection_end().ok().flatten())
}

fn reject_raw_input_failure(
    state: RwSignal<WorkflowState>,
    pending_timer: DebounceSignal,
    target: Option<&HtmlTextAreaElement>,
) {
    pending_timer.update(DebounceTimer::cancel);
    state.update(WorkflowState::reject_internal_failure);
    if let Some(target) = target {
        target.set_value(&state.with(WorkflowState::textarea_value));
    }
}

fn download_artifact(
    state: RwSignal<WorkflowState>,
    pending_timer: DebounceSignal,
    kind: ArtifactKind,
) {
    let result = state.with(|value| {
        value
            .preview()
            .ok_or(WorkflowFailure::Internal)
            .and_then(|preview| {
                trigger_download(preview.artifact(kind)).map_err(|_| WorkflowFailure::Internal)
            })
    });
    if result.is_err() {
        pending_timer.update(DebounceTimer::cancel);
        state.update(WorkflowState::reject_internal_failure);
    }
}

fn diagnostic_value(
    state: RwSignal<WorkflowState>,
    format: impl FnOnce(qr_web::workflow::Diagnostics) -> String,
) -> String {
    state.with(|value| {
        value
            .preview()
            .map(|preview| format(preview.diagnostics()))
            .unwrap_or_else(|| "—".to_owned())
    })
}

fn profile_card_class(selected: bool) -> &'static str {
    if selected {
        "focus-within:ring-brand cursor-pointer rounded-2xl border border-brand bg-pink-50 p-4 ring-2 ring-brand ring-offset-2 transition"
    } else {
        "focus-within:ring-brand cursor-pointer rounded-2xl border border-slate-200 bg-white p-4 transition hover:border-slate-400 focus-within:ring-2 focus-within:ring-offset-2"
    }
}

fn main() {
    leptos::mount::mount_to_body(App);
}
