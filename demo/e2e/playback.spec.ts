import { expect, test } from "./fixtures";

type PlaybackTestWindow = typeof globalThis & {
  createdPlaybackElements?: HTMLMediaElement[];
  createdObjectUrls?: string[];
  revokedObjectUrls?: string[];
  heldPlaybackElements?: HTMLMediaElement[];
  pendingPlaybackElement?: HTMLMediaElement;
  rejectPendingPlayback?: () => void;
  resolvePendingPlayback?: () => void;
};

test("URL Playback Sources load eagerly or remain dormant until play", async ({
  openRoute,
  page,
}) => {
  await page.addInitScript(() => {
    const NativeAudio = window.Audio;
    const elements: HTMLMediaElement[] = [];
    (window as PlaybackTestWindow).createdPlaybackElements = elements;
    Object.defineProperty(window, "Audio", {
      configurable: true,
      value: function Audio(source?: string) {
        const element = new NativeAudio(source);
        elements.push(element);
        return element;
      },
    });
  });
  await openRoute("/playback-source", "Load local and remote media by URL");

  const example = page.getByRole("group", { name: "URL Playback Source" });
  const state = example.locator(".url-playback-state");
  await expect(state).toHaveAttribute("data-source", "empty");
  await expect(state).toHaveAttribute("data-transport", "idle");
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          (window as PlaybackTestWindow).createdPlaybackElements?.length ?? -1,
      ),
    )
    .toBe(0);

  await example.getByRole("button", { name: "Load eager URL" }).click();
  await expect(state).toHaveAttribute("data-source", "playable");
  await expect(state).toHaveAttribute("data-selected-media-type", "audio/wav");
  await expect(state).toHaveAttribute("data-selected-alternative", /^blob:/);
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          (window as PlaybackTestWindow).createdPlaybackElements?.length ?? 0,
      ),
    )
    .toBe(1);

  await example.getByRole("button", { name: "Load on-play URL" }).click();
  await expect(state).toHaveAttribute("data-source", "dormant");
  await expect(example.getByRole("status")).toHaveText("Audio ready to load");
  await expect(state).toHaveAttribute("data-selected-alternative", "none");
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          (window as PlaybackTestWindow).createdPlaybackElements?.length ?? 0,
      ),
    )
    .toBe(1);
  await expect
    .poll(() =>
      page.evaluate(() => {
        const elements = (window as PlaybackTestWindow).createdPlaybackElements;
        return elements?.[0]?.hasAttribute("src") ?? true;
      }),
    )
    .toBe(false);

  await example.getByRole("button", { name: "Play URL Playback Source" }).click();
  await expect(state).toHaveAttribute("data-source", "playable");
  await expect(state).toHaveAttribute("data-transport", "playing");
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          (window as PlaybackTestWindow).createdPlaybackElements?.length ?? 0,
      ),
    )
    .toBe(2);

  await example.getByRole("button", { name: "Replace URL Playback Source" }).click();
  await expect(state).toHaveAttribute("data-source", "playable");
  await expect(state).toHaveAttribute("data-transport", "idle");
  await example.getByRole("button", { name: "Unload URL Playback Source" }).click();
  await expect(state).toHaveAttribute("data-source", "empty");
  await expect(state).toHaveAttribute("data-selected-alternative", "none");
});

test("pausing a loading on-play URL clears only play intent", async ({
  openRoute,
  page,
}) => {
  await openRoute("/playback-source", "Load local and remote media by URL");
  await page.evaluate(() => {
    const held: HTMLMediaElement[] = [];
    (window as PlaybackTestWindow).heldPlaybackElements = held;
    Object.defineProperty(HTMLMediaElement.prototype, "src", {
      configurable: true,
      get() {
        return this.getAttribute("data-held-src") ?? "";
      },
      set(value: string) {
        this.setAttribute("data-held-src", value);
        held.push(this);
      },
    });
    HTMLMediaElement.prototype.load = function () {};
    HTMLMediaElement.prototype.play = function () {
      (window as PlaybackTestWindow).pendingPlaybackElement = this;
      return new Promise((resolve) => {
        (window as PlaybackTestWindow).resolvePendingPlayback = () => resolve();
      });
    };
    HTMLMediaElement.prototype.pause = function () {};
  });

  const example = page.getByRole("group", { name: "URL Playback Source" });
  const state = example.locator(".url-playback-state");
  await example.getByRole("button", { name: "Load on-play URL" }).click();
  await example.getByRole("button", { name: "Play URL Playback Source" }).click();
  await expect(state).toHaveAttribute("data-source", "loading");
  await expect(state).toHaveAttribute("data-transport", "play-pending");

  await example.getByRole("button", { name: "Pause URL Playback Source" }).click();
  await expect(state).toHaveAttribute("data-source", "loading");
  await expect(state).toHaveAttribute("data-transport", "idle");

  await page.evaluate(() => {
    const element = (window as PlaybackTestWindow).pendingPlaybackElement;
    element?.dispatchEvent(new Event("loadedmetadata"));
    element?.dispatchEvent(new Event("canplay"));
    (window as PlaybackTestWindow).resolvePendingPlayback?.();
  });
  await expect(state).toHaveAttribute("data-source", "playable");
  await expect(state).toHaveAttribute("data-transport", "idle");
  await expect(state).toHaveAttribute("data-selected-media-type", "audio/wav");
});

test("URL failure and stale source outcomes stay scoped to their source", async ({
  openRoute,
  page,
}) => {
  await openRoute("/playback-source", "Load local and remote media by URL");
  await page.evaluate(() => {
    const held: HTMLMediaElement[] = [];
    (window as PlaybackTestWindow).heldPlaybackElements = held;
    Object.defineProperty(HTMLMediaElement.prototype, "src", {
      configurable: true,
      get() {
        return this.getAttribute("data-held-src") ?? "";
      },
      set(value: string) {
        this.setAttribute("data-held-src", value);
        held.push(this);
      },
    });
    HTMLMediaElement.prototype.load = function () {};
    HTMLMediaElement.prototype.play = function () {
      return new Promise(() => {});
    };
  });

  const example = page.getByRole("group", { name: "URL Playback Source" });
  const state = example.locator(".url-playback-state");
  await example.getByRole("button", { name: "Load on-play URL" }).click();
  await example.getByRole("button", { name: "Play URL Playback Source" }).click();
  const first = await page.evaluateHandle(
    () => (window as PlaybackTestWindow).heldPlaybackElements?.[0],
  );
  await page.evaluate((element) => {
    Object.defineProperty(element, "error", {
      configurable: true,
      value: { code: 3 },
    });
    element.dispatchEvent(new Event("error"));
  }, first);
  await expect(state).toHaveAttribute("data-source", "failed");
  await expect(state).toHaveAttribute("data-source-failure", "decode");
  await expect(state).toHaveAttribute("data-selected-alternative", "none");

  await example.getByRole("button", { name: "Replace URL Playback Source" }).click();
  await expect(state).toHaveAttribute("data-source", "loading");
  await page.evaluate((element) => {
    element.dispatchEvent(new Event("canplay"));
    element.dispatchEvent(new Event("error"));
    element.dispatchEvent(new Event("playing"));
  }, first);
  await expect(state).toHaveAttribute("data-source", "loading");
  await expect(state).toHaveAttribute("data-source-failure", "none");

  await example.getByRole("button", { name: "Unload URL Playback Source" }).click();
  await expect(state).toHaveAttribute("data-source", "empty");
  await page.evaluate((element) => {
    element.dispatchEvent(new Event("loadedmetadata"));
    element.dispatchEvent(new Event("canplay"));
    element.dispatchEvent(new Event("error"));
  }, first);
  await expect(state).toHaveAttribute("data-source", "empty");
  await expect(state).toHaveAttribute("data-source-failure", "none");
  await first.dispose();
});

test("the library revokes only its own Audio Data URLs", async ({
  openRoute,
  page,
}) => {
  await page.addInitScript(() => {
    const nativeCreate = URL.createObjectURL;
    const nativeRevoke = URL.revokeObjectURL;
    const created: string[] = [];
    const revoked: string[] = [];
    (window as PlaybackTestWindow).createdObjectUrls = created;
    (window as PlaybackTestWindow).revokedObjectUrls = revoked;
    URL.createObjectURL = function (object) {
      const url = nativeCreate.call(this, object);
      created.push(url);
      return url;
    };
    URL.revokeObjectURL = function (url) {
      revoked.push(url);
      return nativeRevoke.call(this, url);
    };
  });
  await openRoute("/playback-source", "Load local and remote media by URL");

  const urlExample = page.getByRole("group", { name: "URL Playback Source" });
  await urlExample.getByRole("button", { name: "Load eager URL" }).click();
  await expect(urlExample.locator(".url-playback-state")).toHaveAttribute(
    "data-source",
    "playable",
  );
  await urlExample
    .getByRole("button", { name: "Replace URL Playback Source" })
    .click();
  await urlExample
    .getByRole("button", { name: "Unload URL Playback Source" })
    .click();
  const applicationUrls = await page.evaluate(
    () => (window as PlaybackTestWindow).createdObjectUrls?.slice(0, 2) ?? [],
  );
  expect(applicationUrls).toHaveLength(2);
  expect(
    await page.evaluate(
      () => (window as PlaybackTestWindow).revokedObjectUrls ?? [],
    ),
  ).toEqual([]);

  await openRoute("/playback", "Load audio only when it is needed");
  await page.getByRole("button", { name: "Play", exact: true }).click();
  await page.getByRole("button", { name: "Replace", exact: true }).click();
  await page.getByRole("button", { name: "Unload", exact: true }).click();
  const ownership = await page.evaluate(() => ({
    created: (window as PlaybackTestWindow).createdObjectUrls ?? [],
    revoked: (window as PlaybackTestWindow).revokedObjectUrls ?? [],
  }));
  expect(ownership.created).toHaveLength(4);
  expect(ownership.revoked.toSorted()).toEqual(ownership.created.toSorted());
  expect(ownership.revoked).not.toContain(applicationUrls[0]);
  expect(ownership.revoked).not.toContain(applicationUrls[1]);
});

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

test("stop resets pending playback and ignores its late outcome", async ({
  openRoute,
  page,
}) => {
  await openRoute("/playback", "Load audio only when it is needed");
  await page.evaluate(() => {
    HTMLMediaElement.prototype.play = function () {
      const testWindow = window as PlaybackTestWindow;
      testWindow.pendingPlaybackElement = this;
      return new Promise((resolve) => {
        testWindow.resolvePendingPlayback = () => resolve();
      });
    };
  });

  const player = page.locator(".dioxus-audio__player");
  const slider = player.getByRole("slider", { name: "Seek audio" });
  await page.getByRole("button", { name: "Play", exact: true }).click();
  await expect(player).toHaveAttribute("data-transport", "play-pending");

  await page.evaluate(() => {
    const element = (window as PlaybackTestWindow).pendingPlaybackElement;
    if (element) {
      element.currentTime = 1;
      element.dispatchEvent(new Event("timeupdate"));
    }
  });
  await expect(slider).toHaveValue("1");

  const stop = page.getByRole("button", { name: "Stop", exact: true });
  await stop.click();
  await expect(player).toHaveAttribute("data-transport", "idle");
  await expect(slider).toHaveValue("0");
  await expect(stop).toBeDisabled();

  await page.evaluate(() => {
    const testWindow = window as PlaybackTestWindow;
    const element = testWindow.pendingPlaybackElement;
    if (element) {
      Object.defineProperty(element, "paused", {
        configurable: true,
        value: false,
      });
      testWindow.resolvePendingPlayback?.();
      element.currentTime = 1.5;
      element.dispatchEvent(new Event("playing"));
      element.dispatchEvent(new Event("timeupdate"));
      element.dispatchEvent(new Event("pause"));
    }
  });
  await expect(player).toHaveAttribute("data-transport", "idle");
  await expect(slider).toHaveValue("0");
  await expect(page.getByRole("alert")).not.toBeVisible();
});

test("whole-source repeat loops and persists through replacement and unload", async ({
  openRoute,
  page,
}) => {
  await openRoute("/playback", "Load audio only when it is needed");
  await page.evaluate(() => {
    const nativePlay = HTMLMediaElement.prototype.play;
    HTMLMediaElement.prototype.play = function () {
      (window as PlaybackTestWindow).pendingPlaybackElement = this;
      return nativePlay.call(this);
    };
  });

  const player = page.locator(".dioxus-audio__player");
  const repeat = page.getByRole("button", { name: "Repeat", exact: true });
  await expect(repeat).toHaveAttribute("aria-pressed", "false");
  await repeat.focus();
  await repeat.press("Space");
  await expect(repeat).toBeFocused();
  await expect(repeat).toHaveAttribute("aria-pressed", "true");
  await expect(player).toHaveAttribute("data-repeat", "true");

  await page.getByRole("button", { name: "Play", exact: true }).click();
  await expect(player).toHaveAttribute("data-transport", "playing");
  await page.evaluate(() => {
    const element = (window as PlaybackTestWindow).pendingPlaybackElement;
    if (element) {
      element.playbackRate = 4;
      element.currentTime = Math.max(0, element.duration - 0.2);
    }
  });
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          (window as PlaybackTestWindow).pendingPlaybackElement?.currentTime ??
          Infinity,
      ),
    )
    .toBeLessThan(1);
  await expect(player).toHaveAttribute("data-transport", "playing");

  await page.getByRole("button", { name: "Replace", exact: true }).click();
  await expect(repeat).toHaveAttribute("aria-pressed", "true");
  await expect(player).toHaveAttribute("data-repeat", "true");

  await page.getByRole("button", { name: "Unload", exact: true }).click();
  await expect(player).toHaveAttribute("data-source", "empty");
  await expect(repeat).toHaveAttribute("aria-pressed", "true");
  await repeat.press("Space");
  await expect(repeat).toBeFocused();
  await expect(repeat).toHaveAttribute("aria-pressed", "false");
  await expect(player).toHaveAttribute("data-repeat", "false");
});

test("mute and direct audibility preferences remain observable and persistent", async ({
  openRoute,
  page,
}) => {
  await openRoute("/playback", "Load audio only when it is needed");
  await page.evaluate(() => {
    const nativePlay = HTMLMediaElement.prototype.play;
    HTMLMediaElement.prototype.play = function () {
      (window as PlaybackTestWindow).pendingPlaybackElement = this;
      return nativePlay.call(this);
    };
  });

  const player = page.locator(".dioxus-audio__player");
  const mute = page.getByRole("button", { name: "Mute", exact: true });
  const level = page.getByRole("slider", {
    name: "Audibility level",
    exact: true,
  });

  await expect(player).toHaveAttribute(
    "data-audibility-capability",
    "best-effort-media-element",
  );
  await page.getByRole("button", { name: "Play", exact: true }).click();
  await expect(player).toHaveAttribute("data-transport", "playing");
  const seek = page.getByRole("slider", { name: "Seek audio" });
  await page.evaluate(() => {
    const element = (window as PlaybackTestWindow).pendingPlaybackElement;
    if (element) {
      element.playbackRate = 0.25;
      element.currentTime = 1;
      element.dispatchEvent(new Event("timeupdate"));
    }
  });
  await expect(seek).toHaveValue("1");

  await mute.focus();
  await mute.press("Space");
  await expect(mute).toBeFocused();
  await expect(mute).toHaveAttribute("aria-pressed", "true");
  await expect(player).toHaveAttribute("data-muted", "true");
  await expect(player).toHaveAttribute("data-transport", "playing");
  await expect
    .poll(async () => Number(await seek.inputValue()), { timeout: 500 })
    .toBeGreaterThan(0.9);
  await expect
    .poll(() =>
      page.evaluate(() => {
        const element = (window as PlaybackTestWindow).pendingPlaybackElement;
        return element ? { muted: element.muted, paused: element.paused } : null;
      }),
    )
    .toEqual({ muted: true, paused: false });

  await level.fill("0.35");
  await expect(level).toBeFocused();
  await expect(level).toHaveValue("0.35");
  await expect(level).toHaveAttribute("aria-valuetext", "35 percent");
  await expect(player).toHaveAttribute("data-audibility-level", "0.35");
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          (window as PlaybackTestWindow).pendingPlaybackElement?.volume ?? null,
      ),
    )
    .toBe(0.35);

  await expect(page.getByRole("button", { name: "Pause", exact: true })).toBeEnabled();
  await expect(page.getByRole("button", { name: "Stop", exact: true })).toBeEnabled();
  await expect(seek).toBeEnabled();
  await expect(page.getByRole("button", { name: /^Playback speed:/ })).toBeEnabled();

  await page.getByRole("button", { name: "Replace", exact: true }).click();
  await expect(player).toHaveAttribute("data-muted", "true");
  await expect(player).toHaveAttribute("data-audibility-level", "0.35");
  await page.getByRole("button", { name: "Play", exact: true }).click();
  await expect
    .poll(() =>
      page.evaluate(() => {
        const element = (window as PlaybackTestWindow).pendingPlaybackElement;
        return element ? { muted: element.muted, volume: element.volume } : null;
      }),
    )
    .toEqual({ muted: true, volume: 0.35 });

  await page.getByRole("button", { name: "Unload", exact: true }).click();
  await expect(player).toHaveAttribute("data-source", "empty");
  await expect(player).toHaveAttribute("data-muted", "true");
  await expect(player).toHaveAttribute("data-audibility-level", "0.35");

  await page.getByRole("button", { name: "Play", exact: true }).click();
  await expect
    .poll(() =>
      page.evaluate(() => {
        const element = (window as PlaybackTestWindow).pendingPlaybackElement;
        return element ? { muted: element.muted, volume: element.volume } : null;
      }),
    )
    .toEqual({ muted: true, volume: 0.35 });
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
