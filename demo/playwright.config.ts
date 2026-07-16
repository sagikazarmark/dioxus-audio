import { defineConfig, devices } from "@playwright/test";

const baseURL = process.env.PLAYWRIGHT_BASE_URL ?? "http://127.0.0.1:8080";

export default defineConfig({
  testDir: "./e2e",
  outputDir: "./build/playwright/test-results",
  fullyParallel: false,
  retries: 0,
  workers: 1,
  reporter: [
    ["line"],
    ["html", { open: "never", outputFolder: "./build/playwright/report" }],
  ],
  timeout: 45_000,
  expect: {
    timeout: 30_000,
  },
  use: {
    baseURL,
    permissions: ["microphone"],
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
    video: "retain-on-failure",
    launchOptions: {
      args: [
        "--use-fake-device-for-media-stream",
        "--use-fake-ui-for-media-stream",
        "--autoplay-policy=no-user-gesture-required",
        `--unsafely-treat-insecure-origin-as-secure=${new URL(baseURL).origin}`,
      ],
    },
  },
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"], channel: "chromium" },
    },
  ],
});
