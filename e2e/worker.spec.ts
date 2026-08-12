import { expect, test } from "@playwright/test";

import { diagnostic, selectProfile } from "./helpers";

test("large preview work leaves the main thread responsive and latest revision wins", async ({
  page,
}) => {
  await page.addInitScript(() => {
    const NativeWorker = Worker;
    window.Worker = new Proxy(NativeWorker, {
      construct(target, argumentsList) {
        window.workerConstructions = (window.workerConstructions ?? 0) + 1;
        return Reflect.construct(target, argumentsList);
      },
    });
    const originalPostMessage = NativeWorker.prototype.postMessage;
    NativeWorker.prototype.postMessage = function (message: unknown, transfer?: Transferable[]) {
      const global = window as typeof window & {
        workerDispatches?: number;
        workerAnimationFrames?: number;
      };
      global.workerDispatches = (global.workerDispatches ?? 0) + 1;
      requestAnimationFrame(() => {
        global.workerAnimationFrames = (global.workerAnimationFrames ?? 0) + 1;
      });
      return Reflect.apply(
        originalPostMessage,
        this,
        transfer === undefined ? [message] : [message, transfer],
      );
    };
  });
  await page.goto("/");
  await selectProfile(page, "Adaptive");
  const logo = page.getByRole("checkbox", { name: /ONE lettermark/ });
  if (await logo.isChecked()) {
    await page.getByText("ONE lettermark", { exact: true }).click();
  }

  const input = page.getByLabel("Text to encode");
  await input.fill("A1a".repeat(700));
  await expect.poll(() => page.evaluate(() => window.workerDispatches ?? 0)).toBeGreaterThan(0);

  await input.fill("123456789");
  await expect(input).toHaveValue("123456789");
  await expect
    .poll(() => page.evaluate(() => window.workerAnimationFrames ?? 0))
    .toBeGreaterThan(0);
  await expect.poll(() => diagnostic(page, "Mode")).toBe("Numeric");
  await expect(page.getByTestId("qr-preview")).toHaveAttribute("aria-label", /Numeric mode/);
  await input.fill("HELLOworld1234567890");
  await expect.poll(() => diagnostic(page, "Mode")).toBe("Mixed");
  await expect(page.getByTestId("download-png")).toBeEnabled();
  expect(await page.evaluate(() => window.workerConstructions)).toBe(1);
});

test("worker message failure leaves an actionable non-pending state", async ({ page }) => {
  await page.route("**/qr-preview-worker_loader.js", async (route) => {
    await route.fulfill({
      contentType: "text/javascript",
      body: "self.onmessage = () => self.postMessage({});",
    });
  });
  await page.goto("/");
  await page.getByLabel("Text to encode").fill("worker failure");

  await expect(page.getByRole("alert")).toContainText("QR generation failed unexpectedly");
  await expect(page.getByTestId("download-svg")).toBeDisabled();
  await expect(page.getByText("Updating preview…")).toHaveCount(0);
});

declare global {
  interface Window {
    workerDispatches?: number;
    workerAnimationFrames?: number;
    workerConstructions?: number;
  }
}
