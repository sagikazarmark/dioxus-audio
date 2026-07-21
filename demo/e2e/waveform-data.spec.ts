import { expect, test } from "./fixtures";

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
