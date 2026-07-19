import { expect, test } from "./fixtures";

test("independent playback controls keep focus and configuration", async ({
  openRoute,
  page,
}) => {
  await openRoute("/playback", "Load audio only when it is needed");
  const controls = page.getByRole("group", {
    name: "Independent playback controls",
  });
  const repeat = controls.getByRole("button", {
    name: "Repeat custom tone",
  });
  await expect(repeat).toHaveAttribute("aria-pressed", "false");
  await repeat.focus();
  await repeat.press("Space");
  await expect(repeat).toBeFocused();
  await expect(repeat).toHaveAttribute("aria-pressed", "true");
  await repeat.press("Space");
  await expect(repeat).toHaveAttribute("aria-pressed", "false");

  const mute = controls.getByRole("button", { name: "Mute custom tone" });
  await mute.focus();
  await mute.press("Space");
  await expect(mute).toBeFocused();
  await expect(mute).toHaveAttribute("aria-pressed", "true");

  const audibility = controls.getByRole("slider", {
    name: "Custom tone audibility",
  });
  await audibility.focus();
  await audibility.press("Home");
  await expect(audibility).toBeFocused();
  await expect(audibility).toHaveValue("0");
  await expect(audibility).toHaveAttribute("aria-valuetext", "0 percent");

  const play = controls.getByRole("button", {
    name: "Play custom tone",
    exact: true,
  });
  await play.focus();
  await play.press("Enter");

  const pause = controls.getByRole("button", {
    name: "Pause custom tone",
    exact: true,
  });
  await expect(pause).toBeFocused();
  await expect(
    page.getByRole("button", { name: "Play", exact: true }),
  ).toBeVisible();

  await pause.press("Enter");
  await expect(play).toBeFocused();
  await expect(controls.getByRole("status")).toHaveText("Audio paused");

  const composedPlay = page.getByRole("button", { name: "Play", exact: true });
  await composedPlay.focus();
  await composedPlay.press("Enter");
  const composedPause = page.getByRole("button", {
    name: "Pause",
    exact: true,
  });
  await expect(composedPause).toBeFocused();
  await composedPause.press("Enter");
  await expect(composedPlay).toBeFocused();

  const slider = controls.getByRole("slider", {
    name: "Custom tone position",
  });
  await slider.focus();
  await slider.press("Home");
  await expect(slider).toHaveValue("0");

  const skip = controls.getByRole("button", {
    name: "Advance custom tone by half a second",
  });
  await skip.focus();
  await skip.press("Enter");
  await expect(slider).toHaveValue("0.5");

  const stop = controls.getByRole("button", { name: "Stop custom tone" });
  await stop.focus();
  await stop.press("Enter");
  await expect(slider).toHaveValue("0");
  await expect(stop).toBeDisabled();

  await slider.focus();
  await slider.press("End");
  await expect(slider).toBeFocused();
  await expect(slider).toHaveAttribute(
    "aria-valuetext",
    "2 seconds of 2 seconds",
  );

  const rate = controls.getByRole("button", {
    name: "Listening rate: 1x",
  });
  await rate.focus();
  await rate.press("Enter");
  await expect(
    controls.getByRole("button", { name: "Listening rate: 1.25x" }),
  ).toBeFocused();
});

test("independent recorder controls expose validity and stable pause focus", async ({
  openRoute,
  page,
}) => {
  await openRoute("/recorder", "Capture, inspect, and replay");
  const controls = page.getByRole("group", {
    name: "Independent recorder controls",
  });

  const start = controls.getByRole("button", { name: "Begin custom recording" });
  await expect(start).toBeEnabled();
  await expect(
    controls.getByRole("button", { name: "Discard custom recording" }),
  ).toBeDisabled();
  await expect(
    controls.getByRole("button", { name: "Hold custom recording" }),
  ).toBeDisabled();
  await expect(
    controls.getByRole("button", { name: "Finish custom recording" }),
  ).toBeDisabled();
  await expect(
    controls.getByRole("button", { name: "Clear custom recorded audio" }),
  ).toBeDisabled();

  const composedStart = page.getByRole("button", {
    name: "Start recording",
    exact: true,
  });
  await composedStart.press("Enter");
  const composedPause = page.getByRole("button", {
    name: "Pause",
    exact: true,
  });
  await expect(composedPause).toBeEnabled();
  await composedPause.focus();
  await composedPause.press("Enter");
  const composedResume = page.getByRole("button", {
    name: "Resume",
    exact: true,
  });
  await expect(composedResume).toBeFocused();
  await composedResume.press("Enter");
  await expect(composedPause).toBeFocused();
  await page
    .getByRole("button", { name: "Cancel recording", exact: true })
    .click();
  await expect(composedStart).toBeEnabled();

  await start.focus();
  await start.press("Enter");
  const pause = controls.getByRole("button", {
    name: "Hold custom recording",
  });
  await expect(pause).toBeEnabled();
  await pause.focus();
  await pause.press("Enter");
  const resume = controls.getByRole("button", {
    name: "Continue custom recording",
  });
  await expect(resume).toBeFocused();
  const announcement = controls.getByRole("status");
  await expect(announcement).toHaveText("Custom recording on hold");
  const pausedAnnouncement = await announcement.textContent();
  await page.waitForTimeout(250);
  expect(await announcement.textContent()).toBe(pausedAnnouncement);

  await resume.press("Enter");
  await expect(pause).toBeFocused();
  await expect(announcement).toHaveText("Custom recording active");

  await controls
    .getByRole("button", { name: "Finish custom recording" })
    .press("Enter");
  const clear = controls.getByRole("button", {
    name: "Clear custom recorded audio",
    exact: true,
  });
  await expect(clear).toBeEnabled();
  await clear.focus();
  await clear.press("Enter");
  await expect(clear).toBeDisabled();
  await expect(start).toBeEnabled();
});
