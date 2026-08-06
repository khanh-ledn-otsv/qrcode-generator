import { readFile } from "node:fs/promises";

import { expect, test } from "@playwright/test";

import { SAFE_PAYLOAD, enterPayload } from "./helpers";

test("generation, configuration, and download leak no payload or external request", async ({
  page,
}) => {
  const consoleMessages: string[] = [];
  page.on("console", (message) => consoleMessages.push(message.text()));
  await page.goto("/");
  const initialUrl = page.url();
  const initialTitle = await page.title();
  const requests: string[] = [];
  page.on("request", (request) => requests.push(request.url()));

  await enterPayload(page, SAFE_PAYLOAD);
  await page.getByText("Print", { exact: true }).click();
  await expect(page.getByRole("radio", { name: /Print/ })).toBeChecked();
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
  expect(
    await page.evaluate(() => ({
      local: Object.keys(localStorage),
      session: Object.keys(sessionStorage),
      historyLength: history.length,
    })),
  ).toEqual({ local: [], session: [], historyLength: 2 });
});
