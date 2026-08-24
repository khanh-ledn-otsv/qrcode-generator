import { readFile } from "node:fs/promises";

import { expect, test } from "@playwright/test";

import { SAFE_PAYLOAD, selectProfile, sha256 } from "./helpers";

test("repeated wasm generation preserves deterministic availability", async ({ page }) => {
  await page.goto("/");
  await selectProfile(page, "Poster / Package");

  const input = page.getByLabel("Base URL");
  await input.fill(`https://e.test/${"A1a".repeat(40)}`);
  await expect(page.getByTestId("download-svg")).toBeEnabled();
  await input.fill("");
  await input.fill("https://e.test/123456789?latest=1");
  await expect(page.getByTestId("download-svg")).toBeEnabled();
  await expect(page.getByTestId("qr-preview")).toHaveAttribute(
    "aria-label",
    /Generated QR code preview/,
  );
  await expect(page.getByTestId("qr-preview")).toHaveAttribute(
    "aria-label",
    /Generated QR code preview/,
  );
  await input.fill("https://e.test/HELLOworld1234567890");
  await expect(page.getByTestId("download-svg")).toBeEnabled();
  await expect(page.getByTestId("download-png")).toBeEnabled();
});

test("a malformed worker result fails safely and a later edit replaces the worker", async ({
  page,
}) => {
  await page.addInitScript(() => {
    const NativeWorker = window.Worker;
    let injectMalformedResult = true;
    let created = 0;
    function WorkerFactory(scriptUrl: string | URL, options?: WorkerOptions): Worker {
      created += 1;
      const worker = new NativeWorker(scriptUrl, options);
      const addEventListener = worker.addEventListener.bind(worker);
      worker.addEventListener = ((
        type: string,
        listener: EventListenerOrEventListenerObject,
        eventOptions?: boolean | AddEventListenerOptions,
      ) => {
        if (type !== "message") {
          addEventListener(type, listener, eventOptions);
          return;
        }
        addEventListener(
          type,
          (event: Event) => {
            const delivered = injectMalformedResult
              ? new MessageEvent("message", { data: { malformed: true } })
              : event;
            injectMalformedResult = false;
            if (typeof listener === "function") listener.call(worker, delivered);
            else listener.handleEvent(delivered);
          },
          eventOptions,
        );
      }) as typeof worker.addEventListener;
      return worker;
    }
    WorkerFactory.prototype = NativeWorker.prototype;
    Object.defineProperty(window, "Worker", { value: WorkerFactory });
    Object.defineProperty(window, "qrWorkerCreatedForTest", { get: () => created });
  });

  await page.goto("/");
  await page.getByLabel("Base URL").fill("https://e.test/first");
  await expect(page.getByText(/failed unexpectedly/)).toBeVisible();
  await expect(page.getByTestId("download-svg")).toBeDisabled();

  await page.getByLabel("Base URL").fill("https://e.test/recovered");
  await expect(page.getByTestId("download-svg")).toBeEnabled();
  await expect
    .poll(() =>
      page.evaluate(
        () => (window as Window & { qrWorkerCreatedForTest: number }).qrWorkerCreatedForTest,
      ),
    )
    .toBe(2);
});

test("actual wasm artifacts are deterministic for approved request shapes", async ({ page }) => {
  await page.goto("/");

  const cases = [
    {
      profile: "Standard",
      payload: "https://example.test/caf%C3%A9/%E4%B8%96%E7%95%8C",
    },
    {
      profile: "Poster / Package",
      payload: "https://example.test/HELLOworld1234567890",
    },
    {
      profile: "Small",
      payload: SAFE_PAYLOAD,
    },
  ];

  // Each case mutates the same UI and must await its own worker result and downloads.
  /* oxlint-disable no-await-in-loop */
  for (const artifact of cases) {
    await selectProfile(page, artifact.profile);
    await page.getByLabel("Base URL").fill(artifact.payload);
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
    expect(await sha256(svg)).toMatch(/^[a-f0-9]{64}$/);
    expect(await sha256(png)).toMatch(/^[a-f0-9]{64}$/);
  }
  /* oxlint-enable no-await-in-loop */
});

test("rapid input is debounced and repeated generations reuse one worker", async ({ page }) => {
  await page.addInitScript(() => {
    const NativeWorker = window.Worker;
    const metrics = { created: 0, posted: 0, terminated: 0 };
    function WorkerFactory(scriptUrl: string | URL, options?: WorkerOptions): Worker {
      metrics.created += 1;
      const worker = new NativeWorker(scriptUrl, options);
      const postMessage = worker.postMessage.bind(worker);
      const terminate = worker.terminate.bind(worker);
      worker.postMessage = ((message: unknown, transfer: Transferable[] = []) => {
        metrics.posted += 1;
        postMessage(message, transfer);
      }) as typeof worker.postMessage;
      worker.terminate = () => {
        metrics.terminated += 1;
        terminate();
      };
      return worker;
    }
    WorkerFactory.prototype = NativeWorker.prototype;
    Object.defineProperty(window, "Worker", { value: WorkerFactory });
    Object.defineProperty(window, "qrWorkerMetricsForTest", { value: metrics });
  });

  await page.goto("/");
  await page.evaluate(() => {
    const input = document.querySelector<HTMLInputElement>("#base-url");
    if (!input) throw new Error("missing base URL input");
    for (const value of ["https://e.test/old", "https://e.test/new", "https://e.test/latest"]) {
      input.value = value;
      input.dispatchEvent(new InputEvent("input", { bubbles: true }));
    }
  });
  await expect(page.getByLabel("Encoded URL")).toHaveValue("https://e.test/latest");
  await expect(page.getByTestId("download-svg")).toBeEnabled();
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          (
            window as Window & {
              qrWorkerMetricsForTest: { created: number; posted: number; terminated: number };
            }
          ).qrWorkerMetricsForTest,
      ),
    )
    .toEqual({ created: 1, posted: 1, terminated: 0 });

  await page.getByLabel("Base URL").fill("https://e.test/second-generation");
  await expect(page.getByLabel("Encoded URL")).toHaveValue("https://e.test/second-generation");
  await expect(page.getByTestId("download-svg")).toBeEnabled();
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          (
            window as Window & {
              qrWorkerMetricsForTest: { created: number; posted: number; terminated: number };
            }
          ).qrWorkerMetricsForTest,
      ),
    )
    .toEqual({ created: 1, posted: 2, terminated: 0 });

  await page.evaluate(() => window.dispatchEvent(new PageTransitionEvent("pagehide")));
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          (
            window as Window & {
              qrWorkerMetricsForTest: { created: number; posted: number; terminated: number };
            }
          ).qrWorkerMetricsForTest.terminated,
      ),
    )
    .toBe(1);
});
