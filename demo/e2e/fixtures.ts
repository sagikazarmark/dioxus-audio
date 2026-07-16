import {
  expect,
  test as base,
  type ConsoleMessage,
  type Page,
  type TestInfo,
} from "@playwright/test";

type DemoFixtures = {
  openRoute: (path: string, heading: string) => Promise<void>;
};

function captureBrowserLogs(
  page: Page,
  testInfo: TestInfo,
): () => Promise<void> {
  const logs: string[] = [];
  const recordConsole = (message: ConsoleMessage) => {
    logs.push(`console.${message.type()}: ${message.text()}`);
  };

  page.on("console", recordConsole);
  page.on("pageerror", (error) =>
    logs.push(`pageerror: ${error.stack ?? error.message}`),
  );
  page.on("requestfailed", (request) => {
    logs.push(
      `requestfailed: ${request.method()} ${request.url()} ${request.failure()?.errorText ?? ""}`,
    );
  });

  return async () => {
    if (testInfo.status !== testInfo.expectedStatus && logs.length > 0) {
      await testInfo.attach("browser.log", {
        body: `${logs.join("\n")}\n`,
        contentType: "text/plain",
      });
    }
  };
}

export const test = base.extend<DemoFixtures>({
  page: async ({ page }, use, testInfo) => {
    const attachLogsOnFailure = captureBrowserLogs(page, testInfo);
    await use(page);
    await attachLogsOnFailure();
  },
  openRoute: async ({ page }, use) => {
    await use(async (path, heading) => {
      const response = await page.goto(path);
      expect(response?.status()).toBe(200);
      await expect(
        page.getByRole("heading", { level: 1, name: heading }),
      ).toBeVisible();
    });
  },
});

export { expect } from "@playwright/test";
