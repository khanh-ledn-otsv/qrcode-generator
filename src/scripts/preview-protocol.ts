export type ProfileValue =
  | "small"
  | "standard"
  | "primary-cta"
  | "hero-campaign"
  | "business-card"
  | "flyer-brochure"
  | "poster-package";

export type PreviewRequest = {
  revision: number;
  payload: string;
  profile: ProfileValue;
  foregroundTheme: string;
  logoEnabled: boolean;
};

export type RawDiagnostics = {
  mode: number;
  ecc: number;
  mask: number;
  minimumVersion: number;
  maximumVersion: number;
  selectedVersion: number;
  brandingIncreasedVersion: boolean;
  matrixModules: number;
  svgSidePixels: number;
  pngSidePixels: number;
  safety: number;
  contrastHundredths: number;
  requestedLogo: boolean;
  renderedLogo: boolean;
  logoFallbackReason: string | null;
  obscuredDataModules: number;
  obscuredRemainderModules: number;
};

export type PreviewResult =
  | {
      revision: number;
      status: "ready";
      preview: {
        svg: string;
        png: ArrayBuffer;
        capacityLimit: number;
        diagnostics: RawDiagnostics;
      };
    }
  | {
      revision: number;
      status: "failed";
      message: string;
    };

export function isPreviewResult(value: unknown): value is PreviewResult {
  if (typeof value !== "object" || value === null) return false;
  const candidate = value as Record<string, unknown>;
  if (!Number.isSafeInteger(candidate.revision)) return false;
  if (candidate.status === "failed") return typeof candidate.message === "string";
  if (candidate.status !== "ready") return false;
  if (typeof candidate.preview !== "object" || candidate.preview === null) return false;
  const preview = candidate.preview as Record<string, unknown>;
  return (
    typeof preview.svg === "string" &&
    preview.png instanceof ArrayBuffer &&
    Number.isSafeInteger(preview.capacityLimit) &&
    typeof preview.diagnostics === "object" &&
    preview.diagnostics !== null
  );
}
