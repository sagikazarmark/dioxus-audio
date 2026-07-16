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
