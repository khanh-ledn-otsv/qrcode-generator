import { readFile } from "node:fs/promises";

import { expect, test } from "@playwright/test";

import { SAFE_PAYLOAD, enterPayload, selectProfile } from "./helpers";

test("payload, logo, configuration, and downloads make no runtime request", async ({ page }) => {
  const consoleMessages: string[] = [];
  page.on("console", (message) => consoleMessages.push(message.text()));
  const requests: string[] = [];
  page.on("request", (request) => requests.push(request.url()));
  await page.goto("/");
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
        /^(?:\/$|\/favicon\.ico$|\/input-[a-f0-9]+\.css$|\/qr-web-[a-f0-9]+(?:_bg)?\.(?:js|wasm)$)/,
      );
      return request.pathname;
    }),
  ).not.toHaveLength(0);

  await enterPayload(page, SAFE_PAYLOAD);
  await page.getByText("ONE lettermark", { exact: true }).click();
  await expect(page.getByRole("checkbox", { name: /ONE lettermark/ })).not.toBeChecked();
  await selectProfile(page, "Print");
  await page.getByText("Transparent", { exact: true }).click();
  await expect(page.getByRole("radio", { name: /Print/ })).toBeChecked();
  await expect(page.getByRole("radio", { name: /Transparent/ })).toBeChecked();
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

  await page.getByText("Opaque white", { exact: true }).click();
  await selectProfile(page, "Adaptive");
  await enterPayload(page, "a".repeat(2_331));
  await expect(page.getByRole("checkbox", { name: /ONE lettermark/ })).not.toBeChecked();
  await expect(page.getByTestId("download-png")).toBeEnabled();
  const [pngDownload] = await Promise.all([
    page.waitForEvent("download"),
    page.getByTestId("download-png").click(),
  ]);
  expect(pngDownload.suggestedFilename()).toBe("qr-code.png");
  const png = await readFile(await pngDownload.path());
  expect(png.readUInt32BE(16)).toBe(1_110);
  expect(png.readUInt32BE(20)).toBe(1_110);
  expect(requests).toEqual([]);
  expect(
    await page.evaluate(() => ({
      local: Object.keys(localStorage),
      session: Object.keys(sessionStorage),
      historyLength: history.length,
    })),
  ).toEqual({ local: [], session: [], historyLength: initialHistoryLength });
});
