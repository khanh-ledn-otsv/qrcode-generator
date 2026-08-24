import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.goto("/");
});

test("validates URLs and composes UTM and custom parameters without replacing existing keys", async ({
  page,
}) => {
  const baseUrl = page.getByLabel("Base URL");
  await baseUrl.fill("not a url");
  await expect(page.getByRole("alert")).toContainText(
    "valid URL beginning with http:// or https://",
  );
  await expect(page.getByTestId("download-svg")).toBeDisabled();

  await baseUrl.fill("https://e.test/p?utm_source=existing#offer");
  await page.getByLabel("utm_source").fill("replacement");
  await page.getByLabel("utm_medium").fill("QR poster");
  await page.getByRole("button", { name: "Add Parameter" }).click();
  await page.getByLabel("Custom parameter 1 name").fill("audience");
  await page.getByLabel("Custom parameter 1 value").fill("ONE staff");

  await expect(page.getByLabel("Encoded URL")).toHaveValue(
    "https://e.test/p?utm_source=existing&utm_medium=QR+poster&audience=ONE+staff#offer",
  );
  await expect(page.getByTestId("download-svg")).toBeEnabled();
  await expect(page.getByText(/typical ASCII maximum: 84/)).toBeVisible();

  const utmToggle = page.getByLabel("Enable UTM configuration");
  await utmToggle.locator("xpath=ancestor::label").click();
  await expect(utmToggle).not.toBeChecked();
  await expect(page.getByLabel("Encoded URL")).toHaveValue(
    "https://e.test/p?utm_source=existing&audience=ONE+staff#offer",
  );
  await expect(page.getByLabel("utm_source")).toHaveValue("replacement");

  await page.getByRole("button", { name: "Remove custom parameter 1" }).click();
  await expect(page.getByLabel("Encoded URL")).toHaveValue(
    "https://e.test/p?utm_source=existing#offer",
  );
});

test("groups digital and print variants while preserving logo controls and specifications", async ({
  page,
}) => {
  await page.getByLabel("Base URL").fill("https://example.test/launch");
  await expect(page.getByLabel("Output variant")).toHaveValue("standard");
  await expect(page.getByRole("radio", { name: "Digital" })).toBeChecked();
  await expect(page.getByRole("checkbox", { name: "ONE logo in QR" })).toBeChecked();

  await page.getByRole("radio", { name: "Print" }).check();
  await expect(page.getByLabel("Output variant")).toHaveValue("business-card");
  await page.getByLabel("Output variant").selectOption("poster-package");
  await expect(page.getByLabel("Output variant")).toHaveValue("poster-package");

  await page.getByTestId("qr-specification").locator("summary").click();
  await expect(page.getByTestId("qr-specification")).toContainText("Logo request");
  await expect(page.getByTestId("release-guidance")).toContainText("URL is never changed");
  await expect(page.getByTestId("download-png")).toBeEnabled();
});

test("renders the local ONE header and remains contained on mobile", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  const logo = page.getByRole("img", { name: "ONE" });
  await expect(logo).toBeVisible();
  await expect(logo).toHaveAttribute("src", "/public/images/one-logotype-white.png");
  await expect(page.getByRole("button", { name: "Usage" })).toBeEnabled();

  await page.getByLabel("Base URL").fill(`https://example.test/${"long-path-".repeat(8)}`);
  const pageWidth = await page.evaluate(() => ({
    client: document.documentElement.clientWidth,
    scroll: document.documentElement.scrollWidth,
  }));
  expect(pageWidth.scroll).toBeLessThanOrEqual(pageWidth.client + 1);
});
