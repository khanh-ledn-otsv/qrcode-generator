import type { PreviewRequest, ProfileValue, RawDiagnostics } from "./preview-protocol";
import { isPreviewResult } from "./preview-protocol";

type Parameter = { id: number; name: string; value: string; syncedName: string | null };
type QueryParameter = { name: string; value: string };

type ComposeResult = {
  status: "ready" | "failed";
  value: string | null;
  message: string | null;
};

const DIGITAL_PROFILES = ["small", "standard", "primary-cta", "hero-campaign"] as const;
const PRINT_PROFILES = ["business-card", "flyer-brochure", "poster-package"] as const;

const DEBOUNCE_MILLISECONDS = 250;
const INTERNAL_FAILURE = "QR generation failed unexpectedly. Change the input and try again.";
const MODE_LABELS = ["Numeric", "Alphanumeric", "Byte", "Kanji", "Mixed"] as const;
const ECC_LABELS = ["L", "M", "Q", "H"] as const;
const UTM_FIELDS = [
  { name: "utm_source", inputId: "utm-source" },
  { name: "utm_medium", inputId: "utm-medium" },
  { name: "utm_campaign", inputId: "utm-campaign" },
] as const;
const UTM_NAMES = new Set<string>(UTM_FIELDS.map(({ name }) => name));

function parseWebUrl(value: string): URL | null {
  if (!/^(?:http|https):\/\/[^/?#]+(?:[/?#]|$)/.test(value)) return null;
  try {
    return new URL(value);
  } catch {
    return null;
  }
}

const state = {
  parameters: [] as Parameter[],
  nextId: 1,
  profile: "standard" as ProfileValue,
  foregroundTheme: "magenta",
  logoEnabled: true,
};

function composeUrl(baseUrl: string, parameters: QueryParameter[]): ComposeResult {
  if (parseWebUrl(baseUrl) === null) {
    return {
      status: "failed",
      value: null,
      message: "Enter a valid URL beginning with http:// or https://.",
    };
  }

  for (const parameter of parameters) {
    if (parameter.name.length === 0) {
      if (parameter.value.length === 0) continue;
      return {
        status: "failed",
        value: null,
        message: "Enter a name for each custom parameter that has a value.",
      };
    }
  }

  return { status: "ready", value: baseUrl, message: null };
}

function querySegmentName(segment: string): string | null {
  const first = new URLSearchParams(segment).keys().next();
  return first.done ? null : first.value;
}

function updateQueryParameter(baseUrl: string, name: string, value: string | null): string {
  const fragmentIndex = baseUrl.indexOf("#");
  const withoutFragment = fragmentIndex === -1 ? baseUrl : baseUrl.slice(0, fragmentIndex);
  const fragment = fragmentIndex === -1 ? "" : baseUrl.slice(fragmentIndex);
  const queryIndex = withoutFragment.indexOf("?");
  const prefix = queryIndex === -1 ? withoutFragment : withoutFragment.slice(0, queryIndex);
  const query = queryIndex === -1 ? "" : withoutFragment.slice(queryIndex + 1);
  const encoded = value === null ? null : new URLSearchParams([[name, value]]).toString();
  const segments: string[] = [];
  let replaced = false;

  for (const segment of query.length === 0 ? [] : query.split("&")) {
    if (querySegmentName(segment) !== name) {
      segments.push(segment);
    } else if (!replaced && encoded !== null) {
      segments.push(encoded);
      replaced = true;
    }
  }
  if (!replaced && encoded !== null) segments.push(encoded);

  return `${prefix}${segments.length === 0 ? "" : `?${segments.join("&")}`}${fragment}`;
}

function updateQueryParameterIfChanged(
  baseUrl: string,
  name: string,
  value: string | null,
): string {
  const parsed = parseWebUrl(baseUrl);
  if (parsed === null) return baseUrl;
  if (value === null ? !parsed.searchParams.has(name) : parsed.searchParams.get(name) === value) {
    return baseUrl;
  }
  return updateQueryParameter(baseUrl, name, value);
}

function syncControlsFromBaseUrl(): void {
  const baseUrl = qs<HTMLInputElement>("base-url").value;
  if (baseUrl.length === 0) {
    for (const { inputId } of UTM_FIELDS) qs<HTMLInputElement>(inputId).value = "";
    state.parameters = [];
    updateCustomParameterRows();
    return;
  }

  const parsed = parseWebUrl(baseUrl);
  if (parsed === null) return;

  let hasUtmParameter = false;
  for (const { name, inputId } of UTM_FIELDS) {
    const input = qs<HTMLInputElement>(inputId);
    hasUtmParameter ||= parsed.searchParams.has(name);
    input.value = parsed.searchParams.get(name) ?? "";
  }
  if (hasUtmParameter) qs<HTMLInputElement>("utm-enabled").checked = true;

  const seen = new Set<string>();
  state.parameters = [];
  for (const [name, value] of parsed.searchParams) {
    if (UTM_NAMES.has(name) || seen.has(name)) continue;
    seen.add(name);
    state.parameters.push({ id: state.nextId++, name, value, syncedName: name });
  }
  updateCustomParameterRows();
}

function syncUtmParameterFromControl(name: string, inputId: string): void {
  const baseUrlInput = qs<HTMLInputElement>("base-url");
  if (parseWebUrl(baseUrlInput.value) === null) return;
  const utmEnabled = qs<HTMLInputElement>("utm-enabled").checked;
  const parameterValue = qs<HTMLInputElement>(inputId).value;
  baseUrlInput.value = updateQueryParameterIfChanged(
    baseUrlInput.value,
    name,
    utmEnabled && parameterValue ? parameterValue : null,
  );
}

function syncAllUtmParametersFromControls(): void {
  for (const { name, inputId } of UTM_FIELDS) {
    syncUtmParameterFromControl(name, inputId);
  }
}

function syncCustomParameterFromControl(parameter: Parameter): void {
  const baseUrlInput = qs<HTMLInputElement>("base-url");
  if (parseWebUrl(baseUrlInput.value) === null) return;
  if (parameter.name.length === 0 && parameter.value.length > 0) return;

  let value = baseUrlInput.value;
  if (parameter.syncedName !== null && parameter.syncedName !== parameter.name) {
    value = updateQueryParameter(value, parameter.syncedName, null);
  }
  if (parameter.name.length > 0) {
    value = updateQueryParameterIfChanged(
      value,
      parameter.name,
      parameter.value.length > 0 ? parameter.value : null,
    );
  }
  parameter.syncedName =
    parameter.name.length > 0 && parameter.value.length > 0 ? parameter.name : null;

  baseUrlInput.value = value;
}

let currentSvg: string | null = null;
let currentPng: Uint8Array | null = null;
let previewWorker: Worker | null = null;
let debounceTimer: number | null = null;
let currentRevision = 0;

function qs<T extends HTMLElement>(id: string): T {
  const element = document.getElementById(id);
  if (!(element instanceof HTMLElement)) {
    throw new Error(`Missing element #${id}`);
  }
  return element as T;
}

function radioValue(name: string): string {
  const checked = document.querySelector<HTMLInputElement>(`input[name="${name}"]:checked`);
  return checked?.value ?? "";
}

function buildParameters(): Array<{ name: string; value: string }> {
  const parameters: Array<{ name: string; value: string }> = [];
  if (qs<HTMLInputElement>("utm-enabled").checked) {
    parameters.push({ name: "utm_source", value: qs<HTMLInputElement>("utm-source").value });
    parameters.push({ name: "utm_medium", value: qs<HTMLInputElement>("utm-medium").value });
    parameters.push({ name: "utm_campaign", value: qs<HTMLInputElement>("utm-campaign").value });
  }
  for (const parameter of state.parameters) {
    parameters.push({ name: parameter.name, value: parameter.value });
  }
  return parameters;
}

function currentProfile(): ProfileValue {
  return qs<HTMLSelectElement>("profile-select").value as ProfileValue;
}

function setProfileOptions(): void {
  const type = radioValue("qr-type");
  const select = qs<HTMLSelectElement>("profile-select");
  const allowed: ReadonlySet<ProfileValue> =
    type === "print"
      ? new Set<ProfileValue>(PRINT_PROFILES)
      : new Set<ProfileValue>(DIGITAL_PROFILES);
  for (const option of Array.from(select.options)) {
    option.hidden = !allowed.has(option.value as ProfileValue);
  }
  if (!allowed.has(select.value as ProfileValue)) {
    select.value = type === "print" ? "business-card" : "standard";
  }
  state.profile = select.value as ProfileValue;
}

function updateCustomParameterRows(): void {
  const container = qs<HTMLDivElement>("custom-params");
  container.innerHTML = "";
  state.parameters.forEach((parameter, index) => {
    const row = document.createElement("div");
    row.className = "grid grid-cols-[minmax(0,1fr)_minmax(0,1fr)_2.5rem] items-center gap-3";

    const name = document.createElement("input");
    name.className =
      "w-full min-w-0 rounded-md border border-border bg-page px-3 py-2 text-sm text-text outline-none focus:border-focus focus:ring-2 focus:ring-focus/20";
    name.placeholder = "Parameter name";
    name.setAttribute("aria-label", `Custom parameter ${index + 1} name`);
    name.value = parameter.name;
    name.addEventListener("input", () => {
      parameter.name = name.value;
      syncCustomParameterFromControl(parameter);
      recompute();
    });

    const value = document.createElement("input");
    value.className =
      "w-full min-w-0 rounded-md border border-border bg-page px-3 py-2 text-sm text-text outline-none focus:border-focus focus:ring-2 focus:ring-focus/20";
    value.placeholder = "Value";
    value.setAttribute("aria-label", `Custom parameter ${index + 1} value`);
    value.value = parameter.value;
    value.addEventListener("input", () => {
      parameter.value = value.value;
      syncCustomParameterFromControl(parameter);
      recompute();
    });

    const remove = document.createElement("button");
    remove.type = "button";
    remove.className =
      "inline-flex h-10 w-10 items-center justify-center rounded-md border border-border bg-page text-text-muted transition hover:border-brand hover:text-brand focus:outline-none focus:ring-2 focus:ring-focus/20";
    remove.setAttribute("aria-label", `Remove custom parameter ${index + 1}`);
    remove.title = "Remove parameter";

    const icon = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    icon.setAttribute("class", "h-5 w-5");
    icon.setAttribute("viewBox", "0 0 24 24");
    icon.setAttribute("fill", "none");
    icon.setAttribute("stroke", "currentColor");
    icon.setAttribute("stroke-width", "1.75");
    icon.setAttribute("stroke-linecap", "round");
    icon.setAttribute("stroke-linejoin", "round");
    icon.setAttribute("aria-hidden", "true");
    const iconPath = document.createElementNS("http://www.w3.org/2000/svg", "path");
    iconPath.setAttribute("d", "M4 7h16M9 7V4h6v3M7 7l1 13h8l1-13M10 11v5M14 11v5");
    icon.append(iconPath);
    remove.append(icon);
    remove.addEventListener("click", () => {
      const baseUrlInput = qs<HTMLInputElement>("base-url");
      if (parameter.syncedName !== null && parseWebUrl(baseUrlInput.value) !== null) {
        baseUrlInput.value = updateQueryParameter(baseUrlInput.value, parameter.syncedName, null);
      }
      state.parameters = state.parameters.filter((item) => item.id !== parameter.id);
      updateCustomParameterRows();
      recompute();
    });

    row.append(name, value, remove);
    container.append(row);
  });
}

function updateDiagnostics(diagnostics: RawDiagnostics): void {
  const mode = MODE_LABELS[diagnostics.mode] ?? "Unknown";
  const ecc = ECC_LABELS[diagnostics.ecc] ?? "Unknown";
  const version = diagnostics.brandingIncreasedVersion
    ? `V${diagnostics.selectedVersion} / V${diagnostics.maximumVersion} max · raised to V${diagnostics.minimumVersion} for branding`
    : `V${diagnostics.selectedVersion} / V${diagnostics.maximumVersion} max`;
  const logo = diagnostics.renderedLogo
    ? `ONE lettermark · ${diagnostics.obscuredDataModules} data · ${diagnostics.obscuredRemainderModules} remainder modules obscured`
    : "None";
  const logoRequest = diagnostics.requestedLogo
    ? diagnostics.logoFallbackReason
      ? `ONE requested; disabled: ${diagnostics.logoFallbackReason}`
      : "ONE requested"
    : "No logo requested";
  const contrast = `${Math.floor(diagnostics.contrastHundredths / 100)}.${String(
    diagnostics.contrastHundredths % 100,
  ).padStart(2, "0")}:1`;

  qs<HTMLElement>("diag-mode").textContent = mode;
  qs<HTMLElement>("diag-ecc").textContent = ecc;
  qs<HTMLElement>("diag-version").textContent = version;
  qs<HTMLElement>("diag-mask").textContent = String(diagnostics.mask);
  qs<HTMLElement>("diag-matrix").textContent =
    `${diagnostics.matrixModules} x ${diagnostics.matrixModules} modules`;
  qs<HTMLElement>("diag-output").textContent =
    `${diagnostics.svgSidePixels} px SVG / ${diagnostics.pngSidePixels} px PNG`;
  qs<HTMLElement>("diag-logo").textContent = logo;
  qs<HTMLElement>("diag-logo-request").textContent = logoRequest;
  qs<HTMLElement>("diag-contrast").textContent = contrast;
  qs<HTMLElement>("diag-safety").textContent = diagnostics.safety === 0 ? "Safe" : "Caution";
}

function setPreviewUnavailable(message: string): void {
  const preview = qs<HTMLDivElement>("qr-preview");
  preview.setAttribute("aria-label", "QR code preview unavailable");
  const placeholder = document.createElement("p");
  placeholder.id = "preview-placeholder";
  placeholder.className = "small";
  placeholder.textContent = message;
  preview.replaceChildren(placeholder);
}

function download(bytes: Uint8Array, mimeType: string, filename: string): void {
  const blob = new Blob([bytes.slice().buffer], { type: mimeType });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = filename;
  link.hidden = true;
  document.body.append(link);
  link.click();
  link.remove();
  URL.revokeObjectURL(url);
}

function clearPendingPreview(): void {
  if (debounceTimer !== null) {
    window.clearTimeout(debounceTimer);
    debounceTimer = null;
  }
  currentRevision += 1;
}

function setGenerationFailure(message: string): void {
  qs<HTMLElement>("export-status").textContent = message;
  qs<HTMLElement>("caution").textContent = "";
  qs<HTMLButtonElement>("download-png").disabled = true;
  qs<HTMLButtonElement>("download-svg").disabled = true;
  setPreviewUnavailable("Enter a valid URL to see the QR preview.");
  currentSvg = null;
  currentPng = null;
}

function handlePreviewResult(result: unknown): void {
  if (!isPreviewResult(result)) {
    previewWorker?.terminate();
    previewWorker = null;
    clearPendingPreview();
    setGenerationFailure(INTERNAL_FAILURE);
    return;
  }
  if (result.revision !== currentRevision) return;

  if (result.status === "failed") {
    setGenerationFailure(result.message);
    return;
  }

  const diagnostics = result.preview.diagnostics;
  const mode = MODE_LABELS[diagnostics.mode] ?? "Unknown";
  const ecc = ECC_LABELS[diagnostics.ecc] ?? "Unknown";
  const previewNode = qs<HTMLDivElement>("qr-preview");
  previewNode.setAttribute(
    "aria-label",
    `Generated QR code preview: ${mode} mode, version ${diagnostics.selectedVersion}, ECC ${ecc}.`,
  );
  previewNode.innerHTML = result.preview.svg;
  updateDiagnostics(diagnostics);
  const encoded = qs<HTMLTextAreaElement>("encoded-url").value;
  qs<HTMLElement>("encoded-url-guidance").textContent =
    `${encoded.length} characters | ${new TextEncoder().encode(encoded).length} UTF-8 bytes | typical ASCII maximum: ${result.preview.capacityLimit}`;

  currentSvg = result.preview.svg;
  currentPng = new Uint8Array(result.preview.png);
  qs<HTMLButtonElement>("download-png").disabled = false;
  qs<HTMLButtonElement>("download-svg").disabled = false;
  qs<HTMLElement>("caution").textContent = state.logoEnabled
    ? "The bundled logo obscures QR data modules. Validate the exported code in its actual environment."
    : "";
  qs<HTMLElement>("export-status").textContent = "SVG and PNG downloads are ready.";
}

function startPreviewWorker(): Worker {
  const worker = new Worker(new URL("./preview-worker.ts", import.meta.url), { type: "module" });
  worker.addEventListener("message", (event: MessageEvent<unknown>) =>
    handlePreviewResult(event.data),
  );
  worker.addEventListener("error", () => {
    worker.terminate();
    if (previewWorker === worker) previewWorker = null;
    clearPendingPreview();
    setGenerationFailure(INTERNAL_FAILURE);
  });
  return worker;
}

function schedulePreview(request: Omit<PreviewRequest, "revision">): void {
  if (debounceTimer !== null) window.clearTimeout(debounceTimer);
  const revision = currentRevision + 1;
  currentRevision = revision;
  qs<HTMLButtonElement>("download-png").disabled = true;
  qs<HTMLButtonElement>("download-svg").disabled = true;
  qs<HTMLElement>("export-status").textContent = "QR preview is updating.";
  setPreviewUnavailable("Updating preview...");

  debounceTimer = window.setTimeout(() => {
    debounceTimer = null;
    previewWorker ??= startPreviewWorker();
    // Worker.postMessage has no target-origin parameter; Oxlint otherwise
    // resolves the Window overload because both globals exist in DOM types.
    // oxlint-disable-next-line unicorn/require-post-message-target-origin
    previewWorker.postMessage({ ...request, revision } satisfies PreviewRequest);
  }, DEBOUNCE_MILLISECONDS);
}

function recompute(): void {
  const baseUrlInput = qs<HTMLInputElement>("base-url");
  const alert = qs<HTMLElement>("url-validation");
  const encoded = qs<HTMLTextAreaElement>("encoded-url");
  const guidance = qs<HTMLElement>("encoded-url-guidance");
  const caution = qs<HTMLElement>("caution");
  const exportStatus = qs<HTMLElement>("export-status");
  const downloadPng = qs<HTMLButtonElement>("download-png");
  const downloadSvg = qs<HTMLButtonElement>("download-svg");

  const compose = composeUrl(baseUrlInput.value, buildParameters());

  qs<HTMLElement>("base-url-counts").textContent =
    `${baseUrlInput.value.length} characters | ${new TextEncoder().encode(baseUrlInput.value).length} UTF-8 bytes`;

  if (compose.status !== "ready" || !compose.value) {
    clearPendingPreview();
    alert.textContent = compose.message ?? "Enter a valid URL beginning with http:// or https://.";
    encoded.value = "";
    guidance.textContent = "The encoded URL will appear after the base URL is valid.";
    caution.textContent = "";
    exportStatus.textContent = alert.textContent;
    downloadPng.disabled = true;
    downloadSvg.disabled = true;
    setPreviewUnavailable("Enter a valid URL to see the QR preview.");
    currentSvg = null;
    currentPng = null;
    return;
  }

  alert.textContent = "";
  encoded.value = compose.value;

  state.profile = currentProfile();
  state.logoEnabled = qs<HTMLInputElement>("bundled-logo").checked;
  state.foregroundTheme = radioValue("foreground-theme") || "magenta";
  guidance.textContent = `${compose.value.length} characters | ${new TextEncoder().encode(compose.value).length} UTF-8 bytes`;
  caution.textContent = "";
  exportStatus.textContent = "QR preview is updating.";
  downloadPng.disabled = true;
  downloadSvg.disabled = true;
  schedulePreview({
    payload: compose.value,
    profile: state.profile,
    foregroundTheme: state.foregroundTheme,
    logoEnabled: state.logoEnabled,
  });
}

function attachEvents(): void {
  const generatorTab = qs<HTMLButtonElement>("tab-generator");
  const usageTab = qs<HTMLButtonElement>("tab-usage");
  const generatorPanel = qs<HTMLDivElement>("generator-panel");
  const usagePanel = qs<HTMLElement>("usage-panel");
  const specification = qs<HTMLDetailsElement>("qr-specification");

  generatorTab.addEventListener("click", () => {
    generatorTab.classList.add("border-b-2", "border-brand", "text-brand");
    generatorTab.classList.remove("text-text-muted");
    usageTab.classList.remove("border-b-2", "border-brand", "text-brand");
    usageTab.classList.add("text-text-muted");
    generatorTab.setAttribute("aria-current", "page");
    usageTab.removeAttribute("aria-current");
    generatorPanel.classList.remove("hidden");
    usagePanel.classList.add("hidden");
    specification.classList.remove("hidden");
  });

  usageTab.addEventListener("click", () => {
    usageTab.classList.add("border-b-2", "border-brand", "text-brand");
    usageTab.classList.remove("text-text-muted");
    generatorTab.classList.remove("border-b-2", "border-brand", "text-brand");
    generatorTab.classList.add("text-text-muted");
    usageTab.setAttribute("aria-current", "page");
    generatorTab.removeAttribute("aria-current");
    usagePanel.classList.remove("hidden");
    generatorPanel.classList.add("hidden");
    specification.classList.add("hidden");
  });

  qs<HTMLButtonElement>("add-param").addEventListener("click", () => {
    state.parameters.push({ id: state.nextId++, name: "", value: "", syncedName: null });
    updateCustomParameterRows();
  });

  qs<HTMLInputElement>("base-url").addEventListener("input", () => {
    syncControlsFromBaseUrl();
    setProfileOptions();
    recompute();
  });
  UTM_FIELDS.forEach(({ name, inputId }) => {
    qs<HTMLElement>(inputId).addEventListener("input", () => {
      syncUtmParameterFromControl(name, inputId);
      setProfileOptions();
      recompute();
    });
  });
  qs<HTMLInputElement>("utm-enabled").addEventListener("change", () => {
    syncAllUtmParametersFromControls();
    setProfileOptions();
    recompute();
  });
  ["profile-select", "bundled-logo"].forEach((id) => {
    qs<HTMLElement>(id).addEventListener("change", () => {
      setProfileOptions();
      recompute();
    });
  });

  for (const input of Array.from(
    document.querySelectorAll<HTMLInputElement>(
      "input[name=qr-type], input[name=foreground-theme]",
    ),
  )) {
    input.addEventListener("change", () => {
      setProfileOptions();
      recompute();
    });
  }

  qs<HTMLButtonElement>("download-png").addEventListener("click", () => {
    if (currentPng !== null) {
      download(currentPng, "image/png", "qr-code.png");
    }
  });
  qs<HTMLButtonElement>("download-svg").addEventListener("click", () => {
    if (currentSvg !== null) {
      download(new TextEncoder().encode(currentSvg), "image/svg+xml", "qr-code.svg");
    }
  });
}

function bootstrap(): void {
  previewWorker = startPreviewWorker();
  setProfileOptions();
  attachEvents();
  updateCustomParameterRows();
  setPreviewUnavailable("Enter a valid URL to see the QR preview.");
  recompute();
}

window.addEventListener("pagehide", () => {
  if (debounceTimer !== null) window.clearTimeout(debounceTimer);
  previewWorker?.terminate();
  previewWorker = null;
});

bootstrap();
