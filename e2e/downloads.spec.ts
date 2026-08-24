import { existsSync } from "node:fs";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";

import { expect, test, type Page } from "@playwright/test";

import { selectProfile, sha256 } from "./helpers";
import { expectDecodedPayload } from "./zxing";

async function rasterizeDownloadedSvg(
  page: Page,
  svgPath: string,
  rasterPath: string,
  sidePixels: number,
): Promise<void> {
  const svg = await readFile(svgPath, "utf8");
  await page.evaluate(
    async ({ source, rasterSidePixels }) => {
      const url = URL.createObjectURL(new Blob([source], { type: "image/svg+xml" }));
      const image = document.createElement("img");
      image.dataset.testid = "downloaded-svg-raster";
      image.src = url;
      image.width = rasterSidePixels;
      image.height = rasterSidePixels;
      image.style.position = "fixed";
      image.style.inset = "0 auto auto 0";
      document.body.append(image);
      await image.decode();
    },
    { source: svg, rasterSidePixels: sidePixels },
  );
  await page.getByTestId("downloaded-svg-raster").screenshot({ path: rasterPath });
  await page.evaluate(() => {
    const image = document.querySelector<HTMLImageElement>('[data-testid="downloaded-svg-raster"]');
    if (image !== null) {
      URL.revokeObjectURL(image.src);
      image.remove();
    }
  });
}

test.beforeEach(async ({ page }) => {
  await page.goto("/");
});

test("downloads fixed filenames and deterministic bytes for the derived URL", async ({ page }) => {
  await selectProfile(page, "Small");
  await page.getByLabel("Base URL").fill("https://e.test/promo");
  await page.getByLabel("utm_source").fill("poster");
  const payload = "https://e.test/promo?utm_source=poster";
  await expect(page.getByLabel("Encoded URL")).toHaveValue(payload);
  await expect(page.getByTestId("download-svg")).toBeEnabled();

  const [firstSvg] = await Promise.all([
    page.waitForEvent("download"),
    page.getByTestId("download-svg").click(),
  ]);
  const [firstPng] = await Promise.all([
    page.waitForEvent("download"),
    page.getByTestId("download-png").click(),
  ]);
  const firstSvgBytes = await readFile(await firstSvg.path());
  const firstPngBytes = await readFile(await firstPng.path());

  const [secondSvg] = await Promise.all([
    page.waitForEvent("download"),
    page.getByTestId("download-svg").click(),
  ]);
  const [secondPng] = await Promise.all([
    page.waitForEvent("download"),
    page.getByTestId("download-png").click(),
  ]);
  const secondSvgBytes = await readFile(await secondSvg.path());
  const secondPngBytes = await readFile(await secondPng.path());

  expect(firstSvg.suggestedFilename()).toBe("qr-code.svg");
  expect(firstPng.suggestedFilename()).toBe("qr-code.png");
  expect(await sha256(firstSvgBytes)).toBe(await sha256(secondSvgBytes));
  expect(await sha256(firstPngBytes)).toBe(await sha256(secondPngBytes));
  expect(firstSvgBytes.toString("utf8")).not.toContain(payload);
  expect(firstPngBytes.readUInt32BE(16)).toBe(300);
  expect(firstPngBytes.readUInt32BE(20)).toBe(300);
});

test("downloaded PNG and SVG independently decode to the displayed encoded URL", async ({
  page,
}, testInfo) => {
  const reader = resolve("tests/oracles/zxing-cpp/build/example/ZXingReader");
  expect(existsSync(reader), "build the manifest-pinned ZXing-C++ reader").toBe(true);
  await selectProfile(page, "Small");
  await page.getByLabel("Base URL").fill("https://e.test/offer#details");
  await page.getByLabel("utm_medium").fill("QR screen");
  const payload = "https://e.test/offer?utm_medium=QR+screen#details";
  await expect(page.getByLabel("Encoded URL")).toHaveValue(payload);
  await expect(page.getByTestId("download-png")).toBeEnabled();

  const [pngDownload] = await Promise.all([
    page.waitForEvent("download"),
    page.getByTestId("download-png").click(),
  ]);
  const [svgDownload] = await Promise.all([
    page.waitForEvent("download"),
    page.getByTestId("download-svg").click(),
  ]);
  expectDecodedPayload(reader, await pngDownload.path(), payload);
  const rasterPath = testInfo.outputPath("derived-url-svg.png");
  await rasterizeDownloadedSvg(page, await svgDownload.path(), rasterPath, 300);
  expectDecodedPayload(reader, rasterPath, payload);
});
