import { expect, test } from "./fixtures";

type BoundedPlaybackTestWindow = typeof globalThis & {
  boundedPlaybackElements?: HTMLMediaElement[];
  boundedPlaybackElement?: HTMLMediaElement;
  boundedPlaybackLog?: string[];
  boundedPlaybackTimeouts?: number[];
  boundedPlayResolvers?: Array<() => void>;
  resolveBoundedPlay?: () => void;
  setBoundedSeeking?: (seeking: boolean) => void;
  setBoundedHidden?: (hidden: boolean) => void;
};

type WaveformTestWindow = typeof globalThis & {
  flushWaveformResize?: () => void;
};

async function capturePlaybackElements(page: import("@playwright/test").Page) {
  await page.addInitScript(() => {
    const NativeAudio = window.Audio;
    const nativeSetTimeout = window.setTimeout;
    const elements: HTMLMediaElement[] = [];
    const timeouts: number[] = [];
    let hidden = false;
    (window as BoundedPlaybackTestWindow).boundedPlaybackElements = elements;
    (window as BoundedPlaybackTestWindow).boundedPlaybackTimeouts = timeouts;
    window.setTimeout = ((handler, timeout = 0, ...args) => {
      timeouts.push(timeout);
      return nativeSetTimeout(handler, timeout, ...args);
    }) as typeof window.setTimeout;
    Object.defineProperty(document, "hidden", {
      configurable: true,
      get: () => hidden,
    });
    (window as BoundedPlaybackTestWindow).setBoundedHidden = (next) => {
      hidden = next;
      document.dispatchEvent(new Event("visibilitychange"));
    };
    Object.defineProperty(window, "Audio", {
      configurable: true,
      value: function Audio(source?: string) {
        const element = new NativeAudio(source);
        elements.push(element);
        return element;
      },
    });
  });
}

async function controlShortPlayback(
  page: import("@playwright/test").Page,
  playOutcome: "hold" | "reject" = "hold",
) {
  await page.evaluate((outcome) => {
    const testWindow = window as BoundedPlaybackTestWindow;
    const element = testWindow.boundedPlaybackElements
      ?.filter((candidate) => candidate.duration > 3)
      .slice(-1)[0];
    if (!element) throw new Error("short Playback element was not created");

    testWindow.boundedPlaybackElement = element;
    testWindow.boundedPlaybackLog = [];
    testWindow.boundedPlayResolvers = [];
    let currentTime = 0;
    let paused = true;
    let seeking = false;
    Object.defineProperty(element, "currentTime", {
      configurable: true,
      get: () => currentTime,
      set: (value: number) => {
        currentTime = value;
        testWindow.boundedPlaybackLog?.push(`seek:${value}`);
      },
    });
    Object.defineProperty(element, "paused", {
      configurable: true,
      get: () => paused,
    });
    Object.defineProperty(element, "seeking", {
      configurable: true,
      get: () => seeking,
    });
    testWindow.setBoundedSeeking = (next) => {
      seeking = next;
    };
    element.pause = () => {
      paused = true;
      testWindow.boundedPlaybackLog?.push("pause");
    };
    element.play = () => {
      paused = false;
      testWindow.boundedPlaybackLog?.push("play");
      return new Promise<void>((resolve, reject) => {
        testWindow.boundedPlayResolvers?.push(resolve);
        testWindow.resolveBoundedPlay = resolve;
        if (outcome === "reject") reject(new Error("bounded activation rejected"));
      });
    };
  }, playOutcome);
}

async function settleBoundedPlayback(page: import("@playwright/test").Page) {
  await page.evaluate(() => {
    const browser = window as BoundedPlaybackTestWindow;
    browser.boundedPlaybackElement?.dispatchEvent(new Event("seeked"));
    browser.resolveBoundedPlay?.();
  });
}

test("Waveform Data preserves mode and channels in a narrow viewport", async ({
  openRoute,
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await openRoute("/waveforms", "Preview and select waveform ranges");

  const magnitude = page.getByRole("img", {
    name: "Mono magnitude Waveform Data",
  });
  await expect(magnitude).toBeVisible();
  await expect(magnitude).toHaveAttribute("data-amplitude-mode", "magnitude");
  await expect(magnitude).toHaveAttribute("data-channel-count", "1");

  const signed = page.getByRole("img", {
    name: "Stereo signed-envelope Waveform Data",
  });
  await signed.scrollIntoViewIfNeeded();
  await expect(signed).toBeInViewport();
  await expect(signed).toHaveAttribute(
    "data-amplitude-mode",
    "signed-envelope",
  );
  await expect(signed).toHaveAttribute("data-channel-count", "2");
  await expect(signed).toHaveAttribute("data-resolution", "1");
  await expect(signed).toHaveAttribute("data-bucket-count", "12");
  await expect(signed.locator("path")).toHaveCount(2);

  const channelBounds = await signed.locator("path").evaluateAll((paths) =>
    paths.map((path) => {
      const bounds = (path as SVGGraphicsElement).getBBox();
      return { top: bounds.y, bottom: bounds.y + bounds.height };
    }),
  );
  expect(channelBounds[0].bottom).toBeLessThanOrEqual(56);
  expect(channelBounds[1].top).toBeGreaterThanOrEqual(56);

  const fitsContainer = await signed.evaluate((element) => {
    const parent = element.parentElement;
    return parent !== null && element.getBoundingClientRect().width <= parent.clientWidth;
  });
  expect(fitsContainer).toBe(true);
});

test("long Waveforms have a complete native keyboard navigation path", async ({
  openRoute,
  page,
}) => {
  await openRoute("/waveforms", "Preview and select waveform ranges");

  const viewport = page.getByRole("group", {
    name: "Four-hour stereo waveform",
  });
  const range = viewport.locator(".dioxus-audio__viewport-range");
  const zoomIn = viewport.getByRole("button", { name: "Zoom in" });
  const overview = viewport.getByRole("slider", {
    name: "Overview position",
  });

  await expect(range).toHaveText("Visible 1:00:00 to 1:30:00");
  await expect(viewport.getByRole("button")).toHaveCount(5);
  await expect(overview).toBeEnabled();
  await expect(overview).toHaveAttribute(
    "aria-valuetext",
    "Visible 1:00:00 to 1:30:00",
  );

  await zoomIn.focus();
  await zoomIn.press("Enter");
  await expect(zoomIn).toBeFocused();
  await expect(range).toHaveText("Visible 1:07:30 to 1:22:30");

  const panBackward = viewport.getByRole("button", { name: "Pan backward" });
  await panBackward.focus();
  await panBackward.press("Enter");
  await expect(panBackward).toBeFocused();
  await expect(range).toHaveText("Visible 1:00:00 to 1:15:00");
  await viewport.getByRole("button", { name: "Pan forward" }).press("Enter");
  await expect(range).toHaveText("Visible 1:07:30 to 1:22:30");

  await viewport.getByRole("button", { name: "Zoom out" }).press("Enter");
  await expect(range).toHaveText("Visible 1:00:00 to 1:30:00");
  await zoomIn.press("Enter");
  await expect(range).toHaveText("Visible 1:07:30 to 1:22:30");

  await overview.focus();
  await overview.press("End");
  await expect(overview).toBeFocused();
  await expect(range).toHaveText("Visible 3:45:00 to 4:00:00");
  await expect(
    viewport.getByRole("button", { name: "Pan forward" }),
  ).toBeDisabled();

  await viewport.getByRole("button", { name: "Reset view" }).click();
  await expect(range).toHaveText("Visible 0:00 to 4:00:00");
  await expect(overview).toBeDisabled();
  await expect(viewport.getByRole("button", { name: "Zoom out" })).toBeDisabled();
});

test("long Waveform resolution follows measured width within bounded geometry", async ({
  openRoute,
  page,
}) => {
  await page.addInitScript(() => {
    const NativeResizeObserver = window.ResizeObserver;
    const pending: Array<() => void> = [];
    let defer = true;
    class DeferredResizeObserver extends NativeResizeObserver {
      constructor(callback: ResizeObserverCallback) {
        let observer: ResizeObserver;
        super((entries) => {
          if (defer) pending.push(() => callback(entries, observer));
          else callback(entries, observer);
        });
        observer = this;
      }
    }
    Object.defineProperty(window, "ResizeObserver", {
      configurable: true,
      value: DeferredResizeObserver,
    });
    (window as WaveformTestWindow).flushWaveformResize = () => {
      defer = false;
      for (const callback of pending.splice(0)) callback();
    };
  });
  await page.setViewportSize({ width: 1200, height: 900 });
  await openRoute("/waveforms", "Preview and select waveform ranges");

  const viewport = page.getByRole("group", {
    name: "Four-hour stereo waveform",
  });
  const waveform = viewport.locator("svg.dioxus-audio__waveform-data");
  await expect(viewport).toHaveAttribute("data-budget-source", "fallback");
  await expect(waveform).toHaveAttribute("data-bucket-budget", "64");
  await expect(waveform).toHaveAttribute("data-resolution", "3");

  await page.evaluate(() => {
    (window as WaveformTestWindow).flushWaveformResize?.();
  });
  await expect(viewport).toHaveAttribute("data-budget-source", "measured");
  const desktopBudget = Number(await waveform.getAttribute("data-bucket-budget"));
  const desktopResolution = Number(await waveform.getAttribute("data-resolution"));
  const desktopBuckets = Number(await waveform.getAttribute("data-bucket-count"));
  expect(desktopBudget).toBeGreaterThan(64);
  expect(desktopBudget).toBeLessThanOrEqual(4096);
  expect(desktopBuckets).toBeLessThanOrEqual(desktopBudget);
  await expect(waveform.locator("path")).toHaveCount(2);
  await expect(waveform.locator("rect")).toHaveCount(0);

  await page.setViewportSize({ width: 390, height: 844 });
  await expect
    .poll(async () => Number(await waveform.getAttribute("data-bucket-budget")))
    .toBeLessThan(desktopBudget);
  const mobileBudget = Number(await waveform.getAttribute("data-bucket-budget"));
  const mobileResolution = Number(await waveform.getAttribute("data-resolution"));
  const mobileBuckets = Number(await waveform.getAttribute("data-bucket-count"));
  expect(mobileResolution).toBeGreaterThan(desktopResolution);
  expect(mobileBuckets).toBeLessThanOrEqual(mobileBudget);

  const containment = await viewport.evaluate((element) => {
    const bounds = element.getBoundingClientRect();
    const parent = element.parentElement?.getBoundingClientRect();
    const svg = element.querySelector("svg")?.getBoundingClientRect();
    return (
      parent !== undefined &&
      svg !== undefined &&
      bounds.left >= parent.left &&
      bounds.right <= parent.right &&
      svg.left >= bounds.left &&
      svg.right <= bounds.right
    );
  });
  expect(containment).toBe(true);
});

test("interactive Waveforms keep keyboard timelines constrained and independent", async ({
  openRoute,
  page,
}) => {
  await openRoute("/waveforms", "Preview and select waveform ranges");

  const primary = page.getByRole("group", {
    name: "Interactive episode waveform",
  });
  const position = primary.getByRole("slider", {
    name: "Episode playback position",
  });
  const start = primary.getByRole("slider", { name: "Episode selection start" });
  const end = primary.getByRole("slider", { name: "Episode selection end" });
  const secondary = page.getByRole("group", {
    name: "Independent short waveform",
  });

  await expect(position).toBeEnabled();
  await expect(position).toHaveAttribute("max", "12");
  await expect(position).toHaveAttribute("step", "0.25");
  await expect(position).toHaveAttribute("aria-valuemax", "2");
  await expect(start).toHaveValue("2.25");
  await expect(start).toHaveAttribute("aria-valuemax", "9.5");
  await expect(end).toHaveValue("9.5");
  await expect(end).toHaveAttribute("aria-valuemin", "2.25");
  await expect(
    secondary.getByRole("slider", { name: "Short selection start" }),
  ).toHaveValue("0.5");

  await position.focus();
  await position.press("End");
  await expect(position).toBeFocused();
  await expect(position).toHaveValue("2");
  await expect(position).toHaveAttribute("aria-valuetext", "2 seconds");

  await start.focus();
  await start.press("PageUp");
  await expect(start).toBeFocused();
  await expect(start).toHaveValue("4.25");
  await expect(page.getByText("Committed selection: 4.25 s to 9.50 s")).toBeVisible();

  await start.press("End");
  await expect(start).toBeFocused();
  await expect(start).toHaveValue("9.5");
  await expect(end).toHaveValue("9.5");
  await expect(page.getByText("Committed selection: 9.50 s to 9.50 s")).toBeVisible();

  await end.focus();
  await end.press("ArrowRight");
  await expect(end).toBeFocused();
  await expect(start).toHaveValue("9.5");
  await expect(end).toHaveValue("9.75");
  await expect(page.getByText("Committed selection: 9.50 s to 9.75 s")).toBeVisible();

  await end.press("Home");
  await expect(end).toBeFocused();
  await expect(start).toHaveValue("9.5");
  await expect(end).toHaveValue("9.5");
  await end.press("ArrowRight");
  await expect(end).toHaveValue("9.75");
  await expect(primary.getByRole("status")).toHaveCount(0);

  await expect(
    secondary.getByRole("slider", { name: "Short selection start" }),
  ).toHaveValue("0.5");
  await expect(
    secondary.getByRole("slider", { name: "Short selection end" }),
  ).toHaveValue("3.5");
});

test("interactive Waveform pointer drags draft once before commit and track hits seek", async ({
  openRoute,
  page,
}) => {
  await openRoute("/waveforms", "Preview and select waveform ranges");

  const waveform = page.getByRole("group", {
    name: "Interactive episode waveform",
  });
  const position = waveform.getByRole("slider", {
    name: "Episode playback position",
  });
  const start = waveform.getByRole("slider", {
    name: "Episode selection start",
  });
  const end = waveform.getByRole("slider", { name: "Episode selection end" });
  const committed = page.getByText(/^Committed selection:/);
  const commits = page.getByText(/^Selection commits:/);

  await expect(position).toBeEnabled();
  await waveform.scrollIntoViewIfNeeded();
  await expect(committed).toHaveText("Committed selection: 2.25 s to 9.50 s");
  await expect(commits).toHaveText("Selection commits: 0");

  const bounds = await start.boundingBox();
  expect(bounds).not.toBeNull();
  const thumbWidth = 24;
  const xForTime = (seconds: number) =>
    bounds!.x + thumbWidth / 2 + (bounds!.width - thumbWidth) * (seconds / 12);
  const y = bounds!.y + bounds!.height / 2;

  await page.mouse.move(xForTime(2.25), y);
  await page.mouse.down();
  await page.mouse.move(xForTime(4.5), y, { steps: 6 });

  await expect(start).toBeFocused();
  await expect(start).toHaveValue("4.5");
  await expect(committed).toHaveText("Committed selection: 2.25 s to 9.50 s");
  await expect(commits).toHaveText("Selection commits: 0");

  await page.mouse.up();
  await expect(committed).toHaveText("Committed selection: 4.50 s to 9.50 s");
  await expect(commits).toHaveText("Selection commits: 1");

  await position.click({
    position: {
      x: thumbWidth / 2 + (bounds!.width - thumbWidth) / 12,
      y: bounds!.height / 2,
    },
  });
  await expect
    .poll(async () => Number(await position.inputValue()))
    .toBeGreaterThan(0.5);
  expect(Number(await position.inputValue())).toBeLessThan(1.5);
  await expect(start).toHaveValue("4.5");
  await expect(end).toHaveValue("9.5");
  await expect(commits).toHaveText("Selection commits: 1");

  const currentPosition = Number(await position.inputValue());
  await page.mouse.move(xForTime(currentPosition), y);
  await page.mouse.down();
  await page.mouse.move(xForTime(10), y, { steps: 4 });
  await expect(position).toHaveValue("2");
  await expect(position).toHaveAttribute("aria-valuetext", "2 seconds");
  await page.mouse.up();
  await expect(position).toHaveValue("2");

  await expect(position).toBeFocused();
  await page.mouse.move(xForTime(4.5), y);
  await page.mouse.down();
  await page.mouse.move(xForTime(5.5), y, { steps: 4 });
  await expect(start).toBeFocused();
  const secondDraftStart = Number(await start.inputValue());
  expect(secondDraftStart).toBeGreaterThan(5);
  expect(secondDraftStart).toBeLessThan(6);
  await expect(commits).toHaveText("Selection commits: 1");
  await page.mouse.up();
  await expect(commits).toHaveText("Selection commits: 2");

  await start.press("End");
  await expect(start).toHaveValue("9.5");
  await expect(end).toHaveValue("9.5");
  await expect(commits).toHaveText("Selection commits: 3");

  const collapsedX = xForTime(9.5);
  await page.mouse.move(collapsedX + thumbWidth / 2, y);
  await page.mouse.down();
  await page.mouse.move(xForTime(10.5) + thumbWidth / 2, y, { steps: 4 });
  await expect(start).toHaveValue("9.5");
  const draftEnd = Number(await end.inputValue());
  expect(draftEnd).toBeGreaterThan(10);
  expect(draftEnd).toBeLessThan(11);
  await expect(commits).toHaveText("Selection commits: 3");
  await page.mouse.up();
  await expect(committed).toHaveText(
    `Committed selection: 9.50 s to ${draftEnd.toFixed(2)} s`,
  );
  await expect(commits).toHaveText("Selection commits: 4");

  await end.press("Home");
  await expect(start).toHaveValue("9.5");
  await expect(end).toHaveValue("9.5");
  await expect(commits).toHaveText("Selection commits: 5");

  await page.mouse.move(collapsedX - thumbWidth / 2, y);
  await page.mouse.down();
  await page.mouse.move(xForTime(8.5) - thumbWidth / 2, y, { steps: 4 });
  const draftStart = Number(await start.inputValue());
  expect(draftStart).toBeGreaterThan(8);
  expect(draftStart).toBeLessThan(9);
  await expect(end).toHaveValue("9.5");
  await expect(commits).toHaveText("Selection commits: 5");
  await page.mouse.up();
  await expect(commits).toHaveText("Selection commits: 6");
});

test("collapsed Waveform Selection handles remain pointer-operable at duration edges", async ({
  openRoute,
  page,
}) => {
  await openRoute("/waveforms", "Preview and select waveform ranges");
  const waveform = page.getByRole("group", {
    name: "Interactive episode waveform",
  });
  const start = waveform.getByRole("slider", {
    name: "Episode selection start",
  });
  const end = waveform.getByRole("slider", { name: "Episode selection end" });
  const commits = page.getByText(/^Selection commits:/);
  await waveform.scrollIntoViewIfNeeded();

  const bounds = await start.boundingBox();
  expect(bounds).not.toBeNull();
  const thumbWidth = 24;
  const xForTime = (seconds: number) =>
    bounds!.x + thumbWidth / 2 + (bounds!.width - thumbWidth) * (seconds / 12);
  const y = bounds!.y + bounds!.height / 2;

  await start.press("Home");
  await end.press("Home");
  await expect(start).toHaveValue("0");
  await expect(end).toHaveValue("0");

  await page.mouse.move(xForTime(0) + thumbWidth, y);
  await page.mouse.down();
  await page.mouse.move(xForTime(1) + thumbWidth, y, { steps: 4 });
  await expect(start).toHaveValue("0");
  expect(Number(await end.inputValue())).toBeGreaterThan(0.5);
  await expect(commits).toHaveText("Selection commits: 2");
  await page.mouse.up();
  await expect(commits).toHaveText("Selection commits: 3");

  await end.press("End");
  await start.press("End");
  await expect(start).toHaveValue("12");
  await expect(end).toHaveValue("12");

  await page.mouse.move(xForTime(12) - thumbWidth, y);
  await page.mouse.down();
  await page.mouse.move(xForTime(11) - thumbWidth, y, { steps: 4 });
  expect(Number(await start.inputValue())).toBeLessThan(11.5);
  await expect(end).toHaveValue("12");
  await expect(commits).toHaveText("Selection commits: 5");
  await page.mouse.up();
  await expect(commits).toHaveText("Selection commits: 6");
});

test("interactive Waveforms fit a mobile viewport without losing controls", async ({
  openRoute,
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await openRoute("/waveforms", "Preview and select waveform ranges");

  const waveform = page.getByRole("group", {
    name: "Interactive episode waveform",
  });
  await waveform.scrollIntoViewIfNeeded();
  await expect(waveform).toBeInViewport();
  await expect(waveform.getByRole("slider")).toHaveCount(3);

  const controlsFit = await waveform.evaluate((element) => {
    const group = element.getBoundingClientRect();
    return [...element.querySelectorAll('input[type="range"]')].every((control) => {
      const bounds = control.getBoundingClientRect();
      return bounds.left >= group.left && bounds.right <= group.right;
    });
  });
  expect(controlsFit).toBe(true);
});

test("Bounded Playback validates authoritative duration and orders pause, seek, and play", async ({
  openRoute,
  page,
}) => {
  await capturePlaybackElements(page);
  await openRoute("/waveforms", "Preview and select waveform ranges");

  const episodeState = page.locator(".episode-bounded-playback-state");
  await expect(episodeState).toHaveAttribute("data-source", "playable");
  await page.getByRole("button", { name: "Play episode selection once" }).click();
  await expect(episodeState).toHaveAttribute("data-phase", "none");
  await expect(episodeState.getByRole("alert")).toContainText(
    "outside the Playback duration",
  );

  const controls = page.getByRole("group", { name: "Short Bounded Playback" });
  const state = controls.locator(".short-bounded-playback-state");
  await expect(state).toHaveAttribute("data-source", "playable");
  await controlShortPlayback(page);

  await controls.getByRole("button", { name: "Play short selection once" }).click();
  await expect(state).toHaveAttribute("data-phase", "seeking");
  expect(
    await page.evaluate(
      () => (window as BoundedPlaybackTestWindow).boundedPlaybackLog,
    ),
  ).toEqual(["pause", "seek:0.5"]);

  await page.evaluate(() => {
    (window as BoundedPlaybackTestWindow).boundedPlaybackElement?.dispatchEvent(
      new Event("seeked"),
    );
  });
  await expect(state).toHaveAttribute("data-phase", "activating");
  expect(
    await page.evaluate(
      () => (window as BoundedPlaybackTestWindow).boundedPlaybackLog,
    ),
  ).toEqual(["pause", "seek:0.5", "play"]);

  await page.evaluate(() => {
    (window as BoundedPlaybackTestWindow).resolveBoundedPlay?.();
  });
  await expect(state).toHaveAttribute("data-phase", "active");
  await expect(state).toHaveAttribute("data-transport", "playing");

  await controls.getByRole("button", { name: "Pause Bounded Playback" }).click();
  await expect(state).toHaveAttribute("data-phase", "paused");
  await controls.getByRole("button", { name: "Resume Bounded Playback" }).click();
  await expect(state).toHaveAttribute("data-phase", "activating");
  await page.evaluate(() => {
    (window as BoundedPlaybackTestWindow).resolveBoundedPlay?.();
  });
  await expect(state).toHaveAttribute("data-phase", "active");

  await page.evaluate(() => {
    const element = (window as BoundedPlaybackTestWindow).boundedPlaybackElement;
    if (!element) throw new Error("controlled Playback element is missing");
    element.currentTime = 3.75;
    element.dispatchEvent(new Event("timeupdate"));
  });
  await expect(state).toHaveAttribute("data-phase", "completed");
  await expect(state).toHaveAttribute("data-position", "3.5");
  await expect(state).toHaveAttribute("data-transport", "paused");
});

test("looped Bounded Playback wraps once and reconciles rate, visibility, repeat, and stop", async ({
  openRoute,
  page,
}) => {
  await capturePlaybackElements(page);
  await openRoute("/waveforms", "Preview and select waveform ranges");
  const controls = page.getByRole("group", { name: "Short Bounded Playback" });
  const state = controls.locator(".short-bounded-playback-state");
  const status = controls.locator(".dioxus-audio__status-announcer");
  await expect(state).toHaveAttribute("data-source", "playable");
  await controlShortPlayback(page);

  await controls
    .getByRole("button", { name: "Toggle short whole-source repeat" })
    .click();
  await expect(state).toHaveAttribute("data-repeat", "true");
  expect(
    await page.evaluate(
      () => (window as BoundedPlaybackTestWindow).boundedPlaybackElement?.loop,
    ),
  ).toBe(true);

  await controls.getByRole("button", { name: "Loop short selection" }).click();
  await expect(state).toHaveAttribute("data-mode", "loop");
  await expect(state).toHaveAttribute("data-phase", "seeking");
  await expect(status).toHaveText("Looped Bounded Playback starting");
  expect(
    await page.evaluate(
      () => (window as BoundedPlaybackTestWindow).boundedPlaybackElement?.loop,
    ),
  ).toBe(false);
  expect(
    await page.evaluate(
      () => (window as BoundedPlaybackTestWindow).boundedPlaybackLog,
    ),
  ).toEqual(["pause", "seek:0.5"]);

  await settleBoundedPlayback(page);
  await expect(state).toHaveAttribute("data-phase", "active");
  await expect(status).toHaveText("");

  await page.evaluate(() => {
    (window as BoundedPlaybackTestWindow).boundedPlaybackTimeouts!.length = 0;
  });
  await controls.getByRole("button", { name: "Set short Playback to 2x" }).click();
  await expect(state).toHaveAttribute("data-rate", "2");
  expect(
    await page.evaluate(
      () => (window as BoundedPlaybackTestWindow).boundedPlaybackElement?.playbackRate,
    ),
  ).toBe(2);
  await expect
    .poll(() =>
      page.evaluate(() =>
        (window as BoundedPlaybackTestWindow).boundedPlaybackTimeouts?.some(
          (timeout) => timeout >= 1_499 && timeout <= 1_501,
        ),
      ),
    )
    .toBe(true);

  await page.evaluate(() => {
    const element = (window as BoundedPlaybackTestWindow).boundedPlaybackElement;
    if (!element) throw new Error("controlled Playback element is missing");
    element.currentTime = 3.75;
    element.dispatchEvent(new Event("timeupdate"));
  });
  await expect(state).toHaveAttribute("data-phase", "wrapping");
  await expect(status).toHaveText("Bounded Playback wrapping");
  expect(
    (await page.evaluate(
      () => (window as BoundedPlaybackTestWindow).boundedPlaybackLog,
    ))?.slice(-2),
  ).toEqual(["pause", "seek:0.5"]);

  await settleBoundedPlayback(page);
  await expect(state).toHaveAttribute("data-phase", "active");

  const seeksBeforeVisibility = await page.evaluate(
    () =>
      (window as BoundedPlaybackTestWindow).boundedPlaybackLog?.filter((entry) =>
        entry.startsWith("seek:0.5"),
      ).length,
  );
  await page.evaluate(() => {
    const browser = window as BoundedPlaybackTestWindow;
    browser.setBoundedHidden?.(true);
    if (!browser.boundedPlaybackElement) {
      throw new Error("controlled Playback element is missing");
    }
    browser.boundedPlaybackElement.currentTime = 17;
    browser.setBoundedHidden?.(false);
  });
  await expect(state).toHaveAttribute("data-phase", "wrapping");
  const seeksAfterVisibility = await page.evaluate(
    () =>
      (window as BoundedPlaybackTestWindow).boundedPlaybackLog?.filter((entry) =>
        entry.startsWith("seek:0.5"),
      ).length,
  );
  expect(seeksAfterVisibility).toBe((seeksBeforeVisibility ?? 0) + 1);

  await controls.getByRole("button", { name: "Cancel Bounded Playback" }).click();
  await expect(state).toHaveAttribute("data-phase", "cancelled");
  await expect(status).toHaveText("Bounded Playback cancelled");
  expect(
    await page.evaluate(
      () => (window as BoundedPlaybackTestWindow).boundedPlaybackElement?.loop,
    ),
  ).toBe(true);

  await controls.getByRole("button", { name: "Loop short selection" }).click();
  await settleBoundedPlayback(page);
  await expect(state).toHaveAttribute("data-phase", "active");
  await page.evaluate(() => {
    const element = (window as BoundedPlaybackTestWindow).boundedPlaybackElement;
    if (!element) throw new Error("controlled Playback element is missing");
    element.currentTime = 3.75;
    element.dispatchEvent(new Event("timeupdate"));
  });
  await expect(state).toHaveAttribute("data-phase", "wrapping");

  await controls.getByRole("button", { name: "Stop short Playback" }).click();
  await expect(state).toHaveAttribute("data-phase", "none");
  await expect(state).toHaveAttribute("data-position", "0");
  await expect(state).toHaveAttribute("data-transport", "idle");
  await settleBoundedPlayback(page);
  await expect(state).toHaveAttribute("data-phase", "none");
  await expect(state).toHaveAttribute("data-transport", "idle");
  expect(
    await page.evaluate(
      () => (window as BoundedPlaybackTestWindow).boundedPlaybackElement?.loop,
    ),
  ).toBe(true);
});

test("committed Waveform Selection edits retarget active Bounded Playback atomically", async ({
  openRoute,
  page,
}) => {
  await capturePlaybackElements(page);
  await openRoute("/waveforms", "Preview and select waveform ranges");
  const controls = page.getByRole("group", { name: "Short Bounded Playback" });
  const state = controls.locator(".short-bounded-playback-state");
  const waveform = controls.getByRole("group", { name: "Independent short waveform" });
  const start = waveform.getByRole("slider", { name: "Short selection start" });
  const end = waveform.getByRole("slider", { name: "Short selection end" });
  await expect(state).toHaveAttribute("data-source", "playable");
  await controlShortPlayback(page);

  await controls.getByRole("button", { name: "Loop short selection" }).click();
  await page.evaluate(() => {
    const browser = window as BoundedPlaybackTestWindow;
    browser.boundedPlaybackElement?.dispatchEvent(new Event("seeked"));
    browser.resolveBoundedPlay?.();
  });
  await expect(state).toHaveAttribute("data-phase", "active");
  await page.evaluate(() => {
    const element = (window as BoundedPlaybackTestWindow).boundedPlaybackElement;
    if (!element) throw new Error("controlled Playback element is missing");
    element.currentTime = 1;
    element.dispatchEvent(new Event("timeupdate"));
  });
  const logBeforePreservedRetarget = await page.evaluate(
    () => (window as BoundedPlaybackTestWindow).boundedPlaybackLog?.slice(),
  );

  await end.press("ArrowLeft");
  await expect(end).toHaveValue("3");
  await expect(state).toHaveAttribute("data-phase", "active");
  expect(
    await page.evaluate(
      () => (window as BoundedPlaybackTestWindow).boundedPlaybackLog,
    ),
  ).toEqual(logBeforePreservedRetarget);

  await start.press("ArrowRight");
  await start.press("ArrowRight");
  await expect(start).toHaveValue("1.5");
  await expect(state).toHaveAttribute("data-phase", "retargeting");
  expect(
    (await page.evaluate(
      () => (window as BoundedPlaybackTestWindow).boundedPlaybackLog,
    ))?.slice(-2),
  ).toEqual(["pause", "seek:1.5"]);

  await start.press("ArrowRight");
  await expect(start).toHaveValue("2");
  await expect(state).toHaveAttribute("data-phase", "retargeting");
  await page.evaluate(() => {
    const browser = window as BoundedPlaybackTestWindow;
    if (!browser.boundedPlaybackElement) {
      throw new Error("controlled Playback element is missing");
    }
    browser.boundedPlaybackElement.currentTime = 1.5;
    browser.boundedPlaybackElement.dispatchEvent(new Event("seeked"));
  });
  await expect(state).toHaveAttribute("data-phase", "retargeting");

  await page.evaluate(() => {
    const browser = window as BoundedPlaybackTestWindow;
    if (!browser.boundedPlaybackElement) {
      throw new Error("controlled Playback element is missing");
    }
    browser.boundedPlaybackElement.currentTime = 2;
    browser.boundedPlaybackElement.dispatchEvent(new Event("seeked"));
  });
  await expect(state).toHaveAttribute("data-phase", "activating");
  await page.evaluate(() => {
    (window as BoundedPlaybackTestWindow).resolveBoundedPlay?.();
  });
  await expect(state).toHaveAttribute("data-phase", "active");
  await expect(state).toHaveAttribute("data-mode", "loop");

  await controls.getByRole("button", { name: "Pause Bounded Playback" }).click();
  await expect(state).toHaveAttribute("data-phase", "paused");
  const playsBeforePausedRetarget = await page.evaluate(
    () =>
      (window as BoundedPlaybackTestWindow).boundedPlaybackLog?.filter(
        (entry) => entry === "play",
      ).length,
  );
  await start.press("ArrowRight");
  await expect(start).toHaveValue("2.5");
  await expect(state).toHaveAttribute("data-phase", "retargeting");
  await page.evaluate(() => {
    (window as BoundedPlaybackTestWindow).boundedPlaybackElement?.dispatchEvent(
      new Event("seeked"),
    );
  });
  await expect(state).toHaveAttribute("data-phase", "paused");
  expect(
    await page.evaluate(
      () =>
        (window as BoundedPlaybackTestWindow).boundedPlaybackLog?.filter(
          (entry) => entry === "play",
        ).length,
    ),
  ).toBe(playsBeforePausedRetarget);
});

test("a new Bounded Playback run rejects superseded seek and play outcomes", async ({
  openRoute,
  page,
}) => {
  await capturePlaybackElements(page);
  await openRoute("/waveforms", "Preview and select waveform ranges");
  const controls = page.getByRole("group", { name: "Short Bounded Playback" });
  const state = controls.locator(".short-bounded-playback-state");
  const start = controls.getByRole("slider", { name: "Short selection start" });
  await expect(state).toHaveAttribute("data-source", "playable");
  await controlShortPlayback(page);

  await controls.getByRole("button", { name: "Play short selection once" }).click();
  await start.press("ArrowRight");
  await expect(start).toHaveValue("1");
  await controls.getByRole("button", { name: "Loop short selection" }).click();
  await expect(state).toHaveAttribute("data-mode", "loop");
  await expect(state).toHaveAttribute("data-phase", "seeking");
  await page.evaluate(() => {
    const element = (window as BoundedPlaybackTestWindow).boundedPlaybackElement;
    if (!element) throw new Error("controlled Playback element is missing");
    element.currentTime = 0.5;
    element.dispatchEvent(new Event("seeked"));
  });
  await expect(state).toHaveAttribute("data-mode", "loop");
  await expect(state).toHaveAttribute("data-phase", "seeking");

  await page.evaluate(() => {
    const browser = window as BoundedPlaybackTestWindow;
    if (!browser.boundedPlaybackElement) {
      throw new Error("controlled Playback element is missing");
    }
    browser.boundedPlaybackElement.currentTime = 1;
    browser.boundedPlaybackElement.dispatchEvent(new Event("seeked"));
  });
  await expect(state).toHaveAttribute("data-phase", "activating");

  await controls.getByRole("button", { name: "Play short selection once" }).click();
  await expect(state).toHaveAttribute("data-mode", "once");
  await expect(state).toHaveAttribute("data-phase", "seeking");
  await page.evaluate(() => {
    const browser = window as BoundedPlaybackTestWindow;
    browser.boundedPlayResolvers?.[0]?.();
    browser.boundedPlaybackElement?.dispatchEvent(new Event("playing"));
  });
  await expect(state).toHaveAttribute("data-mode", "once");
  await expect(state).toHaveAttribute("data-phase", "seeking");

  await settleBoundedPlayback(page);
  await expect(state).toHaveAttribute("data-mode", "once");
  await expect(state).toHaveAttribute("data-phase", "active");
});

test("owner cleanup invalidates pending Bounded Playback outcomes", async ({
  openRoute,
  page,
}) => {
  const pageErrors: Error[] = [];
  page.on("pageerror", (error) => pageErrors.push(error));
  await capturePlaybackElements(page);
  await openRoute("/waveforms", "Preview and select waveform ranges");
  const controls = page.getByRole("group", { name: "Short Bounded Playback" });
  const state = controls.locator(".short-bounded-playback-state");
  await expect(state).toHaveAttribute("data-source", "playable");
  await controlShortPlayback(page);

  await controls.getByRole("button", { name: "Play short selection once" }).click();
  await page.evaluate(() => {
    (window as BoundedPlaybackTestWindow).boundedPlaybackElement?.dispatchEvent(
      new Event("seeked"),
    );
  });
  await expect(state).toHaveAttribute("data-phase", "activating");

  await page.getByRole("link", { name: "Analysis helpers" }).click();
  await expect(page).toHaveURL(/\/analysis$/);
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          (window as BoundedPlaybackTestWindow).boundedPlaybackElement?.hasAttribute(
            "src",
          ) ?? true,
      ),
    )
    .toBe(false);
  await page.evaluate(() => {
    const browser = window as BoundedPlaybackTestWindow;
    browser.boundedPlayResolvers?.[0]?.();
    browser.boundedPlaybackElement?.dispatchEvent(new Event("seeked"));
    browser.boundedPlaybackElement?.dispatchEvent(new Event("playing"));
  });
  await page.waitForTimeout(50);
  expect(pageErrors).toEqual([]);
  await expect(page).toHaveURL(/\/analysis$/);
});

test("direct seek, replacement, and unload invalidate stale Bounded Playback outcomes", async ({
  openRoute,
  page,
}) => {
  await capturePlaybackElements(page);
  await openRoute("/waveforms", "Preview and select waveform ranges");
  const controls = page.getByRole("group", { name: "Short Bounded Playback" });
  const state = controls.locator(".short-bounded-playback-state");
  const status = controls.locator(".dioxus-audio__status-announcer");
  await expect(state).toHaveAttribute("data-source", "playable");
  await controlShortPlayback(page);

  await controls.getByRole("button", { name: "Play short selection once" }).click();
  await controls
    .getByRole("button", { name: "Seek short Playback directly" })
    .click();
  await expect(state).toHaveAttribute("data-phase", "none");
  await expect(state).toHaveAttribute("data-position", "1");
  await expect(state).toHaveAttribute("data-phase", "none");
  await expect(status).toHaveText("Bounded Playback cancelled");
  await expect(page.getByText("Independent selection: 0.50 s to 3.50 s")).toBeVisible();

  await controls.getByRole("button", { name: "Play short selection once" }).click();
  await page.evaluate(() => {
    const testWindow = window as BoundedPlaybackTestWindow;
    testWindow.setBoundedSeeking?.(true);
    testWindow.boundedPlaybackElement?.dispatchEvent(new Event("seeked"));
  });
  await expect(state).toHaveAttribute("data-phase", "seeking");
  await page.evaluate(() => {
    const testWindow = window as BoundedPlaybackTestWindow;
    testWindow.setBoundedSeeking?.(false);
    testWindow.boundedPlaybackElement?.dispatchEvent(new Event("seeked"));
  });
  await expect(state).toHaveAttribute("data-phase", "activating");
  await controls
    .getByRole("button", { name: "Seek short Playback directly" })
    .click();
  await page.evaluate(() => {
    (window as BoundedPlaybackTestWindow).resolveBoundedPlay?.();
  });
  await expect(state).toHaveAttribute("data-phase", "none");

  await controls.getByRole("button", { name: "Play short selection once" }).click();
  const oldElement = await page.evaluateHandle(
    () => (window as BoundedPlaybackTestWindow).boundedPlaybackElement,
  );
  await controls.getByRole("button", { name: "Replace short source" }).click();
  await expect(state).toHaveAttribute("data-phase", "none");
  await expect(state).toHaveAttribute("data-source", "playable");
  await page.evaluate((element) => {
    element?.dispatchEvent(new Event("seeked"));
    (window as BoundedPlaybackTestWindow).resolveBoundedPlay?.();
  }, oldElement);
  await expect(state).toHaveAttribute("data-phase", "none");
  await expect(state).toHaveAttribute("data-transport", "idle");

  await controlShortPlayback(page);
  await controls.getByRole("button", { name: "Play short selection once" }).click();
  await page.evaluate(() => {
    (window as BoundedPlaybackTestWindow).boundedPlaybackElement?.dispatchEvent(
      new Event("seeked"),
    );
  });
  await expect(state).toHaveAttribute("data-phase", "activating");
  const activationElement = await page.evaluateHandle(
    () => (window as BoundedPlaybackTestWindow).boundedPlaybackElement,
  );
  await controls.getByRole("button", { name: "Replace short source" }).click();
  await expect(state).toHaveAttribute("data-source", "playable");
  await page.evaluate((element) => {
    (window as BoundedPlaybackTestWindow).resolveBoundedPlay?.();
    element?.dispatchEvent(new Event("playing"));
  }, activationElement);
  await expect(state).toHaveAttribute("data-phase", "none");
  await expect(state).toHaveAttribute("data-transport", "idle");

  await controlShortPlayback(page);
  await page.evaluate(() => {
    const element = (window as BoundedPlaybackTestWindow).boundedPlaybackElement;
    if (element) element.playbackRate = 16;
  });
  await controls.getByRole("button", { name: "Play short selection once" }).click();
  await page.evaluate(() => {
    const testWindow = window as BoundedPlaybackTestWindow;
    testWindow.boundedPlaybackElement?.dispatchEvent(new Event("seeked"));
    testWindow.resolveBoundedPlay?.();
  });
  await expect(state).toHaveAttribute("data-phase", "active");
  const unloadedElement = await page.evaluateHandle(
    () => (window as BoundedPlaybackTestWindow).boundedPlaybackElement,
  );
  await controls.getByRole("button", { name: "Unload short source" }).click();
  await expect(state).toHaveAttribute("data-source", "empty");
  await expect(state).toHaveAttribute("data-phase", "none");
  await page.evaluate((element) => {
    if (element) {
      element.currentTime = 3.75;
      element.dispatchEvent(new Event("pause"));
      element.dispatchEvent(new Event("timeupdate"));
      element.dispatchEvent(new Event("ended"));
    }
  }, unloadedElement);
  await page.waitForTimeout(300);
  await expect(state).toHaveAttribute("data-source", "empty");
  await expect(state).toHaveAttribute("data-phase", "none");
});

test("bounded seek and activation failures leave ordinary Playback usable", async ({
  openRoute,
  page,
}) => {
  await capturePlaybackElements(page);
  await openRoute("/waveforms", "Preview and select waveform ranges");
  const controls = page.getByRole("group", { name: "Short Bounded Playback" });
  const state = controls.locator(".short-bounded-playback-state");
  await expect(state).toHaveAttribute("data-source", "playable");
  await controlShortPlayback(page);

  await controls.getByRole("button", { name: "Play short selection once" }).click();
  await expect(state).toHaveAttribute("data-failure", "seek-timeout", {
    timeout: 5_000,
  });
  await expect(state).toHaveAttribute("data-source", "playable");

  await controlShortPlayback(page, "reject");
  await controls.getByRole("button", { name: "Play short selection once" }).click();
  await page.evaluate(() => {
    (window as BoundedPlaybackTestWindow).boundedPlaybackElement?.dispatchEvent(
      new Event("seeked"),
    );
  });
  await expect(state).toHaveAttribute("data-failure", "activation-rejected");
  await expect(state).toHaveAttribute("data-source", "playable");

  await controlShortPlayback(page);
  await controls.getByRole("button", { name: "Resume Bounded Playback" }).click();
  await page.evaluate(() => {
    (window as BoundedPlaybackTestWindow).resolveBoundedPlay?.();
  });
  await expect(state).toHaveAttribute("data-phase", "none");
  await expect(state).toHaveAttribute("data-transport", "playing");
  await expect(state).toHaveAttribute("data-source", "playable");
});
