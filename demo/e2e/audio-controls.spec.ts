import { expect, test } from "./fixtures";

test("generated audio can be played, paused, and resumed", async ({
  openRoute,
  page,
}) => {
  await openRoute("/playback", "Load audio only when it is needed");

  await expect(
    page.getByText("Audio loads on first play", { exact: true }),
  ).toBeVisible();
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
    const play = HTMLMediaElement.prototype.play;
    let rejectNext = true;
    HTMLMediaElement.prototype.play = function () {
      if (rejectNext) {
        rejectNext = false;
        return Promise.reject(
          new DOMException("Playback blocked by test", "NotAllowedError"),
        );
      }
      return play.call(this);
    };
  });

  const play = page.getByRole("button", { name: "Play", exact: true });
  await play.click();
  await expect(page.getByRole("alert")).toContainText(
    "browser rejected playback",
  );

  await play.click();
  await expect(
    page.getByRole("button", { name: "Pause", exact: true }),
  ).toBeVisible();
  await expect(page.getByRole("alert")).not.toBeVisible();
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

test("audio controls remain operable in the constrained viewport", async ({
  openRoute,
  page,
}) => {
  await page.setViewportSize({ width: 568, height: 320 });
  await openRoute("/playback", "Load audio only when it is needed");

  const waveform = page.getByRole("img", { name: "Generated tone waveform" });
  await waveform.scrollIntoViewIfNeeded();
  await expect(waveform).toBeInViewport();

  const rate = page.getByRole("button", { name: "Playback speed: 1x" });
  await rate.scrollIntoViewIfNeeded();
  await expect(rate).toBeInViewport();
  await rate.click();
  await expect(
    page.getByRole("button", { name: "Playback speed: 1.5x" }),
  ).toBeInViewport();

  const play = page.getByRole("button", { name: "Play", exact: true });
  await play.click();
  const pause = page.getByRole("button", { name: "Pause", exact: true });
  await expect(pause).toBeInViewport();
  await pause.click();

  await page.getByRole("link", { name: "Record and review" }).click();
  await expect(
    page.getByRole("heading", {
      level: 1,
      name: "Capture, inspect, and replay",
    }),
  ).toBeVisible();

  const start = page.getByRole("button", { name: "Start recording" });
  await start.scrollIntoViewIfNeeded();
  await expect(start).toBeInViewport();
  await start.click();

  const cancel = page.getByRole("button", { name: "Cancel recording" });
  await expect(cancel).toBeVisible();
  await cancel.scrollIntoViewIfNeeded();
  await expect(cancel).toBeInViewport();
  await cancel.click();
  await expect(start).toBeVisible();
});
