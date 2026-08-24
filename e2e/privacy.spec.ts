import { readFile } from "node:fs/promises";

import { expect, test } from "@playwright/test";

import { SAFE_PAYLOAD, enterPayload, selectProfile } from "./helpers";

test("payload, logo, configuration, and downloads make no runtime request", async ({ page }) => {
  const consoleMessages: string[] = [];
  page.on("console", (message) => consoleMessages.push(message.text()));
  const requests: string[] = [];
  page.on("request", (request) => requests.push(request.url()));
  await page.goto("/");
  await expect
    .poll(() =>
      requests.some((requestUrl) => new URL(requestUrl).pathname === "/qr-preview-worker_bg.wasm"),
    )
    .toBe(true);
  const initialUrl = page.url();
  const initialTitle = await page.title();
  const initialHistoryLength = await page.evaluate(() => history.length);
  const bootstrapRequests = requests.splice(0);
  const localOrigin = new URL(initialUrl).origin;
  expect(
    bootstrapRequests.map((requestUrl) => {
      const request = new URL(requestUrl);
      expect(request.origin).toBe(localOrigin);
      expect(request.pathname).toMatch(
        /^(?:\/$|\/favicon\.ico$|\/one-logotype-white\.png$|\/input-[a-f0-9]+\.css$|\/qr-web-[a-f0-9]+(?:_bg)?\.(?:js|wasm)$|\/qr-preview-worker(?:_loader|_bg)?\.(?:js|wasm)$)/,
      );
      return request.pathname;
    }),
  ).not.toHaveLength(0);

  await enterPayload(page, SAFE_PAYLOAD);
  await expect(page.getByRole("checkbox", { name: /Rounded ONE modules/ })).toHaveCount(0);
  await expect(page.getByRole("checkbox", { name: /ONE logo in QR/ })).toBeChecked();
  await selectProfile(page, "Business card");
  await expect(page.getByLabel("Output variant")).toHaveValue("business-card");
  await expect(page.getByRole("group", { name: "Background treatment" })).toHaveCount(0);
  await expect(page.getByTestId("download-png")).toBeEnabled();
  const [svgDownload] = await Promise.all([
    page.waitForEvent("download"),
    page.getByTestId("download-svg").click(),
  ]);

  expect(requests).toEqual([]);
  expect(page.url()).toBe(initialUrl);
  expect(await page.title()).toBe(initialTitle);
  expect(svgDownload.suggestedFilename()).toBe("qr-code.svg");
  expect((await readFile(await svgDownload.path())).toString("utf8")).not.toContain(SAFE_PAYLOAD);
  expect(consoleMessages.join("\n")).not.toContain(SAFE_PAYLOAD);

  const metadata = await page
    .locator("*")
    .evaluateAll((elements) =>
      elements.flatMap((element) => Array.from(element.attributes, (attribute) => attribute.value)),
    );
  expect(metadata.join("\n")).not.toContain(SAFE_PAYLOAD);

  await page.getByRole("checkbox", { name: /ONE logo in QR/ }).click();
  await selectProfile(page, "Poster / Package");
  await enterPayload(page, `https://e.test/${"a".repeat(272)}`);
  await expect(page.getByRole("checkbox", { name: /ONE logo in QR/ })).not.toBeChecked();
  await expect(page.getByTestId("download-png")).toBeEnabled();
  const [pngDownload] = await Promise.all([
    page.waitForEvent("download"),
    page.getByTestId("download-png").click(),
  ]);
  expect(pngDownload.suggestedFilename()).toBe("qr-code.png");
  const png = await readFile(await pngDownload.path());
  expect(png.readUInt32BE(16)).toBe(708);
  expect(png.readUInt32BE(20)).toBe(708);
  expect(requests).toEqual([]);
  expect(
    await page.evaluate(() => ({
      local: Object.keys(localStorage),
      session: Object.keys(sessionStorage),
      historyLength: history.length,
    })),
  ).toEqual({ local: [], session: [], historyLength: initialHistoryLength });
});
