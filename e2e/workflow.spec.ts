import { expect, test } from "@playwright/test";

import { diagnostic, enterPayload } from "./helpers";

test.beforeEach(async ({ page }) => {
  await page.goto("/");
});

test("reports representative modes, UTF-8 counts, and latest debounced input", async ({ page }) => {
  const cases = [
    ["1234567890", "Numeric"],
    ["HELLO WORLD", "Alphanumeric"],
    ["https://example.test/a", "Byte"],
    ["café", "Byte"],
  ] as const;

  // Each case intentionally mutates and observes the same browser page in sequence.
  for (const [payload, mode] of cases) {
    // oxlint-disable-next-line no-await-in-loop
    await enterPayload(page, payload);
    // oxlint-disable-next-line no-await-in-loop
    await expect.poll(() => diagnostic(page, "Mode")).toBe(mode);
  }
  await expect(page.getByText("4 characters", { exact: true })).toBeVisible();
  await expect(page.getByText("5 UTF-8 bytes", { exact: true })).toBeVisible();

  const input = page.getByLabel("Text to encode");
  await input.fill("old value");
  await input.fill("987654321");
  await expect.poll(() => diagnostic(page, "Mode")).toBe("Numeric");
  await expect(page.getByTestId("qr-preview")).toHaveAttribute("aria-label", /Numeric mode/);
});

test("distinguishes the input-limit boundary and keeps exports disabled", async ({ page }) => {
  const input = page.getByLabel("Text to encode");
  await input.fill("x".repeat(4096));
  await expect(page.getByRole("alert")).toContainText("does not fit this profile");
  await expect(page.getByTestId("download-svg")).toBeDisabled();

  await input.fill("x".repeat(4097));
  await expect(page.getByRole("alert")).toContainText("input limit is 4096 bytes");
  await expect(page.getByTestId("download-png")).toBeDisabled();
  await expect(page.getByRole("alert")).toHaveAttribute("aria-live", "polite");
});

test("disposing the page with pending debounce work initializes cleanly", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  await page.getByLabel("Text to encode").fill("pending preview");
  await page.reload();

  await expect(page.getByLabel("Text to encode")).toHaveValue("");
  await expect(page.getByTestId("download-svg")).toBeDisabled();
  expect(pageErrors).toEqual([]);
});

test("profile controls work by keyboard and layouts fit desktop and mobile widths", async ({
  page,
}) => {
  await enterPayload(page, "responsive profile");
  const content = page.getByRole("radio", { name: /Content/ });
  await content.focus();
  await page.keyboard.press("ArrowRight");
  await expect(page.getByRole("radio", { name: /Landing/ })).toBeChecked();
  await expect.poll(() => diagnostic(page, "Version")).toContain("V12 max");
  const landing = page.getByRole("radio", { name: /Landing/ });
  await expect(landing).toBeFocused();
  const focusRing = await landing
    .locator("xpath=..")
    .evaluate((label) => getComputedStyle(label).boxShadow);
  expect(focusRing).not.toBe("none");

  const viewportWidth = await page.evaluate(() => document.documentElement.clientWidth);
  const scrollWidth = await page.evaluate(() => document.documentElement.scrollWidth);
  expect(scrollWidth).toBeLessThanOrEqual(viewportWidth);
  const payloadBox = await page.getByRole("region", { name: "Payload" }).boundingBox();
  const previewBox = await page.getByRole("region", { name: "Preview" }).boundingBox();
  expect(payloadBox).not.toBeNull();
  expect(previewBox).not.toBeNull();
  if (viewportWidth < 1024) {
    expect(previewBox!.y).toBeGreaterThan(payloadBox!.y);
  } else {
    expect(Math.abs(previewBox!.y - payloadBox!.y)).toBeLessThan(4);
  }
});
