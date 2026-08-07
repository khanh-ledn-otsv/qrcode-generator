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
      "NO_COLOR=true trunk build --release && python3 -m http.server 4173 --bind 127.0.0.1 --directory dist",
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
