use std::time::Duration;

use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use leptos::web_sys::{ClipboardEvent, DragEvent, Event, HtmlTextAreaElement, InputEvent};
use qr_render::{
    APPROVED_BACKGROUNDS, Background, FinderStyle, Foreground, LogoStyle, ModuleStyle,
    OutputSafety, ProfileId, Rgba, SUPPORTED_PROFILES,
};
use qr_web::debounce::DebounceTimer;
use qr_web::download::trigger_download;
use qr_web::workflow::{
    ArtifactKind, PreviewRequest, WorkflowFailure, WorkflowState, ecc_label, evaluate_preview,
    link_capacity_guide, mode_label, profile_presentation, textarea_display_utf16_length,
    version_label,
};

const PREVIEW_DEBOUNCE: Duration = Duration::from_millis(250);
type DebounceSignal = RwSignal<DebounceTimer>;

#[derive(Clone, Copy)]
struct BackgroundPresentation {
    name: &'static str,
    value: &'static str,
    description: &'static str,
}

#[component]
fn App() -> impl IntoView {
    let state = RwSignal::new(WorkflowState::new(ProfileId::Content));
    let pending_timer = RwSignal::new(DebounceTimer::default());
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
                    {if profile.id() == ProfileId::Adaptive {
                        format!("Automatic dimensions · up to V{}", profile.maximum_version().number())
                    } else {
                        format!(
                            "{} px SVG · {} px PNG · up to V{}",
                            profile.svg_dimensions().width().get(),
                            profile.png_dimensions().width().get(),
                            profile.maximum_version().number(),
                        )
                    }}
                </span>
            </label>
        }
    });
    let background_options = APPROVED_BACKGROUNDS.map(|background| {
        let presentation = background_presentation(background);
        view! {
                <label class=move || profile_card_class(state.with(|current| current.background() == background))>
                    <input
                        class="peer sr-only"
                        type="radio"
                        name="background-treatment"
                        value=presentation.value
                        disabled=move || state.with(WorkflowState::logo_enabled) && matches!(background, Background::Transparent)
                        prop:checked=move || state.with(|current| current.background() == background)
                        on:change=move |_| {
                            if let Some(Ok(request)) = state.try_update(|current| current.select_background(background)) {
                                schedule_preview(state, pending_timer, request);
                            }
                        }
                    />
                <span class="block text-sm font-bold text-slate-950">{presentation.name}</span>
                <span class="mt-1 block text-xs leading-5 text-slate-600">{presentation.description}</span>
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
                        "Without-logo output uses ECC M; logo output uses ECC H. Fixed variants approve logo placement only at Version 6, while Adaptive approves it through Version 11. The difference is not a fixed character subtraction, and ECC H's nominal percentage is not an occlusion budget."
                    </p>

                    <section class="mt-8" aria-labelledby="variant-guide-heading">
                        <h3 id="variant-guide-heading" class="text-xl font-black text-slate-950">
                            "Which output variant should I choose?"
                        </h3>
                        <p class="mt-2 max-w-3xl text-sm leading-6 text-slate-600">
                            "Choose a fixed variant when your layout needs predictable file dimensions. Choose Adaptive when the link may outgrow those ceilings and the final pixel dimensions can change with the payload."
                        </p>

                        <div class="mt-4 grid gap-4 md:grid-cols-2">
                            <article class="rounded-2xl bg-slate-50 p-5 ring-1 ring-inset ring-slate-200">
                                <h4 class="text-base font-bold text-slate-950">"Inline"</h4>
                                <p class="mt-2 text-sm leading-6 text-slate-600">
                                    "Inline uses a fixed 100 px SVG and 300 px PNG, through Version 6. Choose it for compact interface placements and short links. Its logo stays exactly centered because branded fixed output is limited to Version 6."
                                </p>
                            </article>

                            <article class="rounded-2xl bg-slate-50 p-5 ring-1 ring-inset ring-slate-200">
                                <h4 class="text-base font-bold text-slate-950">"Content"</h4>
                                <p class="mt-2 text-sm leading-6 text-slate-600">
                                    "Content uses a fixed 120 px SVG and 360 px PNG, through Version 8 without the logo. Choose it for article, card, and standard website placements that need more room than Inline. Logo output remains at Version 6."
                                </p>
                            </article>

                            <article class="rounded-2xl bg-slate-50 p-5 ring-1 ring-inset ring-slate-200">
                                <h4 class="text-base font-bold text-slate-950">"Landing"</h4>
                                <p class="mt-2 text-sm leading-6 text-slate-600">
                                    "Landing uses a fixed 150 px SVG and 450 px PNG, through Version 12 without the logo. Choose it for prominent web or campaign placements and longer no-logo links. Logo output remains at Version 6."
                                </p>
                            </article>

                            <article class="rounded-2xl bg-slate-50 p-5 ring-1 ring-inset ring-slate-200">
                                <h4 class="text-base font-bold text-slate-950">"Print"</h4>
                                <p class="mt-2 text-sm leading-6 text-slate-600">
                                    "Print uses a fixed 160 px SVG and 480 px PNG, through Version 13 without the logo. Choose it as the largest fixed starting artifact, then place it at 25–30 mm or larger and test the final material. Logo output remains at Version 6."
                                </p>
                            </article>

                            <article class="rounded-2xl bg-fuchsia-50 p-5 ring-1 ring-inset ring-fuchsia-200 md:col-span-2">
                                <h4 class="text-base font-bold text-slate-950">"Adaptive"</h4>
                                <p class="mt-2 text-sm leading-6 text-slate-700">
                                    "Adaptive selects the smallest QR version that fits your exact text, then sizes the square output from that matrix and a four-module quiet zone on every side. SVG uses 4 pixels per logical module and PNG uses 6, with no surplus padding, so the downloaded dimensions grow with longer links."
                                </p>
                                <p class="mt-2 text-sm leading-6 text-slate-700">
                                    "With the logo enabled, the logo is exactly centered at Version 6. Versions 7–11 move it six modules above center, while keeping it horizontally centered, to avoid protected alignment modules. Version 12 or higher rejects the logo because a safe placement has not been approved; disable the logo to preserve the exact link and continue through Version 40."
                                </p>
                                <p class="mt-2 text-sm leading-6 text-slate-700">
                                    "Choose Adaptive for variable or long links, or when a fixed variant reports a capacity or logo-placement limit. Choose a fixed variant instead when predictable dimensions or an exactly centered logo matter more than maximum capacity."
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
                            <h3 class="text-base font-bold text-slate-950">"Need the logo? Choose Adaptive"</h3>
                            <p class="mt-2 text-sm leading-6 text-slate-600">
                                "Adaptive sizes the output for the QR version and uses reviewed, version-aware logo placement through Version 11. If the link needs Version 12 or higher, disable the logo."
                            </p>
                        </article>

                        <article class="rounded-2xl bg-slate-50 p-5 ring-1 ring-inset ring-slate-200">
                            <h3 class="text-base font-bold text-slate-950">"PNG resolution"</h3>
                            <p class="mt-2 text-sm leading-6 text-slate-600">
                                "Fixed-size profiles download PNGs at 3× their listed SVG width. Adaptive PNGs use 6 pixels per module versus 4 for SVG, so their width is 1.5×."
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

                        <fieldset class="mt-8" aria-describedby="payload-caution">
                            <legend class="text-sm font-semibold text-slate-800">"Background treatment"</legend>
                            <div class="mt-3 grid gap-3 sm:grid-cols-2">{background_options}</div>
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
                                            schedule_preview(state, pending_timer, request);
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
                                    class:hidden=move || state.with(|value| value.preview().is_none() || matches!(value.background(), Background::Transparent))
                                    class="[&>svg]:block [&>svg]:h-auto"
                                    aria-hidden="true"
                                    inner_html=move || state.with(|value| value.preview().map(|preview| preview.svg().to_owned()).unwrap_or_default())
                                ></div>
                                <div
                                    class:hidden=move || state.with(|value| value.preview().is_none() || !matches!(value.background(), Background::Transparent))
                                    class="grid w-full grid-cols-2 gap-2"
                                    aria-hidden="true"
                                >
                                    <figure data-testid="transparent-surface-preview" class="relative grid aspect-square place-items-center rounded-lg border border-slate-300 bg-white p-2">
                                        <div class="[&>svg]:block [&>svg]:h-auto" inner_html=move || state.with(|value| value.preview().map(|preview| preview.svg().to_owned()).unwrap_or_default())></div>
                                        <figcaption class="absolute bottom-1 left-1 rounded bg-white/90 px-2 py-1 text-xs font-bold text-slate-800">"White"</figcaption>
                                    </figure>
                                    <figure data-testid="transparent-surface-preview" class="relative grid aspect-square place-items-center rounded-lg border border-slate-300 bg-slate-200 p-2">
                                        <div class="[&>svg]:block [&>svg]:h-auto" inner_html=move || state.with(|value| value.preview().map(|preview| preview.svg().to_owned()).unwrap_or_default())></div>
                                        <figcaption class="absolute bottom-1 left-1 rounded bg-white/90 px-2 py-1 text-xs font-bold text-slate-800">"Light gray"</figcaption>
                                    </figure>
                                    <figure data-testid="transparent-surface-preview" class="relative grid aspect-square place-items-center rounded-lg border border-slate-700 bg-slate-900 p-2">
                                        <div class="[&>svg]:block [&>svg]:h-auto" inner_html=move || state.with(|value| value.preview().map(|preview| preview.svg().to_owned()).unwrap_or_default())></div>
                                        <figcaption class="absolute bottom-1 left-1 rounded bg-white/90 px-2 py-1 text-xs font-bold text-slate-800">"Dark"</figcaption>
                                    </figure>
                                    <figure data-testid="transparent-surface-preview" class="preview-surface-patterned relative grid aspect-square place-items-center rounded-lg border border-slate-400 p-2">
                                        <div class="[&>svg]:block [&>svg]:h-auto" inner_html=move || state.with(|value| value.preview().map(|preview| preview.svg().to_owned()).unwrap_or_default())></div>
                                        <figcaption class="absolute bottom-1 left-1 rounded bg-white/90 px-2 py-1 text-xs font-bold text-slate-800">"Patterned"</figcaption>
                                    </figure>
                                </div>
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
                                <Diagnostic label="Foreground" value=move || diagnostic_value(state, |details| foreground_color(details.foreground()).to_owned()) />
                                <Diagnostic label="Background" value=move || diagnostic_value(state, |details| background_presentation(details.background()).name.to_owned()) />
                                <Diagnostic label="Non-finder modules" value=move || diagnostic_value(state, |details| module_style_label(details.module_style()).to_owned()) />
                                <Diagnostic label="Finders" value=move || diagnostic_value(state, |details| finder_style_label(details.finder_style()).to_owned()) />
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
                                <p class="mt-2">"Transparent output and logo output need extra validation. Test the final artifact with the actual camera, scanner, screen, print material, and placement environment before distribution."</p>
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
    request: PreviewRequest,
) {
    pending_timer.update(DebounceTimer::cancel);
    let revision = request.revision();
    match set_timeout_with_handle(
        move || {
            let result = evaluate_preview(&request);
            state.update(|value| {
                _ = value.complete_preview(revision, result);
            });
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
        "focus-within:ring-brand block cursor-pointer rounded-2xl border border-brand bg-pink-50 p-4 ring-2 ring-brand ring-offset-2 transition"
    } else {
        "focus-within:ring-brand block cursor-pointer rounded-2xl border border-slate-200 bg-white p-4 transition hover:border-slate-400 focus-within:ring-2 focus-within:ring-offset-2"
    }
}

fn foreground_color(foreground: Foreground) -> &'static str {
    match foreground {
        Foreground::Brand => "#BD0F72",
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

fn background_presentation(background: Background) -> BackgroundPresentation {
    match background {
        Background::Opaque(Rgba::WHITE) => BackgroundPresentation {
            name: "Opaque white",
            value: "white",
            description: "Known placement contrast",
        },
        Background::Transparent => BackgroundPresentation {
            name: "Transparent",
            value: "transparent",
            description: "Requires placement checks",
        },
        Background::Opaque(_) => BackgroundPresentation {
            name: "Unapproved opaque color",
            value: "unapproved",
            description: "Unavailable",
        },
    }
}

const fn module_style_label(style: ModuleStyle) -> &'static str {
    match style {
        ModuleStyle::CompactDots => "Compact dots",
    }
}

const fn finder_style_label(style: FinderStyle) -> &'static str {
    match style {
        FinderStyle::StandardSquare => "Standard square",
    }
}

fn logo_label(style: LogoStyle, placement: Option<qr_render::LogoPlacement>) -> String {
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

fn logo_bounds_label(placement: Option<qr_render::LogoPlacement>) -> String {
    let Some(placement) = placement else {
        return "None".to_owned();
    };
    let source = placement.source_bounds();
    let knockout = placement.knockout_bounds();
    format!(
        "source ({}, {}) {} × {} modules · knockout ({}, {}) {} × {} modules · {} module protected clearance",
        module_decimal(source.left_ten_thousandths()),
        module_decimal(source.top_ten_thousandths()),
        module_decimal(source.width_ten_thousandths()),
        module_decimal(source.height_ten_thousandths()),
        knockout.left().get(),
        knockout.top().get(),
        knockout.width().get(),
        knockout.height().get(),
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

fn contrast_label(ratio: Option<qr_render::ContrastRatio>) -> String {
    ratio.map_or_else(
        || "Unknown on placement surface".to_owned(),
        |ratio| {
            format!(
                "{}.{:02}:1",
                ratio.hundredths() / 100,
                ratio.hundredths() % 100
            )
        },
    )
}

fn safety_label(safety: OutputSafety) -> &'static str {
    match safety {
        OutputSafety::Safe => "Safe",
        OutputSafety::Caution => "Caution",
    }
}

fn main() {
    leptos::mount::mount_to_body(App);
}
