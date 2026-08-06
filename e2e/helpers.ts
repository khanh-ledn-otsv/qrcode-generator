import { expect, type Page } from "@playwright/test";

export const SAFE_PAYLOAD = 'safe/<script>alert("payload")</script>';

export async function enterPayload(page: Page, payload: string): Promise<void> {
  const input = page.getByLabel("Text to encode");
  await input.fill(payload);
  await expect(page.getByTestId("download-svg")).toBeEnabled();
}

export async function diagnostic(page: Page, label: string): Promise<string> {
  const term = page.locator("dt", { hasText: new RegExp(`^${label}$`) });
  return term.locator("xpath=following-sibling::dd").innerText();
}

export async function sha256(bytes: Buffer): Promise<string> {
  const { createHash } = await import("node:crypto");
  return createHash("sha256").update(bytes).digest("hex");
}
