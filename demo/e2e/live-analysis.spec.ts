import { expect, test } from "./fixtures";

type LiveAnalysisBrowserState = {
  hidden: boolean;
  nextNode: number;
  activeNode: number;
  reads: Record<
    number,
    { time: number; frequency: number; timeReadAt: number[] }
  >;
};

test("live Analysis follows Analyser and consumer lifetimes", async ({
  openRoute,
  page,
}) => {
  await page.addInitScript(() => {
    const browser = globalThis as typeof globalThis & {
      liveAnalysisState?: LiveAnalysisBrowserState;
    };
    const state: LiveAnalysisBrowserState = {
      hidden: false,
      nextNode: 0,
      activeNode: 0,
      reads: {},
    };
    browser.liveAnalysisState = state;
    Object.defineProperty(document, "hidden", {
      configurable: true,
      get: () => state.hidden,
    });

    const createAnalyser = AudioContext.prototype.createAnalyser;
    AudioContext.prototype.createAnalyser = function () {
      const node = createAnalyser.call(this);
      const id = ++state.nextNode;
      state.activeNode = id;
      state.reads[id] = { time: 0, frequency: 0, timeReadAt: [] };
      Object.defineProperty(node, "getByteTimeDomainData", {
        value: (values: Uint8Array) => {
          const reads = ++state.reads[id].time;
          state.reads[id].timeReadAt.push(performance.now());
          values.fill(Math.min(255, 128 + id * 16 + (reads % 8)));
        },
      });
      Object.defineProperty(node, "getByteFrequencyData", {
        value: (values: Uint8Array) => {
          const reads = ++state.reads[id].frequency;
          values.fill(Math.min(255, id * 32 + (reads % 8)));
        },
      });
      return node;
    };
  });

  await openRoute("/visualizers", "Render live audio analysis");
  const section = page
    .getByRole("heading", { level: 2, name: "Reactive Analysis snapshots" })
    .locator("..");
  const demo = section.getByRole("region", { name: "Example demo" });
  const primary = demo.getByRole("group", {
    name: "Primary Analysis consumer",
  });
  const secondary = demo.getByRole("group", {
    name: "Secondary Analysis consumer",
  });

  await demo.getByRole("button", { name: "Start primary Recording" }).click();
  await expect(primary.getByText("Analysis available")).toBeVisible();
  await expect(secondary.getByText("Analysis available")).toBeVisible();
  await expect(primary.getByText(/Sample rate: \d+ Hz/)).toBeVisible();
  await expect(primary.getByText("FFT size: 256")).toBeVisible();
  await expect(primary.getByText("Frequency bins: 128")).toBeVisible();
  await expect(primary.getByText(/Bin width: \d+(\.\d+)? Hz/)).toBeVisible();
  await expect(primary.getByText("Decibel range: -100 to -30 dB")).toBeVisible();
  await expect(primary.getByText("Smoothing: 0.8")).toBeVisible();
  await expect(primary.getByText(/RMS level: \d+\.\d{3}/)).toBeVisible();
  await expect(primary.locator("[aria-live], [role=status]")).toHaveCount(0);
  const firstNode = await activeNode(page);

  await demo
    .getByRole("button", { name: "Start replacement Recording" })
    .click();
  await expect(demo.getByText("Replacement Analyser available")).toBeVisible();
  const replacementNode = await activeNode(page);
  expect(replacementNode).toBeGreaterThan(firstNode);
  expect(await timeSample(primary)).toBeLessThan(0.2);
  await demo.getByRole("button", { name: "Use replacement Analyser" }).click();
  await expect.poll(() => timeSample(primary)).toBeGreaterThan(0.2);
  const oldReads = await readCount(page, firstNode);
  await page.waitForTimeout(300);
  expect(await readCount(page, firstNode)).toBe(oldReads);

  await demo
    .getByRole("button", { name: "Cancel replacement Recording" })
    .click();
  await expect(primary.getByText("Analysis unavailable")).toBeVisible();
  await expect(secondary.getByText("Analysis unavailable")).toBeVisible();

  await demo
    .getByRole("button", { name: "Start replacement Recording" })
    .click();
  await expect(primary.getByText("Analysis available")).toBeVisible();
  const currentNode = await activeNode(page);
  expect(currentNode).toBeGreaterThan(replacementNode);

  await demo
    .getByRole("button", { name: "Unmount primary Analysis consumer" })
    .click();
  await expect(primary).toHaveCount(0);
  const oneConsumerReads = await readCount(page, currentNode);
  await expect
    .poll(() => readCount(page, currentNode))
    .toBeGreaterThan(oneConsumerReads);
  await expect(secondary.getByText("Analysis available")).toBeVisible();
  const cadenceStart = await timeReadCount(page, currentNode);
  await page.waitForTimeout(600);
  const cadenceTimes = await timeReadTimes(page, currentNode, cadenceStart);
  expect(cadenceTimes.length).toBeGreaterThanOrEqual(6);
  const cadenceIntervals = cadenceTimes
    .slice(1)
    .map((time, index) => time - cadenceTimes[index]);
  const averageCadence =
    cadenceIntervals.reduce((sum, interval) => sum + interval, 0) /
    cadenceIntervals.length;
  expect(averageCadence).toBeGreaterThan(65);
  expect(averageCadence).toBeLessThan(110);

  await page.evaluate(() => {
    const browser = globalThis as typeof globalThis & {
      liveAnalysisState?: LiveAnalysisBrowserState;
    };
    browser.liveAnalysisState!.hidden = true;
  });
  await page.waitForTimeout(350);
  const hiddenReads = await readCount(page, currentNode);
  await page.waitForTimeout(350);
  expect(await readCount(page, currentNode)).toBe(hiddenReads);

  await page.evaluate(() => {
    const browser = globalThis as typeof globalThis & {
      liveAnalysisState?: LiveAnalysisBrowserState;
    };
    browser.liveAnalysisState!.hidden = false;
  });
  await expect
    .poll(() => readCount(page, currentNode))
    .toBeGreaterThan(hiddenReads);

  await demo
    .getByRole("button", { name: "Unmount secondary Analysis consumer" })
    .click();
  await expect(secondary).toHaveCount(0);
  await page.waitForTimeout(150);
  const unmountedReads = await readCount(page, currentNode);
  await page.waitForTimeout(300);
  expect(await readCount(page, currentNode)).toBe(unmountedReads);
});

async function activeNode(page: import("@playwright/test").Page) {
  return page.evaluate(() => {
    const browser = globalThis as typeof globalThis & {
      liveAnalysisState?: LiveAnalysisBrowserState;
    };
    return browser.liveAnalysisState!.activeNode;
  });
}

async function readCount(
  page: import("@playwright/test").Page,
  node: number,
) {
  return page.evaluate((id) => {
    const browser = globalThis as typeof globalThis & {
      liveAnalysisState?: LiveAnalysisBrowserState;
    };
    const reads = browser.liveAnalysisState!.reads[id];
    return reads.time + reads.frequency;
  }, node);
}

async function timeSample(locator: import("@playwright/test").Locator) {
  const text = await locator.getByText(/Time sample:/).textContent();
  return Number(text?.split(":").at(-1));
}

async function timeReadCount(
  page: import("@playwright/test").Page,
  node: number,
) {
  return page.evaluate((id) => {
    const browser = globalThis as typeof globalThis & {
      liveAnalysisState?: LiveAnalysisBrowserState;
    };
    return browser.liveAnalysisState!.reads[id].timeReadAt.length;
  }, node);
}

async function timeReadTimes(
  page: import("@playwright/test").Page,
  node: number,
  start: number,
) {
  return page.evaluate(
    ({ id, offset }) => {
      const browser = globalThis as typeof globalThis & {
        liveAnalysisState?: LiveAnalysisBrowserState;
      };
      return browser.liveAnalysisState!.reads[id].timeReadAt.slice(offset);
    },
    { id: node, offset: start },
  );
}
