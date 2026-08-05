use leptos::prelude::*;

#[component]
fn App() -> impl IntoView {
    view! {
        <main class="relative isolate grid min-h-screen place-items-center overflow-hidden bg-slate-50 px-6 py-16">
            <div class="absolute inset-0 -z-20 bg-[radial-gradient(circle_at_top,#fce7f3_0%,#f8fafc_46%,#f1f5f9_100%)]"></div>
            <div class="bg-brand/15 absolute left-1/2 top-0 -z-10 h-80 w-80 -translate-x-1/2 -translate-y-1/2 rounded-full blur-3xl"></div>

            <section class="w-full max-w-2xl rounded-3xl border border-slate-200 bg-white/85 p-8 text-center shadow-2xl shadow-slate-300/50 backdrop-blur-xl sm:p-14">
                <div class="bg-brand/10 text-brand mx-auto mb-8 grid size-14 place-items-center rounded-2xl ring-1 ring-inset ring-brand/15">
                    <span class="text-2xl font-bold">"Q"</span>
                </div>

                <p class="text-brand mb-4 text-sm font-semibold uppercase tracking-[0.28em]">
                    "Leptos + Tailwind CSS"
                </p>
                <h1 class="text-5xl font-extrabold tracking-tight text-slate-950 sm:text-7xl">
                    "Hello, "
                    <span class="text-brand">"world!"</span>
                </h1>
                <p class="mx-auto mt-6 max-w-lg text-lg leading-8 text-slate-600">
                    "Your QR code generator has a new look and is ready for the next feature."
                </p>

                <div class="mt-10 flex flex-col items-center justify-center gap-3 sm:flex-row">
                    <button class="bg-brand hover:bg-brand-dark focus-visible:outline-brand w-full rounded-xl px-6 py-3.5 font-semibold text-white shadow-lg shadow-pink-200 transition hover:-translate-y-0.5 focus-visible:outline-2 focus-visible:outline-offset-2 sm:w-auto">
                        "Create a QR code"
                    </button>
                    <span class="text-sm text-slate-500">"Built with Rust and WebAssembly"</span>
                </div>
            </section>
        </main>
    }
}

fn main() {
    leptos::mount::mount_to_body(App);
}
