//! Static "Usage" tab content: guidance on how and when to use the generated
//! QR codes. Every technical fact here must match `qr-render`'s compiled
//! profiles, approved themes, and logo policy — not a design mockup.

use leptos::prelude::*;

#[component]
pub fn UsageGuide() -> impl IntoView {
    view! {
        <div class="mt-8 space-y-10" data-testid="usage-guide">
            <section aria-labelledby="usage-overall-heading">
                <h2 id="usage-overall-heading" class="text-lg font-semibold text-text">"Overall"</h2>
                <p class="mt-2 text-sm leading-6 text-text-muted">
                    "This guideline defines the technical implementation requirements for the QR codes this tool generates for ONE digital products and customer communications. Every QR code is built entirely in your browser: the payload is never sent to a server."
                </p>
            </section>

            <section aria-labelledby="usage-principal-heading">
                <h2 id="usage-principal-heading" class="text-lg font-semibold text-text">"Principal"</h2>
                <div class="mt-4 grid gap-6 sm:grid-cols-3">
                    <div>
                        <img class="h-12 w-12 object-contain" src="/public/images/qr-sample.png" alt="" aria-hidden="true" />
                        <h3 class="mt-3 text-sm font-semibold text-text">"Brand consistency"</h3>
                        <p class="mt-1 text-sm text-text-muted">
                            "QR codes present a consistent appearance across ONE products: ONE magenta " <code>"#BD0F72"</code> " by default or black, rounded modules with standard square finders, an opaque white background, and the bundled ONE lettermark when the selected version supports it."
                        </p>
                    </div>
                    <div>
                        <img class="h-12 w-12 object-contain" src="/public/images/scan-icon.png" alt="" aria-hidden="true" />
                        <h3 class="mt-3 text-sm font-semibold text-text">"Technical precision"</h3>
                        <p class="mt-1 text-sm text-text-muted">
                            "Successful scanning depends on adequate contrast and clear space. Every export enforces a 4.5:1 minimum foreground/background contrast ratio and a 4-module quiet zone automatically; there is no way to configure an unsafe combination in this tool."
                        </p>
                    </div>
                    <div>
                        <img class="h-12 w-12 object-contain" src="/public/images/accessibility-icon.png" alt="" aria-hidden="true" />
                        <h3 class="mt-3 text-sm font-semibold text-text">"Accessibility"</h3>
                        <p class="mt-1 text-sm text-text-muted">
                            "Everyone should be able to reach the same destination regardless of whether they can scan the code. Always pair a placed QR code with a visible link, button, or instructions as a fallback."
                        </p>
                    </div>
                </div>
            </section>

            <section aria-labelledby="usage-when-heading">
                <h2 id="usage-when-heading" class="text-lg font-semibold text-text">"When to use"</h2>
                <p class="mt-2 text-sm leading-6 text-text-muted">
                    "Use a QR code when it provides a clear, meaningful bridge between where someone is now and a digital destination. The right output variant depends on whether the code is shown on a screen or printed."
                </p>
                <div class="mt-4 grid gap-6 sm:grid-cols-2">
                    <div>
                        <h3 class="text-sm font-semibold text-text">"1. Digital"</h3>
                        <p class="mt-1 text-sm text-text-muted">"Use a QR code in a digital interface when the destination is meant to be opened on a different device."</p>
                        <ul class="mt-2 list-inside list-disc text-sm text-text-muted">
                            <li>"Continue an activity on a mobile device"</li>
                            <li>"Open a mobile-specific experience"</li>
                            <li>"Transfer a URL, session, or information from desktop to mobile"</li>
                            <li>"Pair or connect a device"</li>
                        </ul>
                    </div>
                    <div>
                        <h3 class="text-sm font-semibold text-text">"2. Print"</h3>
                        <p class="mt-1 text-sm text-text-muted">"Use a QR code in physical materials when it provides a convenient bridge from the physical experience to a digital destination."</p>
                        <ul class="mt-2 list-inside list-disc text-sm text-text-muted">
                            <li>"Posters and signage"</li>
                            <li>"Printed documents: brochures, flyers, business cards, packaging"</li>
                            <li>"Exhibition and event materials, product or service information"</li>
                            <li>"Shipment or logistics documents"</li>
                        </ul>
                    </div>
                </div>
            </section>

            <section aria-labelledby="usage-anatomy-heading">
                <h2 id="usage-anatomy-heading" class="text-lg font-semibold text-text">"Anatomy"</h2>
                <p class="mt-2 text-sm leading-6 text-text-muted">
                    "Every generated symbol is a regular square array of modules made of function patterns — quiet zone, finder patterns, separators, timing patterns, and format/version information — surrounding an encoding region of data and error-correction codewords. Function modules are never repurposed to carry data or logo pixels."
                </p>
                <img class="mt-4 max-w-md rounded-lg border border-border" src="/public/images/qr-anatomy-diagram.png" alt="Diagram of a QR code labeling its quiet zone, finder pattern, separator, version info, and data/error-correction codewords" />
            </section>

            <section aria-labelledby="usage-rules-heading">
                <h2 id="usage-rules-heading" class="text-lg font-semibold text-text">"Standard generation rule"</h2>
                <p class="mt-2 text-sm leading-6 text-text-muted">
                    "Each QR code this tool exports follows the same fixed rules; none of these can be overridden per code."
                </p>
                <table class="mt-4 w-full border-collapse text-sm">
                    <thead>
                        <tr class="border-b border-border text-left text-xs font-semibold uppercase tracking-wide text-text-muted">
                            <th class="py-2 pr-4">"Property"</th>
                            <th class="py-2">"Standard"</th>
                        </tr>
                    </thead>
                    <tbody class="divide-y divide-border">
                        <tr><td class="py-2 pr-4 font-semibold text-text">"QR standard"</td><td class="py-2 text-text-muted">"ISO/IEC 18004:2024"</td></tr>
                        <tr><td class="py-2 pr-4 font-semibold text-text">"Export format"</td><td class="py-2 text-text-muted">"SVG or PNG"</td></tr>
                        <tr><td class="py-2 pr-4 font-semibold text-text">"Error correction"</td><td class="py-2 text-text-muted">"M by default; H automatically when the ONE logo is enabled"</td></tr>
                        <tr><td class="py-2 pr-4 font-semibold text-text">"Quiet zone"</td><td class="py-2 text-text-muted">"4 modules on every side"</td></tr>
                        <tr><td class="py-2 pr-4 font-semibold text-text">"ONE logo"</td><td class="py-2 text-text-muted">"Bundled ONE lettermark, available from Version 6 through Version 11; automatically omitted at Version 12 and above so the payload is never lost"</td></tr>
                        <tr><td class="py-2 pr-4 font-semibold text-text">"Colors"</td><td class="py-2 text-text-muted">"ONE magenta " <code>"#BD0F72"</code> " (default) or black " <code>"#000000"</code> ", always on an opaque white background"</td></tr>
                        <tr><td class="py-2 pr-4 font-semibold text-text">"Generator"</td><td class="py-2 text-text-muted">"This QR Generation Tool — runs entirely in your browser, no upload"</td></tr>
                    </tbody>
                </table>
            </section>

            <section aria-labelledby="usage-sizes-heading">
                <h2 id="usage-sizes-heading" class="text-lg font-semibold text-text">"Size and version recommendations"</h2>
                <p class="mt-2 text-sm leading-6 text-text-muted">
                    "Pick the output variant that matches where the code is used. Every variant fixes its own canvas size and approved QR version range, so a larger payload may need a variant with a wider range."
                </p>
                <table class="mt-4 w-full border-collapse text-sm">
                    <thead>
                        <tr class="border-b border-border text-left text-xs font-semibold uppercase tracking-wide text-text-muted">
                            <th class="py-2 pr-4">"Type"</th>
                            <th class="py-2 pr-4">"Usage"</th>
                            <th class="py-2 pr-4">"Recommended size"</th>
                            <th class="py-2">"QR version range"</th>
                        </tr>
                    </thead>
                    <tbody class="divide-y divide-border">
                        <tr><td class="py-2 pr-4 font-semibold text-text">"Digital"</td><td class="py-2 pr-4 text-text-muted">"Small — web footer, secondary CTA"</td><td class="py-2 pr-4 text-text-muted">"100 x 100 px"</td><td class="py-2 text-text-muted">"V5 - V6"</td></tr>
                        <tr><td class="py-2 pr-4 font-semibold text-text">"Digital"</td><td class="py-2 pr-4 text-text-muted">"Standard — general web content"</td><td class="py-2 pr-4 text-text-muted">"120 x 120 px"</td><td class="py-2 text-text-muted">"V5 - V8"</td></tr>
                        <tr><td class="py-2 pr-4 font-semibold text-text">"Digital"</td><td class="py-2 pr-4 text-text-muted">"Primary CTA — download app, continue on mobile"</td><td class="py-2 pr-4 text-text-muted">"160 x 160 px"</td><td class="py-2 text-text-muted">"V5 - V12"</td></tr>
                        <tr><td class="py-2 pr-4 font-semibold text-text">"Digital"</td><td class="py-2 pr-4 text-text-muted">"Hero / Campaign — landing page, campaign"</td><td class="py-2 pr-4 text-text-muted">"200 x 200 px"</td><td class="py-2 text-text-muted">"V8 - V12"</td></tr>
                        <tr><td class="py-2 pr-4 font-semibold text-text">"Print"</td><td class="py-2 pr-4 text-text-muted">"Business card"</td><td class="py-2 pr-4 text-text-muted">"25 mm"</td><td class="py-2 text-text-muted">"V5 - V12, test in actual print"</td></tr>
                        <tr><td class="py-2 pr-4 font-semibold text-text">"Print"</td><td class="py-2 pr-4 text-text-muted">"Flyer / Brochure"</td><td class="py-2 pr-4 text-text-muted">"30 mm"</td><td class="py-2 text-text-muted">"V5 - V12, test in actual print"</td></tr>
                        <tr><td class="py-2 pr-4 font-semibold text-text">"Print"</td><td class="py-2 pr-4 text-text-muted">"Poster / Package"</td><td class="py-2 pr-4 text-text-muted">"40 mm"</td><td class="py-2 text-text-muted">"V5 - V12, test in actual print, consider material surface"</td></tr>
                    </tbody>
                </table>
                <p class="mt-2 text-xs text-text-muted">
                    "Print sizes are 150 dpi artifact conversions, not a physical-size guarantee. Always test the exported file on the final material, device, and viewing distance before shipping it."
                </p>
            </section>

            <section aria-labelledby="usage-signage-heading">
                <h2 id="usage-signage-heading" class="text-lg font-semibold text-text">"Large signage: use viewing distance, not pixels"</h2>
                <p class="mt-2 text-sm leading-6 text-text-muted">
                    "This tool does not encode a physical size — it exports SVG or PNG pixels. For large-format displays or signage, choose a physical print size using the scanner's expected distance rather than picking pixels alone, then export SVG and scale it to that size."
                </p>
                <table class="mt-4 w-full max-w-sm border-collapse text-sm">
                    <thead>
                        <tr class="border-b border-border text-left text-xs font-semibold uppercase tracking-wide text-text-muted">
                            <th class="py-2 pr-4">"Viewing distance"</th>
                            <th class="py-2">"Approx. QR size"</th>
                        </tr>
                    </thead>
                    <tbody class="divide-y divide-border">
                        <tr><td class="py-2 pr-4 text-text-muted">"0.5 m"</td><td class="py-2 text-text-muted">"50 mm"</td></tr>
                        <tr><td class="py-2 pr-4 text-text-muted">"1 m"</td><td class="py-2 text-text-muted">"100 mm"</td></tr>
                        <tr><td class="py-2 pr-4 text-text-muted">"2 m"</td><td class="py-2 text-text-muted">"200 mm"</td></tr>
                        <tr><td class="py-2 pr-4 text-text-muted">"3 m"</td><td class="py-2 text-text-muted">"300 mm"</td></tr>
                        <tr><td class="py-2 pr-4 text-text-muted">"5 m"</td><td class="py-2 text-text-muted">"500 mm"</td></tr>
                        <tr><td class="py-2 pr-4 text-text-muted">"10 m"</td><td class="py-2 text-text-muted">"1000 mm"</td></tr>
                    </tbody>
                </table>
            </section>
        </div>
    }
}
