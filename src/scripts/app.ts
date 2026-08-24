import { qs, radioValue } from "./dom";
import { createPreviewController } from "./preview-controller";
import type { ProfileValue } from "./preview-protocol";
import {
  composeUrl,
  parseWebUrl,
  updateQueryParameter,
  updateQueryParameterIfChanged,
} from "./url-composer";

type Parameter = { id: number; name: string; value: string; syncedName: string | null };

const DIGITAL_PROFILES = ["small", "standard", "primary-cta", "hero-campaign"] as const;
const PRINT_PROFILES = ["business-card", "flyer-brochure", "poster-package"] as const;
const PROFILE_LABELS: Record<ProfileValue, string> = {
  small: "Small",
  standard: "Standard",
  "primary-cta": "Primary CTA",
  "hero-campaign": "Hero / Campaign",
  "business-card": "Business card",
  "flyer-brochure": "Flyer / Brochure",
  "poster-package": "Poster / Package",
};

const UTM_FIELDS = [
  { name: "utm_source", inputId: "utm-source" },
  { name: "utm_medium", inputId: "utm-medium" },
  { name: "utm_campaign", inputId: "utm-campaign" },
] as const;
const UTM_NAMES = new Set<string>(UTM_FIELDS.map(({ name }) => name));

const state = {
  parameters: [] as Parameter[],
  nextId: 1,
  profile: "standard" as ProfileValue,
  foregroundTheme: "magenta",
};

const previewController = createPreviewController();

function syncUtmPanelVisibility(): void {
  const toggle = qs<HTMLInputElement>("utm-enabled");
  qs<HTMLDivElement>("utm-content").hidden = !toggle.checked;
  toggle.setAttribute("aria-expanded", String(toggle.checked));
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
  syncUtmPanelVisibility();

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
  const profiles: readonly ProfileValue[] = type === "print" ? PRINT_PROFILES : DIGITAL_PROFILES;
  const allowed = new Set(profiles);
  const selectedProfile = allowed.has(select.value as ProfileValue)
    ? (select.value as ProfileValue)
    : type === "print"
      ? "business-card"
      : "standard";
  const optionsMatch =
    select.options.length === profiles.length &&
    profiles.every((profile, index) => select.options[index]?.value === profile);

  if (!optionsMatch) {
    select.replaceChildren(
      ...profiles.map((profile) => {
        const option = document.createElement("option");
        option.value = profile;
        option.textContent = PROFILE_LABELS[profile];
        return option;
      }),
    );
  }
  select.value = selectedProfile;
  state.profile = selectedProfile;
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

    const iconTemplate = qs<HTMLTemplateElement>("remove-parameter-icon-template");
    remove.append(iconTemplate.content.cloneNode(true));
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

function recompute(): void {
  const baseUrlInput = qs<HTMLInputElement>("base-url");
  const alert = qs<HTMLElement>("url-validation");
  const encoded = qs<HTMLTextAreaElement>("encoded-url");
  const guidance = qs<HTMLElement>("encoded-url-guidance");

  const compose = composeUrl(baseUrlInput.value, buildParameters());

  qs<HTMLElement>("base-url-counts").textContent =
    `${baseUrlInput.value.length} characters | ${new TextEncoder().encode(baseUrlInput.value).length} UTF-8 bytes`;

  if (compose.status !== "ready" || !compose.value) {
    alert.textContent = compose.message ?? "Enter a valid URL beginning with http:// or https://.";
    encoded.value = "";
    guidance.textContent = "The encoded URL will appear after the base URL is valid.";
    previewController.clear(alert.textContent);
    return;
  }

  alert.textContent = "";
  encoded.value = compose.value;

  state.profile = currentProfile();
  state.foregroundTheme = radioValue("foreground-theme") || "magenta";
  guidance.textContent = `${compose.value.length} characters | ${new TextEncoder().encode(compose.value).length} UTF-8 bytes`;
  previewController.request({
    payload: compose.value,
    profile: state.profile,
    foregroundTheme: state.foregroundTheme,
    logoEnabled: true,
  });
}

function attachEvents(): void {
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
    syncUtmPanelVisibility();
    syncAllUtmParametersFromControls();
    setProfileOptions();
    recompute();
  });
  qs<HTMLSelectElement>("profile-select").addEventListener("change", () => {
    setProfileOptions();
    recompute();
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
    previewController.download("png");
  });
  qs<HTMLButtonElement>("download-svg").addEventListener("click", () => {
    previewController.download("svg");
  });
}

function bootstrap(): void {
  setProfileOptions();
  syncUtmPanelVisibility();
  attachEvents();
  updateCustomParameterRows();
  recompute();
}

window.addEventListener("pagehide", () => previewController.destroy());

bootstrap();
