import { expect, test } from "./fixtures";

type PlaybackTestWindow = typeof globalThis & {
  pendingPlaybackElement?: HTMLMediaElement;
  rejectPendingPlayback?: () => void;
};

test("generated audio can be played, paused, and resumed", async ({
  openRoute,
  page,
}) => {
  await openRoute("/playback", "Load audio only when it is needed");

  await expect(
    page.getByText("Audio loads on first play", { exact: true }),
  ).toBeVisible();
  const player = page.locator(".dioxus-audio__player");
  await expect(player).toHaveAttribute("data-source", "empty");
  await expect(player).toHaveAttribute("data-transport", "idle");
  await expect(player).toHaveAttribute("data-readiness", "unavailable");
  await expect(
    page.getByRole("img", { name: "Generated tone waveform" }),
  ).toBeVisible();

  await page.getByRole("button", { name: "Play", exact: true }).click();
  await expect(
    page.getByText("Audio bytes loaded", { exact: true }),
  ).toBeVisible();

  const pause = page.getByRole("button", { name: "Pause", exact: true });
  await expect(pause).toBeVisible();
  await pause.click();

  const play = page.getByRole("button", { name: "Play", exact: true });
  await expect(play).toBeVisible();
  await play.click();
  await expect(pause).toBeVisible();
});

test("a rejected playback attempt is visible and can be retried", async ({
  openRoute,
  page,
}) => {
  await openRoute("/playback", "Load audio only when it is needed");
  await page.evaluate(() => {
    const nativePlay = HTMLMediaElement.prototype.play;
    let holdNext = true;
    HTMLMediaElement.prototype.play = function () {
      const testWindow = window as PlaybackTestWindow;
      testWindow.pendingPlaybackElement = this;
      if (holdNext) {
        holdNext = false;
        return new Promise((_, reject) => {
          testWindow.rejectPendingPlayback = () =>
            reject(new DOMException("Playback blocked by test", "NotAllowedError"));
        });
      }
      return nativePlay.call(this);
    };
  });

  const player = page.locator(".dioxus-audio__player");
  const play = page.getByRole("button", { name: "Play", exact: true });
  await play.click();
  await expect(player).toHaveAttribute("data-source", "playable");
  await expect(player).toHaveAttribute("data-transport", "play-pending");
  await expect(player).toHaveAttribute("data-play-failure", "none");

  await page.evaluate(() => {
    const element = (window as PlaybackTestWindow).pendingPlaybackElement;
    if (element) {
      Object.defineProperty(element, "paused", {
        configurable: true,
        value: false,
      });
      element.dispatchEvent(new Event("playing"));
    }
  });
  await expect(player).toHaveAttribute("data-transport", "play-pending");

  await page.evaluate(() => {
    (window as PlaybackTestWindow).rejectPendingPlayback?.();
  });
  await expect(page.getByRole("alert")).toContainText(
    "browser rejected playback",
  );
  await expect(player).toHaveAttribute("data-source", "playable");
  await expect(player).toHaveAttribute("data-transport", "paused");
  await expect(player).toHaveAttribute(
    "data-play-failure",
    "interaction-required",
  );

  await play.click();
  await expect(
    page.getByRole("button", { name: "Pause", exact: true }),
  ).toBeVisible();
  await expect(page.getByRole("alert")).not.toBeVisible();
  await expect(player).toHaveAttribute("data-transport", "playing");
  await expect(player).toHaveAttribute("data-play-failure", "none");

  await page.evaluate(() => {
    const element = (window as PlaybackTestWindow).pendingPlaybackElement;
    if (element) {
      Object.defineProperty(element, "readyState", {
        configurable: true,
        value: 2,
      });
      element.dispatchEvent(new Event("waiting"));
    }
  });
  await expect(player).toHaveAttribute("data-transport", "playing");
  await expect(player).toHaveAttribute("data-readiness", "waiting");

  await page.evaluate(() => {
    (window as PlaybackTestWindow).pendingPlaybackElement?.dispatchEvent(
      new Event("canplay"),
    );
  });
  await expect(player).toHaveAttribute("data-readiness", "playable");

  await page.evaluate(() => {
    (window as PlaybackTestWindow).pendingPlaybackElement?.dispatchEvent(
      new Event("pause"),
    );
  });
  await expect(player).toHaveAttribute("data-transport", "playing");
});

test("replacement and unload ignore stale playback outcomes", async ({
  openRoute,
  page,
}) => {
  await openRoute("/playback", "Load audio only when it is needed");
  await page.evaluate(() => {
    const nativePlay = HTMLMediaElement.prototype.play;
    let holdNext = true;
    HTMLMediaElement.prototype.play = function () {
      if (holdNext) {
        holdNext = false;
        const testWindow = window as PlaybackTestWindow;
        testWindow.pendingPlaybackElement = this;
        return new Promise((_, reject) => {
          testWindow.rejectPendingPlayback = () =>
            reject(new DOMException("Stale rejection", "NotAllowedError"));
        });
      }
      return nativePlay.call(this);
    };
  });

  const player = page.locator(".dioxus-audio__player");
  await page.getByRole("button", { name: "Play", exact: true }).click();
  await expect(player).toHaveAttribute("data-transport", "play-pending");

  await page.getByRole("button", { name: "Replace", exact: true }).click();
  await expect(player).toHaveAttribute("data-source", "playable");
  await expect(player).toHaveAttribute("data-transport", "idle");

  await page.evaluate(() => {
    const testWindow = window as PlaybackTestWindow;
    testWindow.rejectPendingPlayback?.();
    testWindow.pendingPlaybackElement?.dispatchEvent(new Event("error"));
  });
  await expect(player).toHaveAttribute("data-play-failure", "none");
  await expect(page.getByRole("alert")).not.toBeVisible();

  await page.getByRole("button", { name: "Unload", exact: true }).click();
  await expect(player).toHaveAttribute("data-source", "empty");
  await expect(player).toHaveAttribute("data-transport", "idle");
  await expect(player).toHaveAttribute("data-readiness", "unavailable");
});

test("owner unmount ignores a pending play rejection", async ({
  openRoute,
  page,
}) => {
  const pageErrors: Error[] = [];
  page.on("pageerror", (error) => pageErrors.push(error));
  await openRoute("/playback", "Load audio only when it is needed");
  await page.evaluate(() => {
    HTMLMediaElement.prototype.play = function () {
      return new Promise((_, reject) => {
        (window as PlaybackTestWindow).rejectPendingPlayback = () =>
          reject(new DOMException("Late rejection", "NotAllowedError"));
      });
    };
  });

  await page.getByRole("button", { name: "Play", exact: true }).click();
  await expect(page.locator(".dioxus-audio__player")).toHaveAttribute(
    "data-transport",
    "play-pending",
  );

  await page.getByRole("link", { name: "Analysis helpers", exact: true }).click();
  await expect(
    page.getByRole("heading", {
      level: 1,
      name: "Process audio data without a browser",
    }),
  ).toBeVisible();
  await page.evaluate(() => {
    (window as PlaybackTestWindow).rejectPendingPlayback?.();
  });
  await page.waitForTimeout(0);

  expect(pageErrors).toEqual([]);
});

test("playback rate cycles through every documented speed", async ({
  openRoute,
  page,
}) => {
  await openRoute("/playback", "Load audio only when it is needed");

  for (const [current, next] of [
    ["1x", "1.5x"],
    ["1.5x", "2x"],
    ["2x", "1x"],
  ] as const) {
    await page
      .getByRole("button", { name: `Playback speed: ${current}` })
      .click();
    const activeRate = page.getByRole("button", {
      name: `Playback speed: ${next}`,
    });
    await expect(activeRate).toBeVisible();
    await expect(activeRate).toHaveText(next);
  }
});
