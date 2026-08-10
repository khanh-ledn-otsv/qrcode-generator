import { existsSync } from "node:fs";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { spawnSync } from "node:child_process";

import { expect, test } from "@playwright/test";

import { SAFE_PAYLOAD, enterPayload, selectProfile, sha256 } from "./helpers";

const ZXING_COMMIT = "8dd1cf5c4fd6fb6211bb96713db926ac6f2cf825";
const ZXING_VERSION = "ZXingReader version 3.0.2";
const LONG_ONE_URL =
  "https://www.one-line.com/en/news/notice-mandatory-advance-cargo-declaration-acd-reference-number-imports-kenya";

test.beforeEach(async ({ page }) => {
  await page.goto("/");
  await enterPayload(page, SAFE_PAYLOAD);
});

test("downloads fixed filenames and exact deterministic SVG and PNG bytes", async ({ page }) => {
  await selectProfile(page, "Inline");
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
  // These hashes pin the Inline-profile V6 branded artifacts.
  expect(await sha256(svg)).toBe(
    "f89917a017feaa36f0e5af8df5a0044688e230a6d995788c6a0ae911102070b0",
  );
  expect(await sha256(png)).toBe(
    "1476e4c8657cf9a1ea7129ad5319ed69d3d4bc3b973f8ac88e5ea32dcaff73aa",
  );
});

test("downloaded PNG independently decodes with the pinned reader", async ({ page }) => {
  const decodePayload = "hello";
  await enterPayload(page, decodePayload);
  await selectProfile(page, "Inline");
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

  const [download] = await Promise.all([
    page.waitForEvent("download"),
    page.getByTestId("download-png").click(),
  ]);
  const result = spawnSync(
    reader,
    ["-formats", "QRCode", "-single", "-bytes", await download.path()],
    { encoding: "utf8" },
  );
  expect(result.status, result.stderr).toBe(0);
  expect(result.stdout.trim()).toBe(decodePayload);
});

test("downloads and decodes the deterministic Adaptive Version 10 artifacts", async ({ page }) => {
  await selectProfile(page, "Adaptive");
  await enterPayload(page, LONG_ONE_URL);
  await expect(page.getByTestId("download-svg")).toBeEnabled();
  const [svgDownload] = await Promise.all([
    page.waitForEvent("download"),
    page.getByTestId("download-svg").click(),
  ]);
  const [pngDownload] = await Promise.all([
    page.waitForEvent("download"),
    page.getByTestId("download-png").click(),
  ]);
  const [repeatedSvgDownload] = await Promise.all([
    page.waitForEvent("download"),
    page.getByTestId("download-svg").click(),
  ]);
  const [repeatedPngDownload] = await Promise.all([
    page.waitForEvent("download"),
    page.getByTestId("download-png").click(),
  ]);
  const svg = await readFile(await svgDownload.path());
  const png = await readFile(await pngDownload.path());
  expect(await readFile(await repeatedSvgDownload.path())).toEqual(svg);
  expect(await readFile(await repeatedPngDownload.path())).toEqual(png);
  expect(svg.toString("utf8")).toContain('width="260"');
  expect(png.readUInt32BE(16)).toBe(390);
  expect(png.readUInt32BE(20)).toBe(390);

  const source = resolve("tests/oracles/zxing-cpp");
  const reader = resolve(source, "build/example/ZXingReader");
  const result = spawnSync(
    reader,
    ["-formats", "QRCode", "-single", "-bytes", await pngDownload.path()],
    { encoding: "utf8" },
  );
  expect(result.status, result.stderr).toBe(0);
  expect(result.stdout.trim()).toBe(LONG_ONE_URL);
});

test("downloads and decodes deterministic Adaptive Version 40 artifacts", async ({ page }) => {
  const payload = "a".repeat(2_331);
  await selectProfile(page, "Adaptive");
  await page.getByText("ONE lettermark", { exact: true }).click();
  await enterPayload(page, payload);

  const [svgDownload] = await Promise.all([
    page.waitForEvent("download"),
    page.getByTestId("download-svg").click(),
  ]);
  const [pngDownload] = await Promise.all([
    page.waitForEvent("download"),
    page.getByTestId("download-png").click(),
  ]);
  const [repeatedSvgDownload] = await Promise.all([
    page.waitForEvent("download"),
    page.getByTestId("download-svg").click(),
  ]);
  const [repeatedPngDownload] = await Promise.all([
    page.waitForEvent("download"),
    page.getByTestId("download-png").click(),
  ]);
  const svg = await readFile(await svgDownload.path());
  const png = await readFile(await pngDownload.path());
  expect(await readFile(await repeatedSvgDownload.path())).toEqual(svg);
  expect(await readFile(await repeatedPngDownload.path())).toEqual(png);
  expect(svg.toString("utf8")).toContain('width="740"');
  expect(png.readUInt32BE(16)).toBe(1_110);
  expect(png.readUInt32BE(20)).toBe(1_110);
  expect(await sha256(svg)).toBe(
    "1e715d8cb8a2e8e59173450babdd40a861b2c4edc072670b4e7a426860895656",
  );
  expect(await sha256(png)).toBe(
    "bc8275c8028ca3b0a400678499c87d1b5f69c0bc307ce9c64522fe47ded27a73",
  );

  const source = resolve("tests/oracles/zxing-cpp");
  const reader = resolve(source, "build/example/ZXingReader");
  const result = spawnSync(
    reader,
    ["-formats", "QRCode", "-single", "-bytes", await pngDownload.path()],
    { encoding: "utf8", maxBuffer: 16 * 1024 },
  );
  expect(result.status, result.stderr).toBe(0);
  expect(result.stdout.trim()).toBe(payload);
});
