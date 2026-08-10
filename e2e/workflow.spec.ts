import { expect, test } from "@playwright/test";

import { diagnostic, enterPayload, selectProfile } from "./helpers";

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
  await selectProfile(page, "Inline");

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

test("Adaptive preview keeps rounded modules visible at its declared size", async ({ page }) => {
  await selectProfile(page, "Adaptive");
  await enterPayload(page, "adaptive preview visibility");

  const preview = page
    .getByTestId("qr-preview")
    .locator('svg:visible:not([data-role="bundled-logo"])');
  await expect(preview).toHaveAttribute("width", /\d+/);
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

test("uses one opaque white rounded ONE appearance", async ({ page }) => {
  await enterPayload(page, "approved color workflow");

  const logo = page.getByRole("checkbox", { name: /ONE lettermark/ });
  await expect(logo).toBeChecked();
  await page.getByText("ONE lettermark", { exact: true }).click();
  await expect(logo).not.toBeChecked();
  await expect(page.getByRole("group", { name: "Foreground color" })).toHaveCount(0);
  await expect(page.getByRole("group", { name: "Background treatment" })).toHaveCount(0);

  await expect.poll(() => diagnostic(page, "Foreground")).toBe("#BD0F72");
  await expect.poll(() => diagnostic(page, "Background")).toBe("Opaque white");
  await expect.poll(() => diagnostic(page, "Modules")).toBe("Rounded ONE");
  await expect.poll(() => diagnostic(page, "Contrast")).toBe("6.04:1");
  await expect.poll(() => diagnostic(page, "Safety")).toBe("Safe");
  await expect(page.getByTestId("qr-preview").locator("path").first()).toHaveAttribute(
    "fill",
    "#bd0f72",
  );
  await expect(page.getByTestId("download-svg")).toBeEnabled();
  await expect(page.getByTestId("qr-preview").locator("rect").first()).toHaveAttribute(
    "fill",
    "#ffffff",
  );
});

test("logo mode is enabled by default and can be turned off", async ({ page }) => {
  await enterPayload(page, "a".repeat(30));
  const logo = page.getByRole("checkbox", { name: /ONE lettermark/ });

  await expect(logo).toBeChecked();
  await expect.poll(() => diagnostic(page, "ECC")).toBe("H");
  await expect
    .poll(() => diagnostic(page, "Version"))
    .toBe("V6 / V8 max · raised to V6 for branding");
  await expect.poll(() => diagnostic(page, "Safety")).toBe("Caution");
  await expect
    .poll(() => diagnostic(page, "Logo"))
    .toBe("ONE lettermark · 105 data · 0 remainder modules obscured");
  const renderedLogos = page.getByTestId("qr-preview").locator('[data-role="bundled-logo"]');
  await expect(renderedLogos).toHaveCount(1);
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
  await page.getByText("ONE lettermark", { exact: true }).click();
  await expect(logo).not.toBeChecked();
  await expect.poll(() => diagnostic(page, "ECC")).toBe("M");
});

test("fixed profiles recommend Adaptive when centered branding is unavailable", async ({
  page,
}) => {
  await selectProfile(page, "Print");
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
  await selectProfile(page, "Adaptive");
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
  await selectProfile(page, "Adaptive");
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
  await selectProfile(page, "Adaptive");
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

test("always uses rounded ONE modules without an appearance control", async ({ page }) => {
  await enterPayload(page, "rounded ONE output");
  await expect.poll(() => diagnostic(page, "Modules")).toBe("Rounded ONE");
  await expect(page.getByText("ONE appearance", { exact: true })).toHaveCount(0);
  await expect(page.getByRole("group", { name: "Data module shape" })).toHaveCount(0);
  await expect(page.getByRole("checkbox", { name: /Rounded ONE modules/ })).toHaveCount(0);
  await expect(page.getByTestId("download-svg")).toBeEnabled();
  await expect(page.getByTestId("download-png")).toBeEnabled();
  const modulePath = page.getByTestId("qr-preview").locator("path").first();
  await expect(modulePath).not.toHaveAttribute("shape-rendering", "crispEdges");
  await expect(modulePath).toHaveAttribute("d", /a0\.375 0\.375/);
  await expect(modulePath).toHaveAttribute("d", /M4 4h1v1h-1z/);
});

test("explains export, physical sizing, and placement validation before generation", async ({
  page,
}) => {
  await page.goto("/");

  const guidance = page.getByTestId("release-guidance");
  await expect(guidance).toContainText("Choose SVG first");
  await expect(guidance).toContainText("25–30 mm or larger");
  await expect(guidance).toContainText("Logo output needs extra validation");
  await expect(guidance).toContainText("actual camera, scanner, screen, print material");
});

test("guides long-link profile, logo, and PNG choices accurately", async ({ page }) => {
  const guide = page.getByTestId("link-guide");

  const capacityTable = page.getByRole("table", {
    name: "Maximum typical ASCII link length by output variant",
  });
  await expect(capacityTable.getByRole("columnheader")).toHaveText([
    "Output variant",
    "Without logo",
    "With logo",
  ]);
  await expect(capacityTable.getByRole("row")).toHaveText([
    "Output variantWithout logoWith logo",
    "Inline106 characters / bytes58 characters / bytes",
    "Content152 characters / bytes58 characters / bytes",
    "Landing287 characters / bytes58 characters / bytes",
    "Print331 characters / bytes58 characters / bytes",
    "Adaptive2,331 characters / bytes137 characters / bytes",
  ]);

  await expect(guide).toContainText("A shorter URL usually produces a smaller, less dense QR code");
  await expect(guide).toContainText("For a long link, try no logo");
  await expect(guide).toContainText("standard ECC M and avoids covering QR modules");
  await expect(guide).toContainText("version-aware logo placement through Version 11");
  await expect(guide).toContainText("If the link needs Version 12 or higher, disable the logo");
  await expect(guide).toContainText("Fixed-size profiles download PNGs at 3×");
  await expect(guide).toContainText("their width is 1.5×");
  await expect(guide).toContainText("Scan before you use it");
  await expect(guide).toContainText(
    "Always scan the final QR code before publishing or printing it",
  );
  await expect(guide).toContainText("same size, material, screen, and placement");
  await expect(guide).toContainText("ASCII links that use QR Byte mode");
  await expect(guide).toContainText("scheme, host, path, query, and fragment");
  await expect(guide).toContainText("Non-ASCII characters can use multiple UTF-8 bytes");
  await expect(guide).toContainText("Fixed variants approve logo placement only at Version 6");
  await expect(guide).toContainText("The difference is not a fixed character subtraction");
  await expect(guide).toContainText("ECC H's nominal percentage is not an occlusion budget");
  await expect(guide).toContainText("The preview result for your exact text is authoritative");

  await expect(
    guide.getByRole("heading", { name: "Which output variant should I choose?" }),
  ).toBeVisible();
  await expect(guide.getByRole("heading", { name: "Inline", exact: true })).toBeVisible();
  await expect(guide.getByRole("heading", { name: "Content", exact: true })).toBeVisible();
  await expect(guide.getByRole("heading", { name: "Landing", exact: true })).toBeVisible();
  await expect(guide.getByRole("heading", { name: "Print", exact: true })).toBeVisible();
  await expect(guide.getByRole("heading", { name: "Adaptive", exact: true })).toBeVisible();
  await expect(guide).toContainText("Inline uses a fixed 100 px SVG and 300 px PNG");
  await expect(guide).toContainText("Content uses a fixed 120 px SVG and 360 px PNG");
  await expect(guide).toContainText("Landing uses a fixed 150 px SVG and 450 px PNG");
  await expect(guide).toContainText("Print uses a fixed 160 px SVG and 480 px PNG");
  await expect(guide).toContainText("selects the smallest QR version that fits your exact text");
  await expect(guide).toContainText("four-module quiet zone");
  await expect(guide).toContainText("the logo is exactly centered at Version 6");
  await expect(guide).toContainText("Versions 7–11 move it six modules above center");
  await expect(guide).toContainText("Version 12 or higher rejects the logo");
});
