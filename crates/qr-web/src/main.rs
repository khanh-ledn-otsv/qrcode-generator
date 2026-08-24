use std::time::Duration;

use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use leptos::web_sys::{ClipboardEvent, DragEvent, Event, HtmlTextAreaElement, InputEvent};
use qr_render::{ForegroundTheme, LogoStyle, OutputSafety, ProfileId, SUPPORTED_PROFILES};
use qr_web::debounce::DebounceTimer;
use qr_web::download::trigger_download;
use qr_web::preview_worker::PreviewWorker;
use qr_web::workflow::{
    ArtifactKind, PreviewRequest, WorkflowFailure, WorkflowState, ecc_label, link_capacity_guide,
    mode_label, profile_presentation, textarea_display_utf16_length, version_label,
};

const PREVIEW_DEBOUNCE: Duration = Duration::from_millis(250);
type DebounceSignal = RwSignal<DebounceTimer>;
type PreviewWorkerStore = StoredValue<Option<PreviewWorker>, LocalStorage>;

#[component]
fn App() -> impl IntoView {
    let state = RwSignal::new(WorkflowState::new(ProfileId::Standard));
    let pending_timer = RwSignal::new(DebounceTimer::default());
    let pending_edit_start = RwSignal::new(None::<u32>);
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
                            schedule_preview(state, pending_timer, preview_worker, request);
                        }
                    }
                />
                <span class="block text-sm font-bold text-slate-950">{presentation.name()}</span>
                <span class="mt-1 block text-xs leading-5 text-slate-600">
                    {format!(
                        "{} px SVG · {} px PNG · V{}–V{}",
                        profile.svg_dimensions().width().get(),
                        profile.png_dimensions().width().get(),
                        profile.minimum_version().number(),
                        profile.maximum_version().number(),
                    )}
                </span>
            </label>
        }
    });
    let foreground_options = [ForegroundTheme::Magenta, ForegroundTheme::Black].map(|theme| {
        view! {
            <label class=move || profile_card_class(state.with(|value| value.foreground_theme() == theme))>
                <input
                    class="peer sr-only"
                    type="radio"
                    name="foreground-theme"
                    value=foreground_theme_value(theme)
                    prop:checked=move || state.with(|value| value.foreground_theme() == theme)
                    on:change=move |_| {
                        if let Some(Ok(request)) = state.try_update(|value| value.set_foreground_theme(theme)) {
                            schedule_preview(state, pending_timer, preview_worker, request);
                        }
                    }
                />
                <span class="flex items-center gap-2 text-sm font-bold text-slate-950">
                    <span class=foreground_theme_swatch_class(theme) aria-hidden="true"></span>
                    <span>{foreground_theme_label(theme)}</span>
                </span>
                <span class="mt-1 block text-xs leading-5 text-slate-600">{foreground_theme_guidance(theme)}</span>
            </label>
        }
    });
    let link_capacity_rows = link_capacity_guide().map(|row| {
        let profile = profile_presentation(row.profile_id());
        view! {
            <tr class="border-t border-slate-200">
                <th scope="row" class="px-4 py-3 text-left font-semibold text-slate-950">
                    {profile.name()}
                </th>
                <td class="px-4 py-3 text-right text-slate-700">
                    {format_capacity(row.without_logo_ascii_bytes())}
                </td>
                <td class="px-4 py-3 text-right text-slate-700">
                    {format_capacity(row.with_logo_ascii_bytes())}
                </td>
            </tr>
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
                        "Your text stays in this browser. Standard output refits at error correction M; logo output refits at H with a Version 6 minimum."
                    </p>
                </header>

                <section
                    class="mb-6 rounded-3xl border border-slate-200 bg-white p-5 shadow-sm sm:p-8"
                    aria-labelledby="link-guide-heading"
                    data-testid="link-guide"
                >
                    <div class="max-w-3xl">
                        <p class="text-brand text-sm font-bold uppercase tracking-[0.18em]">"Practical guide"</p>
                        <h2 id="link-guide-heading" class="mt-2 text-2xl font-black tracking-tight text-slate-950">
                            "For links that scan easily"
                        </h2>
                        <p class="mt-2 text-sm leading-6 text-slate-600">
                            "Longer links need more QR modules. These choices help keep the code simpler and preserve room for branding."
                        </p>
                    </div>

                    <div class="mt-6 overflow-x-auto rounded-2xl ring-1 ring-inset ring-slate-200">
                        <table class="w-full min-w-[34rem] border-collapse text-sm">
                            <caption class="px-4 pb-3 pt-4 text-left font-bold text-slate-950">
                                "Maximum typical ASCII link length by output variant"
                            </caption>
                            <thead class="bg-slate-100 text-slate-700">
                                <tr>
                                    <th scope="col" class="px-4 py-3 text-left font-semibold">"Output variant"</th>
                                    <th scope="col" class="px-4 py-3 text-right font-semibold">"Without logo"</th>
                                    <th scope="col" class="px-4 py-3 text-right font-semibold">"With logo"</th>
                                </tr>
                            </thead>
                            <tbody>{link_capacity_rows}</tbody>
                        </table>
                    </div>
                    <p class="mt-3 text-sm leading-6 text-slate-600">
                        "These totals cover ASCII links that use QR Byte mode, including the scheme, host, path, query, and fragment. Each ASCII character is one byte. Non-ASCII characters can use multiple UTF-8 bytes plus encoding overhead, while links limited to the QR alphanumeric set can sometimes fit more. The preview result for your exact text is authoritative."
                    </p>
                    <p class="mt-3 text-sm leading-6 text-slate-600">
                        "Without-logo output uses ECC M; logo output uses ECC H. Every fixed variant approves the centered logo only at Version 6. The difference is not a fixed character subtraction, and ECC H's nominal percentage is not an occlusion budget."
                    </p>

                    <section class="mt-8" aria-labelledby="variant-guide-heading">
                        <h3 id="variant-guide-heading" class="text-xl font-black text-slate-950">
                            "Which output variant should I choose?"
                        </h3>
                        <p class="mt-2 max-w-3xl text-sm leading-6 text-slate-600">
                            "Choose the named destination that matches your placement. Every export keeps the same fixed dimensions in the preview, SVG, PNG, diagnostics, and downloads."
                        </p>

                        <div class="mt-4 grid gap-4 md:grid-cols-2">
                            <article class="rounded-2xl bg-slate-50 p-5 ring-1 ring-inset ring-slate-200">
                                <h4 class="text-base font-bold text-slate-950">"Small"</h4>
                                <p class="mt-2 text-sm leading-6 text-slate-600">
                                    "Small uses a fixed 100 px SVG and 300 px PNG for web footers and secondary calls to action. It supports Versions 5–6."
                                </p>
                            </article>

                            <article class="rounded-2xl bg-slate-50 p-5 ring-1 ring-inset ring-slate-200">
                                <h4 class="text-base font-bold text-slate-950">"Standard"</h4>
                                <p class="mt-2 text-sm leading-6 text-slate-600">
                                    "Standard uses a fixed 120 px SVG and 360 px PNG for general web content. It supports Versions 5–8."
                                </p>
                            </article>

                            <article class="rounded-2xl bg-slate-50 p-5 ring-1 ring-inset ring-slate-200">
                                <h4 class="text-base font-bold text-slate-950">"Primary CTA"</h4>
                                <p class="mt-2 text-sm leading-6 text-slate-600">
                                    "Primary CTA uses a fixed 160 px SVG and 480 px PNG for download-app and continue-on-mobile calls to action. It supports Versions 5–12."
                                </p>
                            </article>

                            <article class="rounded-2xl bg-slate-50 p-5 ring-1 ring-inset ring-slate-200">
                                <h4 class="text-base font-bold text-slate-950">"Hero / Campaign"</h4>
                                <p class="mt-2 text-sm leading-6 text-slate-600">
                                    "Hero / Campaign uses a fixed 200 px SVG and 600 px PNG for landing pages and campaigns. It supports Versions 8–12."
                                </p>
                            </article>

                            <article class="rounded-2xl bg-fuchsia-50 p-5 ring-1 ring-inset ring-fuchsia-200 md:col-span-2">
                                <h4 class="text-base font-bold text-slate-950">"Print"</h4>
                                <p class="mt-2 text-sm leading-6 text-slate-700">
                                    "Business card is 25 mm converted to 148 px, Flyer / Brochure is 30 mm converted to 177 px, and Poster / Package is 40 mm converted to 236 px. Their PNG exports are 444 px, 531 px, and 708 px respectively."
                                </p>
                                <p class="mt-2 text-sm leading-6 text-slate-700">
                                    "These pixel values are a 150 dpi artifact policy, not a physical-size guarantee. Test the final output with the owner, printer, material, and surface."
                                </p>
                                <p class="mt-2 text-sm leading-6 text-slate-700">
                                    "Version ranges retain a scannable integer module pitch after the four-module quiet zone and logo constraint. A Version v symbol has a logical width of 4v + 25 modules."
                                </p>
                            </article>
                        </div>
                    </section>

                    <div class="mt-6 grid gap-4 md:grid-cols-2">
                        <article class="rounded-2xl bg-slate-50 p-5 ring-1 ring-inset ring-slate-200">
                            <h3 class="text-base font-bold text-slate-950">"Keep the link short"</h3>
                            <p class="mt-2 text-sm leading-6 text-slate-600">
                                "A shorter URL usually produces a smaller, less dense QR code. If the destination URL is long, shorten it when you have a trustworthy short link."
                            </p>
                        </article>

                        <article class="rounded-2xl bg-slate-50 p-5 ring-1 ring-inset ring-slate-200">
                            <h3 class="text-base font-bold text-slate-950">"For a long link, try no logo"</h3>
                            <p class="mt-2 text-sm leading-6 text-slate-600">
                                "Turning off the logo uses standard ECC M and avoids covering QR modules, giving the fixed-size profiles more room for the link."
                            </p>
                        </article>

                        <article class="rounded-2xl bg-slate-50 p-5 ring-1 ring-inset ring-slate-200">
                            <h3 class="text-base font-bold text-slate-950">"Need the logo? Keep it compact"</h3>
                            <p class="mt-2 text-sm leading-6 text-slate-600">
                                "Every fixed output keeps the approved centered logo at Version 6. If the exact payload needs another version, disable the logo without changing the text."
                            </p>
                        </article>

                        <article class="rounded-2xl bg-slate-50 p-5 ring-1 ring-inset ring-slate-200">
                            <h3 class="text-base font-bold text-slate-950">"PNG resolution"</h3>
                            <p class="mt-2 text-sm leading-6 text-slate-600">
                                "Every fixed-size profile downloads PNGs at 3× its listed SVG width."
                            </p>
                        </article>

                        <article class="rounded-2xl bg-emerald-50 p-5 ring-1 ring-inset ring-emerald-200 md:col-span-2">
                            <h3 class="text-base font-bold text-slate-950">"Scan before you use it"</h3>
                            <p class="mt-2 text-sm leading-6 text-slate-700">
                                "Always scan the final QR code before publishing or printing it. Test the downloaded file with a real phone or scanner in the same size, material, screen, and placement where people will use it."
                            </p>
                        </article>
                    </div>
                </section>

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
                                            preview_worker,
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
                                    Some(Ok(request)) => schedule_preview(state, pending_timer, preview_worker, request),
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
                                        preview_worker,
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

                        <fieldset class="mt-8">
                            <legend class="text-sm font-semibold text-slate-800">"Foreground color"</legend>
                            <div class="mt-3 grid gap-3 sm:grid-cols-2">{foreground_options}</div>
                        </fieldset>

                        <fieldset class="mt-8" aria-describedby="payload-caution">
                            <legend class="text-sm font-semibold text-slate-800">"Bundled logo"</legend>
                            <label class=move || profile_card_class(state.with(WorkflowState::logo_enabled))>
                                <input
                                    class="peer sr-only"
                                    type="checkbox"
                                    name="bundled-logo"
                                    prop:checked=move || state.with(WorkflowState::logo_enabled)
                                    on:change=move |event| {
                                        if let Some(Ok(request)) = state.try_update(|current| current.set_logo_enabled(event_target_checked(&event))) {
                                            schedule_preview(state, pending_timer, preview_worker, request);
                                        }
                                    }
                                />
                                <span class="block text-sm font-bold text-slate-950">"ONE lettermark"</span>
                                <span class="mt-1 block text-xs leading-5 text-slate-600">"Uses ECC H and an opaque white knockout"</span>
                            </label>
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
                                    class="[&>svg]:block [&>svg]:h-auto"
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
                                <Diagnostic label="Mask" value=move || diagnostic_value(state, |details| details.mask().number().to_string()) />
                                <Diagnostic label="Version" value=move || diagnostic_value(state, version_label) />
                                <Diagnostic label="Data bits" value=move || diagnostic_value(state, |details| format!("{} / {}", details.used_data_bits(), details.available_data_bits())) />
                                <Diagnostic label="Data codewords" value=move || diagnostic_value(state, |details| details.data_codewords().to_string()) />
                                <Diagnostic label="Matrix" value=move || diagnostic_value(state, |details| format!("{} × {} modules", details.matrix_modules(), details.matrix_modules())) />
                                <Diagnostic label="Quiet zone" value=move || diagnostic_value(state, |details| format!("{} modules per side", details.quiet_zone_modules())) />
                                <Diagnostic label="PNG geometry" value=move || diagnostic_value(state, |details| format!("{} px/module · {} px symbol · {} px padding", details.module_scale(), details.rendered_symbol_side_pixels(), details.outer_padding_per_side())) />
                                <Diagnostic label="Output" value=move || diagnostic_value(state, |details| format!("{} px SVG · {} px PNG", details.svg_side_pixels(), details.png_side_pixels())) />
                                <Diagnostic label="Foreground" value=move || diagnostic_value(state, |details| foreground_diagnostic_label(details.foreground_theme())) />
                                <Diagnostic label="Background" value=move || diagnostic_value(state, |_| "Opaque white".to_owned()) />
                                <Diagnostic label="Modules" value=move || diagnostic_value(state, |_| "Rounded ONE".to_owned()) />
                                <Diagnostic label="Logo" value=move || diagnostic_value(state, |details| logo_label(details.logo_style(), details.logo_placement())) />
                                <Diagnostic label="Logo bounds" value=move || diagnostic_value(state, |details| logo_bounds_label(details.logo_placement())) />
                                <Diagnostic label="Contrast" value=move || diagnostic_value(state, |details| contrast_label(details.contrast_ratio())) />
                                <Diagnostic label="Safety" value=move || diagnostic_value(state, |details| safety_label(details.safety()).to_owned()) />
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
                            <div data-testid="release-guidance" class="mt-4 rounded-2xl bg-amber-50 px-4 py-3 text-sm leading-6 text-amber-950">
                                <p>"Choose SVG first when resizing or preparing print output. Place printed codes at 25–30 mm or larger."</p>
                                <p class="mt-2">"Logo output needs extra validation. Test the final artifact with the actual camera, scanner, screen, print material, and placement environment before distribution."</p>
                            </div>
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

fn apply_raw_textarea_insertion(
    state: RwSignal<WorkflowState>,
    pending_timer: DebounceSignal,
    preview_worker: PreviewWorkerStore,
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
        schedule_preview(state, pending_timer, preview_worker, request);
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
        "focus-within:ring-brand block cursor-pointer rounded-2xl border border-brand bg-pink-50 p-4 ring-2 ring-brand ring-offset-2 transition"
    } else {
        "focus-within:ring-brand block cursor-pointer rounded-2xl border border-slate-200 bg-white p-4 transition hover:border-slate-400 focus-within:ring-2 focus-within:ring-offset-2"
    }
}

fn format_capacity(bytes: usize) -> String {
    let amount = if bytes >= 1_000 {
        format!("{},{:03}", bytes / 1_000, bytes % 1_000)
    } else {
        bytes.to_string()
    };
    format!("{amount} characters / bytes")
}

fn foreground_theme_value(theme: ForegroundTheme) -> &'static str {
    match theme {
        ForegroundTheme::Magenta => "magenta",
        ForegroundTheme::Black => "black",
    }
}

fn foreground_theme_label(theme: ForegroundTheme) -> &'static str {
    match theme {
        ForegroundTheme::Magenta => "ONE magenta",
        ForegroundTheme::Black => "Black",
    }
}

fn foreground_theme_swatch_class(theme: ForegroundTheme) -> &'static str {
    match theme {
        ForegroundTheme::Magenta => {
            "h-4 w-4 rounded-full bg-brand ring-1 ring-inset ring-slate-300"
        }
        ForegroundTheme::Black => "h-4 w-4 rounded-full bg-black ring-1 ring-inset ring-slate-300",
    }
}

fn foreground_theme_guidance(theme: ForegroundTheme) -> &'static str {
    match theme {
        ForegroundTheme::Magenta => "Brand foreground with matching logo",
        ForegroundTheme::Black => "Black foreground with matching logo",
    }
}

fn foreground_diagnostic_label(theme: ForegroundTheme) -> String {
    match theme {
        ForegroundTheme::Magenta => "ONE magenta #BD0F72".to_owned(),
        ForegroundTheme::Black => "Black #000000".to_owned(),
    }
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

fn logo_bounds_label(placement: Option<qr_web::workflow::LogoDiagnostics>) -> String {
    let Some(placement) = placement else {
        return "None".to_owned();
    };
    format!(
        "source ({}, {}) {} × {} modules · knockout ({}, {}) {} × {} modules · {} module protected clearance",
        module_decimal(placement.source_left_ten_thousandths()),
        module_decimal(placement.source_top_ten_thousandths()),
        module_decimal(placement.source_width_ten_thousandths()),
        module_decimal(placement.source_height_ten_thousandths()),
        placement.knockout_left(),
        placement.knockout_top(),
        placement.knockout_width(),
        placement.knockout_height(),
        placement.protected_clearance(),
    )
}

fn module_decimal(ten_thousandths: u32) -> String {
    let whole = ten_thousandths / 10_000;
    let fractional = ten_thousandths % 10_000;
    if fractional == 0 {
        whole.to_string()
    } else {
        format!("{whole}.{fractional:04}")
            .trim_end_matches('0')
            .to_owned()
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
