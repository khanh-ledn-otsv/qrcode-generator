use std::time::Duration;

use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use leptos::web_sys::Event;
use qr_render::{ForegroundTheme, LogoStyle, OutputSafety, ProfileId, SUPPORTED_PROFILES};
use qr_web::debounce::DebounceTimer;
use qr_web::download::trigger_download;
use qr_web::preview_worker::PreviewWorker;
use qr_web::url_payload::{UrlPayloadError, UrlPayloadState};
use qr_web::workflow::{
    ArtifactKind, PreviewRequest, WorkflowFailure, WorkflowState, ecc_label, link_capacity_guide,
    mode_label, profile_presentation, version_label,
};

use crate::usage::UsageGuide;

mod usage;

const PREVIEW_DEBOUNCE: Duration = Duration::from_millis(250);
type DebounceSignal = RwSignal<DebounceTimer>;
type PreviewWorkerStore = StoredValue<Option<PreviewWorker>, LocalStorage>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Tab {
    Generator,
    Usage,
}

#[component]
fn App() -> impl IntoView {
    let active_tab = RwSignal::new(Tab::Generator);
    let state = RwSignal::new(WorkflowState::new(ProfileId::Standard));
    let url_state = RwSignal::new(UrlPayloadState::default());
    let pending_timer = RwSignal::new(DebounceTimer::default());
    let preview_worker = StoredValue::new_local(None);
    let worker = PreviewWorker::new(
        move |response| {
            let (revision, result) = response.into_preview_result();
            state.update(|value| {
                _ = value.complete_preview(revision, result);
            });
        },
        move |revision| {
            if let Some(revision) = revision {
                state.update(|value| {
                    _ = value.complete_preview(revision, Err(WorkflowFailure::Internal));
                });
            }
        },
    )
    .ok();
    preview_worker.update_value(|slot| *slot = worker);

    Owner::on_cleanup(move || {
        pending_timer.update(DebounceTimer::cancel);
        preview_worker.update_value(|slot| *slot = None);
    });

    view! {
        <header class="h-16 bg-brand px-4 sm:px-6">
            <div class="mx-auto flex h-full max-w-5xl items-center justify-center">
                <img class="h-10 w-auto object-contain" src="/public/images/one-logotype-white.png" alt="ONE" />
            </div>
        </header>
        <main class="bg-page px-4 py-8 sm:px-6 sm:py-12">
            <div class="mx-auto max-w-5xl">
                <header>
                    <h1 class="text-2xl font-bold text-text sm:text-3xl">"Create a safe QR code"</h1>
                    <p class="mt-1 text-sm text-text-muted">"Standard QR code follows ISO/IEC 18004:2024"</p>
                </header>

                <nav class="mt-8 flex border-b border-border" aria-label="QR generator sections">
                    <button
                        type="button"
                        class="px-3 py-2 text-sm font-semibold"
                        class:border-b-2=move || active_tab.get() == Tab::Generator
                        class:border-brand=move || active_tab.get() == Tab::Generator
                        class:text-brand=move || active_tab.get() == Tab::Generator
                        class:text-text-muted=move || active_tab.get() != Tab::Generator
                        aria-current=move || (active_tab.get() == Tab::Generator).then_some("page")
                        on:click=move |_| active_tab.set(Tab::Generator)
                    >"Generator"</button>
                    <button
                        type="button"
                        class="px-3 py-2 text-sm font-semibold"
                        class:border-b-2=move || active_tab.get() == Tab::Usage
                        class:border-brand=move || active_tab.get() == Tab::Usage
                        class:text-brand=move || active_tab.get() == Tab::Usage
                        class:text-text-muted=move || active_tab.get() != Tab::Usage
                        aria-current=move || (active_tab.get() == Tab::Usage).then_some("page")
                        on:click=move |_| active_tab.set(Tab::Usage)
                    >"Usage"</button>
                </nav>

                <div class:hidden=move || active_tab.get() != Tab::Usage>
                    <UsageGuide />
                </div>

                <div class:hidden=move || active_tab.get() != Tab::Generator class="mt-6 grid items-start gap-8 lg:grid-cols-[minmax(0,3fr)_minmax(20rem,2fr)]">
                    <section aria-labelledby="settings-heading">
                        <h2 id="settings-heading" class="text-lg font-semibold text-text">"Settings"</h2>

                        <label for="base-url" class="mt-4 block text-sm font-semibold text-text">"Base URL"<span class="text-brand">" *"</span></label>
                        <input
                            id="base-url"
                            type="url"
                            class="mt-1 w-full rounded-md border border-border bg-page px-3 py-2 text-sm text-text outline-none focus:border-focus focus:ring-2 focus:ring-focus/20"
                            placeholder="E.g. https://example.com/promo"
                            autocomplete="url"
                            aria-describedby="base-url-counts url-validation"
                            aria-invalid=move || state.with(|value| value.validation_message().is_some())
                            prop:value=move || url_state.with(|value| value.base_url().to_owned())
                            on:input=move |event| {
                                url_state.update(|value| value.set_base_url(event_target_value(&event)));
                                update_url_preview(url_state, state, pending_timer, preview_worker);
                            }
                        />
                        <p id="base-url-counts" class="mt-1 text-xs text-text-muted">
                            {move || url_state.with(|value| format!("{} characters | {} UTF-8 bytes", value.base_url().chars().count(), value.base_url().len()))}
                        </p>
                        <p id="url-validation" class="mt-2 min-h-5 text-sm font-semibold text-red-700" role="alert" aria-live="polite">
                            {move || state.with(WorkflowState::validation_message).unwrap_or_default()}
                        </p>

                        <section class="mt-4 rounded-lg bg-surface p-4" aria-labelledby="utm-heading">
                            <div class="flex items-center justify-between gap-4">
                                <h3 id="utm-heading" class="text-sm font-semibold text-text">"UTM Configuration"</h3>
                                <label class="relative inline-flex cursor-pointer items-center">
                                    <input
                                        class="peer sr-only"
                                        type="checkbox"
                                        name="utm-enabled"
                                        prop:checked=move || url_state.with(UrlPayloadState::utm_enabled)
                                        on:change=move |event| {
                                            url_state.update(|value| value.set_utm_enabled(event_target_checked(&event)));
                                            update_url_preview(url_state, state, pending_timer, preview_worker);
                                        }
                                    />
                                    <span class="pointer-events-none h-5 w-9 rounded-full bg-text-muted transition peer-checked:bg-brand peer-focus:ring-2 peer-focus:ring-focus peer-focus:ring-offset-2 after:absolute after:left-0.5 after:top-0.5 after:h-4 after:w-4 after:rounded-full after:bg-page after:transition peer-checked:after:translate-x-4"></span>
                                    <span class="sr-only">"Enable UTM configuration"</span>
                                </label>
                            </div>

                            <div class="mt-4 grid gap-4 sm:grid-cols-2" class:hidden=move || !url_state.with(UrlPayloadState::utm_enabled)>
                                <label class="block text-xs font-semibold text-text">
                                    "utm_source"
                                    <input class="mt-1 w-full rounded-md border border-border bg-page px-3 py-2 text-sm font-regular outline-none focus:border-focus focus:ring-2 focus:ring-focus/20" placeholder="Where the scan comes from" prop:value=move || url_state.with(|value| value.utm_source().to_owned()) on:input=move |event| { url_state.update(|value| value.set_utm_source(event_target_value(&event))); update_url_preview(url_state, state, pending_timer, preview_worker); } />
                                </label>
                                <label class="block text-xs font-semibold text-text">
                                    "utm_medium"
                                    <input class="mt-1 w-full rounded-md border border-border bg-page px-3 py-2 text-sm font-regular outline-none focus:border-focus focus:ring-2 focus:ring-focus/20" placeholder="The channel" prop:value=move || url_state.with(|value| value.utm_medium().to_owned()) on:input=move |event| { url_state.update(|value| value.set_utm_medium(event_target_value(&event))); update_url_preview(url_state, state, pending_timer, preview_worker); } />
                                </label>
                                <label class="block text-xs font-semibold text-text sm:col-span-2">
                                    "utm_campaign"
                                    <input class="mt-1 w-full rounded-md border border-border bg-page px-3 py-2 text-sm font-regular outline-none focus:border-focus focus:ring-2 focus:ring-focus/20" placeholder="Campaign name" prop:value=move || url_state.with(|value| value.utm_campaign().to_owned()) on:input=move |event| { url_state.update(|value| value.set_utm_campaign(event_target_value(&event))); update_url_preview(url_state, state, pending_timer, preview_worker); } />
                                </label>
                            </div>

                            <div class="mt-4 grid gap-3">
                                {move || url_state.with(|value| value.custom_parameters().to_vec()).into_iter().map(|parameter| {
                                    let id = parameter.id();
                                    view! {
                                        <div class="grid gap-2 sm:grid-cols-[1fr_1fr_auto]">
                                            <input aria-label=format!("Custom parameter {} name", id + 1) class="rounded-md border border-border bg-page px-3 py-2 text-sm outline-none focus:border-focus focus:ring-2 focus:ring-focus/20" placeholder="Parameter name" prop:value=move || url_state.with(|value| value.custom_parameters().iter().find(|item| item.id() == id).map(|item| item.name().to_owned()).unwrap_or_default()) on:input=move |event| { url_state.update(|value| { _ = value.set_custom_parameter_name(id, event_target_value(&event)); }); update_url_preview(url_state, state, pending_timer, preview_worker); } />
                                            <input aria-label=format!("Custom parameter {} value", id + 1) class="rounded-md border border-border bg-page px-3 py-2 text-sm outline-none focus:border-focus focus:ring-2 focus:ring-focus/20" placeholder="Value" prop:value=move || url_state.with(|value| value.custom_parameters().iter().find(|item| item.id() == id).map(|item| item.value().to_owned()).unwrap_or_default()) on:input=move |event| { url_state.update(|value| { _ = value.set_custom_parameter_value(id, event_target_value(&event)); }); update_url_preview(url_state, state, pending_timer, preview_worker); } />
                                            <button type="button" class="rounded-md border border-border px-3 py-2 text-sm font-semibold text-text hover:border-brand hover:text-brand focus:outline-none focus:ring-2 focus:ring-focus" aria-label=format!("Remove custom parameter {}", id + 1) on:click=move |_| { url_state.update(|value| { _ = value.remove_custom_parameter(id); }); update_url_preview(url_state, state, pending_timer, preview_worker); }>"Remove"</button>
                                        </div>
                                    }
                                }).collect_view()}
                            </div>
                            <button type="button" class="mt-4 text-sm font-semibold text-brand hover:text-brand-dark focus:outline-none focus:ring-2 focus:ring-focus focus:ring-offset-2" on:click=move |_| { url_state.update(|value| { _ = value.add_custom_parameter(); }); }>"+ Add Parameter"</button>
                        </section>

                        <label for="encoded-url" class="mt-4 block text-sm font-semibold text-text">"Encoded URL" <span class="font-regular italic text-text-muted">"- Automatic generation"</span></label>
                        <textarea id="encoded-url" class="mt-1 min-h-20 w-full resize-none rounded-md border border-border bg-surface px-3 py-2 text-sm text-text" readonly=true prop:value=move || url_state.with(|value| value.compose().unwrap_or_default())></textarea>
                        <p class="mt-1 text-xs text-text-muted">
                            {move || encoded_url_guidance(url_state, state)}
                        </p>
                    </section>

                    <section class="rounded-lg bg-surface p-4" aria-labelledby="preview-heading">
                        <h2 id="preview-heading" class="text-lg font-semibold text-text">"Preview"</h2>
                        <fieldset class="mt-4">
                            <legend class="text-xs font-semibold text-text">"QR Type"<span class="text-brand">" *"</span></legend>
                            <div class="mt-1 flex gap-4 text-sm">
                                <label class="flex items-center gap-2"><input type="radio" name="qr-type" checked=move || is_digital(state.with(WorkflowState::profile_id)) on:change=move |_| select_profile(state, pending_timer, preview_worker, ProfileId::Standard) />"Digital"</label>
                                <label class="flex items-center gap-2"><input type="radio" name="qr-type" checked=move || !is_digital(state.with(WorkflowState::profile_id)) on:change=move |_| select_profile(state, pending_timer, preview_worker, ProfileId::BusinessCard) />"Print"</label>
                            </div>
                        </fieldset>

                        <label for="profile-select" class="mt-3 block text-xs font-semibold text-text">"Output variant"</label>
                        <select id="profile-select" class="mt-1 w-full rounded-md border border-border bg-page px-3 py-2 text-sm outline-none focus:border-focus focus:ring-2 focus:ring-focus/20" on:change=move |event| { if let Some(profile) = profile_from_value(&event_target_value(&event)) { select_profile(state, pending_timer, preview_worker, profile); } }>
                            <option value="small" hidden=move || !is_digital(state.with(WorkflowState::profile_id)) selected=move || state.with(WorkflowState::profile_id) == ProfileId::Small>"Small"</option>
                            <option value="standard" hidden=move || !is_digital(state.with(WorkflowState::profile_id)) selected=move || state.with(WorkflowState::profile_id) == ProfileId::Standard>"Standard"</option>
                            <option value="primary-cta" hidden=move || !is_digital(state.with(WorkflowState::profile_id)) selected=move || state.with(WorkflowState::profile_id) == ProfileId::PrimaryCta>"Primary CTA"</option>
                            <option value="hero-campaign" hidden=move || !is_digital(state.with(WorkflowState::profile_id)) selected=move || state.with(WorkflowState::profile_id) == ProfileId::HeroCampaign>"Hero / Campaign"</option>
                            <option value="business-card" hidden=move || is_digital(state.with(WorkflowState::profile_id)) selected=move || state.with(WorkflowState::profile_id) == ProfileId::BusinessCard>"Business card"</option>
                            <option value="flyer-brochure" hidden=move || is_digital(state.with(WorkflowState::profile_id)) selected=move || state.with(WorkflowState::profile_id) == ProfileId::FlyerBrochure>"Flyer / Brochure"</option>
                            <option value="poster-package" hidden=move || is_digital(state.with(WorkflowState::profile_id)) selected=move || state.with(WorkflowState::profile_id) == ProfileId::PosterPackage>"Poster / Package"</option>
                        </select>

                        <fieldset class="mt-4">
                            <legend class="text-xs font-semibold text-text">"Color type"<span class="text-brand">" *"</span></legend>
                            <div class="mt-1 flex gap-4 text-sm">
                                <label class="flex items-center gap-2"><input type="radio" name="foreground-theme" checked=move || state.with(WorkflowState::foreground_theme) == ForegroundTheme::Magenta on:change=move |_| select_foreground(state, pending_timer, preview_worker, ForegroundTheme::Magenta) />"Magenta"</label>
                                <label class="flex items-center gap-2"><input type="radio" name="foreground-theme" checked=move || state.with(WorkflowState::foreground_theme) == ForegroundTheme::Black on:change=move |_| select_foreground(state, pending_timer, preview_worker, ForegroundTheme::Black) />"Black"</label>
                            </div>
                        </fieldset>

                        <label class="mt-4 flex items-center justify-between gap-4 text-sm font-semibold text-text">
                            <span>"ONE logo in QR"</span>
                            <input type="checkbox" name="bundled-logo" prop:checked=move || state.with(WorkflowState::logo_enabled) on:change=move |event| { if let Some(Ok(request)) = state.try_update(|value| value.set_logo_enabled(event_target_checked(&event))) { schedule_preview(state, pending_timer, preview_worker, request); } } />
                        </label>

                        <div class="mt-4 grid min-h-72 place-items-center rounded-lg border border-border bg-page p-4" role="img" aria-label=move || state.with(|value| value.preview().map(|preview| preview.accessible_label()).unwrap_or_else(|| "QR code preview unavailable".to_owned())) data-testid="qr-preview">
                            <div class:hidden=move || state.with(|value| value.preview().is_none()) class="[&>svg]:block [&>svg]:h-auto [&>svg]:max-h-60 [&>svg]:max-w-full" aria-hidden="true" inner_html=move || state.with(|value| value.preview().map(|preview| preview.svg().to_owned()).unwrap_or_default())></div>
                            <p class:hidden=move || state.with(|value| value.preview().is_some()) class="max-w-56 text-center text-sm text-text-muted">{move || if state.with(WorkflowState::is_pending) { "Updating preview..." } else { "Enter a valid URL to see the QR preview." }}</p>
                        </div>

                        <p class="mt-3 min-h-5 text-xs font-semibold text-amber-800" role="status">{move || state.with(|value| value.caution().map(|message| format!("Caution: {message}")).unwrap_or_default())}</p>
                        <div class="mt-3 grid gap-3 sm:grid-cols-2">
                            <button type="button" class="rounded-md border border-brand bg-page px-4 py-2 text-sm font-semibold text-brand hover:bg-brand-light focus:outline-none focus:ring-2 focus:ring-focus disabled:cursor-not-allowed disabled:border-border disabled:text-text-muted" disabled=move || !state.with(WorkflowState::exports_enabled) data-testid="download-png" on:click=move |_| download_artifact(state, pending_timer, ArtifactKind::Png)>"Download PNG"</button>
                            <button type="button" class="rounded-md bg-brand px-4 py-2 text-sm font-semibold text-white hover:bg-brand-dark focus:outline-none focus:ring-2 focus:ring-focus focus:ring-offset-2 disabled:cursor-not-allowed disabled:bg-border disabled:text-text-muted" disabled=move || !state.with(WorkflowState::exports_enabled) data-testid="download-svg" on:click=move |_| download_artifact(state, pending_timer, ArtifactKind::Svg)>"Download SVG"</button>
                        </div>
                        <p id="export-status" class="mt-2 text-xs text-text-muted" role="status">{move || state.with(|value| value.export_disabled_reason().unwrap_or_else(|| "SVG and PNG downloads are ready.".to_owned()))}</p>
                    </section>
                </div>

                <details class="mt-8 border-t border-border py-4" data-testid="qr-specification" class:hidden=move || active_tab.get() != Tab::Generator>
                    <summary class="cursor-pointer text-sm font-semibold text-text focus:outline-none focus:ring-2 focus:ring-focus">"QR code specification"</summary>
                    <div class="mt-4 grid gap-6 lg:grid-cols-2">
                        <dl class="grid grid-cols-2 gap-4 text-sm">
                            <Diagnostic label="Mode" value=move || diagnostic_value(state, |details| mode_label(details.mode()).to_owned()) />
                            <Diagnostic label="ECC" value=move || diagnostic_value(state, |details| ecc_label(details.ecc()).to_owned()) />
                            <Diagnostic label="Version" value=move || diagnostic_value(state, version_label) />
                            <Diagnostic label="Mask" value=move || diagnostic_value(state, |details| details.mask().number().to_string()) />
                            <Diagnostic label="Matrix" value=move || diagnostic_value(state, |details| format!("{} x {} modules", details.matrix_modules(), details.matrix_modules())) />
                            <Diagnostic label="Output" value=move || diagnostic_value(state, |details| format!("{} px SVG / {} px PNG", details.svg_side_pixels(), details.png_side_pixels())) />
                            <Diagnostic label="Logo" value=move || diagnostic_value(state, |details| logo_label(details.logo_style(), details.logo_placement())) />
                            <Diagnostic label="Logo request" value=move || diagnostic_value(state, |details| logo_request_label(details.requested_logo_style(), details.logo_fallback_reason())) />
                            <Diagnostic label="Contrast" value=move || diagnostic_value(state, |details| contrast_label(details.contrast_ratio())) />
                            <Diagnostic label="Safety" value=move || diagnostic_value(state, |details| safety_label(details.safety()).to_owned()) />
                        </dl>
                        <div class="text-sm leading-6 text-text-muted" data-testid="release-guidance">
                            <p>"Choose SVG when resizing or preparing print output. Test every downloaded QR code with the final camera, screen, material, size, and placement."</p>
                            <p class="mt-2">"Logo output uses ECC H and may fall back to an unbranded code when the approved geometry is unavailable. The URL is never changed by that fallback."</p>
                        </div>
                    </div>
                </details>
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
    preview_worker: PreviewWorkerStore,
    request: PreviewRequest,
) {
    pending_timer.update(DebounceTimer::cancel);
    let revision = request.revision();
    match set_timeout_with_handle(
        move || {
            let dispatched = preview_worker
                .try_with_value(|worker| {
                    worker
                        .as_ref()
                        .ok_or(qr_web::preview_worker::PreviewWorkerError::Startup)?
                        .dispatch(&request)
                })
                .is_some_and(|result| result.is_ok());
            if !dispatched {
                state.update(|value| {
                    _ = value.complete_preview(revision, Err(WorkflowFailure::Internal));
                });
            }
        },
        PREVIEW_DEBOUNCE,
    ) {
        Ok(handle) => pending_timer.update(|timer| timer.replace(handle)),
        Err(_) => state.update(|value| {
            _ = value.complete_preview(revision, Err(WorkflowFailure::Internal));
        }),
    }
}

fn update_url_preview(
    url_state: RwSignal<UrlPayloadState>,
    state: RwSignal<WorkflowState>,
    pending_timer: DebounceSignal,
    preview_worker: PreviewWorkerStore,
) {
    match url_state.with(UrlPayloadState::compose) {
        Ok(payload) => {
            if let Some(Ok(request)) = state.try_update(|value| value.set_payload(payload)) {
                schedule_preview(state, pending_timer, preview_worker, request);
            }
        }
        Err(error) => {
            pending_timer.update(DebounceTimer::cancel);
            state.update(|value| {
                value.reject_url_failure(match error {
                    UrlPayloadError::InvalidBaseUrl => WorkflowFailure::InvalidUrl,
                    UrlPayloadError::MissingParameterName => WorkflowFailure::MissingParameterName,
                });
            });
        }
    }
}

fn encoded_url_guidance(
    url_state: RwSignal<UrlPayloadState>,
    state: RwSignal<WorkflowState>,
) -> String {
    let Ok(encoded_url) = url_state.with(UrlPayloadState::compose) else {
        return "The encoded URL will appear after the base URL is valid.".to_owned();
    };
    let maximum = link_capacity_guide()
        .into_iter()
        .find(|row| row.profile_id() == state.with(WorkflowState::profile_id))
        .map(|row| {
            if state.with(WorkflowState::logo_enabled) {
                row.with_logo_ascii_bytes()
            } else {
                row.without_logo_ascii_bytes()
            }
        })
        .unwrap_or_default();
    format!(
        "{} characters | {} UTF-8 bytes | typical ASCII maximum: {}",
        encoded_url.chars().count(),
        encoded_url.len(),
        maximum,
    )
}

const fn is_digital(profile_id: ProfileId) -> bool {
    matches!(
        profile_id,
        ProfileId::Small | ProfileId::Standard | ProfileId::PrimaryCta | ProfileId::HeroCampaign
    )
}

fn profile_from_value(value: &str) -> Option<ProfileId> {
    SUPPORTED_PROFILES
        .into_iter()
        .find(|profile| profile_presentation(profile.id()).value() == value)
        .map(|profile| profile.id())
}

fn select_profile(
    state: RwSignal<WorkflowState>,
    pending_timer: DebounceSignal,
    preview_worker: PreviewWorkerStore,
    profile_id: ProfileId,
) {
    if let Some(Ok(request)) = state.try_update(|value| value.select_profile(profile_id)) {
        schedule_preview(state, pending_timer, preview_worker, request);
    }
}

fn select_foreground(
    state: RwSignal<WorkflowState>,
    pending_timer: DebounceSignal,
    preview_worker: PreviewWorkerStore,
    theme: ForegroundTheme,
) {
    if let Some(Ok(request)) = state.try_update(|value| value.set_foreground_theme(theme)) {
        schedule_preview(state, pending_timer, preview_worker, request);
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

fn logo_label(style: LogoStyle, placement: Option<qr_web::workflow::LogoDiagnostics>) -> String {
    match (style, placement) {
        (LogoStyle::None, _) => "None".to_owned(),
        (LogoStyle::Bundled, Some(placement)) => format!(
            "ONE lettermark · {} data · {} remainder modules obscured",
            placement.obscured_data_modules(),
            placement.obscured_remainder_modules()
        ),
        (LogoStyle::Bundled, None) => "Unavailable".to_owned(),
    }
}

fn logo_request_label(style: LogoStyle, fallback_reason: Option<&'static str>) -> String {
    match (style, fallback_reason) {
        (LogoStyle::Bundled, Some(reason)) => format!("ONE requested; disabled: {reason}"),
        (LogoStyle::Bundled, None) => "ONE requested".to_owned(),
        (LogoStyle::None, _) => "No logo requested".to_owned(),
    }
}

fn contrast_label(ratio: qr_render::ContrastRatio) -> String {
    format!(
        "{}.{:02}:1",
        ratio.hundredths() / 100,
        ratio.hundredths() % 100
    )
}

fn safety_label(safety: OutputSafety) -> &'static str {
    match safety {
        OutputSafety::Safe => "Safe",
        OutputSafety::Caution => "Caution",
    }
}

fn main() {
    use std::{cell::RefCell, rc::Rc};

    use leptos::wasm_bindgen::closure::Closure;

    let Some(body) = leptos::prelude::document().body() else {
        return;
    };
    let app = Rc::new(RefCell::new(Some(leptos::mount::mount_to(body, App))));
    let app_on_pagehide = Rc::clone(&app);
    let pagehide = Closure::<dyn FnMut(Event)>::new(move |event: Event| {
        let persisted = event
            .dyn_ref::<leptos::web_sys::PageTransitionEvent>()
            .is_some_and(leptos::web_sys::PageTransitionEvent::persisted);
        if !persisted {
            _ = app_on_pagehide.borrow_mut().take();
        }
    });
    if leptos::prelude::window()
        .add_event_listener_with_callback("pagehide", pagehide.as_ref().unchecked_ref())
        .is_ok()
    {
        pagehide.forget();
    }
}
