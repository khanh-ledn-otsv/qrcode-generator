import { useCallback, useEffect, useRef, useState } from "react";

import type { PreviewRequest, RawDiagnostics } from "../scripts/preview-protocol";
import { isPreviewResult } from "../scripts/preview-protocol";

type PreviewRequestWithoutRevision = Omit<PreviewRequest, "revision">;
type DownloadFormat = "png" | "svg";

export type PreviewView = {
  ariaLabel: string;
  capacityLimit: number | null;
  caution: string;
  diagnostics: RawDiagnostics | null;
  exportStatus: string;
  placeholder: string;
  ready: boolean;
  svg: string | null;
};

const DEBOUNCE_MILLISECONDS = 250;
const INTERNAL_FAILURE = "QR generation failed unexpectedly. Change the input and try again.";
const MODE_LABELS = ["Numeric", "Alphanumeric", "Byte", "Kanji", "Mixed"] as const;
const ECC_LABELS = ["L", "M", "Q", "H"] as const;
const EMPTY_PREVIEW: PreviewView = {
  ariaLabel: "QR code preview unavailable",
  capacityLimit: null,
  caution: "",
  diagnostics: null,
  exportStatus: "Enter a URL to generate a QR code.",
  placeholder: "Enter a valid URL to see the QR preview.",
  ready: false,
  svg: null,
};

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

export function useQrPreview(
  request: PreviewRequestWithoutRevision | null,
  invalidMessage: string,
): { download(format: DownloadFormat): void; preview: PreviewView } {
  const [preview, setPreview] = useState<PreviewView>(EMPTY_PREVIEW);
  const workerRef = useRef<Worker | null>(null);
  const timerRef = useRef<number | null>(null);
  const revisionRef = useRef(0);
  const svgRef = useRef<string | null>(null);
  const pngRef = useRef<Uint8Array | null>(null);

  const clearTimer = useCallback(() => {
    if (timerRef.current !== null) {
      window.clearTimeout(timerRef.current);
      timerRef.current = null;
    }
  }, []);

  const invalidatePending = useCallback(() => {
    clearTimer();
    revisionRef.current += 1;
  }, [clearTimer]);

  const setGenerationFailure = useCallback((message: string) => {
    svgRef.current = null;
    pngRef.current = null;
    setPreview({ ...EMPTY_PREVIEW, exportStatus: message });
  }, []);

  const handlePreviewResult = useCallback(
    (value: unknown) => {
      if (!isPreviewResult(value)) {
        workerRef.current?.terminate();
        workerRef.current = null;
        invalidatePending();
        setGenerationFailure(INTERNAL_FAILURE);
        return;
      }
      if (value.revision !== revisionRef.current) return;

      if (value.status === "failed") {
        setGenerationFailure(value.message);
        return;
      }

      const { diagnostics } = value.preview;
      const mode = MODE_LABELS[diagnostics.mode] ?? "Unknown";
      const ecc = ECC_LABELS[diagnostics.ecc] ?? "Unknown";
      svgRef.current = value.preview.svg;
      pngRef.current = new Uint8Array(value.preview.png);
      setPreview({
        ariaLabel: `Generated QR code preview: ${mode} mode, version ${diagnostics.selectedVersion}, ECC ${ecc}.`,
        capacityLimit: value.preview.capacityLimit,
        caution:
          "The bundled logo obscures QR data modules. Validate the exported code in its actual environment.",
        diagnostics,
        exportStatus: "SVG and PNG downloads are ready.",
        placeholder: "",
        ready: true,
        svg: value.preview.svg,
      });
    },
    [invalidatePending, setGenerationFailure],
  );

  const startWorker = useCallback(() => {
    const worker = new Worker(new URL("../scripts/preview-worker.ts", import.meta.url), {
      type: "module",
    });
    worker.addEventListener("message", (event: MessageEvent<unknown>) => {
      handlePreviewResult(event.data);
    });
    worker.addEventListener("error", () => {
      worker.terminate();
      if (workerRef.current === worker) workerRef.current = null;
      invalidatePending();
      setGenerationFailure(INTERNAL_FAILURE);
    });
    return worker;
  }, [handlePreviewResult, invalidatePending, setGenerationFailure]);

  // This effect deliberately reflects the external worker/debounce lifecycle in
  // UI state; the state is not derivable from props alone.
  // oxlint-disable react/set-state-in-effect
  useEffect(() => {
    workerRef.current = startWorker();
    const destroy = () => {
      clearTimer();
      workerRef.current?.terminate();
      workerRef.current = null;
    };
    window.addEventListener("pagehide", destroy);
    return () => {
      window.removeEventListener("pagehide", destroy);
      destroy();
    };
  }, [clearTimer, startWorker]);

  useEffect(() => {
    invalidatePending();
    if (request === null) {
      setGenerationFailure(invalidMessage);
      return;
    }

    const revision = revisionRef.current;
    svgRef.current = null;
    pngRef.current = null;
    setPreview({
      ...EMPTY_PREVIEW,
      exportStatus: "QR preview is updating.",
      placeholder: "Updating preview...",
    });
    timerRef.current = window.setTimeout(() => {
      timerRef.current = null;
      if (workerRef.current === null) workerRef.current = startWorker();
      // Worker.postMessage has no target-origin parameter; Oxlint otherwise
      // resolves the Window overload because both globals exist in DOM types.
      // oxlint-disable-next-line unicorn/require-post-message-target-origin
      workerRef.current.postMessage({ ...request, revision } satisfies PreviewRequest);
    }, DEBOUNCE_MILLISECONDS);

    return clearTimer;
    // Every callback used by the timer is included so a compiler rewrite cannot
    // leave the effect holding a stale worker lifecycle function.
    // oxlint-disable-next-line react/exhaustive-effect-dependencies
  }, [clearTimer, invalidMessage, invalidatePending, request, setGenerationFailure, startWorker]);
  // oxlint-enable react/set-state-in-effect

  const download = useCallback((format: DownloadFormat) => {
    if (format === "png" && pngRef.current !== null) {
      downloadBytes(pngRef.current, "image/png", "qr-code.png");
    }
    if (format === "svg" && svgRef.current !== null) {
      downloadBytes(new TextEncoder().encode(svgRef.current), "image/svg+xml", "qr-code.svg");
    }
  }, []);

  return { download, preview };
}
