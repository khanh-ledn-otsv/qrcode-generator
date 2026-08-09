import { existsSync } from "node:fs";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { spawnSync } from "node:child_process";

import { expect, test } from "@playwright/test";

import { SAFE_PAYLOAD, enterPayload, sha256 } from "./helpers";

const ZXING_COMMIT = "8dd1cf5c4fd6fb6211bb96713db926ac6f2cf825";
const ZXING_VERSION = "ZXingReader version 3.0.2";

test.beforeEach(async ({ page }) => {
  await page.goto("/");
  await enterPayload(page, SAFE_PAYLOAD);
});

test("downloads fixed filenames and exact deterministic SVG and PNG bytes", async ({ page }) => {
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
  expect(svgText).toMatch(/\bwidth="120"/);
  expect(svgText).toMatch(/\bheight="120"/);
  expect(svgText).not.toContain(SAFE_PAYLOAD);
  expect(png.subarray(0, 8)).toEqual(Buffer.from("89504e470d0a1a0a", "hex"));
  expect(png.readUInt32BE(16)).toBe(360);
  expect(png.readUInt32BE(20)).toBe(360);
  // These hashes pin the default Content-profile V6 branded artifacts.
  expect(await sha256(svg)).toBe(
    "a100700e43bc6eb1881c7a2cdb94a0c370845b0dc79b167c23ae480cd3977405",
  );
  expect(await sha256(png)).toBe(
    "0a04d1af3deb289c5c7cba419e2813e0550a2bcc9be5b9aa5f46982d2c7652bf",
  );
});

test("downloaded PNG independently decodes with the pinned reader", async ({ page }) => {
  const decodePayload = "hello";
  await enterPayload(page, decodePayload);
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
