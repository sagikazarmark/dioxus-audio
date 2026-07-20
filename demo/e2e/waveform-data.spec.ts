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
