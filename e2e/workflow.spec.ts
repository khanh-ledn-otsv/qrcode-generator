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
  await page.getByText("Inline", { exact: true }).click();

  const preview = page
    .getByTestId("qr-preview")
    .locator('svg:visible:not([data-role="bundled-logo"])');
  await expect(page.getByRole("radio", { name: /Inline/ })).toBeChecked();
  await expect(page.getByRole("radio", { name: /Inline/ })).toHaveAccessibleName(
    /100 px SVG · 300 px PNG · up to V6/,
  );
  await expect(preview).toHaveAttribute("width", "100");
  await expect(preview).toHaveAttribute("height", "100");
  const box = await preview.boundingBox();
  expect(box).not.toBeNull();
  expect(box!.width).toBeCloseTo(100, 2);
  expect(box!.height).toBeCloseTo(100, 2);
  await expect
    .poll(() => diagnostic(page, "Version"))
    .toBe("V6 / V6 max · raised to V6 for branding");
  await expect(page.getByTestId("download-svg")).toBeEnabled();
  await expect(page.getByTestId("download-png")).toBeEnabled();
});

test("Adaptive preview keeps compact modules visible at its declared size", async ({ page }) => {
  await page.getByText("Adaptive", { exact: true }).click();
  await enterPayload(page, "adaptive preview visibility");

  const preview = page
    .getByTestId("qr-preview")
    .locator('svg:visible:not([data-role="bundled-logo"])');
  await expect(preview).toHaveAttribute("width", "196");
  const visiblePixelsOutsideLargeArtwork = await preview.evaluate(async (svg) => {
    const markup = new XMLSerializer().serializeToString(svg);
    const image = new Image();
    image.src = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(markup)}`;
    await image.decode();
    const width = Number(svg.getAttribute("width"));
    const height = Number(svg.getAttribute("height"));
    const extent = svg.viewBox.baseVal.width;
    const pixelsPerModule = width / extent;
    const canvas = new OffscreenCanvas(width, height);
    const context = canvas.getContext("2d");
    if (context === null) return 0;
    context.fillStyle = "#ffffff";
    context.fillRect(0, 0, width, height);
    context.drawImage(image, 0, 0, width, height);
    const pixels = context.getImageData(0, 0, width, height).data;
    let visible = 0;
    for (let y = 0; y < height; y += 1) {
      for (let x = 0; x < width; x += 1) {
        const moduleX = x / pixelsPerModule;
        const moduleY = y / pixelsPerModule;
        const inFinder =
          (moduleX >= 4 && moduleX < 11 && moduleY >= 4 && moduleY < 11) ||
          (moduleX >= 38 && moduleX < 45 && moduleY >= 4 && moduleY < 11) ||
          (moduleX >= 4 && moduleX < 11 && moduleY >= 38 && moduleY < 45);
        const inLogoArea = moduleX >= 16 && moduleX < 33 && moduleY >= 15 && moduleY < 34;
        if (inFinder || inLogoArea) continue;
        const offset = (y * width + x) * 4;
        const red = pixels[offset] ?? 255;
        const green = pixels[offset + 1] ?? 255;
        const blue = pixels[offset + 2] ?? 255;
        if (red > green + 20 && blue > green + 20 && green < 245) visible += 1;
      }
    }
    return visible;
  });

  expect(visiblePixelsOutsideLargeArtwork).toBeGreaterThan(200);
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
  await expect
    .poll(() => diagnostic(page, "Version"))
    .toBe("V6 / V8 max · raised to V6 for branding");
  await expect.poll(() => diagnostic(page, "Safety")).toBe("Caution");
  await expect
    .poll(() => diagnostic(page, "Logo"))
    .toBe("ONE lettermark · 105 data · 0 remainder modules obscured");
  await expect(transparent).toBeDisabled();
  const renderedLogos = page.getByTestId("qr-preview").locator('[data-role="bundled-logo"]');
  await expect(renderedLogos).toHaveCount(5);
  await expect(renderedLogos.first()).toHaveAttribute("x", "18");
  await expect(renderedLogos.first()).toHaveAttribute("y", "22.0625");
  await expect(renderedLogos.first()).toHaveAttribute("width", "13");
  await expect(renderedLogos.first()).toHaveAttribute("height", "4.8750");

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

test("fixed profiles recommend Adaptive when centered branding is unavailable", async ({
  page,
}) => {
  await page.getByText("Print", { exact: true }).click();
  await page.getByLabel("Text to encode").fill("a".repeat(59));

  await expect(page.getByRole("alert")).toContainText(
    "Try Adaptive for long payloads and version-aware logo placement.",
  );
  await expect(page.getByTestId("download-svg")).toBeDisabled();
  await expect(page.getByTestId("download-png")).toBeDisabled();
});

test("Adaptive preserves and exports the long ONE URL at Version 10", async ({ page }) => {
  const payload =
    "https://www.one-line.com/en/news/notice-mandatory-advance-cargo-declaration-acd-reference-number-imports-kenya";
  await page.getByText("Adaptive", { exact: true }).click();
  await enterPayload(page, payload);

  await expect(page.getByRole("radio", { name: /Adaptive/ })).toBeChecked();
  await expect(page.getByRole("radio", { name: /Adaptive/ })).toHaveAccessibleName(
    /Automatic dimensions · up to V40/,
  );
  await expect(page.getByLabel("Text to encode")).toHaveValue(payload);
  await expect.poll(() => diagnostic(page, "Version")).toBe("V10 / V40 max");
  await expect.poll(() => diagnostic(page, "ECC")).toBe("H");
  await expect
    .poll(() => diagnostic(page, "PNG geometry"))
    .toBe("6 px/module · 390 px symbol · 0 px padding");
  await expect.poll(() => diagnostic(page, "Output")).toBe("260 px SVG · 390 px PNG");
  await expect
    .poll(() => diagnostic(page, "Logo"))
    .toBe("ONE lettermark · 105 data · 0 remainder modules obscured");
  await expect
    .poll(() => diagnostic(page, "Logo bounds"))
    .toBe(
      "source (22, 20.0625) 13 × 4.875 modules · knockout (21, 19) 15 × 7 modules · 0 module protected clearance",
    );
  const renderedLogo = page.getByTestId("qr-preview").locator('[data-role="bundled-logo"]').first();
  await expect(renderedLogo).toHaveAttribute("x", "26");
  await expect(renderedLogo).toHaveAttribute("y", "24.0625");
  await expect(renderedLogo).toHaveAttribute("width", "13");
  await expect(renderedLogo).toHaveAttribute("height", "4.8750");
  await expect(page.getByTestId("download-svg")).toBeEnabled();
  await expect(page.getByTestId("download-png")).toBeEnabled();
});

test("Adaptive grows through Version 11 and gates unreviewed higher-version branding", async ({
  page,
}) => {
  await page.getByText("Adaptive", { exact: true }).click();
  const versionElevenUrl = `https://example.test/${"a".repeat(105)}`;
  await enterPayload(page, versionElevenUrl);

  await expect.poll(() => diagnostic(page, "Version")).toBe("V11 / V40 max");
  await expect.poll(() => diagnostic(page, "Output")).toBe("276 px SVG · 414 px PNG");
  await expect(page.getByTestId("download-svg")).toBeEnabled();
  await expect(page.getByTestId("download-png")).toBeEnabled();

  const versionTwelveUrl = `https://example.test/${"a".repeat(120)}`;
  await page.getByLabel("Text to encode").fill(versionTwelveUrl);
  await expect(page.getByRole("alert")).toContainText(
    "Adaptive logo placement is approved only through QR Version 11; disable the logo to keep this exact payload.",
  );
  await expect(page.getByTestId("download-svg")).toBeDisabled();

  await page.getByText("ONE lettermark", { exact: true }).click();
  await expect(page.getByLabel("Text to encode")).toHaveValue(versionTwelveUrl);
  await expect(page.getByTestId("download-svg")).toBeEnabled();
  await expect(page.getByTestId("download-png")).toBeEnabled();
});

test("Adaptive reaches the exact unbranded Version 40 boundary", async ({ page }) => {
  await page.getByText("Adaptive", { exact: true }).click();
  await page.getByText("ONE lettermark", { exact: true }).click();
  await enterPayload(page, "a".repeat(2_331));

  await expect.poll(() => diagnostic(page, "Version")).toBe("V40 / V40 max");
  await expect.poll(() => diagnostic(page, "ECC")).toBe("M");
  await expect.poll(() => diagnostic(page, "Output")).toBe("740 px SVG · 1110 px PNG");
  await expect
    .poll(() => diagnostic(page, "PNG geometry"))
    .toBe("6 px/module · 1110 px symbol · 0 px padding");

  await page.getByLabel("Text to encode").fill("a".repeat(2_332));
  await expect(page.getByRole("alert")).toContainText(
    "The payload does not fit this profile's maximum QR version 40.",
  );
  await expect(page.getByTestId("download-svg")).toBeDisabled();
  await expect(page.getByTestId("download-png")).toBeDisabled();
});

test("uses compact dots and standard square finders without a shape control", async ({ page }) => {
  await enterPayload(page, "approved styling workflow");

  await expect(page.getByRole("group", { name: "Data module shape" })).toHaveCount(0);
  await expect.poll(() => diagnostic(page, "Non-finder modules")).toBe("Compact dots");
  await expect.poll(() => diagnostic(page, "Finders")).toBe("Standard square");
  await expect(page.getByTestId("download-svg")).toBeEnabled();
  await expect(page.getByTestId("download-png")).toBeEnabled();
  const modulePath = page.getByTestId("qr-preview").locator("path").first();
  await expect(modulePath).toHaveAttribute("d", /M\d+\.275 \d+\.500a0\.225 0\.225 0 1 0 0\.450 0/);
  await expect(modulePath).toHaveAttribute("d", /M4 4h1v1h-1z/);
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
