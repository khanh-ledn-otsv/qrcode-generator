import type { PreviewRequest, RawDiagnostics } from "./preview-protocol";
import { isPreviewResult } from "./preview-protocol";

import { qs } from "./dom";

type PreviewRequestWithoutRevision = Omit<PreviewRequest, "revision">;
type DownloadFormat = "png" | "svg";

export type PreviewController = {
  clear(message: string): void;
  destroy(): void;
  download(format: DownloadFormat): void;
  request(request: PreviewRequestWithoutRevision): void;
};

const DEBOUNCE_MILLISECONDS = 250;
const INTERNAL_FAILURE = "QR generation failed unexpectedly. Change the input and try again.";
const MODE_LABELS = ["Numeric", "Alphanumeric", "Byte", "Kanji", "Mixed"] as const;
const ECC_LABELS = ["L", "M", "Q", "H"] as const;

function setPreviewUnavailable(message: string): void {
  const preview = qs<HTMLDivElement>("qr-preview");
  preview.setAttribute("aria-label", "QR code preview unavailable");
  const placeholder = document.createElement("p");
  placeholder.id = "preview-placeholder";
  placeholder.className = "max-w-56 text-center text-sm text-text-muted";
  placeholder.textContent = message;
  preview.replaceChildren(placeholder);
}

function downloadBytes(bytes: Uint8Array, mimeType: string, filename: string): void {
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

export function createPreviewController(): PreviewController {
  let currentSvg: string | null = null;
  let currentPng: Uint8Array | null = null;
  let previewWorker: Worker | null = null;
  let debounceTimer: number | null = null;
  let currentRevision = 0;
  let latestLogoEnabled = true;

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
    qs<HTMLElement>("caution").textContent = latestLogoEnabled
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

  function clear(message: string): void {
    clearPendingPreview();
    setGenerationFailure(message);
  }

  function request(previewRequest: PreviewRequestWithoutRevision): void {
    if (debounceTimer !== null) window.clearTimeout(debounceTimer);
    const revision = currentRevision + 1;
    currentRevision = revision;
    latestLogoEnabled = previewRequest.logoEnabled;
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
      previewWorker.postMessage({ ...previewRequest, revision } satisfies PreviewRequest);
    }, DEBOUNCE_MILLISECONDS);
  }

  function download(format: DownloadFormat): void {
    if (format === "png" && currentPng !== null) {
      downloadBytes(currentPng, "image/png", "qr-code.png");
    }
    if (format === "svg" && currentSvg !== null) {
      downloadBytes(new TextEncoder().encode(currentSvg), "image/svg+xml", "qr-code.svg");
    }
  }

  function destroy(): void {
    if (debounceTimer !== null) window.clearTimeout(debounceTimer);
    debounceTimer = null;
    previewWorker?.terminate();
    previewWorker = null;
  }

  previewWorker = startPreviewWorker();
  setPreviewUnavailable("Enter a valid URL to see the QR preview.");

  return { clear, destroy, download, request };
}
