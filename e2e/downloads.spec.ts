import { existsSync } from "node:fs";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { spawnSync } from "node:child_process";

import { expect, test } from "@playwright/test";

import { SAFE_PAYLOAD, enterPayload, sha256 } from "./helpers";

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
  expect(svg.toString("utf8")).toMatch(/^<svg /);
  expect(svg.toString("utf8")).not.toContain(SAFE_PAYLOAD);
  expect(png.subarray(0, 8)).toEqual(Buffer.from("89504e470d0a1a0a", "hex"));
  expect(await sha256(svg)).toBe(
    "271ca0e86f33cfd9c8febdd031447ba5c9088947d5aa94f65f4de064019b8080",
  );
  expect(await sha256(png)).toBe(
    "139610a415ccf86ad47d932318abd86ec7d7dbbffe267df8a12f2001b2ef505d",
  );
});

test("downloaded PNG independently decodes when the pinned reader is installed", async ({ page }) => {
  const source = resolve("tests/oracles/zxing-cpp");
  const reader = resolve(source, "build/example/ZXingReader");
  test.skip(!existsSync(reader), "requires the manifest-pinned ZXing-C++ reader");

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
  expect(result.stdout.trim()).toBe(SAFE_PAYLOAD);
});
