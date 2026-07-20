import { expect, test } from "./fixtures";
import type { Page } from "@playwright/test";

type GraphBrowserState = {
  contexts: AudioContext[];
  analyserNodes: AnalyserNode[];
  gainNodes: GainNode[];
  closeCalls: number;
  disconnectCalls: number;
  detachedMedia: number;
  activationCalls: { operation: "resume" | "play"; turn: number }[];
  pauseCalls: number;
  resolveLateActivation?: () => void;
};

type GraphWindow = typeof globalThis & {
  graphBrowserState?: GraphBrowserState;
};

async function installGraphInstrumentation(page: Page) {
  await page.addInitScript(() => {
    const state: GraphBrowserState = {
      contexts: [],
      analyserNodes: [],
      gainNodes: [],
      closeCalls: 0,
      disconnectCalls: 0,
      detachedMedia: 0,
      activationCalls: [],
      pauseCalls: 0,
    };
    (window as GraphWindow).graphBrowserState = state;

    const createAnalyser = AudioContext.prototype.createAnalyser;
    AudioContext.prototype.createAnalyser = function () {
      if (!state.contexts.includes(this)) state.contexts.push(this);
      const analyser = createAnalyser.call(this);
      state.analyserNodes.push(analyser);
      return analyser;
    };

    const createGain = AudioContext.prototype.createGain;
    AudioContext.prototype.createGain = function () {
      const gain = createGain.call(this);
      state.gainNodes.push(gain);
      return gain;
    };

    const close = AudioContext.prototype.close;
    AudioContext.prototype.close = function () {
      state.closeCalls += 1;
      return close.call(this);
    };

    const disconnect = AudioNode.prototype.disconnect;
    AudioNode.prototype.disconnect = function (...args: unknown[]) {
      state.disconnectCalls += 1;
      return disconnect.apply(this, args as []);
    };

    const removeAttribute = HTMLMediaElement.prototype.removeAttribute;
    HTMLMediaElement.prototype.removeAttribute = function (name: string) {
      if (name === "src" && this.hasAttribute("src")) state.detachedMedia += 1;
      return removeAttribute.call(this, name);
    };
  });
}

test("graph-backed Playback keeps pre-gain Analysis and stable graph identity", async ({
  openRoute,
  page,
}) => {
  await installGraphInstrumentation(page);
  await openRoute("/graph-playback", "Analyse audio before effective graph gain");

  const example = page.getByRole("group", { name: "Graph-backed Playback" });
  const state = example.locator(".graph-playback-state");
  const analysis = example.locator(".graph-analysis-state");
  await expect(state).toHaveAttribute("data-graph", "awaiting-source");
  await expect(state).toHaveAttribute("data-analyser", "absent");
  expect(await browserCounts(page)).toMatchObject({ contexts: 0, analysers: 0 });

  await example.getByRole("button", { name: "Load graph tone", exact: true }).click();
  await expect(state).toHaveAttribute("data-source", "playable");
  await expect(state).toHaveAttribute("data-analyser", "present");
  expect(await browserCounts(page)).toMatchObject({ contexts: 1, analysers: 1, gains: 1 });

  await example.getByRole("button", { name: "Play graph tone", exact: true }).click();
  await expect(state).toHaveAttribute("data-graph", "running");
  await expect(state).toHaveAttribute("data-transport", "playing");
  await expect(state).toHaveAttribute("data-analyser-available", "true");
  await expect(analysis).toHaveAttribute("data-analysis", "available");
  await expect
    .poll(async () => Number(await analysis.getAttribute("data-analysis-level")))
    .toBeGreaterThan(0.02);

  await example.getByRole("button", { name: "Mute graph tone", exact: true }).click();
  await expect(state).toHaveAttribute("data-muted", "true");
  await expect.poll(() => activeGain(page)).toBe(0);
  await expect(analysis).toHaveAttribute("data-analysis", "available");
  await expect
    .poll(async () => Number(await analysis.getAttribute("data-analysis-level")))
    .toBeGreaterThan(0.02);

  await example.getByRole("slider", { name: "Graph tone audibility" }).fill("0");
  await expect(state).toHaveAttribute("data-audibility-level", "0");
  await expect.poll(() => activeGain(page)).toBe(0);
  await expect(analysis).toHaveAttribute("data-analysis", "available");
  await expect
    .poll(async () => Number(await analysis.getAttribute("data-analysis-level")))
    .toBeGreaterThan(0.02);

  await example.getByRole("button", { name: "Replace graph tone", exact: true }).click();
  await expect(state).toHaveAttribute("data-source", "playable");
  await expect(state).toHaveAttribute("data-muted", "true");
  await expect(state).toHaveAttribute("data-audibility-level", "0");
  expect(await browserCounts(page)).toMatchObject({ contexts: 1, analysers: 1, gains: 1 });

  await example.getByRole("button", { name: "Unload graph tone", exact: true }).click();
  await expect(state).toHaveAttribute("data-graph", "awaiting-source");
  await expect(state).toHaveAttribute("data-analyser", "present");
  await expect(state).toHaveAttribute("data-analyser-available", "false");
  await expect(analysis).toHaveAttribute("data-analysis", "unavailable");
  await example.getByRole("button", { name: "Check retained Analyser" }).click();
  await expect(example.locator(".retained-analyser-state")).toHaveAttribute(
    "data-available",
    "false",
  );

  await example.getByRole("button", { name: "Load graph tone", exact: true }).click();
  expect(await browserCounts(page)).toMatchObject({ contexts: 1, analysers: 1, gains: 1 });
  await example
    .getByRole("button", { name: "Unmount graph-backed Playback" })
    .click();
  await expect(example.getByText("Graph-backed Playback owner unmounted")).toBeVisible();
  await example.getByRole("button", { name: "Check retained Analyser" }).click();
  await expect(example.locator(".retained-analyser-state")).toHaveAttribute(
    "data-available",
    "false",
  );
  await expect.poll(() => browserCounts(page)).toMatchObject({
    closeCalls: 1,
    detachedMedia: 3,
  });
  expect((await browserCounts(page)).disconnectCalls).toBeGreaterThanOrEqual(5);
});

test("graph activation rejection and interruption pause media and can be retried", async ({
  openRoute,
  page,
}) => {
  const pageErrors: Error[] = [];
  page.on("pageerror", (error) => pageErrors.push(error));
  await installGraphInstrumentation(page);
  await openRoute("/graph-playback", "Analyse audio before effective graph gain");
  await page.evaluate(() => {
    const state = (window as GraphWindow).graphBrowserState!;
    const resume = AudioContext.prototype.resume;
    let rejectNextPlay = true;
    let turn = 0;
    AudioContext.prototype.resume = function () {
      if (!state.contexts.includes(this)) state.contexts.push(this);
      state.activationCalls.push({ operation: "resume", turn });
      queueMicrotask(() => (turn += 1));
      return resume.call(this);
    };
    HTMLMediaElement.prototype.play = function () {
      state.activationCalls.push({ operation: "play", turn });
      if (rejectNextPlay) {
        rejectNextPlay = false;
        return Promise.reject(
          new DOMException("Media play blocked by test", "NotAllowedError"),
        );
      }
      return Promise.resolve();
    };
    HTMLMediaElement.prototype.pause = function () {
      state.pauseCalls += 1;
    };
  });

  const example = page.getByRole("group", { name: "Graph-backed Playback" });
  const graphState = example.locator(".graph-playback-state");
  await example.getByRole("button", { name: "Load graph tone", exact: true }).click();
  await example.getByRole("button", { name: "Play graph tone", exact: true }).click();
  await expect(graphState).toHaveAttribute("data-graph", "interaction-required");
  await expect(graphState).toHaveAttribute("data-transport", "paused");
  const firstActivation = await page.evaluate(
    () => (window as GraphWindow).graphBrowserState!.activationCalls.slice(0, 2),
  );
  expect(firstActivation.map((call) => call.operation)).toEqual(["resume", "play"]);
  expect(firstActivation[0].turn).toBe(firstActivation[1].turn);
  await page.evaluate(() => {
    (window as GraphWindow).graphBrowserState!.contexts[0].dispatchEvent(
      new Event("statechange"),
    );
  });
  await expect(graphState).toHaveAttribute("data-graph", "interaction-required");

  await example.getByRole("button", { name: "Play graph tone", exact: true }).click();
  await expect(graphState).toHaveAttribute("data-graph", "running");
  await expect(graphState).toHaveAttribute("data-transport", "playing");

  await page.evaluate(async () => {
    const context = (window as GraphWindow).graphBrowserState!.contexts[0];
    await context.suspend();
  });
  await expect(graphState).toHaveAttribute("data-graph", "interaction-required");
  await expect(graphState).toHaveAttribute("data-transport", "paused");
  expect(
    await page.evaluate(() => (window as GraphWindow).graphBrowserState!.pauseCalls),
  ).toBeGreaterThanOrEqual(2);

  await page.evaluate(() => {
    const state = (window as GraphWindow).graphBrowserState!;
    AudioContext.prototype.resume = function () {
      return new Promise((resolve) => {
        state.resolveLateActivation = () => resolve();
      });
    };
  });
  await example.getByRole("button", { name: "Play graph tone", exact: true }).click();
  await expect(graphState).toHaveAttribute("data-transport", "play-pending");
  await example
    .getByRole("button", { name: "Unmount graph-backed Playback" })
    .click();
  await page.evaluate(() => {
    (window as GraphWindow).graphBrowserState!.resolveLateActivation?.();
  });
  await page.waitForTimeout(0);
  await example.getByRole("button", { name: "Check retained Analyser" }).click();
  await expect(example.locator(".retained-analyser-state")).toHaveAttribute(
    "data-available",
    "false",
  );
  expect(pageErrors).toEqual([]);
});

test("terminal graph setup failure permanently degrades the owner to direct Playback", async ({
  openRoute,
  page,
}) => {
  await installGraphInstrumentation(page);
  await openRoute("/graph-playback", "Analyse audio before effective graph gain");
  await page.evaluate(() => {
    AudioContext.prototype.createGain = function () {
      throw new DOMException("Graph setup failed by test", "NotSupportedError");
    };
    HTMLMediaElement.prototype.play = function () {
      return Promise.resolve();
    };
  });

  const example = page.getByRole("group", { name: "Graph-backed Playback" });
  const state = example.locator(".graph-playback-state");
  await example.getByRole("button", { name: "Load graph tone", exact: true }).click();
  await expect(state).toHaveAttribute("data-source", "playable");
  await expect(state).toHaveAttribute("data-graph", "unavailable");
  await expect(state).toHaveAttribute("data-analyser", "absent");
  await expect(state).toHaveAttribute(
    "data-audibility-capability",
    "besteffortmediaelement",
  );

  await example.getByRole("button", { name: "Play graph tone", exact: true }).click();
  await expect(state).toHaveAttribute("data-transport", "playing");
  await example.getByRole("button", { name: "Mute graph tone", exact: true }).click();
  await expect(state).toHaveAttribute("data-muted", "true");
  await example.getByRole("slider", { name: "Graph tone audibility" }).fill("0.35");
  await expect(state).toHaveAttribute("data-audibility-level", "0.35");

  await example.getByRole("button", { name: "Replace graph tone", exact: true }).click();
  await expect(state).toHaveAttribute("data-graph", "unavailable");
  expect(await browserCounts(page)).toMatchObject({ contexts: 1, analysers: 1 });
  await expect.poll(() => browserCounts(page)).toMatchObject({ closeCalls: 1 });

  await example
    .getByRole("button", { name: "Unmount graph-backed Playback" })
    .click();
  await page.waitForTimeout(0);
  expect((await browserCounts(page)).closeCalls).toBe(1);
});

async function browserCounts(page: Page) {
  return page.evaluate(() => {
    const state = (window as GraphWindow).graphBrowserState!;
    return {
      contexts: state.contexts.length,
      analysers: state.analyserNodes.length,
      gains: state.gainNodes.length,
      closeCalls: state.closeCalls,
      disconnectCalls: state.disconnectCalls,
      detachedMedia: state.detachedMedia,
    };
  });
}

async function activeGain(page: Page) {
  return page.evaluate(
    () => (window as GraphWindow).graphBrowserState!.gainNodes[0]?.gain.value,
  );
}
