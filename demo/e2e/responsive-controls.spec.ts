import { expect, test } from "./fixtures";

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

  const audibility = page.getByRole("slider", { name: "Audibility level" });
  await audibility.scrollIntoViewIfNeeded();
  await expect(audibility).toBeInViewport();
  await audibility.fill("0.5");
  await expect(audibility).toHaveAttribute("aria-valuetext", "50 percent");

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
