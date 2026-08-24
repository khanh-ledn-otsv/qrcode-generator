import { existsSync } from "node:fs";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { spawnSync } from "node:child_process";

import { expect, test, type Page } from "@playwright/test";

import { SAFE_PAYLOAD, diagnostic, enterPayload, selectProfile, sha256 } from "./helpers";
import { expectDecodedPayload, expectDecodedPayloadBytes } from "./zxing";

const ZXING_COMMIT = "8dd1cf5c4fd6fb6211bb96713db926ac6f2cf825";
const ZXING_VERSION = "ZXingReader version 3.0.2";
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
      image.style.zIndex = "2147483647";
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

async function downloadAndDecodeArtifacts(
  page: Page,
  reader: string,
  payload: string,
  svgRasterPath: string,
  svgRasterSidePixels: number,
): Promise<void> {
  const [pngDownload] = await Promise.all([
    page.waitForEvent("download"),
    page.getByTestId("download-png").click(),
  ]);
  const [svgDownload] = await Promise.all([
    page.waitForEvent("download"),
    page.getByTestId("download-svg").click(),
  ]);
  expectDecodedPayload(reader, await pngDownload.path(), payload);
  await rasterizeDownloadedSvg(page, await svgDownload.path(), svgRasterPath, svgRasterSidePixels);
  expectDecodedPayload(reader, svgRasterPath, payload);
}

test.beforeEach(async ({ page }) => {
  await page.goto("/");
  await enterPayload(page, SAFE_PAYLOAD);
});

test("downloads fixed filenames and exact deterministic SVG and PNG bytes", async ({ page }) => {
  await test.step("decoder comparison preserves exact boundary bytes", () => {
    const payload = "  line one\nline two\n";
    expectDecodedPayloadBytes(Buffer.from(payload, "utf8"), payload);
    expect(() => expectDecodedPayloadBytes(Buffer.from(" padded "), "padded ")).toThrow();
    expect(() => expectDecodedPayloadBytes(Buffer.from("padded \n"), "padded ")).toThrow();
  });

  await selectProfile(page, "Small");
  const [svgDownload] = await Promise.all([
    page.waitForEvent("download"),
    page.getByTestId("download-svg").click(),
  ]);
  const [pngDownload] = await Promise.all([
    page.waitForEvent("download"),
    page.getByTestId("download-png").click(),
  ]);
  expect(svgDownload.suggestedFilename()).toBe("qr-code.svg");
  expect(pngDownload.suggestedFilename()).toBe("qr-code.png");

  const svg = await readFile(await svgDownload.path());
  const png = await readFile(await pngDownload.path());
  const svgText = svg.toString("utf8");
  expect(svgText).toMatch(/^<svg /);
  expect(svgText).toMatch(/\bwidth="100"/);
  expect(svgText).toMatch(/\bheight="100"/);
  expect(svgText).not.toContain(SAFE_PAYLOAD);
  expect(png.subarray(0, 8)).toEqual(Buffer.from("89504e470d0a1a0a", "hex"));
  expect(png.readUInt32BE(16)).toBe(300);
  expect(png.readUInt32BE(20)).toBe(300);
  // These hashes pin the default branded rounded-module Small artifacts.
  expect(await sha256(svg)).toBe(
    "42e06fc03b3961344d1ac890de93a47c63fa0b020a742be600a14ce59d966598",
  );
  expect(await sha256(png)).toBe(
    "86cfac3bcf061f0b7c744e3abb5918134153e9a70ef368c669eae86bd0492efc",
  );
});

test("downloads deterministic black branded SVG and PNG artifacts", async ({ page }, testInfo) => {
  await selectProfile(page, "Standard");
  await page.getByText("Black", { exact: true }).click();
  await expect.poll(() => diagnostic(page, "Foreground")).toBe("Black #000000");

  const [svgDownload] = await Promise.all([
    page.waitForEvent("download"),
    page.getByTestId("download-svg").click(),
  ]);
  const [pngDownload] = await Promise.all([
    page.waitForEvent("download"),
    page.getByTestId("download-png").click(),
  ]);
  const svg = await readFile(await svgDownload.path());
  const png = await readFile(await pngDownload.path());
  const svgText = svg.toString("utf8");

  expect(svgText).toContain('fill="#000000"');
  expect(svgText).not.toContain("#bd0f72");
  expect(png.readUInt32BE(16)).toBe(360);
  expect(png.readUInt32BE(20)).toBe(360);
  expect(await sha256(svg)).toBe(
    "646fb42929a4d23b53fb3dff90dfd65d7f67ce4fd0aaf69421302c5393da58e8",
  );
  expect(await sha256(png)).toBe(
    "48a353b79bbe8bd16e8b52ab485148f21593b47ba30239f371672d0540266ba4",
  );

  const source = resolve("tests/oracles/zxing-cpp");
  const reader = resolve(source, "build/example/ZXingReader");
  const svgRaster = testInfo.outputPath("black-branded-svg.png");
  expectDecodedPayload(reader, await pngDownload.path(), SAFE_PAYLOAD);
  await rasterizeDownloadedSvg(page, await svgDownload.path(), svgRaster, 360);
  expectDecodedPayload(reader, svgRaster, SAFE_PAYLOAD);
});

test("downloaded PNGs and SVGs independently decode common payload formats", async ({
  page,
}, testInfo) => {
  const source = resolve("tests/oracles/zxing-cpp");
  const reader = resolve(source, "build/example/ZXingReader");
  expect(
    existsSync(reader),
    "build the manifest-pinned ZXing-C++ reader documented in tests/oracles/README.md",
  ).toBe(true);

  const commit = spawnSync("git", ["-C", source, "rev-parse", "HEAD"], {
    encoding: "utf8",
  });
  expect(commit.status, commit.stderr).toBe(0);
  expect(commit.stdout.trim()).toBe(ZXING_COMMIT);
  const version = spawnSync(reader, ["-version"], { encoding: "utf8" });
  expect(version.status, version.stderr).toBe(0);
  expect(version.stdout.trim()).toBe(ZXING_VERSION);
  await selectProfile(page, "Small");

  const cases = [
    { name: "short number", payload: "1234567890" },
    { name: "number with leading zeroes", payload: "00012345678901234567890" },
    {
      name: "long number",
      payload: "31415926535897932384626433832795028841971693993751",
    },
    { name: "alphanumeric text", payload: "MEET AT GATE 7 AT 09:30" },
    { name: "plain text", payload: "Bring ID, badge, and ticket #42." },
    { name: "text with boundary spaces", payload: "  keep these spaces  " },
    { name: "mixed encoding text", payload: "HELLOworld1234567890" },
    { name: "UTF-8 text", payload: "café 世界 🚀" },
    {
      name: "markup-like text",
      payload: 'safe/<script>alert("payload")</script>',
    },
    { name: "short HTTPS link", payload: "https://example.test/a" },
    {
      name: "HTTPS link with query and fragment",
      payload: "https://example.test/a?x=1&y=two#details",
    },
    {
      name: "HTTPS link with encoded characters",
      payload: "https://example.test/a%20b?next=%2Fhome",
    },
    {
      name: "email link",
      payload: "mailto:support@example.test?subject=QR%20code",
    },
  ] as const;

  // Each case traverses the UI, preview worker, download adapter, and independent decoder.
  /* oxlint-disable no-await-in-loop */
  for (const [index, scenario] of cases.entries()) {
    await test.step(scenario.name, async () => {
      await enterPayload(page, scenario.payload);
      const svgRaster = testInfo.outputPath(`common-payload-${index}-svg.png`);
      await downloadAndDecodeArtifacts(page, reader, scenario.payload, svgRaster, 300);
    });
  }
  /* oxlint-enable no-await-in-loop */
});

test("downloads and independently decodes multiline text", async ({ page }, testInfo) => {
  await selectProfile(page, "Small");
  const payload = "line one\nline two\n";
  const input = page.getByLabel("Text to encode");
  await input.fill("line one");
  await input.press("Enter");
  await input.pressSequentially("line two");
  await input.press("Enter");
  await expect(input).toHaveValue(payload);
  await expect(page.getByTestId("download-svg")).toBeEnabled();

  const source = resolve("tests/oracles/zxing-cpp");
  const reader = resolve(source, "build/example/ZXingReader");
  const svgRaster = testInfo.outputPath("multiline-svg.png");
  await downloadAndDecodeArtifacts(page, reader, payload, svgRaster, 300);
});
