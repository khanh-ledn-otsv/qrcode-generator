import { expect, test } from "@playwright/test";

import { diagnostic, enterPayload, selectProfile } from "./helpers";

test.beforeEach(async ({ page }) => {
  await page.goto("/");
});

test("reports URL counts, validation, and latest debounced input", async ({ page }) => {
  const input = page.getByLabel("Base URL");
  await input.fill("invalid");
  await expect(page.getByRole("alert")).toContainText("valid URL");
  await expect(page.getByTestId("download-svg")).toBeDisabled();

  await input.fill("https://e.test/old");
  await input.fill("https://e.test/latest?value=caf%C3%A9");
  await expect(page.getByLabel("Encoded URL")).toHaveValue("https://e.test/latest?value=caf%C3%A9");
  await expect(page.locator("#base-url-counts")).toHaveText("37 characters | 37 UTF-8 bytes");
  await expect(page.getByTestId("download-svg")).toBeEnabled();
  await expect(page.locator("#encoded-url-guidance")).toContainText("typical ASCII maximum");
});

test("profile controls are keyboard accessible and preserve declared geometry", async ({
  page,
}) => {
  await enterPayload(page, "https://e.test/geometry");
  const select = page.getByLabel("Output variant");
  await expect(select.locator("option")).toHaveText([
    "Small",
    "Standard",
    "Primary CTA",
    "Hero / Campaign",
  ]);
  await select.focus();
  await select.selectOption("small");
  await expect(select).toBeFocused();
  await expect(select).toHaveValue("small");

  const preview = page
    .getByTestId("qr-preview")
    .locator('svg:visible:not([data-role="bundled-logo"])');
  await expect(preview).toHaveAttribute("width", "100");
  await expect(preview).toHaveAttribute("height", "100");
  await expect(page.getByTestId("download-png")).toBeEnabled();

  await expect.poll(() => diagnostic(page, "Output")).toBe("100 px SVG / 300 px PNG");
  await expect.poll(() => diagnostic(page, "Version")).toContain("V6 max");

  await page.getByRole("radio", { name: "Print" }).check();
  await expect(select.locator("option")).toHaveText([
    "Business card",
    "Flyer / Brochure",
    "Poster / Package",
  ]);
  await expect(select).toHaveValue("business-card");
});

test("approved colors preserve the automatic QR logo without changing the URL", async ({
  page,
}) => {
  const url = "https://e.test/brand";
  await enterPayload(page, url);
  const encoded = page.getByLabel("Encoded URL");
  await expect(page.getByRole("checkbox", { name: "ONE logo in QR" })).toHaveCount(0);
  await expect(page.getByTestId("qr-preview").locator('[data-role="bundled-logo"]')).toHaveCount(1);

  await page.getByRole("radio", { name: "Black" }).check();
  await expect(page.getByTestId("qr-preview").locator("path").first()).toHaveAttribute(
    "fill",
    "#000000",
  );
  await expect(page.getByTestId("qr-preview").locator('[data-role="bundled-logo"]')).toHaveCount(1);
  await expect(encoded).toHaveValue(url);
  await expect(page.getByText(/typical ASCII maximum: 84/)).toBeVisible();
});

test("long branded URLs use the existing no-logo fallback and keep the exact URL", async ({
  page,
}) => {
  await selectProfile(page, "Poster / Package");
  const url = `https://e.test/${"a".repeat(130)}`;
  await enterPayload(page, url);
  await expect(page.getByLabel("Encoded URL")).toHaveValue(url);
  await expect.poll(() => diagnostic(page, "Logo request")).toContain("disabled");
  await expect(page.getByTestId("download-svg")).toBeEnabled();
});

test("specification is expanded by default and remains collapsible", async ({ page }) => {
  const details = page.getByTestId("qr-specification");
  await expect(details).toHaveAttribute("open", "");
  await expect(page.getByTestId("release-guidance")).toContainText("Choose SVG");
  await expect(page.getByTestId("release-guidance")).toContainText("final camera");
  await expect(page.getByTestId("release-guidance")).toContainText("URL is never changed");
  await details.locator("summary").click();
  await expect(details).not.toHaveAttribute("open", "");
});
