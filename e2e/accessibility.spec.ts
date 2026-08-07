import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

import { enterPayload } from "./helpers";

test.beforeEach(async ({ page }) => {
  await page.goto("/");
});

for (const state of ["default", "caution", "transparent", "logo", "invalid"] as const) {
  test(`has no automated accessibility violations in the ${state} state`, async ({ page }) => {
    if (state === "default") await enterPayload(page, "accessible QR");
    if (state === "caution") await enterCautionPayload(page);
    if (state === "transparent") {
      await enterPayload(page, "transparent QR");
      await page.getByRole("radio", { name: /Transparent/ }).focus();
      await page.keyboard.press("Space");
      await expect(page.getByTestId("transparent-surface-preview")).toHaveCount(4);
    }
    if (state === "logo") {
      await enterPayload(page, "accessible logo QR");
      await page.getByText("ONE lettermark", { exact: true }).click();
      await expect(page.getByRole("checkbox", { name: /ONE lettermark/ })).toBeChecked();
    }
    if (state === "invalid") {
      await page.getByLabel("Text to encode").fill("x".repeat(4097));
      await expect(page.getByRole("alert")).toContainText("input limit");
    }

    const results = await new AxeBuilder({ page }).analyze();
    expect(results.violations).toEqual([]);
  });
}

test("warnings, focus, preview labels, and disabled reasons are programmatic", async ({ page }) => {
  const input = page.getByLabel("Text to encode");
  await enterCautionPayload(page);
  await expect(page.getByRole("status").filter({ hasText: "Caution:" })).toBeVisible();
  await expect(input).toBeFocused();
  await expect(page.getByTestId("qr-preview")).toHaveAttribute(
    "aria-label",
    /^Generated QR code preview: .* mode, version \d+, ECC M\.$/,
  );
  await expect(page.getByTestId("qr-preview")).not.toHaveAttribute("aria-label", /line one/);

  await input.fill("");
  await expect(page.getByTestId("download-svg")).toBeDisabled();
  await expect(page.locator("#export-status")).toContainText("Enter text");
  await expect(page.getByTestId("download-svg")).toHaveAttribute(
    "aria-describedby",
    "export-status",
  );
});

async function enterCautionPayload(page: Parameters<typeof enterPayload>[0]): Promise<void> {
  const input = page.getByLabel("Text to encode");
  await input.fill("line one");
  await input.press("End");
  await input.press("Enter");
  await input.pressSequentially("line two");
  await expect(page.getByTestId("download-svg")).toBeEnabled();
}
