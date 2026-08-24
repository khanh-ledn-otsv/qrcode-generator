import { useEffect, useMemo, useRef, useState } from "react";
import type { ComponentProps } from "react";

import type { ProfileValue } from "../scripts/preview-protocol";
import {
  composeUrl,
  parseWebUrl,
  updateQueryParameter,
  updateQueryParameterIfChanged,
} from "../scripts/url-composer";
import QrSpecification from "./QrSpecification";
import { useQrPreview } from "./useQrPreview";

type Parameter = { id: number; name: string; value: string; syncedName: string | null };
type QrType = "digital" | "print";
type UtmName = "utm_source" | "utm_medium" | "utm_campaign";
type UtmValues = Record<UtmName, string>;

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
  { name: "utm_source", id: "utm-source", placeholder: "Where the scan comes from" },
  { name: "utm_medium", id: "utm-medium", placeholder: "The channel" },
  { name: "utm_campaign", id: "utm-campaign", placeholder: "Campaign name" },
] as const;
const UTM_NAMES = new Set<string>(UTM_FIELDS.map(({ name }) => name));
const EMPTY_UTM_VALUES: UtmValues = { utm_source: "", utm_medium: "", utm_campaign: "" };

function byteLength(value: string): number {
  return new TextEncoder().encode(value).length;
}

type PrivateInputProps = Omit<
  ComponentProps<"input">,
  "defaultValue" | "onChange" | "onInput" | "value"
> & {
  onValueChange(value: string): void;
  value: string;
};

function PrivateInput({ onValueChange, value, ...props }: PrivateInputProps) {
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    const input = inputRef.current;
    if (input !== null && input.value !== value) input.value = value;
  }, [value]);

  return (
    <input
      {...props}
      ref={inputRef}
      onInput={(event) => onValueChange(event.currentTarget.value)}
    />
  );
}

function TrashIcon() {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="24"
      height="24"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.75"
      strokeLinecap="round"
      strokeLinejoin="round"
      className="lucide lucide-trash-2 h-5 w-5"
      aria-hidden="true"
    >
      <path d="M3 6h18" />
      <path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2" />
      <path d="M19 6l-1 14c0 1-1 2-2 2H8c-1 0-2-1-2-2L5 6" />
      <path d="M10 11v6" />
      <path d="M14 11v6" />
    </svg>
  );
}

export default function QrGenerator() {
  const [baseUrl, setBaseUrl] = useState("");
  const [utmEnabled, setUtmEnabled] = useState(true);
  const [utmValues, setUtmValues] = useState<UtmValues>(EMPTY_UTM_VALUES);
  const [parameters, setParameters] = useState<Parameter[]>([]);
  const [qrType, setQrType] = useState<QrType>("digital");
  const [profile, setProfile] = useState<ProfileValue>("standard");
  const [foregroundTheme, setForegroundTheme] = useState("magenta");
  const nextParameterId = useRef(1);

  const composed = useMemo(() => {
    const queryParameters = utmEnabled
      ? UTM_FIELDS.map(({ name }) => ({ name, value: utmValues[name] }))
      : [];
    return composeUrl(baseUrl, [
      ...queryParameters,
      ...parameters.map(({ name, value }) => ({ name, value })),
    ]);
  }, [baseUrl, parameters, utmEnabled, utmValues]);
  const invalidMessage =
    composed.message ?? "Enter a valid URL beginning with http:// or https://.";
  const previewRequest = useMemo(
    () =>
      composed.status === "ready" && composed.value !== null
        ? {
            foregroundTheme,
            logoEnabled: true,
            payload: composed.value,
            profile,
          }
        : null,
    [composed.status, composed.value, foregroundTheme, profile],
  );
  const { download, preview } = useQrPreview(previewRequest, invalidMessage);
  const encodedUrl = composed.status === "ready" ? (composed.value ?? "") : "";
  const encodedGuidance = preview.ready
    ? `${encodedUrl.length} characters | ${byteLength(encodedUrl)} UTF-8 bytes | typical ASCII maximum: ${preview.capacityLimit}`
    : encodedUrl.length > 0
      ? `${encodedUrl.length} characters | ${byteLength(encodedUrl)} UTF-8 bytes`
      : "The encoded URL will appear after the base URL is valid.";
  const profiles: readonly ProfileValue[] = qrType === "print" ? PRINT_PROFILES : DIGITAL_PROFILES;

  function handleBaseUrlChange(value: string): void {
    setBaseUrl(value);
    if (value.length === 0) {
      setUtmValues(EMPTY_UTM_VALUES);
      setParameters([]);
      return;
    }

    const parsed = parseWebUrl(value);
    if (parsed === null) return;
    const nextUtmValues = { ...EMPTY_UTM_VALUES };
    let hasUtmParameter = false;
    for (const { name } of UTM_FIELDS) {
      if (parsed.searchParams.has(name)) hasUtmParameter = true;
      nextUtmValues[name] = parsed.searchParams.get(name) ?? "";
    }
    setUtmValues(nextUtmValues);
    if (hasUtmParameter) setUtmEnabled(true);

    const seen = new Set<string>();
    const nextParameters: Parameter[] = [];
    for (const [name, parameterValue] of parsed.searchParams) {
      if (UTM_NAMES.has(name) || seen.has(name)) continue;
      seen.add(name);
      nextParameters.push({
        id: nextParameterId.current++,
        name,
        syncedName: name,
        value: parameterValue,
      });
    }
    setParameters(nextParameters);
  }

  function handleUtmValueChange(name: UtmName, value: string): void {
    setUtmValues((current) => ({ ...current, [name]: value }));
    if (parseWebUrl(baseUrl) !== null) {
      setBaseUrl(updateQueryParameterIfChanged(baseUrl, name, utmEnabled && value ? value : null));
    }
  }

  function handleUtmToggle(enabled: boolean): void {
    setUtmEnabled(enabled);
    if (parseWebUrl(baseUrl) === null) return;
    let nextBaseUrl = baseUrl;
    for (const { name } of UTM_FIELDS) {
      nextBaseUrl = updateQueryParameterIfChanged(
        nextBaseUrl,
        name,
        enabled && utmValues[name] ? utmValues[name] : null,
      );
    }
    setBaseUrl(nextBaseUrl);
  }

  function updateCustomParameter(id: number, field: "name" | "value", fieldValue: string): void {
    const parameter = parameters.find((item) => item.id === id);
    if (parameter === undefined) return;
    const updated = { ...parameter, [field]: fieldValue };
    let nextBaseUrl = baseUrl;

    if (parseWebUrl(baseUrl) !== null && !(updated.name.length === 0 && updated.value.length > 0)) {
      if (updated.syncedName !== null && updated.syncedName !== updated.name) {
        nextBaseUrl = updateQueryParameter(nextBaseUrl, updated.syncedName, null);
      }
      if (updated.name.length > 0) {
        nextBaseUrl = updateQueryParameterIfChanged(
          nextBaseUrl,
          updated.name,
          updated.value.length > 0 ? updated.value : null,
        );
      }
      updated.syncedName =
        updated.name.length > 0 && updated.value.length > 0 ? updated.name : null;
    }

    setBaseUrl(nextBaseUrl);
    setParameters((current) => current.map((item) => (item.id === id ? updated : item)));
  }

  function removeCustomParameter(parameter: Parameter): void {
    if (parameter.syncedName !== null && parseWebUrl(baseUrl) !== null) {
      setBaseUrl(updateQueryParameter(baseUrl, parameter.syncedName, null));
    }
    setParameters((current) => current.filter((item) => item.id !== parameter.id));
  }

  function handleQrTypeChange(nextType: QrType): void {
    const nextProfiles: readonly ProfileValue[] =
      nextType === "print" ? PRINT_PROFILES : DIGITAL_PROFILES;
    setQrType(nextType);
    if (!nextProfiles.includes(profile)) {
      setProfile(nextType === "print" ? "business-card" : "standard");
    }
  }

  return (
    <>
      <div className="grid items-start gap-8 lg:grid-cols-[minmax(0,3fr)_minmax(20rem,2fr)]">
        <section aria-labelledby="settings-heading">
          <h2 id="settings-heading" className="text-lg font-semibold text-text">
            Settings
          </h2>
          <label htmlFor="base-url" className="mt-4 block text-sm font-semibold text-text">
            Base URL <span className="text-brand">*</span>
          </label>
          <PrivateInput
            id="base-url"
            type="url"
            className="mt-1 w-full rounded-md border border-border bg-page px-3 py-2 text-sm text-text outline-none focus:border-focus focus:ring-2 focus:ring-focus/20"
            placeholder="E.g. https://example.com/promo"
            autoComplete="url"
            aria-describedby="base-url-counts url-validation"
            value={baseUrl}
            onValueChange={handleBaseUrlChange}
          />
          <p id="base-url-counts" className="mt-1 text-xs text-text-muted">
            {baseUrl.length} characters | {byteLength(baseUrl)} UTF-8 bytes
          </p>
          <p
            id="url-validation"
            className="mt-2 min-h-5 text-sm font-semibold text-red-700"
            role="alert"
            aria-live="polite"
          >
            {composed.status === "failed" ? invalidMessage : ""}
          </p>
          <section className="mt-4 rounded-lg bg-surface p-4" aria-labelledby="utm-heading">
            <div className="flex items-center justify-between gap-4">
              <h3 id="utm-heading" className="text-sm font-semibold text-text">
                UTM Configuration
              </h3>
              <label className="relative inline-flex cursor-pointer items-center">
                <input
                  id="utm-enabled"
                  className="peer sr-only"
                  type="checkbox"
                  checked={utmEnabled}
                  aria-label="Enable UTM configuration"
                  aria-controls="utm-content"
                  aria-expanded={utmEnabled}
                  onChange={(event) => handleUtmToggle(event.currentTarget.checked)}
                />
                <span className="pointer-events-none h-5 w-9 rounded-full bg-text-muted transition peer-checked:bg-brand after:absolute after:top-0.5 after:left-0.5 after:h-4 after:w-4 after:rounded-full after:bg-page after:transition peer-checked:after:translate-x-4" />
                <span className="sr-only">Enable UTM configuration</span>
              </label>
            </div>
            <div id="utm-content" hidden={!utmEnabled}>
              <div id="utm-fields" className="mt-4 grid gap-4 sm:grid-cols-2">
                {UTM_FIELDS.map(({ id, name, placeholder }) => (
                  <label
                    key={name}
                    className={`block text-xs font-semibold text-text ${name === "utm_campaign" ? "sm:col-span-2" : ""}`}
                  >
                    {name}
                    <PrivateInput
                      id={id}
                      className="mt-1 w-full rounded-md border border-border bg-page px-3 py-2 text-sm outline-none focus:border-focus focus:ring-2 focus:ring-focus/20"
                      aria-label={name}
                      placeholder={placeholder}
                      value={utmValues[name]}
                      onValueChange={(value) => handleUtmValueChange(name, value)}
                    />
                  </label>
                ))}
              </div>
              <div id="custom-params" className="mt-4 grid gap-3">
                {parameters.map((parameter, index) => (
                  <div
                    key={parameter.id}
                    className="grid grid-cols-[minmax(0,1fr)_minmax(0,1fr)_2.5rem] items-center gap-3"
                  >
                    <PrivateInput
                      className="w-full min-w-0 rounded-md border border-border bg-page px-3 py-2 text-sm text-text outline-none focus:border-focus focus:ring-2 focus:ring-focus/20"
                      placeholder="Parameter name"
                      aria-label={`Custom parameter ${index + 1} name`}
                      value={parameter.name}
                      onValueChange={(value) => updateCustomParameter(parameter.id, "name", value)}
                    />
                    <PrivateInput
                      className="w-full min-w-0 rounded-md border border-border bg-page px-3 py-2 text-sm text-text outline-none focus:border-focus focus:ring-2 focus:ring-focus/20"
                      placeholder="Value"
                      aria-label={`Custom parameter ${index + 1} value`}
                      value={parameter.value}
                      onValueChange={(value) => updateCustomParameter(parameter.id, "value", value)}
                    />
                    <button
                      type="button"
                      className="inline-flex h-10 w-10 items-center justify-center rounded-md border border-border bg-page text-text-muted transition hover:border-brand hover:text-brand focus:ring-2 focus:ring-focus/20 focus:outline-none"
                      aria-label={`Remove custom parameter ${index + 1}`}
                      title="Remove parameter"
                      onClick={() => removeCustomParameter(parameter)}
                    >
                      <TrashIcon />
                    </button>
                  </div>
                ))}
              </div>
              <button
                id="add-param"
                className="mt-4 text-sm font-semibold text-brand hover:text-brand-dark"
                type="button"
                onClick={() => {
                  setParameters((current) => [
                    ...current,
                    {
                      id: nextParameterId.current++,
                      name: "",
                      syncedName: null,
                      value: "",
                    },
                  ]);
                }}
              >
                + Add Parameter
              </button>
            </div>
          </section>
          <label htmlFor="encoded-url" className="mt-4 block text-sm font-semibold text-text">
            Encoded URL{" "}
            <span className="font-normal text-text-muted italic">- Automatic generation</span>
          </label>
          <textarea
            id="encoded-url"
            className="mt-1 min-h-20 w-full resize-none rounded-md border border-border bg-surface px-3 py-2 text-sm text-text"
            rows={3}
            readOnly
            aria-label="Encoded URL"
            value={encodedUrl}
          />
          <p id="encoded-url-guidance" className="mt-1 text-xs text-text-muted">
            {encodedGuidance}
          </p>
        </section>

        <section className="rounded-lg bg-surface p-4" aria-labelledby="preview-heading">
          <h2 id="preview-heading" className="text-lg font-semibold text-text">
            Preview
          </h2>
          <fieldset className="mt-4">
            <legend className="text-xs font-semibold text-text">
              QR Type <span className="text-brand">*</span>
            </legend>
            <div className="mt-1 flex gap-4 text-sm">
              {(["digital", "print"] as const).map((value) => (
                <label key={value} className="flex items-center gap-2">
                  <input
                    type="radio"
                    name="qr-type"
                    value={value}
                    checked={qrType === value}
                    onChange={() => handleQrTypeChange(value)}
                  />
                  {value === "digital" ? "Digital" : "Print"}
                </label>
              ))}
            </div>
          </fieldset>
          <label htmlFor="profile-select" className="mt-3 block text-xs font-semibold text-text">
            Output variant
          </label>
          <select
            id="profile-select"
            className="mt-1 w-full rounded-md border border-border bg-page px-3 py-2 text-sm outline-none focus:border-focus focus:ring-2 focus:ring-focus/20"
            aria-label="Output variant"
            value={profile}
            onChange={(event) => setProfile(event.currentTarget.value as ProfileValue)}
          >
            {profiles.map((value) => (
              <option key={value} value={value}>
                {PROFILE_LABELS[value]}
              </option>
            ))}
          </select>
          <fieldset className="mt-4">
            <legend className="text-xs font-semibold text-text">
              Color type <span className="text-brand">*</span>
            </legend>
            <div className="mt-1 flex gap-4 text-sm">
              {(["magenta", "black"] as const).map((value) => (
                <label key={value} className="flex items-center gap-2">
                  <input
                    type="radio"
                    name="foreground-theme"
                    value={value}
                    checked={foregroundTheme === value}
                    onChange={() => setForegroundTheme(value)}
                  />
                  {value === "magenta" ? "Magenta" : "Black"}
                </label>
              ))}
            </div>
          </fieldset>
          {preview.svg === null ? (
            <figure
              id="qr-preview"
              className="mt-4 grid min-h-72 place-items-center rounded-lg border border-border bg-page p-4"
              data-testid="qr-preview"
              aria-label={preview.ariaLabel}
            >
              <p id="preview-placeholder" className="max-w-56 text-center text-sm text-text-muted">
                {preview.placeholder}
              </p>
            </figure>
          ) : (
            <figure
              id="qr-preview"
              className="mt-4 grid min-h-72 place-items-center rounded-lg border border-border bg-page p-4"
              data-testid="qr-preview"
              aria-label={preview.ariaLabel}
              dangerouslySetInnerHTML={{ __html: preview.svg }}
            />
          )}
          <output id="caution" className="mt-3 block min-h-5 text-xs font-semibold text-amber-800">
            {preview.caution}
          </output>
          <div className="mt-3 grid gap-3 sm:grid-cols-2">
            <button
              id="download-png"
              className="rounded-md border border-brand bg-page px-4 py-2 text-sm font-semibold text-brand disabled:border-border disabled:text-text-muted"
              data-testid="download-png"
              type="button"
              disabled={!preview.ready}
              onClick={() => download("png")}
            >
              Download PNG
            </button>
            <button
              id="download-svg"
              className="rounded-md bg-brand px-4 py-2 text-sm font-semibold text-white disabled:bg-border disabled:text-text-muted"
              data-testid="download-svg"
              type="button"
              disabled={!preview.ready}
              onClick={() => download("svg")}
            >
              Download SVG
            </button>
          </div>
          <output id="export-status" className="mt-2 block text-xs text-text-muted">
            {preview.exportStatus}
          </output>
        </section>
      </div>
      <QrSpecification diagnostics={preview.diagnostics} />
    </>
  );
}
