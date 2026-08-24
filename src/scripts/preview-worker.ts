/// <reference lib="webworker" />

import initWasm, { capacity_limit, generate_preview } from "../generated/wasm/qr_web.js";

import type { PreviewRequest, PreviewResult, RawDiagnostics } from "./preview-protocol";

const INTERNAL_FAILURE = "QR generation failed unexpectedly. Change the input and try again.";
const wasmReady = initWasm();
const worker = self as DedicatedWorkerGlobalScope;

function failureMessage(error: unknown): string {
  if (typeof error === "string" && error.length > 0) return error;
  if (error instanceof Error && error.message.length > 0) return error.message;
  return INTERNAL_FAILURE;
}

worker.addEventListener("message", async (event: MessageEvent<PreviewRequest>) => {
  const request = event.data;
  try {
    await wasmReady;
    const generated = generate_preview(
      request.payload,
      request.profile,
      request.foregroundTheme,
      request.logoEnabled,
    );
    const png = generated.png();
    const diagnostics: RawDiagnostics = {
      mode: generated.mode(),
      ecc: generated.ecc(),
      mask: generated.mask(),
      minimumVersion: generated.minimum_version(),
      maximumVersion: generated.maximum_version(),
      selectedVersion: generated.selected_version(),
      brandingIncreasedVersion: generated.branding_increased_version(),
      matrixModules: generated.matrix_modules(),
      svgSidePixels: generated.svg_side_pixels(),
      pngSidePixels: generated.png_side_pixels(),
      safety: generated.safety(),
      contrastHundredths: generated.contrast_hundredths(),
      requestedLogo: generated.requested_logo(),
      renderedLogo: generated.rendered_logo(),
      logoFallbackReason: generated.logo_fallback_reason() ?? null,
      obscuredDataModules: generated.obscured_data_modules(),
      obscuredRemainderModules: generated.obscured_remainder_modules(),
    };
    const response: PreviewResult = {
      revision: request.revision,
      status: "ready",
      preview: {
        svg: generated.svg(),
        png: png.buffer,
        capacityLimit: capacity_limit(request.profile, request.logoEnabled),
        diagnostics,
      },
    };
    generated.free();
    // DedicatedWorkerGlobalScope.postMessage has no target-origin parameter.
    // oxlint-disable-next-line unicorn/require-post-message-target-origin
    worker.postMessage(response, [png.buffer]);
  } catch (error) {
    const response: PreviewResult = {
      revision: request.revision,
      status: "failed",
      message: failureMessage(error),
    };
    // oxlint-disable-next-line unicorn/require-post-message-target-origin
    worker.postMessage(response);
  }
});
