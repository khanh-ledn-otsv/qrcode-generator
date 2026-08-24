import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.goto("/");
});

test("validates URLs and synchronizes UTM and custom parameters with the base URL", async ({
  page,
}) => {
  const baseUrl = page.getByLabel("Base URL");
  await baseUrl.fill("not a url");
  await expect(page.getByRole("alert")).toContainText(
    "valid URL beginning with http:// or https://",
  );
  await expect(page.getByTestId("download-svg")).toBeDisabled();

  await baseUrl.fill(
    "https://e.test/p?utm_source=existing&audience=initial%20group&fixed=a%20b#offer",
  );
  await expect(page.getByLabel("utm_source")).toHaveValue("existing");
  await expect(page.getByLabel("Custom parameter 1 name")).toHaveValue("audience");
  await expect(page.getByLabel("Custom parameter 1 value")).toHaveValue("initial group");
  await expect(page.getByLabel("Custom parameter 2 name")).toHaveValue("fixed");
  await expect(page.getByLabel("Custom parameter 2 value")).toHaveValue("a b");
  await expect(page.getByLabel("Custom parameter 1 name")).toHaveClass(/rounded-md/);
  await expect(page.getByRole("button", { name: "Remove custom parameter 1" })).toHaveClass(
    /inline-flex/,
  );
  await expect(
    page.getByRole("button", { name: "Remove custom parameter 1" }).locator("svg"),
  ).toHaveClass(/lucide-trash-2/);

  await page.getByLabel("utm_source").fill("replacement");
  await page.getByLabel("utm_medium").fill("QR poster");
  await page.getByLabel("Custom parameter 1 value").fill("ONE staff");
  await page.getByRole("button", { name: "Add Parameter" }).click();
  await page.getByLabel("Custom parameter 3 name").fill("channel");
  await page.getByLabel("Custom parameter 3 value").fill("internal");

  await expect(baseUrl).toHaveValue(
    "https://e.test/p?utm_source=replacement&audience=ONE+staff&fixed=a%20b&utm_medium=QR+poster&channel=internal#offer",
  );
  await expect(page.getByLabel("Encoded URL")).toHaveValue(
    "https://e.test/p?utm_source=replacement&audience=ONE+staff&fixed=a%20b&utm_medium=QR+poster&channel=internal#offer",
  );
  await expect(page.getByTestId("download-svg")).toBeEnabled();
  await expect(page.getByText(/typical ASCII maximum: 84/)).toBeVisible();

  await page.getByRole("button", { name: "Remove custom parameter 1" }).click();
  await expect(baseUrl).toHaveValue(
    "https://e.test/p?utm_source=replacement&fixed=a%20b&utm_medium=QR+poster&channel=internal#offer",
  );

  const utmToggle = page.getByLabel("Enable UTM configuration");
  const utmPanel = page.getByRole("region", { name: "UTM Configuration" });
  const expandedHeight = await utmPanel.evaluate(
    (element) => element.getBoundingClientRect().height,
  );
  await utmToggle.locator("xpath=ancestor::label").click();
  await expect(utmToggle).not.toBeChecked();
  await expect(utmToggle).toHaveAttribute("aria-expanded", "false");
  await expect(page.locator("#utm-content")).toBeHidden();
  expect(await utmPanel.evaluate((element) => element.getBoundingClientRect().height)).toBeLessThan(
    expandedHeight,
  );
  await expect(baseUrl).toHaveValue("https://e.test/p?fixed=a%20b&channel=internal#offer");
  await expect(page.getByLabel("Encoded URL")).toHaveValue(
    "https://e.test/p?fixed=a%20b&channel=internal#offer",
  );
  await expect(page.getByLabel("utm_source")).toHaveValue("replacement");

  await baseUrl.fill("https://e.test/new?utm_campaign=launch&ref=a%20b#details");
  await expect(utmToggle).toBeChecked();
  await expect(utmToggle).toHaveAttribute("aria-expanded", "true");
  await expect(page.locator("#utm-content")).toBeVisible();
  await expect(page.getByLabel("utm_source")).toHaveValue("");
  await expect(page.getByLabel("utm_campaign")).toHaveValue("launch");
  await expect(page.getByLabel("Custom parameter 1 name")).toHaveValue("ref");
  await expect(page.getByLabel("Custom parameter 1 value")).toHaveValue("a b");
  await expect(page.getByLabel("Encoded URL")).toHaveValue(
    "https://e.test/new?utm_campaign=launch&ref=a%20b#details",
  );

  await baseUrl.fill("");
  await expect(page.getByLabel("utm_source")).toHaveValue("");
  await expect(page.getByLabel("utm_medium")).toHaveValue("");
  await expect(page.getByLabel("utm_campaign")).toHaveValue("");
  await expect(page.getByLabel(/Custom parameter \d+ name/)).toHaveCount(0);
  await expect(page.getByLabel("Encoded URL")).toHaveValue("");
});

test("keeps the guideline on a separate page and preserves generator controls", async ({
  page,
}) => {
  await page.getByLabel("Base URL").fill("https://example.test/launch");
  await expect(page.getByLabel("Output variant")).toHaveValue("standard");
  await expect(page.getByRole("radio", { name: "Digital" })).toBeChecked();
  await expect(page.getByRole("checkbox", { name: "ONE logo in QR" })).toHaveCount(0);
  await page.getByRole("link", { name: "Guideline" }).click();
  await expect(page).toHaveURL(/\/guideline\/$/);
  await expect(page.getByRole("link", { name: "Guideline" })).toHaveAttribute(
    "aria-current",
    "page",
  );
  await expect(page.getByTestId("usage-guide")).toBeVisible();
  await expect(page.getByTestId("qr-specification")).toHaveCount(0);
  await page.getByRole("link", { name: "Generator" }).click();
  await expect(page).toHaveURL(/\/$/);
  await expect(page.getByTestId("qr-specification")).toBeVisible();
  await expect(page.getByLabel("Base URL")).toHaveValue("");
  await page.getByLabel("Base URL").fill("https://example.test/launch");

  await page.getByRole("radio", { name: "Print" }).check();
  await expect(page.getByLabel("Output variant")).toHaveValue("business-card");
  await page.getByLabel("Output variant").selectOption("poster-package");
  await expect(page.getByLabel("Output variant")).toHaveValue("poster-package");

  await expect(page.getByTestId("qr-specification")).toContainText("Logo request");
  await expect(page.getByTestId("release-guidance")).toContainText("URL is never changed");
  await expect(page.getByTestId("download-png")).toBeEnabled();
});

test("keeps the generator contained on mobile", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await expect(page.getByRole("link", { name: "Guideline" })).toBeVisible();

  await page.getByLabel("Base URL").fill(`https://example.test/${"long-path-".repeat(8)}`);
  const pageWidth = await page.evaluate(() => ({
    client: document.documentElement.clientWidth,
    scroll: document.documentElement.scrollWidth,
  }));
  expect(pageWidth.scroll).toBeLessThanOrEqual(pageWidth.client + 1);
});
