import { existsSync } from "node:fs";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { spawnSync } from "node:child_process";

import { expect, test } from "@playwright/test";

import { SAFE_PAYLOAD, enterPayload, sha256 } from "./helpers";

const ZXING_COMMIT = "8dd1cf5c4fd6fb6211bb96713db926ac6f2cf825";
const ZXING_VERSION = "ZXingReader version 3.0.2";
const LONG_ONE_URL =
  "https://www.one-line.com/en/news/notice-mandatory-advance-cargo-declaration-acd-reference-number-imports-kenya";

test.beforeEach(async ({ page }) => {
  await page.goto("/");
  await enterPayload(page, SAFE_PAYLOAD);
});

test("downloads fixed filenames and exact deterministic SVG and PNG bytes", async ({ page }) => {
  await page.getByText("Inline", { exact: true }).click();
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
    "7fad56cf665cab5d92b892bfdda5e02008df7ca7f49dd2a9a7fca106fcae521e",
  );
  expect(await sha256(png)).toBe(
    "223312fd10b53d8f26500bc4c679d3d9b11105f4f12233fadb1168ab86e33fb9",
  );
});

test("downloaded PNG independently decodes with the pinned reader", async ({ page }) => {
  const decodePayload = "hello";
  await enterPayload(page, decodePayload);
  await page.getByText("Inline", { exact: true }).click();
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

test("downloads and decodes the deterministic Adaptive Branded Version 10 artifacts", async ({
  page,
}) => {
  await page.getByText("Adaptive Branded", { exact: true }).click();
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
  const svg = await readFile(await svgDownload.path());
  const png = await readFile(await pngDownload.path());
  expect(await sha256(svg)).toBe(
    "15223845dcf6fbedbc4e4144ad4592c4bc7ec8b6720a44e994055596024b3f35",
  );
  expect(await sha256(png)).toBe(
    "f7c17465e80d4697f5af14ca99ef17ae67ef51d3722a3a60b7dec2ba600e5756",
  );
  expect(png.readUInt32BE(16)).toBe(540);
  expect(png.readUInt32BE(20)).toBe(540);

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
