import { useState } from "react";

import type { RawDiagnostics } from "../scripts/preview-protocol";

type Props = {
  diagnostics: RawDiagnostics | null;
};

const MODE_LABELS = ["Numeric", "Alphanumeric", "Byte", "Kanji", "Mixed"] as const;
const ECC_LABELS = ["L", "M", "Q", "H"] as const;

function diagnosticValues(diagnostics: RawDiagnostics | null): Record<string, string> {
  if (diagnostics === null) return {};
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

  return {
    "diag-contrast": contrast,
    "diag-ecc": ECC_LABELS[diagnostics.ecc] ?? "Unknown",
    "diag-logo": logo,
    "diag-logo-request": logoRequest,
    "diag-mask": String(diagnostics.mask),
    "diag-matrix": `${diagnostics.matrixModules} x ${diagnostics.matrixModules} modules`,
    "diag-mode": MODE_LABELS[diagnostics.mode] ?? "Unknown",
    "diag-output": `${diagnostics.svgSidePixels} px SVG / ${diagnostics.pngSidePixels} px PNG`,
    "diag-safety": diagnostics.safety === 0 ? "Safe" : "Caution",
    "diag-version": version,
  };
}

const diagnosticLabels = [
  ["diag-mode", "Mode"],
  ["diag-ecc", "ECC"],
  ["diag-version", "Version"],
  ["diag-mask", "Mask"],
  ["diag-matrix", "Matrix"],
  ["diag-output", "Output"],
  ["diag-logo", "Logo"],
  ["diag-logo-request", "Logo request"],
  ["diag-contrast", "Contrast"],
  ["diag-safety", "Safety"],
] as const;

export default function QrSpecification({ diagnostics }: Props) {
  const [isOpen, setIsOpen] = useState(true);
  const values = diagnosticValues(diagnostics);

  return (
    <details
      id="qr-specification"
      className="mt-8 py-4"
      data-testid="qr-specification"
      open={isOpen}
      onToggle={({ currentTarget }) => setIsOpen(currentTarget.open)}
    >
      <summary className="cursor-pointer text-sm font-semibold text-text">
        QR code specification
      </summary>
      <div className="mt-4 grid gap-6 lg:grid-cols-2">
        <dl className="grid grid-cols-2 gap-4 text-sm">
          {diagnosticLabels.map(([id, label]) => (
            <div key={id}>
              <dt className="text-xs font-semibold tracking-wide text-slate-500 uppercase">
                {label}
              </dt>
              <dd id={id} className="mt-1 font-bold text-slate-900">
                {values[id] ?? "-"}
              </dd>
            </div>
          ))}
        </dl>
        <div
          id="release-guidance"
          className="text-sm leading-6 text-text-muted"
          data-testid="release-guidance"
        >
          <p>
            Choose SVG when resizing or preparing print output. Test every downloaded QR code with
            the final camera, screen, material, size, and placement.
          </p>
          <p className="mt-2">
            Logo output uses ECC H and may fall back to an unbranded code when the approved geometry
            is unavailable. The URL is never changed by that fallback.
          </p>
        </div>
      </div>
    </details>
  );
}
