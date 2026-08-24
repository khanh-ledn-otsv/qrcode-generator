import { expect, type Page } from "@playwright/test";

export const SAFE_PAYLOAD = "https://e.test/safe?value=%3Cscript%3E";

export async function enterPayload(page: Page, payload: string): Promise<void> {
  const input = page.getByLabel("Base URL");
  await input.fill(payload);
  await expect(page.getByTestId("download-svg")).toBeEnabled();
}

export async function selectProfile(page: Page, name: string): Promise<void> {
  const values: Record<string, string> = {
    Small: "small",
    Standard: "standard",
    "Primary CTA": "primary-cta",
    "Hero / Campaign": "hero-campaign",
    "Business card": "business-card",
    "Flyer / Brochure": "flyer-brochure",
    "Poster / Package": "poster-package",
  };
  const value = values[name];
  if (value === undefined) throw new Error(`Unknown profile: ${name}`);
  const digital = ["small", "standard", "primary-cta", "hero-campaign"].includes(value);
  await page.getByRole("radio", { name: digital ? "Digital" : "Print" }).check();
  await page.getByLabel("Output variant").selectOption(value);
  await expect(page.getByLabel("Output variant")).toHaveValue(value);
}

export async function diagnostic(page: Page, label: string): Promise<string> {
  const term = page.locator("dt", { hasText: new RegExp(`^${label}$`) });
  return (await term.locator("xpath=following-sibling::dd").textContent()) ?? "";
}

export async function sha256(bytes: Buffer): Promise<string> {
  const { createHash } = await import("node:crypto");
  return createHash("sha256").update(bytes).digest("hex");
}
