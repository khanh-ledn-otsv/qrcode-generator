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

test("profile controls work by keyboard", async ({ page }) => {
  await enterPayload(page, "keyboard profile");
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
});

test("shows the opaque preview at its real SVG size", async ({ page }) => {
  await enterPayload(page, "real-size preview");

  const preview = page
    .getByTestId("qr-preview")
    .locator('svg:visible:not([data-role="bundled-logo"])');
  await expect(preview).toHaveAttribute("width", "120");
  await expect(preview).toHaveAttribute("height", "120");
  const box = await preview.boundingBox();
  expect(box).not.toBeNull();
  expect(box!.width).toBeCloseTo(120, 2);
  expect(box!.height).toBeCloseTo(120, 2);
});

test("uses only magenta and shows transparent placement cautions", async ({ page }) => {
  await enterPayload(page, "approved color workflow");

  const logo = page.getByRole("checkbox", { name: /ONE lettermark/ });
  await expect(logo).toBeChecked();
  await page.getByText("ONE lettermark", { exact: true }).click();
  await expect(logo).not.toBeChecked();

  const white = page.getByRole("radio", { name: /Opaque white/ });
  const transparent = page.getByRole("radio", { name: /Transparent/ });
  await expect(white).toBeChecked();
  await expect(page.getByRole("group", { name: "Foreground color" })).toHaveCount(0);
  await expect(
    page.getByRole("group", { name: "Background treatment" }).getByRole("radio"),
  ).toHaveCount(2);

  await expect.poll(() => diagnostic(page, "Foreground")).toBe("#BD0F72");
  await expect.poll(() => diagnostic(page, "Contrast")).toBe("6.04:1");
  await expect(page.getByTestId("qr-preview").locator("path").first()).toHaveAttribute(
    "fill",
    "#bd0f72",
  );

  await transparent.focus();
  await page.keyboard.press("Space");
  await expect.poll(() => diagnostic(page, "Safety")).toBe("Caution");
  await expect.poll(() => diagnostic(page, "Contrast")).toBe("Unknown on placement surface");
  await expect(page.getByRole("status").filter({ hasText: "Transparent output" })).toBeVisible();
  const surfaces = page.getByTestId("transparent-surface-preview");
  await expect(surfaces).toHaveCount(4);
  await expect(surfaces).toHaveText(["White", "Light gray", "Dark", "Patterned"]);
  await expect(page.getByTestId("download-svg")).toBeEnabled();
  await expect(surfaces.first().locator("rect")).toHaveCount(0);
});

test("logo mode is selected by default, uses ECC H, and requires opaque white", async ({
  page,
}) => {
  await enterPayload(page, "a".repeat(30));
  const logo = page.getByRole("checkbox", { name: /ONE lettermark/ });
  const transparent = page.getByRole("radio", { name: /Transparent/ });

  await expect(logo).toBeChecked();
  await expect.poll(() => diagnostic(page, "ECC")).toBe("H");
  await expect.poll(() => diagnostic(page, "Safety")).toBe("Caution");
  await expect.poll(() => diagnostic(page, "Logo")).toContain("ONE lettermark");
  await expect(transparent).toBeDisabled();
  const renderedLogos = page.getByTestId("qr-preview").locator('[data-role="bundled-logo"]');
  await expect(renderedLogos).toHaveCount(5);

  const logoCard = page.getByText("ONE lettermark", { exact: true }).locator("..");
  await expect(logoCard).toHaveCSS("display", "block");
  const cardWidthRatio = await logoCard.evaluate((card) => {
    const parentWidth = card.parentElement?.getBoundingClientRect().width ?? 0;
    return card.getBoundingClientRect().width / parentWidth;
  });
  expect(cardWidthRatio).toBeGreaterThan(0.95);

  const sourceWidthRatio = await renderedLogos.first().evaluate((renderedLogo) => {
    const outerSvg = renderedLogo.ownerSVGElement;
    if (!(renderedLogo instanceof SVGSVGElement) || outerSvg === null) return 0;
    return renderedLogo.width.baseVal.value / outerSvg.viewBox.baseVal.width;
  });
  expect(sourceWidthRatio).toBeGreaterThanOrEqual(0.18);

  const visibleArtworkCoverage = await renderedLogos.first().evaluate((renderedLogo) => {
    if (!(renderedLogo instanceof SVGSVGElement)) return { width: 0, height: 0 };
    const shapes = [...renderedLogo.querySelectorAll<SVGGraphicsElement>("path, polygon")];
    const boxes = shapes.map((shape) => shape.getBBox());
    const left = Math.min(...boxes.map((box) => box.x));
    const top = Math.min(...boxes.map((box) => box.y));
    const right = Math.max(...boxes.map((box) => box.x + box.width));
    const bottom = Math.max(...boxes.map((box) => box.y + box.height));
    return {
      width: (right - left) / renderedLogo.viewBox.baseVal.width,
      height: (bottom - top) / renderedLogo.viewBox.baseVal.height,
    };
  });
  expect(visibleArtworkCoverage.width).toBeGreaterThanOrEqual(0.9);
  expect(visibleArtworkCoverage.height).toBeGreaterThanOrEqual(0.85);
});

test("uses square modules and standard square finders without a shape control", async ({
  page,
}) => {
  await enterPayload(page, "approved styling workflow");

  await expect(page.getByRole("group", { name: "Data module shape" })).toHaveCount(0);
  await expect.poll(() => diagnostic(page, "Function modules")).toBe("Square");
  await expect.poll(() => diagnostic(page, "Finders")).toBe("Standard square");
  await expect(page.getByTestId("download-svg")).toBeEnabled();
  await expect(page.getByTestId("download-png")).toBeEnabled();
  await expect(page.getByTestId("qr-preview").locator("path").first()).toHaveAttribute(
    "d",
    /M\d+ \d+h1v1h-1z/,
  );
});

test("explains export, physical sizing, and placement validation before generation", async ({
  page,
}) => {
  await page.goto("/");

  const guidance = page.getByTestId("release-guidance");
  await expect(guidance).toContainText("Choose SVG first");
  await expect(guidance).toContainText("25–30 mm or larger");
  await expect(guidance).toContainText("Transparent output and logo output need extra validation");
  await expect(guidance).toContainText("actual camera, scanner, screen, print material");
});
