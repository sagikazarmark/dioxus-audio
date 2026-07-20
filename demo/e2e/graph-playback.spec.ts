import { expect, test } from "./fixtures";
import type { Page } from "@playwright/test";

type GraphBrowserState = {
  contexts: AudioContext[];
  analyserNodes: AnalyserNode[];
  gainNodes: GainNode[];
  graphAttachments: {
    element: HTMLMediaElement;
    crossOrigin: string | null;
    src: string;
  }[];
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
      graphAttachments: [],
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

    const createMediaElementSource =
      AudioContext.prototype.createMediaElementSource;
    AudioContext.prototype.createMediaElementSource = function (element) {
      state.graphAttachments.push({
        element,
        crossOrigin: element.crossOrigin,
        src: element.getAttribute("src") ?? "",
      });
      return createMediaElementSource.call(this, element);
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

test("anonymous-CORS alternatives support graph Analysis, gain, replacement, and unload", async ({
  openRoute,
  page,
}) => {
  await installGraphInstrumentation(page);
  await page.route("https://media.example/allowed.wav", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "audio/wav",
      headers: { "Access-Control-Allow-Origin": "*" },
      body: toneWav(440),
    });
  });
  await openRoute("/graph-playback", "Analyse audio before effective graph gain");

  const example = page.getByRole("group", { name: "Graph-backed Playback" });
  const state = example.locator(".graph-playback-state");
  const analysis = example.locator(".graph-analysis-state");
  await example
    .getByRole("button", { name: "Load graph-ineligible alternatives", exact: true })
    .click();
  await expect(state).toHaveAttribute("data-source", "failed");
  await expect(state).toHaveAttribute("data-source-failure", "graph-ineligible");
  await expect(state).toHaveAttribute(
    "data-alternative-failures",
    "graph-ineligible,graph-ineligible",
  );
  expect((await browserCounts(page)).graphAttachments).toBe(0);

  await example
    .getByRole("button", { name: "Load anonymous-CORS alternative", exact: true })
    .click();
  await expect(state).toHaveAttribute("data-source", "playable");
  await expect(state).toHaveAttribute(
    "data-selected-alternative",
    "https://media.example/allowed.wav",
  );
  expect(
    await page.evaluate(() =>
      (window as GraphWindow).graphBrowserState!.graphAttachments.map(
        ({ crossOrigin, src }) => ({ crossOrigin, src }),
      ),
    ),
  ).toEqual([{ crossOrigin: "anonymous", src: "" }]);

  await example.getByRole("button", { name: "Play graph tone", exact: true }).click();
  await expect(state).toHaveAttribute("data-graph", "running");
  await expect(state).toHaveAttribute("data-transport", "playing");
  await expect
    .poll(async () => Number(await analysis.getAttribute("data-analysis-level")))
    .toBeGreaterThan(0.02);

  await example.getByRole("button", { name: "Mute graph tone", exact: true }).click();
  await example.getByRole("slider", { name: "Graph tone audibility" }).fill("0.35");
  await expect(state).toHaveAttribute("data-muted", "true");
  await expect(state).toHaveAttribute("data-audibility-level", "0.35");
  await expect.poll(() => activeGain(page)).toBe(0);
  await expect
    .poll(async () => Number(await analysis.getAttribute("data-analysis-level")))
    .toBeGreaterThan(0.02);

  const beforeAudioDataReplacement = await browserCounts(page);
  await example.getByRole("button", { name: "Replace graph tone", exact: true }).click();
  await expect(state).toHaveAttribute("data-source", "playable");
  await expect(state).toHaveAttribute("data-muted", "true");
  await expect(state).toHaveAttribute("data-audibility-level", "0.35");
  const afterAudioDataReplacement = await browserCounts(page);
  expect(afterAudioDataReplacement).toMatchObject({
    contexts: 1,
    analysers: 1,
    gains: 1,
  });
  expect(afterAudioDataReplacement.disconnectCalls).toBe(
    beforeAudioDataReplacement.disconnectCalls + 1,
  );
  expect(afterAudioDataReplacement.detachedMedia).toBe(
    beforeAudioDataReplacement.detachedMedia + 1,
  );
  await example.getByRole("button", { name: "Check retained Analyser" }).click();
  await expect(example.locator(".retained-analyser-state")).toHaveAttribute(
    "data-available",
    "true",
  );

  const beforeUrlReplacement = await browserCounts(page);
  await example
    .getByRole("button", { name: "Load anonymous-CORS alternative", exact: true })
    .click();
  await expect(state).toHaveAttribute("data-source", "playable");
  expect(await browserCounts(page)).toMatchObject({
    contexts: 1,
    analysers: 1,
    gains: 1,
    graphAttachments: 3,
  });
  const afterUrlReplacement = await browserCounts(page);
  expect(afterUrlReplacement.disconnectCalls).toBe(
    beforeUrlReplacement.disconnectCalls + 1,
  );
  expect(afterUrlReplacement.detachedMedia).toBe(
    beforeUrlReplacement.detachedMedia + 1,
  );
  await example.getByRole("button", { name: "Check retained Analyser" }).click();
  await expect(example.locator(".retained-analyser-state")).toHaveAttribute(
    "data-available",
    "true",
  );

  await example.getByRole("button", { name: "Unload graph tone", exact: true }).click();
  await expect(state).toHaveAttribute("data-graph", "awaiting-source");
  await expect(state).toHaveAttribute("data-analyser-available", "false");
  await example.getByRole("button", { name: "Check retained Analyser" }).click();
  await expect(example.locator(".retained-analyser-state")).toHaveAttribute(
    "data-available",
    "false",
  );
});

test("denied anonymous CORS falls back across mixed Playback Source alternatives", async ({
  openRoute,
  page,
}) => {
  await installGraphInstrumentation(page);
  const requests: string[] = [];
  await page.route("https://media.example/*.wav", async (route) => {
    requests.push(route.request().url());
    const allowed = route.request().url().endsWith("/allowed.wav");
    await route.fulfill({
      status: 200,
      contentType: "audio/wav",
      headers: {
        "Access-Control-Allow-Origin": allowed ? "*" : "https://denied.example",
      },
      body: toneWav(allowed ? 440 : 330),
    });
  });
  await openRoute("/graph-playback", "Analyse audio before effective graph gain");

  const example = page.getByRole("group", { name: "Graph-backed Playback" });
  const state = example.locator(".graph-playback-state");
  await example
    .getByRole("button", {
      name: "Load mixed Playback Source alternatives",
      exact: true,
    })
    .click();

  await expect(state).toHaveAttribute("data-source", "playable");
  await expect(state).toHaveAttribute(
    "data-selected-alternative",
    "https://media.example/allowed.wav",
  );
  expect(requests).toEqual([
    "https://media.example/denied.wav",
    "https://media.example/allowed.wav",
  ]);
  expect(
    await page.evaluate(() =>
      (window as GraphWindow).graphBrowserState!.graphAttachments.map(
        ({ crossOrigin, src }) => ({ crossOrigin, src }),
      ),
    ),
  ).toEqual([
    { crossOrigin: "anonymous", src: "" },
    { crossOrigin: "anonymous", src: "" },
  ]);
  expect(await browserCounts(page)).toMatchObject({ contexts: 1, analysers: 1 });
});

test("failure after graph-backed alternative selection is terminal", async ({
  openRoute,
  page,
}) => {
  await installGraphInstrumentation(page);
  const requests: string[] = [];
  await page.route("https://media.example/*.wav", async (route) => {
    requests.push(route.request().url());
    await route.fulfill({
      status: 200,
      contentType: "audio/wav",
      headers: { "Access-Control-Allow-Origin": "*" },
      body: toneWav(440),
    });
  });
  await openRoute("/graph-playback", "Analyse audio before effective graph gain");

  const example = page.getByRole("group", { name: "Graph-backed Playback" });
  const state = example.locator(".graph-playback-state");
  await example
    .getByRole("button", { name: "Load selected-failure alternatives", exact: true })
    .click();
  await expect(state).toHaveAttribute(
    "data-selected-alternative",
    "https://media.example/allowed.wav",
  );

  await page.evaluate(() => {
    const element = (window as GraphWindow).graphBrowserState!.graphAttachments[0]
      .element;
    Object.defineProperty(element, "error", {
      configurable: true,
      value: { code: 2 },
    });
    element.dispatchEvent(new Event("error"));
  });
  await expect(state).toHaveAttribute("data-source", "failed");
  await expect(state).toHaveAttribute("data-source-failure", "network");
  await expect(state).toHaveAttribute(
    "data-selected-alternative",
    "https://media.example/allowed.wav",
  );
  expect(requests).toEqual(["https://media.example/allowed.wav"]);
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
      graphAttachments: state.graphAttachments.length,
      closeCalls: state.closeCalls,
      disconnectCalls: state.disconnectCalls,
      detachedMedia: state.detachedMedia,
    };
  });
}

function toneWav(frequency: number): Buffer {
  const sampleRate = 44_100;
  const seconds = 2;
  const sampleCount = sampleRate * seconds;
  const dataSize = sampleCount * 2;
  const bytes = Buffer.alloc(44 + dataSize);
  bytes.write("RIFF", 0);
  bytes.writeUInt32LE(36 + dataSize, 4);
  bytes.write("WAVEfmt ", 8);
  bytes.writeUInt32LE(16, 16);
  bytes.writeUInt16LE(1, 20);
  bytes.writeUInt16LE(1, 22);
  bytes.writeUInt32LE(sampleRate, 24);
  bytes.writeUInt32LE(sampleRate * 2, 28);
  bytes.writeUInt16LE(2, 32);
  bytes.writeUInt16LE(16, 34);
  bytes.write("data", 36);
  bytes.writeUInt32LE(dataSize, 40);
  for (let index = 0; index < sampleCount; index += 1) {
    const time = index / sampleRate;
    const sample = Math.sin(frequency * time * Math.PI * 2) * 0.18;
    bytes.writeInt16LE(Math.round(sample * 32_767), 44 + index * 2);
  }
  return bytes;
}

async function activeGain(page: Page) {
  return page.evaluate(
    () => (window as GraphWindow).graphBrowserState!.gainNodes[0]?.gain.value,
  );
}
