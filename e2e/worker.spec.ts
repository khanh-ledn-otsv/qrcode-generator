import { readFile } from "node:fs/promises";

import { expect, test } from "@playwright/test";

import { SAFE_PAYLOAD, diagnostic, selectProfile, sha256 } from "./helpers";

test("large preview work leaves the main thread responsive and latest revision wins", async ({
  page,
}) => {
  await page.addInitScript(() => {
    const NativeWorker = Worker;
    window.Worker = new Proxy(NativeWorker, {
      construct(target, argumentsList) {
        window.workerConstructions = (window.workerConstructions ?? 0) + 1;
        const worker = Reflect.construct(target, argumentsList);
        let messageHandler: ((event: MessageEvent) => void) | null = null;
        let firstResponse = true;
        worker.addEventListener("message", (event) => {
          const deliver = () => messageHandler?.call(worker, event);
          if (event.data?.startedRevision !== undefined) {
            window.workerWorkStarted = true;
            const input = document.querySelector<HTMLTextAreaElement>("#qr-payload");
            if (input) {
              input.value = "123456789";
              window.workerResponsiveInput = input.value === "123456789";
              input.focus();
            }
            window.workerResponsiveActionAt = Date.now();
            window.workerResponsiveFocus = document.activeElement === input;
            requestAnimationFrame(() => {
              window.workerAnimationFrames = (window.workerAnimationFrames ?? 0) + 1;
              window.workerAnimationFrameAt = Date.now();
            });
            deliver();
            return;
          }
          if (event.data?.releasedRevision !== undefined) {
            window.workerReleasedBuffers = (window.workerReleasedBuffers ?? 0) + 1;
            deliver();
            return;
          }
          window.workerCompletedAt = event.data?.completedAt;
          window.workerWorkCompleted = true;
          if (firstResponse) {
            firstResponse = false;
            window.workerResponseHeld = true;
            setTimeout(() => {
              window.workerResponseHeld = false;
              window.workerDelayedResponses = (window.workerDelayedResponses ?? 0) + 1;
              deliver();
            }, 750);
          } else {
            deliver();
          }
        });
        Object.defineProperty(worker, "onmessage", {
          configurable: true,
          get: () => messageHandler,
          set: (handler) => {
            messageHandler = handler;
          },
        });
        return worker;
      },
    });
    const originalPostMessage = NativeWorker.prototype.postMessage;
    NativeWorker.prototype.postMessage = function (message: unknown, transfer?: Transferable[]) {
      const global = window as typeof window & {
        workerDispatches?: number;
        workerAnimationFrames?: number;
      };
      global.workerDispatches = (global.workerDispatches ?? 0) + 1;
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
  await expect.poll(() => page.evaluate(() => window.workerWorkStarted ?? false)).toBe(true);

  await expect.poll(() => page.evaluate(() => window.workerResponsiveFocus)).toBe(true);
  expect(await page.evaluate(() => window.workerResponsiveInput)).toBe(true);
  await input.fill("");
  await input.fill("123456789");
  await expect
    .poll(() => page.evaluate(() => window.workerAnimationFrames ?? 0))
    .toBeGreaterThan(0);
  await expect.poll(() => diagnostic(page, "Mode")).toBe("Numeric");
  await expect(page.getByTestId("qr-preview")).toHaveAttribute("aria-label", /Numeric mode/);
  await expect
    .poll(() => page.evaluate(() => window.workerDelayedResponses ?? 0))
    .toBeGreaterThan(0);
  expect(
    await page.evaluate(
      () => (window.workerResponsiveActionAt ?? Infinity) < (window.workerCompletedAt ?? 0),
    ),
  ).toBe(true);
  expect(
    await page.evaluate(
      () => (window.workerAnimationFrameAt ?? Infinity) < (window.workerCompletedAt ?? 0),
    ),
  ).toBe(true);
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
      body: `self.postMessage({ workerReady: true });
self.onmessage = (event) => {
  const { revision } = JSON.parse(event.data);
  self.postMessage({ revision });
};`,
    });
  });
  await page.goto("/");
  await page.getByLabel("Text to encode").fill("worker failure");

  await expect(page.getByRole("alert")).toContainText("QR generation failed unexpectedly");
  await expect(page.getByTestId("download-svg")).toBeDisabled();
  await expect(page.getByText("Updating preview…")).toHaveCount(0);
});

test("a stale malformed response cannot reject newer pending work", async ({ page }) => {
  await page.route("**/qr-preview-worker_loader.js", async (route) => {
    await route.fulfill({
      contentType: "text/javascript",
      body: `self.postMessage({ workerReady: true });
let requestCount = 0;
self.onmessage = (event) => {
  requestCount += 1;
  const { revision } = JSON.parse(event.data);
  if (requestCount === 1) {
    setTimeout(() => self.postMessage({
      metadata: JSON.stringify({ revision, result: { status: "Ready", value: {} } }),
      png: new Uint8Array(),
    }), 600);
  }
};`,
    });
  });
  await page.goto("/");
  const input = page.getByLabel("Text to encode");
  await input.fill("older request");
  await page.waitForTimeout(350);
  await input.fill("newer request");

  await page.waitForTimeout(750);
  await expect(page.getByText("Updating preview…")).toBeVisible();
  await expect(page.getByRole("alert")).toHaveText("");
  await expect(page.getByTestId("download-svg")).toBeDisabled();
});

test("a malformed worker is replaced and a later edit succeeds", async ({ page }) => {
  await page.addInitScript(() => {
    const NativeWorker = Worker;
    window.Worker = new Proxy(NativeWorker, {
      construct(target, argumentsList) {
        window.workerConstructions = (window.workerConstructions ?? 0) + 1;
        if ((window.workerConstructions ?? 0) > 1) {
          argumentsList[0] = `${String(argumentsList[0])}?recovery=${window.workerConstructions}`;
        }
        const worker = Reflect.construct(target, argumentsList);
        worker.addEventListener("message", (event) => {
          if (event.data?.metadata !== undefined) {
            window.workerValidResponses = (window.workerValidResponses ?? 0) + 1;
          }
        });
        return worker;
      },
    });
  });
  let workerLoads = 0;
  await page.route("**/qr-preview-worker_loader.js", async (route) => {
    workerLoads += 1;
    if (workerLoads === 1) {
      await route.fulfill({
        contentType: "text/javascript",
        body: `self.postMessage({ workerReady: true });
self.onmessage = (event) => {
  const { revision } = JSON.parse(event.data);
  self.postMessage({ revision });
};`,
      });
    } else {
      await route.fallback();
    }
  });
  await page.goto("/");
  await page.getByText("ONE lettermark", { exact: true }).click();
  const input = page.getByLabel("Text to encode");
  await input.fill("malformed worker");
  await expect(page.getByRole("alert")).toContainText("QR generation failed unexpectedly");

  await input.fill("replacement worker");
  await expect
    .poll(() => page.evaluate(() => window.workerConstructions ?? 0))
    .toBeGreaterThanOrEqual(2);
  await expect.poll(() => page.evaluate(() => window.workerValidResponses ?? 0)).toBeGreaterThan(0);
  await expect(page.getByTestId("download-svg")).toBeEnabled();
  expect(workerLoads).toBe(1);
});

test("actual worker bytes match native UTF-8 and mixed-mode artifacts", async ({ page }) => {
  await page.goto("/");
  const logo = page.getByRole("checkbox", { name: /ONE lettermark/ });
  if (await logo.isChecked()) {
    await page.getByText("ONE lettermark", { exact: true }).click();
  }

  const cases = [
    {
      profile: "Content",
      payload: "café 世界",
      logo: false,
      svg: "bea37d9a62d77d2e69f0b8f6b4adec99d0628d4d625393c4eed5a4af341f08c8",
      png: "b2e60cbd647e80f293e7bbfb9c91506abd756f2566efd2f96aeb2f8f48da6536",
    },
    {
      profile: "Adaptive",
      payload: "HELLOworld1234567890",
      logo: false,
      svg: "a5c853e91140f592bf0246ebcb3e4efaaaa6ab837eb9aacf8fe4b731435f7f2f",
      png: "01059af352d601496d10fe6e3cfc0ff09402f0c6db32dbf5d91c4fca942ea219",
    },
    {
      profile: "Inline",
      payload: SAFE_PAYLOAD,
      logo: true,
      svg: "42e06fc03b3961344d1ac890de93a47c63fa0b020a742be600a14ce59d966598",
      png: "86cfac3bcf061f0b7c744e3abb5918134153e9a70ef368c669eae86bd0492efc",
    },
    {
      profile: "Adaptive",
      payload: "a".repeat(2_331),
      logo: false,
      svg: "13f93d47f419c88c6ae23167d346ded1896dab987488ecfe4582f5f21f7573f5",
      png: "08a579af8d94164c6be39e9c4e8be4f49c55af702849bd8c0afdf0cd6469023a",
    },
  ];

  // Each case mutates the same UI and must await its own worker result and downloads.
  /* oxlint-disable no-await-in-loop */
  for (const artifact of cases) {
    await selectProfile(page, artifact.profile);
    if ((await logo.isChecked()) !== artifact.logo) {
      await page.getByText("ONE lettermark", { exact: true }).click();
    }
    await page.getByLabel("Text to encode").fill(artifact.payload);
    await expect(page.getByTestId("download-svg")).toBeEnabled();
    const [svgDownload] = await Promise.all([
      page.waitForEvent("download"),
      page.getByTestId("download-svg").click(),
    ]);
    const [pngDownload] = await Promise.all([
      page.waitForEvent("download"),
      page.getByTestId("download-png").click(),
    ]);
    expect(await sha256(await readFile(await svgDownload.path()))).toBe(artifact.svg);
    expect(await sha256(await readFile(await pngDownload.path()))).toBe(artifact.png);
  }
  /* oxlint-enable no-await-in-loop */
});

test("repeated generations reuse one worker and page disposal terminates it", async ({ page }) => {
  await page.addInitScript(() => {
    const NativeWorker = Worker;
    window.Worker = new Proxy(NativeWorker, {
      construct(target, argumentsList) {
        window.workerConstructions = (window.workerConstructions ?? 0) + 1;
        const worker = Reflect.construct(target, argumentsList);
        worker.addEventListener("message", (event) => {
          if (event.data?.releasedRevision !== undefined) {
            window.workerReleasedBuffers = (window.workerReleasedBuffers ?? 0) + 1;
          }
        });
        return worker;
      },
    });
    const nativeTerminate = NativeWorker.prototype.terminate;
    NativeWorker.prototype.terminate = function () {
      window.workerTerminations = (window.workerTerminations ?? 0) + 1;
      return Reflect.apply(nativeTerminate, this, []);
    };
  });
  await page.goto("/");
  await selectProfile(page, "Adaptive");
  const logo = page.getByRole("checkbox", { name: /ONE lettermark/ });
  if (await logo.isChecked()) {
    await page.getByText("ONE lettermark", { exact: true }).click();
  }
  const input = page.getByLabel("Text to encode");
  // Sequential completion is the behavior under test: each transferred buffer must be released.
  /* oxlint-disable no-await-in-loop */
  for (let index = 1; index <= 12; index += 1) {
    const payload = `generation-${index}-${"x".repeat(index * 20)}`;
    await input.fill(payload);
    await expect(page.getByTestId("download-png")).toBeEnabled();
  }
  /* oxlint-enable no-await-in-loop */
  expect(await page.evaluate(() => window.workerConstructions)).toBe(1);
  await expect
    .poll(() => page.evaluate(() => window.workerReleasedBuffers ?? 0))
    .toBeGreaterThanOrEqual(12);

  await page.evaluate(() => window.dispatchEvent(new PageTransitionEvent("pagehide")));
  await expect.poll(() => page.evaluate(() => window.workerTerminations ?? 0)).toBe(1);
});

declare global {
  interface Window {
    workerDispatches?: number;
    workerAnimationFrames?: number;
    workerAnimationFrameAt?: number;
    workerConstructions?: number;
    workerDelayedResponses?: number;
    workerResponseHeld?: boolean;
    workerTerminations?: number;
    workerWorkStarted?: boolean;
    workerWorkCompleted?: boolean;
    workerResponsiveActionAt?: number;
    workerResponsiveFocus?: boolean;
    workerResponsiveInput?: boolean;
    workerCompletedAt?: number;
    workerReleasedBuffers?: number;
    workerValidResponses?: number;
  }
}
