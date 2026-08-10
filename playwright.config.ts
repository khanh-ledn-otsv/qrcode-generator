import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  outputDir: "target/playwright-results",
  reporter: "line",
  retries: 0,
  use: {
    baseURL: "http://127.0.0.1:4173",
    locale: "en-US",
    trace: "retain-on-failure",
  },
  webServer: {
    command:
      'if [ "${QR_E2E_USE_EXISTING_DIST:-0}" = "1" ]; then test -f dist/index.html || { echo "dist/index.html is missing; run pnpm run build first." >&2; exit 1; }; else NO_COLOR=true trunk build --release; fi; python3 -m http.server 4173 --bind 127.0.0.1 --directory dist',
    port: 4173,
    reuseExistingServer: false,
    timeout: 120_000,
  },
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],
});
