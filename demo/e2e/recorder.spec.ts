import { expect, test } from "./fixtures";

test("best-effort chunk boundaries can be requested while active or paused", async ({
  openRoute,
  page,
}) => {
  await page.addInitScript(() => {
    const browser = globalThis as typeof globalThis & {
      requestedChunkBoundaries?: number;
    };
    MediaRecorder.prototype.requestData = function () {
      const boundary = (browser.requestedChunkBoundaries ?? 0) + 1;
      browser.requestedChunkBoundaries = boundary;
      const bytes = new Uint8Array(6 + boundary);
      this.dispatchEvent(
        new BlobEvent("dataavailable", {
          data: new Blob([bytes], { type: this.mimeType }),
        }),
      );
    };
  });
  await openRoute("/recorder", "Capture, inspect, and replay");
  const demo = page.getByRole("region", { name: "Example demo" });
  const lifecycle = demo.getByRole("log", {
    name: "Recording lifecycle events",
  });
  const requestBoundary = demo.getByRole("button", {
    name: "Request chunk boundary",
  });

  await expect(requestBoundary).toBeDisabled();
  await demo.getByRole("button", { name: "Start recording" }).click();
  await expect(requestBoundary).toBeEnabled();
  await requestBoundary.click();
  await expect
    .poll(() =>
      page.evaluate(() => {
        const browser = globalThis as typeof globalThis & {
          requestedChunkBoundaries?: number;
        };
        return browser.requestedChunkBoundaries ?? 0;
      }),
    )
    .toBe(1);
  await expect
    .poll(async () =>
      (await lifecycle.locator("li").allTextContents()).some((event) =>
        event.includes("| bytes 7 |"),
      ),
    )
    .toBe(true);

  await demo.getByRole("button", { name: "Pause", exact: true }).click();
  await expect(
    demo.getByText("Recording paused", { exact: true }),
  ).toBeVisible();
  await expect(requestBoundary).toBeEnabled();
  await requestBoundary.click();
  await expect
    .poll(() =>
      page.evaluate(() => {
        const browser = globalThis as typeof globalThis & {
          requestedChunkBoundaries?: number;
        };
        return browser.requestedChunkBoundaries ?? 0;
      }),
    )
    .toBe(2);
  await expect
    .poll(async () =>
      (await lifecycle.locator("li").allTextContents()).some((event) =>
        event.includes("| bytes 8 |"),
      ),
    )
    .toBe(true);
  await expect(
    demo.getByText("Recording paused", { exact: true }),
  ).toBeVisible();

  const chunks = (await lifecycle.locator("li").allTextContents())
    .map((event) =>
      event.match(
        /^Primary chunk \| (Recorder \d+ Recording \d+) \| sequence (\d+) \| bytes (\d+) \|/,
      ),
    )
    .filter((match): match is RegExpMatchArray => match !== null);
  expect(chunks.map((chunk) => Number(chunk[2]))).toEqual(
    chunks.map((_, sequence) => sequence),
  );
  const requestedChunks = chunks.filter((chunk) =>
    [7, 8].includes(Number(chunk[3])),
  );
  expect(requestedChunks).toHaveLength(2);
  expect(new Set(requestedChunks.map((chunk) => chunk[1])).size).toBe(1);
});

test("incremental conversion failure ends delivery but not final completion", async ({
  openRoute,
  page,
}) => {
  await page.addInitScript(() => {
    const browser = globalThis as typeof globalThis & {
      failBoundaryConversion?: boolean;
      failNextDataEvent?: boolean;
      failedConversion?: boolean;
    };
    const failedBlobs = new WeakSet<Blob>();
    const requestData = MediaRecorder.prototype.requestData;
    MediaRecorder.prototype.requestData = function () {
      if (browser.failBoundaryConversion) {
        browser.failBoundaryConversion = false;
        browser.failNextDataEvent = true;
      }
      return requestData.call(this);
    };
    const onDataAvailable = Object.getOwnPropertyDescriptor(
      MediaRecorder.prototype,
      "ondataavailable",
    );
    if (onDataAvailable?.get && onDataAvailable.set) {
      Object.defineProperty(MediaRecorder.prototype, "ondataavailable", {
        configurable: true,
        get: onDataAvailable.get,
        set(handler: ((event: BlobEvent) => void) | null) {
          const wrapped = handler
            ? (event: BlobEvent) => {
                if (browser.failNextDataEvent) {
                  browser.failNextDataEvent = false;
                  failedBlobs.add(event.data);
                }
                handler.call(this, event);
              }
            : null;
          onDataAvailable.set?.call(this, wrapped);
        },
      });
    }
    const arrayBuffer = Blob.prototype.arrayBuffer;
    Blob.prototype.arrayBuffer = function () {
      if (failedBlobs.has(this)) {
        failedBlobs.delete(this);
        browser.failedConversion = true;
        return Promise.reject("forced incremental conversion failure");
      }
      return arrayBuffer.call(this);
    };
  });
  await openRoute("/recorder", "Capture, inspect, and replay");
  const demo = page.getByRole("region", { name: "Example demo" });
  const lifecycle = demo.getByRole("log", {
    name: "Recording lifecycle events",
  });

  await demo.getByRole("button", { name: "Start recording" }).click();
  await expect
    .poll(async () =>
      (await lifecycle.locator("li").allTextContents()).filter((event) =>
        event.startsWith("Primary chunk"),
      ).length,
    )
    .toBeGreaterThan(0);
  await page.evaluate(() => {
    const browser = globalThis as typeof globalThis & {
      failBoundaryConversion?: boolean;
    };
    browser.failBoundaryConversion = true;
  });
  await demo.getByRole("button", { name: "Request chunk boundary" }).click();

  await expect
    .poll(async () =>
      (await lifecycle.locator("li").allTextContents()).find((event) =>
        event.startsWith("Chunk delivery failed"),
      ),
    )
    .toMatch(/^Chunk delivery failed \| Recorder \d+ Recording \d+ \| sequence \d+ \|/);
  expect(
    await page.evaluate(() => {
      const browser = globalThis as typeof globalThis & {
        failedConversion?: boolean;
      };
      return browser.failedConversion;
    }),
  ).toBe(true);
  await expect(demo.getByText("Recording", { exact: true }).first()).toBeVisible();

  const eventsAtFailure = await lifecycle.locator("li").allTextContents();
  const failure = eventsAtFailure
    .find((event) => event.startsWith("Chunk delivery failed"))
    ?.match(
      /^Chunk delivery failed \| (Recorder \d+ Recording \d+) \| sequence (\d+) \|/,
    );
  const transferred = eventsAtFailure.filter((event) =>
    event.startsWith("Primary chunk"),
  );
  expect(failure).not.toBeNull();
  expect(Number(failure?.[2])).toBe(transferred.length);

  await page.waitForTimeout(500);
  expect(
    (await lifecycle.locator("li").allTextContents()).filter((event) =>
      event.startsWith("Primary chunk"),
    ),
  ).toHaveLength(transferred.length);

  await demo.getByRole("button", { name: "Stop recording" }).click();
  await expect(
    demo.getByRole("status").filter({ hasText: "Recording ready" }),
  ).toBeVisible();
  const finalEvents = await lifecycle.locator("li").allTextContents();
  expect(
    finalEvents.filter((event) => event.startsWith("Primary chunk")),
  ).toHaveLength(transferred.length);
  expect(finalEvents.at(-1)).toMatch(
    new RegExp(`^Completed \\| ${failure?.[1]} \\| bytes [1-9]\\d*$`),
  );
});

test("a Recorder failure terminates chunk delivery and completed output", async ({
  openRoute,
  page,
}) => {
  await page.addInitScript(() => {
    const browser = globalThis as typeof globalThis & {
      activeMediaRecorder?: MediaRecorder;
    };
    const onError = Object.getOwnPropertyDescriptor(
      MediaRecorder.prototype,
      "onerror",
    );
    if (onError?.get && onError.set) {
      Object.defineProperty(MediaRecorder.prototype, "onerror", {
        configurable: true,
        get: onError.get,
        set(handler: ((event: Event) => void) | null) {
          browser.activeMediaRecorder = this;
          onError.set?.call(this, handler);
        },
      });
    }
  });
  await openRoute("/recorder", "Capture, inspect, and replay");
  const demo = page.getByRole("region", { name: "Example demo" });
  const lifecycle = demo.getByRole("log", {
    name: "Recording lifecycle events",
  });

  await demo.getByRole("button", { name: "Start recording" }).click();
  await expect
    .poll(async () =>
      (await lifecycle.locator("li").allTextContents()).filter((event) =>
        event.startsWith("Primary chunk"),
      ).length,
    )
    .toBeGreaterThan(0);
  const firstChunk = (await lifecycle.locator("li").allTextContents())
    .find((event) => event.startsWith("Primary chunk"))
    ?.match(/^Primary chunk \| (Recorder \d+ Recording \d+) \|/);

  await page.evaluate(() => {
    const browser = globalThis as typeof globalThis & {
      activeMediaRecorder?: MediaRecorder;
    };
    browser.activeMediaRecorder?.dispatchEvent(new Event("error"));
    browser.activeMediaRecorder?.stop();
  });

  await expect
    .poll(async () =>
      (await lifecycle.locator("li").allTextContents()).filter((event) =>
        event.startsWith("Failed"),
      ),
    )
    .toEqual([
      `Failed | ${firstChunk?.[1]} | media recorder failed`,
    ]);
  await expect(
    demo.getByRole("status").filter({ hasText: "Recording ready" }),
  ).toHaveCount(0);
  const eventsAtFailure = await lifecycle.locator("li").allTextContents();
  const transferredAtFailure = eventsAtFailure.filter((event) =>
    event.startsWith("Primary chunk"),
  ).length;
  await page.waitForTimeout(500);
  const finalEvents = await lifecycle.locator("li").allTextContents();
  expect(
    finalEvents.filter((event) => event.startsWith("Primary chunk")),
  ).toHaveLength(transferredAtFailure);
  expect(finalEvents.filter((event) => event.startsWith("Failed"))).toHaveLength(
    1,
  );
  expect(finalEvents.some((event) => event.startsWith("Completed"))).toBe(false);
});

test("final assembly failure publishes one identified failure and no Recorded Audio", async ({
  openRoute,
  page,
}) => {
  await page.addInitScript(() => {
    const browser = globalThis as typeof globalThis & {
      failFinalAssembly?: boolean;
    };
    const fragments = new WeakSet<Blob>();
    const onDataAvailable = Object.getOwnPropertyDescriptor(
      MediaRecorder.prototype,
      "ondataavailable",
    );
    if (onDataAvailable?.get && onDataAvailable.set) {
      Object.defineProperty(MediaRecorder.prototype, "ondataavailable", {
        configurable: true,
        get: onDataAvailable.get,
        set(handler: ((event: BlobEvent) => void) | null) {
          const wrapped = handler
            ? (event: BlobEvent) => {
                fragments.add(event.data);
                handler.call(this, event);
              }
            : null;
          onDataAvailable.set?.call(this, wrapped);
        },
      });
    }
    const arrayBuffer = Blob.prototype.arrayBuffer;
    Blob.prototype.arrayBuffer = function () {
      if (browser.failFinalAssembly && !fragments.has(this)) {
        browser.failFinalAssembly = false;
        return Promise.reject("forced final assembly failure");
      }
      return arrayBuffer.call(this);
    };
  });
  await openRoute("/recorder", "Capture, inspect, and replay");
  const demo = page.getByRole("region", { name: "Example demo" });
  const lifecycle = demo.getByRole("log", {
    name: "Recording lifecycle events",
  });

  await demo.getByRole("button", { name: "Start recording" }).click();
  await expect
    .poll(async () =>
      (await lifecycle.locator("li").allTextContents()).filter((event) =>
        event.startsWith("Primary chunk"),
      ).length,
    )
    .toBeGreaterThan(0);
  const firstChunk = (await lifecycle.locator("li").allTextContents())
    .find((event) => event.startsWith("Primary chunk"))
    ?.match(/^Primary chunk \| (Recorder \d+ Recording \d+) \|/);
  await page.evaluate(() => {
    const browser = globalThis as typeof globalThis & {
      failFinalAssembly?: boolean;
    };
    browser.failFinalAssembly = true;
  });
  await demo.getByRole("button", { name: "Stop recording" }).click();

  await expect
    .poll(async () =>
      (await lifecycle.locator("li").allTextContents()).filter((event) =>
        event.startsWith("Failed"),
      ),
    )
    .toEqual([
      `Failed | ${firstChunk?.[1]} | forced final assembly failure`,
    ]);
  await expect(
    demo.getByRole("status").filter({ hasText: "Recording ready" }),
  ).toHaveCount(0);
  const finalEvents = await lifecycle.locator("li").allTextContents();
  expect(finalEvents.filter((event) => event.startsWith("Failed"))).toHaveLength(
    1,
  );
  expect(finalEvents.some((event) => event.startsWith("Completed"))).toBe(false);
});

test("discard invalidates a requested boundary conversion", async ({
  openRoute,
  page,
}) => {
  await page.addInitScript(() => {
    const browser = globalThis as typeof globalThis & {
      delayBoundaryConversion?: boolean;
      delayNextDataEvent?: boolean;
      boundaryConversionStarted?: boolean;
    };
    const delayedBlobs = new WeakSet<Blob>();
    const requestData = MediaRecorder.prototype.requestData;
    MediaRecorder.prototype.requestData = function () {
      if (browser.delayBoundaryConversion) {
        browser.delayBoundaryConversion = false;
        browser.delayNextDataEvent = true;
      }
      return requestData.call(this);
    };
    const onDataAvailable = Object.getOwnPropertyDescriptor(
      MediaRecorder.prototype,
      "ondataavailable",
    );
    if (onDataAvailable?.get && onDataAvailable.set) {
      Object.defineProperty(MediaRecorder.prototype, "ondataavailable", {
        configurable: true,
        get: onDataAvailable.get,
        set(handler: ((event: BlobEvent) => void) | null) {
          const wrapped = handler
            ? (event: BlobEvent) => {
                if (browser.delayNextDataEvent) {
                  browser.delayNextDataEvent = false;
                  delayedBlobs.add(event.data);
                }
                handler.call(this, event);
              }
            : null;
          onDataAvailable.set?.call(this, wrapped);
        },
      });
    }
    const arrayBuffer = Blob.prototype.arrayBuffer;
    Blob.prototype.arrayBuffer = async function () {
      if (delayedBlobs.has(this)) {
        delayedBlobs.delete(this);
        browser.boundaryConversionStarted = true;
        await new Promise((resolve) => setTimeout(resolve, 500));
      }
      return arrayBuffer.call(this);
    };
  });
  await openRoute("/recorder", "Capture, inspect, and replay");
  const demo = page.getByRole("region", { name: "Example demo" });
  const lifecycle = demo.getByRole("log", {
    name: "Recording lifecycle events",
  });

  await demo.getByRole("button", { name: "Start recording" }).click();
  await expect
    .poll(async () =>
      (await lifecycle.locator("li").allTextContents()).filter((event) =>
        event.startsWith("Primary chunk"),
      ).length,
    )
    .toBeGreaterThan(0);
  await page.evaluate(() => {
    const browser = globalThis as typeof globalThis & {
      delayBoundaryConversion?: boolean;
    };
    browser.delayBoundaryConversion = true;
  });
  await demo.getByRole("button", { name: "Request chunk boundary" }).click();
  await expect
    .poll(() =>
      page.evaluate(() => {
        const browser = globalThis as typeof globalThis & {
          boundaryConversionStarted?: boolean;
        };
        return browser.boundaryConversionStarted ?? false;
      }),
    )
    .toBe(true);
  const chunksBeforeDiscard = (
    await lifecycle.locator("li").allTextContents()
  ).filter((event) => event.startsWith("Primary chunk")).length;

  await demo.getByRole("button", { name: "Cancel recording" }).click();
  await expect
    .poll(async () =>
      (await lifecycle.locator("li").allTextContents()).filter((event) =>
        event.startsWith("Discarded"),
      ).length,
    )
    .toBe(1);
  await page.waitForTimeout(700);

  const finalEvents = await lifecycle.locator("li").allTextContents();
  expect(
    finalEvents.filter((event) => event.startsWith("Primary chunk")),
  ).toHaveLength(chunksBeforeDiscard);
  expect(
    finalEvents.some((event) => event.startsWith("Chunk delivery failed")),
  ).toBe(false);
  expect(finalEvents.some((event) => event.startsWith("Completed"))).toBe(false);
  await expect(
    demo.getByRole("status").filter({ hasText: "Recording ready" }),
  ).toHaveCount(0);
});

test("a recording can be paused, resumed, completed, and played", async ({
  openRoute,
  page,
}) => {
  await page.addInitScript(() => {
    const browser = globalThis as typeof globalThis & {
      capturedRecorderConstraints?: MediaStreamConstraints[];
      activeChunkConversions?: number;
      maxChunkConversions?: number;
    };
    const mediaDevices = navigator.mediaDevices;
    const getUserMedia = mediaDevices.getUserMedia.bind(mediaDevices);
    Object.defineProperty(mediaDevices, "getUserMedia", {
      value: (constraints: MediaStreamConstraints) => {
        (browser.capturedRecorderConstraints ??= []).push(constraints);
        return getUserMedia(constraints);
      },
    });
    const arrayBuffer = Blob.prototype.arrayBuffer;
    Blob.prototype.arrayBuffer = async function () {
      browser.activeChunkConversions =
        (browser.activeChunkConversions ?? 0) + 1;
      browser.maxChunkConversions = Math.max(
        browser.maxChunkConversions ?? 0,
        browser.activeChunkConversions,
      );
      await new Promise((resolve) => setTimeout(resolve, 150));
      try {
        return await arrayBuffer.call(this);
      } finally {
        browser.activeChunkConversions -= 1;
      }
    };
  });
  await openRoute("/recorder", "Capture, inspect, and replay");
  const demo = page.getByRole("region", { name: "Example demo" });

  await expect(
    demo.getByRole("combobox", { name: "Audio input" }),
  ).toBeVisible();
  await demo.getByRole("button", { name: "Start recording" }).click();

  await expect(
    demo.getByText("Requested sample rate: ideal 48000 Hz"),
  ).toBeVisible();
  const audioConstraints = await page.evaluate(() => {
    const browser = globalThis as typeof globalThis & {
      capturedRecorderConstraints?: MediaStreamConstraints[];
    };
    return browser.capturedRecorderConstraints?.at(-1)?.audio;
  });
  expect(audioConstraints).toMatchObject({
    channelCount: { ideal: 1 },
    sampleRate: { ideal: 48_000 },
    echoCancellation: { ideal: false },
    noiseSuppression: { ideal: false },
    latency: { ideal: 0.02 },
  });
  await demo
    .getByRole("button", { name: "Use 44100 Hz for future recordings" })
    .click();
  await expect(
    demo.getByText("Requested sample rate: ideal 48000 Hz"),
  ).toBeVisible();
  await expect(demo.getByText(/Effective sample rate: \d+ Hz/)).toBeVisible();
  await expect(demo.getByText(/Selected media type: audio\//)).toBeVisible();
  const selectedMediaType = (
    await demo.getByText(/Selected media type: audio\//).textContent()
  )?.replace("Selected media type: ", "");
  const lifecycle = demo.getByRole("log", {
    name: "Recording lifecycle events",
  });
  await expect
    .poll(async () =>
      (await lifecycle.locator("li").allTextContents()).filter((event) =>
        event.startsWith("Primary chunk"),
      ).length,
    )
    .toBeGreaterThan(0);
  const chunksBeforePause = (
    await lifecycle.locator("li").allTextContents()
  ).filter((event) => event.startsWith("Primary chunk")).length;
  await demo
    .getByRole("button", {
      name: "Use alternate chunk callback for future recordings",
    })
    .click();

  const pause = demo.getByRole("button", { name: "Pause", exact: true });
  await expect(pause).toBeVisible();
  await pause.click();
  await expect(
    demo.getByText("Recording paused", { exact: true }),
  ).toBeVisible();

  await demo.getByRole("button", { name: "Resume", exact: true }).click();
  await expect(pause).toBeVisible();
  await page.waitForTimeout(1_000);
  await demo.getByRole("button", { name: "Stop recording" }).click();

  const completed = demo
    .getByRole("status")
    .filter({ hasText: "Recording ready" });
  await expect(completed).toContainText(
    /\d+:\d{2} \| audio\/[^|]+ \| [1-9]\d* bytes/,
  );

  const firstRecordingEvents = await lifecycle.locator("li").allTextContents();
  const firstChunks = firstRecordingEvents
    .map((event) =>
      event.match(
        /^Primary chunk \| (Recorder \d+ Recording \d+) \| sequence (\d+) \| bytes (\d+) \| (audio\/.+)$/,
      ),
    )
    .filter((match): match is RegExpMatchArray => match !== null);
  expect(firstChunks.length).toBeGreaterThan(1);
  expect(firstChunks.length).toBeGreaterThan(chunksBeforePause);
  expect(firstChunks.map((chunk) => Number(chunk[2]))).toEqual(
    firstChunks.map((_, sequence) => sequence),
  );
  expect(new Set(firstChunks.map((chunk) => chunk[1])).size).toBe(1);
  expect(new Set(firstChunks.map((chunk) => chunk[4]))).toEqual(
    new Set([selectedMediaType]),
  );
  expect(firstChunks.every((chunk) => Number(chunk[3]) > 0)).toBe(true);
  expect(firstRecordingEvents.some((event) => event.startsWith("Alternate chunk"))).toBe(
    false,
  );
  const completion = firstRecordingEvents.at(-1)?.match(
    /^Completed \| (Recorder \d+ Recording \d+) \| bytes (\d+)$/,
  );
  expect(completion).not.toBeNull();
  expect(completion?.[1]).toBe(firstChunks[0][1]);
  expect(Number(completion?.[2])).toBe(
    firstChunks.reduce((total, chunk) => total + Number(chunk[3]), 0),
  );
  expect(
    await page.evaluate(() => {
      const browser = globalThis as typeof globalThis & {
        maxChunkConversions?: number;
      };
      return browser.maxChunkConversions;
    }),
  ).toBe(1);

  const waveform = demo.getByRole("img", { name: "Recorded waveform" });
  await expect(waveform).toBeVisible();
  await expect(waveform.locator("rect").first()).toBeVisible();

  await demo.getByRole("button", { name: "Start recording" }).click();
  await expect(
    demo.getByText("Requested sample rate: ideal 44100 Hz"),
  ).toBeVisible();
  await expect
    .poll(async () =>
      page.evaluate(() => {
        const browser = globalThis as typeof globalThis & {
          capturedRecorderConstraints?: MediaStreamConstraints[];
        };
        const audio = browser.capturedRecorderConstraints?.at(-1)?.audio;
        return typeof audio === "object" && audio !== null
          ? (audio as { sampleRate?: { ideal?: number } }).sampleRate?.ideal
          : undefined;
      }),
    )
    .toBe(44_100);
  await expect
    .poll(async () =>
      (await lifecycle.locator("li").allTextContents()).filter((event) =>
        event.startsWith("Alternate chunk"),
      ).length,
    )
    .toBeGreaterThan(0);
  const beforeDiscard = await lifecycle.locator("li").allTextContents();
  const firstAlternate = beforeDiscard
    .find((event) => event.startsWith("Alternate chunk"))
    ?.match(
      /^Alternate chunk \| (Recorder \d+ Recording \d+) \| sequence (\d+) \|/,
    );
  expect(firstAlternate?.[1]).not.toBe(firstChunks[0][1]);
  expect(firstAlternate?.[2]).toBe("0");
  await demo.getByRole("button", { name: "Cancel recording" }).click();
  await expect
    .poll(async () =>
      (await lifecycle.locator("li").allTextContents()).some((event) =>
        event.startsWith("Discarded"),
      ),
    )
    .toBe(true);
  const transferredAtDiscard = (
    await lifecycle.locator("li").allTextContents()
  ).filter((event) => event.startsWith("Alternate chunk")).length;
  await page.waitForTimeout(500);
  const afterDiscard = await lifecycle.locator("li").allTextContents();
  expect(
    afterDiscard.filter((event) => event.startsWith("Alternate chunk")).length,
  ).toBe(transferredAtDiscard);

  await demo.getByRole("button", { name: "Play", exact: true }).click();
  await expect(
    demo.getByRole("button", { name: "Pause", exact: true }),
  ).toBeVisible();
});

test("source end drains final chunks before completion", async ({
  openRoute,
  page,
}) => {
  await page.addInitScript(() => {
    const browser = globalThis as typeof globalThis & {
      recorderTrack?: MediaStreamTrack;
      finalBlobSize?: number;
      finalConvertedBlobSize?: number;
      captureNextDataEvent?: boolean;
      captureNextConversion?: boolean;
      finalConversionStarted?: boolean;
      finalConversionActive?: boolean;
      recorderEndedListener?: EventListenerOrEventListenerObject | null;
    };
    const addEventListener = MediaStreamTrack.prototype.addEventListener;
    MediaStreamTrack.prototype.addEventListener = function (
      type,
      listener,
      options,
    ) {
      if (type === "ended") {
        browser.recorderTrack = this;
        browser.recorderEndedListener = listener;
      }
      return addEventListener.call(this, type, listener, options);
    };
    const onDataAvailable = Object.getOwnPropertyDescriptor(
      MediaRecorder.prototype,
      "ondataavailable",
    );
    if (onDataAvailable?.get && onDataAvailable.set) {
      Object.defineProperty(MediaRecorder.prototype, "ondataavailable", {
        configurable: true,
        get: onDataAvailable.get,
        set(handler: ((event: BlobEvent) => void) | null) {
          const wrapped = handler
            ? (event: BlobEvent) => {
                if (browser.captureNextDataEvent) {
                  browser.captureNextDataEvent = false;
                  browser.captureNextConversion = true;
                  browser.finalBlobSize = event.data.size;
                }
                handler.call(this, event);
              }
            : null;
          onDataAvailable.set?.call(this, wrapped);
        },
      });
    }
    const arrayBuffer = Blob.prototype.arrayBuffer;
    Blob.prototype.arrayBuffer = async function () {
      if (browser.captureNextConversion) {
        browser.captureNextConversion = false;
        browser.finalConvertedBlobSize = this.size;
        browser.finalConversionStarted = true;
        browser.finalConversionActive = true;
        await new Promise((resolve) => setTimeout(resolve, 250));
        try {
          return await arrayBuffer.call(this);
        } finally {
          browser.finalConversionActive = false;
        }
      }
      return arrayBuffer.call(this);
    };
  });
  await openRoute("/recorder", "Capture, inspect, and replay");
  const demo = page.getByRole("region", { name: "Example demo" });
  const lifecycle = demo.getByRole("log", {
    name: "Recording lifecycle events",
  });

  await demo.getByRole("button", { name: "Start recording" }).click();
  await expect
    .poll(async () => lifecycle.locator("li").count())
    .toBeGreaterThan(0);
  const chunksBeforeSourceEnd = await lifecycle.locator("li").count();
  await page.waitForTimeout(50);
  await page.evaluate(() => {
    const browser = globalThis as typeof globalThis & {
      recorderTrack?: MediaStreamTrack;
      recorderEndedListener?: EventListenerOrEventListenerObject | null;
      captureNextDataEvent?: boolean;
    };
    browser.captureNextDataEvent = true;
    const listener = browser.recorderEndedListener;
    const event = new Event("ended");
    if (typeof listener === "function") {
      listener.call(browser.recorderTrack, event);
    } else {
      listener?.handleEvent(event);
    }
  });
  await expect
    .poll(() =>
      page.evaluate(() => {
        const browser = globalThis as typeof globalThis & {
          finalConversionStarted?: boolean;
        };
        return browser.finalConversionStarted ?? false;
      }),
    )
    .toBe(true);
  const finalBlobSize = await page.evaluate(() => {
    const browser = globalThis as typeof globalThis & {
      finalBlobSize?: number;
      finalConvertedBlobSize?: number;
      finalConversionActive?: boolean;
    };
    return {
      size: browser.finalBlobSize ?? 0,
      convertedSize: browser.finalConvertedBlobSize ?? 0,
      conversionActive: browser.finalConversionActive ?? false,
    };
  });
  expect(finalBlobSize.size).toBeGreaterThan(0);
  expect(finalBlobSize.convertedSize).toBe(finalBlobSize.size);
  expect(finalBlobSize.conversionActive).toBe(true);
  await expect(
    demo.getByRole("status").filter({ hasText: "Recording ready" }),
  ).toHaveCount(0);

  await expect(
    demo.getByRole("status").filter({ hasText: "Recording ready" }),
  ).toBeVisible();
  await expect(
    demo.getByText("Recording completion cause: SourceEnded", { exact: true }),
  ).toBeVisible();
  const events = await lifecycle.locator("li").allTextContents();
  expect(events.length - 1).toBeGreaterThan(chunksBeforeSourceEnd);
  expect(events.at(-1)).toMatch(
    /^Completed \| Recorder \d+ Recording \d+/,
  );
  expect(events.slice(0, -1).every((event) => event.includes("chunk"))).toBe(true);
  const finalChunkBytes = events.at(-2)?.match(/\| bytes (\d+) \|/)?.[1];
  expect(Number(finalChunkBytes)).toBe(finalBlobSize.size);
  await page.waitForTimeout(300);
  await expect(lifecycle.locator("li")).toHaveCount(events.length);
});

test("source-ended completion survives an already inactive browser recorder", async ({
  openRoute,
  page,
}) => {
  await page.addInitScript(() => {
    const browser = globalThis as typeof globalThis & {
      suppliedTrack?: MediaStreamTrack;
      activeSuppliedRecorder?: MediaRecorder;
      suppliedEndedListener?: EventListenerOrEventListenerObject | null;
    };
    const mediaDevices = navigator.mediaDevices;
    const getUserMedia = mediaDevices.getUserMedia.bind(mediaDevices);
    Object.defineProperty(mediaDevices, "getUserMedia", {
      value: async (constraints: MediaStreamConstraints) => {
        const stream = await getUserMedia(constraints);
        browser.suppliedTrack = stream.getAudioTracks()[0];
        return stream;
      },
    });
    const addEventListener = MediaStreamTrack.prototype.addEventListener;
    MediaStreamTrack.prototype.addEventListener = function (
      type,
      listener,
      options,
    ) {
      if (type === "ended") {
        browser.suppliedEndedListener = listener;
      }
      return addEventListener.call(this, type, listener, options);
    };
    const start = MediaRecorder.prototype.start;
    MediaRecorder.prototype.start = function (timeslice?: number) {
      browser.activeSuppliedRecorder = this;
      if (timeslice === undefined) {
        return start.call(this);
      }
      return start.call(this, timeslice);
    };
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  await openRoute("/recorder", "Capture, inspect, and replay");
  const demo = page.getByRole("region", { name: "Example demo" });

  await demo
    .getByRole("button", { name: "Prepare supplied Recording Source" })
    .click();
  await demo
    .getByRole("button", { name: "Start supplied recording" })
    .click();
  await expect(demo.getByText("Recording", { exact: true }).first()).toBeVisible();
  await page.evaluate(() => {
    const browser = globalThis as typeof globalThis & {
      suppliedTrack?: MediaStreamTrack;
      activeSuppliedRecorder?: MediaRecorder;
      suppliedEndedListener?: EventListenerOrEventListenerObject | null;
    };
    browser.activeSuppliedRecorder?.stop();
    const listener = browser.suppliedEndedListener;
    const event = new Event("ended");
    if (typeof listener === "function") {
      listener.call(browser.suppliedTrack, event);
    } else {
      listener?.handleEvent(event);
    }
  });

  await expect(
    demo.getByRole("status").filter({ hasText: "Recording ready" }),
  ).toBeVisible();
  await expect(
    demo.getByText("Recording completion cause: SourceEnded", { exact: true }),
  ).toBeVisible();
  expect(
    await page.evaluate(() => {
      const browser = globalThis as typeof globalThis & {
        suppliedTrack?: MediaStreamTrack;
      };
      return browser.suppliedTrack?.readyState;
    }),
  ).toBe("live");
  expect(pageErrors).toEqual([]);
});

test("unmount suppresses an in-flight chunk conversion", async ({
  openRoute,
  page,
}) => {
  await page.addInitScript(() => {
    const browser = globalThis as typeof globalThis & {
      chunkArrayBufferCalls?: number;
    };
    Blob.prototype.arrayBuffer = async function () {
      browser.chunkArrayBufferCalls = (browser.chunkArrayBufferCalls ?? 0) + 1;
      await new Promise((resolve) => setTimeout(resolve, 500));
      return Promise.reject("forced conversion failure after unmount");
    };
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  await openRoute("/recorder", "Capture, inspect, and replay");
  const demo = page.getByRole("region", { name: "Example demo" });
  const lifecycle = demo.getByRole("log", {
    name: "Recording lifecycle events",
  });

  await demo.getByRole("button", { name: "Start recording" }).click();
  await expect
    .poll(() =>
      page.evaluate(() => {
        const browser = globalThis as typeof globalThis & {
          chunkArrayBufferCalls?: number;
        };
        return browser.chunkArrayBufferCalls ?? 0;
      }),
    )
    .toBeGreaterThan(0);
  await expect(lifecycle.locator("li")).toHaveCount(0);

  await demo.getByRole("button", { name: "Unmount recorder" }).click();
  await expect(demo.getByText("Recorder unmounted", { exact: true })).toBeVisible();
  await page.waitForTimeout(700);

  await expect(lifecycle.locator("li")).toHaveCount(0);
  expect(pageErrors).toEqual([]);
});

test("cancelling source acquisition publishes an identified discard", async ({
  openRoute,
  page,
}) => {
  await page.addInitScript(() => {
    const mediaDevices = navigator.mediaDevices;
    const getUserMedia = mediaDevices.getUserMedia.bind(mediaDevices);
    Object.defineProperty(mediaDevices, "getUserMedia", {
      value: async (constraints: MediaStreamConstraints) => {
        await new Promise((resolve) => setTimeout(resolve, 500));
        return getUserMedia(constraints);
      },
    });
  });
  await openRoute("/recorder", "Capture, inspect, and replay");
  const demo = page.getByRole("region", { name: "Example demo" });
  const lifecycle = demo.getByRole("log", {
    name: "Recording lifecycle events",
  });

  await demo.getByRole("button", { name: "Start recording" }).click();
  await demo
    .getByRole("button", { name: "Cancel recording preparation" })
    .click();

  await expect(lifecycle.locator("li")).toHaveText(
    /^Discarded \| Recorder \d+ Recording \d+$/,
  );
  await page.waitForTimeout(700);
  await expect(lifecycle.locator("li")).toHaveCount(1);
});

test("a stale acquisition failure cannot change a replacement Recording", async ({
  openRoute,
  page,
}) => {
  await page.addInitScript(() => {
    let acquisition = 0;
    const mediaDevices = navigator.mediaDevices;
    const getUserMedia = mediaDevices.getUserMedia.bind(mediaDevices);
    Object.defineProperty(mediaDevices, "getUserMedia", {
      value: async (constraints: MediaStreamConstraints) => {
        acquisition += 1;
        if (acquisition === 1) {
          await new Promise((resolve) => setTimeout(resolve, 500));
          throw new DOMException("stale denial", "NotAllowedError");
        }
        return getUserMedia(constraints);
      },
    });
  });
  await openRoute("/recorder", "Capture, inspect, and replay");
  const demo = page.getByRole("region", { name: "Example demo" });
  const start = demo.getByRole("button", { name: "Start recording" });

  await start.click();
  await demo
    .getByRole("button", { name: "Cancel recording preparation" })
    .click();
  await start.click();
  await expect(demo.getByText("Recording", { exact: true }).first()).toBeVisible();
  await page.waitForTimeout(700);

  await expect(demo.getByText("Recording", { exact: true }).first()).toBeVisible();
  await expect(
    demo.getByText("Recorder microphone permission: Granted", { exact: true }),
  ).toBeVisible();
  await expect(demo.getByText("stale denial", { exact: true })).toHaveCount(0);
});

test("a cancelled recording is discarded and another can be completed", async ({
  openRoute,
  page,
}) => {
  await openRoute("/recorder", "Capture, inspect, and replay");
  const demo = page.getByRole("region", { name: "Example demo" });
  const start = demo.getByRole("button", { name: "Start recording" });
  const completed = demo
    .getByRole("status")
    .filter({ hasText: "Recording ready" });

  await start.click();
  await demo.getByRole("button", { name: "Cancel recording" }).click();

  await expect(start).toBeVisible();
  await expect(completed).toHaveCount(0);

  await start.click();
  const stop = demo.getByRole("button", { name: "Stop recording" });
  await expect(stop).toBeVisible();
  await page.waitForTimeout(500);
  await stop.click();

  await expect(completed).toHaveCount(1);
  await expect(completed).toContainText("Recording ready");
});

test("recognized constraints remain distinct from unsupported fields", async ({
  openRoute,
  page,
}) => {
  await page.addInitScript(() => {
    const mediaDevices = navigator.mediaDevices;
    const supported = mediaDevices.getSupportedConstraints();
    Object.defineProperty(mediaDevices, "getSupportedConstraints", {
      value: () => ({ ...supported, noiseSuppression: false }),
    });
  });
  await openRoute("/recorder", "Capture, inspect, and replay");
  const demo = page.getByRole("region", { name: "Example demo" });

  await expect(demo.getByText("Sample rate: recognized")).toBeVisible();
  await expect(demo.getByText("Noise suppression: unrecognized")).toBeVisible();
});

test("an impossible exact constraint reports structured failure", async ({
  openRoute,
  page,
}) => {
  await openRoute("/recorder", "Capture, inspect, and replay");
  const demo = page.getByRole("region", { name: "Example demo" });

  await demo
    .getByRole("button", { name: "Require impossible sample rate" })
    .click();
  await demo.getByRole("button", { name: "Start recording" }).click();

  await expect(demo.getByRole("alert")).toHaveText(
    "Rejected exact constraint: sampleRate",
  );
});

test("a supplied Recording Source is analysed, recorded, played, and preserved", async ({
  openRoute,
  page,
}) => {
  await page.addInitScript(() => {
    const browser = globalThis as typeof globalThis & {
      suppliedGetUserMediaCalls?: number;
      suppliedTrack?: MediaStreamTrack;
      suppliedSharedConsumer?: MediaStream;
      suppliedTrackStopCalls?: number;
      suppliedAnalysisReads?: number;
    };
    const mediaDevices = navigator.mediaDevices;
    const getUserMedia = mediaDevices.getUserMedia.bind(mediaDevices);
    Object.defineProperty(mediaDevices, "getUserMedia", {
      value: async (constraints: MediaStreamConstraints) => {
        browser.suppliedGetUserMediaCalls =
          (browser.suppliedGetUserMediaCalls ?? 0) + 1;
        const stream = await getUserMedia(constraints);
        browser.suppliedTrack = stream.getAudioTracks()[0];
        browser.suppliedSharedConsumer = new MediaStream([
          browser.suppliedTrack,
        ]);
        return stream;
      },
    });
    const stop = MediaStreamTrack.prototype.stop;
    MediaStreamTrack.prototype.stop = function () {
      if (this === browser.suppliedTrack) {
        browser.suppliedTrackStopCalls =
          (browser.suppliedTrackStopCalls ?? 0) + 1;
      }
      return stop.call(this);
    };
    const readTimeDomain = AnalyserNode.prototype.getByteTimeDomainData;
    AnalyserNode.prototype.getByteTimeDomainData = function (array) {
      browser.suppliedAnalysisReads =
        (browser.suppliedAnalysisReads ?? 0) + 1;
      return readTimeDomain.call(this, array);
    };
  });
  await openRoute("/recorder", "Capture, inspect, and replay");
  const demo = page.getByRole("region", { name: "Example demo" });

  await demo
    .getByRole("button", { name: "Prepare supplied Recording Source" })
    .click();
  await expect(
    demo.getByText("Supplied Recording Source ready", { exact: true }),
  ).toBeVisible();
  await demo
    .getByRole("button", { name: "Start supplied recording" })
    .click();

  await expect(demo.getByText("Recording", { exact: true }).first()).toBeVisible();
  await expect(
    demo.getByText("Recorder microphone permission: Unknown", { exact: true }),
  ).toBeVisible();
  await expect(
    demo.getByText("Recorder input identity: unknown", { exact: true }),
  ).toBeVisible();
  await expect
    .poll(() =>
      page.evaluate(() => {
        const browser = globalThis as typeof globalThis & {
          suppliedAnalysisReads?: number;
        };
        return browser.suppliedAnalysisReads ?? 0;
      }),
    )
    .toBeGreaterThan(0);
  expect(
    await page.evaluate(() => {
      const browser = globalThis as typeof globalThis & {
        suppliedGetUserMediaCalls?: number;
      };
      return browser.suppliedGetUserMediaCalls;
    }),
  ).toBe(1);

  await page.waitForTimeout(1_100);
  await demo.getByRole("button", { name: "Pause", exact: true }).click();
  await expect(
    demo.getByText("Recording paused", { exact: true }),
  ).toBeVisible();
  await expect(demo.getByRole("timer", { name: "Recording elapsed time" })).not.toHaveText(
    "0:00",
  );
  await demo.getByRole("button", { name: "Resume", exact: true }).click();
  await expect(demo.getByText("Recording", { exact: true }).first()).toBeVisible();

  await demo.getByRole("button", { name: "Stop recording" }).click();
  await expect(
    demo.getByRole("status").filter({ hasText: "Recording ready" }),
  ).toBeVisible();
  await expect(
    demo.getByText("Recorded input identity: unknown", { exact: true }),
  ).toBeVisible();
  await demo.getByRole("button", { name: "Play", exact: true }).click();
  await expect(
    demo.getByRole("button", { name: "Pause", exact: true }),
  ).toBeVisible();

  expect(
    await page.evaluate(() => {
      const browser = globalThis as typeof globalThis & {
        suppliedTrack?: MediaStreamTrack;
        suppliedSharedConsumer?: MediaStream;
        suppliedTrackStopCalls?: number;
      };
      return {
        readyState: browser.suppliedTrack?.readyState,
        sharedReadyState:
          browser.suppliedSharedConsumer?.getAudioTracks()[0]?.readyState,
        stopCalls: browser.suppliedTrackStopCalls ?? 0,
      };
    }),
  ).toEqual({ readyState: "live", sharedReadyState: "live", stopCalls: 0 });
});

test("supplied source interruption is observable without pausing Recording or changing application policy", async ({
  openRoute,
  page,
}) => {
  await page.addInitScript(() => {
    const browser = globalThis as typeof globalThis & {
      suppliedTrack?: MediaStreamTrack;
      suppliedMuteListener?: EventListenerOrEventListenerObject | null;
      suppliedUnmuteListener?: EventListenerOrEventListenerObject | null;
    };
    const mediaDevices = navigator.mediaDevices;
    const getUserMedia = mediaDevices.getUserMedia.bind(mediaDevices);
    Object.defineProperty(mediaDevices, "getUserMedia", {
      value: async (constraints: MediaStreamConstraints) => {
        const stream = await getUserMedia(constraints);
        browser.suppliedTrack = stream.getAudioTracks()[0];
        return stream;
      },
    });
    const addEventListener = MediaStreamTrack.prototype.addEventListener;
    MediaStreamTrack.prototype.addEventListener = function (
      type,
      listener,
      options,
    ) {
      if (this.kind === "audio" && type === "mute") {
        browser.suppliedMuteListener = listener;
      }
      if (this.kind === "audio" && type === "unmute") {
        browser.suppliedUnmuteListener = listener;
      }
      return addEventListener.call(this, type, listener, options);
    };
  });
  await openRoute("/recorder", "Capture, inspect, and replay");
  const demo = page.getByRole("region", { name: "Example demo" });

  await demo
    .getByRole("button", { name: "Prepare supplied Recording Source" })
    .click();
  await demo
    .getByRole("button", { name: "Start supplied recording" })
    .click();
  await expect(
    demo.getByText("Recording Source availability: Live", { exact: true }),
  ).toBeVisible();
  await expect(demo.getByText("Recording", { exact: true }).first()).toBeVisible();

  await page.evaluate(() => {
    const browser = globalThis as typeof globalThis & {
      suppliedTrack?: MediaStreamTrack;
    };
    if (browser.suppliedTrack) {
      browser.suppliedTrack.enabled = false;
    }
  });
  await page.waitForTimeout(200);
  await expect(
    demo.getByText("Recording Source availability: Live", { exact: true }),
  ).toBeVisible();
  expect(
    await page.evaluate(() => {
      const browser = globalThis as typeof globalThis & {
        suppliedTrack?: MediaStreamTrack;
      };
      return browser.suppliedTrack?.enabled;
    }),
  ).toBe(false);

  await page.evaluate(() => {
    const browser = globalThis as typeof globalThis & {
      suppliedTrack?: MediaStreamTrack;
      suppliedMuteListener?: EventListenerOrEventListenerObject | null;
    };
    const listener = browser.suppliedMuteListener;
    const event = new Event("mute");
    if (typeof listener === "function") {
      listener.call(browser.suppliedTrack, event);
    } else {
      listener?.handleEvent(event);
    }
  });

  await expect(
    demo.getByText("Recording Source availability: Interrupted", {
      exact: true,
    }),
  ).toBeVisible();
  await expect(demo.getByText("Recording", { exact: true }).first()).toBeVisible();
  await page.waitForTimeout(1_100);
  await expect(
    demo.getByRole("timer", { name: "Recording elapsed time" }),
  ).not.toHaveText("0:00");
  expect(
    await page.evaluate(() => {
      const browser = globalThis as typeof globalThis & {
        suppliedTrack?: MediaStreamTrack;
      };
      return browser.suppliedTrack?.enabled;
    }),
  ).toBe(false);

  await page.evaluate(() => {
    const browser = globalThis as typeof globalThis & {
      suppliedTrack?: MediaStreamTrack;
      suppliedUnmuteListener?: EventListenerOrEventListenerObject | null;
    };
    const listener = browser.suppliedUnmuteListener;
    const event = new Event("unmute");
    if (typeof listener === "function") {
      listener.call(browser.suppliedTrack, event);
    } else {
      listener?.handleEvent(event);
    }
  });
  await expect(
    demo.getByText("Recording Source availability: Live", { exact: true }),
  ).toBeVisible();
  await expect(demo.getByText("Recording", { exact: true }).first()).toBeVisible();
});

test("requested completion and discard win over a later supplied source end", async ({
  openRoute,
  page,
}) => {
  await page.addInitScript(() => {
    const browser = globalThis as typeof globalThis & {
      suppliedTrack?: MediaStreamTrack;
      suppliedEndedListener?: EventListenerOrEventListenerObject | null;
      endSourceDuringStop?: boolean;
    };
    const mediaDevices = navigator.mediaDevices;
    const getUserMedia = mediaDevices.getUserMedia.bind(mediaDevices);
    Object.defineProperty(mediaDevices, "getUserMedia", {
      value: async (constraints: MediaStreamConstraints) => {
        const stream = await getUserMedia(constraints);
        browser.suppliedTrack = stream.getAudioTracks()[0];
        return stream;
      },
    });
    const addEventListener = MediaStreamTrack.prototype.addEventListener;
    MediaStreamTrack.prototype.addEventListener = function (
      type,
      listener,
      options,
    ) {
      if (type === "ended") {
        browser.suppliedEndedListener = listener;
      }
      return addEventListener.call(this, type, listener, options);
    };
    const stop = MediaRecorder.prototype.stop;
    MediaRecorder.prototype.stop = function () {
      const result = stop.call(this);
      if (browser.endSourceDuringStop) {
        browser.endSourceDuringStop = false;
        const listener = browser.suppliedEndedListener;
        const event = new Event("ended");
        if (typeof listener === "function") {
          listener.call(browser.suppliedTrack, event);
        } else {
          listener?.handleEvent(event);
        }
      }
      return result;
    };
  });
  await openRoute("/recorder", "Capture, inspect, and replay");
  const demo = page.getByRole("region", { name: "Example demo" });
  const startSupplied = demo.getByRole("button", {
    name: "Start supplied recording",
  });
  const lifecycle = demo.getByRole("log", {
    name: "Recording lifecycle events",
  });

  await demo
    .getByRole("button", { name: "Prepare supplied Recording Source" })
    .click();
  await startSupplied.click();
  await expect(demo.getByText("Recording", { exact: true }).first()).toBeVisible();
  await page.evaluate(() => {
    const browser = globalThis as typeof globalThis & {
      endSourceDuringStop?: boolean;
    };
    browser.endSourceDuringStop = true;
  });
  await demo.getByRole("button", { name: "Stop recording" }).click();

  await expect(
    demo.getByText("Recording completion cause: Requested", { exact: true }),
  ).toBeVisible();

  await startSupplied.click();
  await expect(demo.getByText("Recording", { exact: true }).first()).toBeVisible();
  await page.evaluate(() => {
    const browser = globalThis as typeof globalThis & {
      endSourceDuringStop?: boolean;
    };
    browser.endSourceDuringStop = true;
  });
  await demo.getByRole("button", { name: "Cancel recording" }).click();

  await expect
    .poll(async () =>
      (await lifecycle.locator("li").allTextContents()).filter((event) =>
        event.startsWith("Discarded"),
      ).length,
    )
    .toBe(1);
  await expect(
    demo.getByText("Recording completion cause: Unavailable", { exact: true }),
  ).toBeVisible();
});

test("supplied Recording Sources reject zero, multiple, and ended audio tracks without stopping them", async ({
  openRoute,
  page,
}) => {
  await page.addInitScript(() => {
    const browser = globalThis as typeof globalThis & {
      sourceVariant?: "zero" | "multiple" | "ended";
      sourceTracksByVariant?: Record<string, MediaStreamTrack[]>;
      suppliedGetUserMediaCalls?: number;
      suppliedTrackStopCalls?: number;
    };
    browser.sourceTracksByVariant = {};
    const stop = MediaStreamTrack.prototype.stop;
    MediaStreamTrack.prototype.stop = function () {
      if (
        Object.values(browser.sourceTracksByVariant ?? {}).some((tracks) =>
          tracks.includes(this),
        )
      ) {
        browser.suppliedTrackStopCalls =
          (browser.suppliedTrackStopCalls ?? 0) + 1;
      }
      return stop.call(this);
    };
    const mediaDevices = navigator.mediaDevices;
    const getUserMedia = mediaDevices.getUserMedia.bind(mediaDevices);
    Object.defineProperty(mediaDevices, "getUserMedia", {
      value: async (constraints: MediaStreamConstraints) => {
        browser.suppliedGetUserMediaCalls =
          (browser.suppliedGetUserMediaCalls ?? 0) + 1;
        const acquired = await getUserMedia(constraints);
        const audio = acquired.getAudioTracks()[0];
        const variant = browser.sourceVariant ?? "zero";
        const tracks =
          variant === "zero"
            ? []
            : variant === "multiple"
              ? [audio, audio.clone()]
              : [audio];
        browser.sourceTracksByVariant![variant] = tracks;
        if (variant === "ended") {
          audio.stop();
        }
        return new MediaStream(tracks);
      },
    });
  });
  await openRoute("/recorder", "Capture, inspect, and replay");
  const demo = page.getByRole("region", { name: "Example demo" });
  const prepare = demo.getByRole("button", {
    name: "Prepare supplied Recording Source",
  });
  const start = demo.getByRole("button", { name: "Start supplied recording" });
  await demo
    .getByRole("checkbox", {
      name: "Stop supplied audio track on Recorder cleanup",
    })
    .check();
  await expect(
    demo.getByText("Future supplied source shutdown: StopAudioTracks", {
      exact: true,
    }),
  ).toBeVisible();

  const validationCases = [
    ["zero", "Recording Source must contain exactly one audio track"],
    ["multiple", "Recording Source must contain exactly one audio track"],
    ["ended", "Recording Source audio track must be live"],
  ] as const;
  for (const [index, [variant, message]] of validationCases.entries()) {
    await page.evaluate((sourceVariant) => {
      const browser = globalThis as typeof globalThis & {
        sourceVariant?: "zero" | "multiple" | "ended";
      };
      browser.sourceVariant = sourceVariant;
    }, variant);
    await prepare.click();
    await expect(
      demo.getByText("Supplied Recording Source ready", { exact: true }),
    ).toHaveAttribute("data-generation", String(index + 1));
    const beforeStart = await page.evaluate(() => {
      const browser = globalThis as typeof globalThis & {
        suppliedTrackStopCalls?: number;
      };
      return browser.suppliedTrackStopCalls ?? 0;
    });
    await start.click();
    await expect(demo.getByRole("alert")).toHaveText(message);
    expect(
      await page.evaluate(() => {
        const browser = globalThis as typeof globalThis & {
          suppliedTrackStopCalls?: number;
        };
        return browser.suppliedTrackStopCalls ?? 0;
      }),
    ).toBe(beforeStart);
    if (variant === "multiple") {
      expect(
        await page.evaluate(() => {
          const browser = globalThis as typeof globalThis & {
            sourceTracksByVariant?: Record<string, MediaStreamTrack[]>;
          };
          return browser.sourceTracksByVariant?.multiple.map(
            (track) => track.readyState,
          );
        }),
      ).toEqual(["live", "live"]);
    }
    if (variant === "ended") {
      expect(
        await page.evaluate(() => {
          const browser = globalThis as typeof globalThis & {
            sourceTracksByVariant?: Record<string, MediaStreamTrack[]>;
          };
          return browser.sourceTracksByVariant?.ended[0].readyState;
        }),
      ).toBe("ended");
    }
  }

  expect(
    await page.evaluate(() => {
      const browser = globalThis as typeof globalThis & {
        suppliedGetUserMediaCalls?: number;
      };
      return browser.suppliedGetUserMediaCalls;
    }),
  ).toBe(3);
});

test("a supplied disabled and browser-muted audio track records without its video track", async ({
  openRoute,
  page,
}) => {
  await page.addInitScript(() => {
    const browser = globalThis as typeof globalThis & {
      suppliedAudioTrack?: MediaStreamTrack;
      suppliedVideoTrack?: MediaStreamTrack;
      suppliedTrackStopCalls?: number;
      recordedAudioTracks?: number;
      recordedVideoTracks?: number;
      suppliedCanvasStream?: MediaStream;
    };
    const stop = MediaStreamTrack.prototype.stop;
    MediaStreamTrack.prototype.stop = function () {
      if (
        this === browser.suppliedAudioTrack ||
        this === browser.suppliedVideoTrack
      ) {
        browser.suppliedTrackStopCalls =
          (browser.suppliedTrackStopCalls ?? 0) + 1;
      }
      return stop.call(this);
    };
    const mediaDevices = navigator.mediaDevices;
    const getUserMedia = mediaDevices.getUserMedia.bind(mediaDevices);
    Object.defineProperty(mediaDevices, "getUserMedia", {
      value: async (constraints: MediaStreamConstraints) => {
        const acquired = await getUserMedia(constraints);
        const audio = acquired.getAudioTracks()[0];
        audio.enabled = false;
        Object.defineProperty(audio, "muted", {
          configurable: true,
          get: () => true,
        });
        const canvas = document.createElement("canvas");
        const canvasStream = canvas.captureStream();
        const video = canvasStream.getVideoTracks()[0];
        browser.suppliedAudioTrack = audio;
        browser.suppliedVideoTrack = video;
        browser.suppliedCanvasStream = canvasStream;
        return new MediaStream([audio, video]);
      },
    });
    const start = MediaRecorder.prototype.start;
    MediaRecorder.prototype.start = function (timeslice?: number) {
      browser.recordedAudioTracks = this.stream.getAudioTracks().length;
      browser.recordedVideoTracks = this.stream.getVideoTracks().length;
      if (timeslice === undefined) {
        return start.call(this);
      }
      return start.call(this, timeslice);
    };
  });
  await openRoute("/recorder", "Capture, inspect, and replay");
  const demo = page.getByRole("region", { name: "Example demo" });

  await demo
    .getByRole("button", { name: "Prepare supplied Recording Source" })
    .click();
  await demo
    .getByRole("button", { name: "Start supplied recording" })
    .click();
  await expect(demo.getByText("Recording", { exact: true }).first()).toBeVisible();
  await expect(
    demo.getByText("Recording Source availability: Interrupted", {
      exact: true,
    }),
  ).toBeVisible();
  await expect(
    demo.getByText("Requested sample rate: not started", { exact: true }),
  ).toBeVisible();
  await expect(
    demo.getByText("Recorder microphone permission: Unknown", { exact: true }),
  ).toBeVisible();
  await expect(
    demo.getByText("Microphone muted by the device", { exact: true }),
  ).toHaveCount(0);
  await expect(
    demo.getByText("Effective sample rate: unknown", { exact: true }),
  ).toBeVisible();

  await page.waitForTimeout(500);
  await demo.getByRole("button", { name: "Stop recording" }).click();
  await expect(
    demo.getByRole("status").filter({ hasText: "Recording ready" }),
  ).toBeVisible();

  expect(
    await page.evaluate(() => {
      const browser = globalThis as typeof globalThis & {
        suppliedAudioTrack?: MediaStreamTrack;
        suppliedVideoTrack?: MediaStreamTrack;
        suppliedTrackStopCalls?: number;
        recordedAudioTracks?: number;
        recordedVideoTracks?: number;
      };
      return {
        audioEnabled: browser.suppliedAudioTrack?.enabled,
        audioMuted: browser.suppliedAudioTrack?.muted,
        audioState: browser.suppliedAudioTrack?.readyState,
        videoState: browser.suppliedVideoTrack?.readyState,
        recordedAudioTracks: browser.recordedAudioTracks,
        recordedVideoTracks: browser.recordedVideoTracks,
        stopCalls: browser.suppliedTrackStopCalls ?? 0,
      };
    }),
  ).toEqual({
    audioEnabled: false,
    audioMuted: true,
    audioState: "live",
    videoState: "live",
    recordedAudioTracks: 1,
    recordedVideoTracks: 0,
    stopCalls: 0,
  });
});

test("a supplied Recording Source is preserved across every Recorder cleanup path", async ({
  openRoute,
  page,
}) => {
  await page.addInitScript(() => {
    const browser = globalThis as typeof globalThis & {
      suppliedTrack?: MediaStreamTrack;
      suppliedTrackStopCalls?: number;
      failSuppliedRecorderConstruction?: boolean;
      failSuppliedRecorderStart?: boolean;
      activeSuppliedRecorder?: MediaRecorder;
      removedSuppliedTrackListeners?: number;
    };
    const mediaDevices = navigator.mediaDevices;
    const getUserMedia = mediaDevices.getUserMedia.bind(mediaDevices);
    Object.defineProperty(mediaDevices, "getUserMedia", {
      value: async (constraints: MediaStreamConstraints) => {
        const stream = await getUserMedia(constraints);
        browser.suppliedTrack = stream.getAudioTracks()[0];
        return stream;
      },
    });
    const stop = MediaStreamTrack.prototype.stop;
    MediaStreamTrack.prototype.stop = function () {
      if (this === browser.suppliedTrack) {
        browser.suppliedTrackStopCalls =
          (browser.suppliedTrackStopCalls ?? 0) + 1;
      }
      return stop.call(this);
    };
    const removeEventListener = MediaStreamTrack.prototype.removeEventListener;
    MediaStreamTrack.prototype.removeEventListener = function (
      type,
      listener,
      options,
    ) {
      if (this.kind === "audio" && ["mute", "unmute", "ended"].includes(type)) {
        browser.removedSuppliedTrackListeners =
          (browser.removedSuppliedTrackListeners ?? 0) + 1;
      }
      return removeEventListener.call(this, type, listener, options);
    };
    const start = MediaRecorder.prototype.start;
    MediaRecorder.prototype.start = function (timeslice?: number) {
      browser.activeSuppliedRecorder = this;
      if (browser.failSuppliedRecorderStart) {
        throw new DOMException("forced supplied start rejection");
      }
      if (timeslice === undefined) {
        return start.call(this);
      }
      return start.call(this, timeslice);
    };
    const NativeMediaRecorder = MediaRecorder;
    Object.defineProperty(globalThis, "MediaRecorder", {
      configurable: true,
      value: new Proxy(NativeMediaRecorder, {
        construct(target, argumentsList) {
          if (browser.failSuppliedRecorderConstruction) {
            throw new DOMException("forced supplied startup failure");
          }
          return Reflect.construct(target, argumentsList);
        },
      }),
    });
  });
  await openRoute("/recorder", "Capture, inspect, and replay");
  const demo = page.getByRole("region", { name: "Example demo" });
  const lifecycle = demo.getByRole("log", {
    name: "Recording lifecycle events",
  });
  const startSupplied = demo.getByRole("button", {
    name: "Start supplied recording",
  });

  await demo
    .getByRole("button", { name: "Prepare supplied Recording Source" })
    .click();
  await page.evaluate(() => {
    const browser = globalThis as typeof globalThis & {
      failSuppliedRecorderConstruction?: boolean;
    };
    browser.failSuppliedRecorderConstruction = true;
  });
  await startSupplied.click();
  await expect
    .poll(async () =>
      (await lifecycle.locator("li").allTextContents()).filter((event) =>
        event.startsWith("Failed"),
      ).length,
    )
    .toBe(1);
  await expectSuppliedTrackToBePreserved(page);

  await page.evaluate(() => {
    const browser = globalThis as typeof globalThis & {
      failSuppliedRecorderConstruction?: boolean;
      failSuppliedRecorderStart?: boolean;
    };
    browser.failSuppliedRecorderConstruction = false;
    browser.failSuppliedRecorderStart = true;
  });
  await startSupplied.click();
  await expect
    .poll(async () =>
      (await lifecycle.locator("li").allTextContents()).filter((event) =>
        event.startsWith("Failed"),
      ).length,
    )
    .toBe(2);
  expect(
    await page.evaluate(() => {
      const browser = globalThis as typeof globalThis & {
        activeSuppliedRecorder?: MediaRecorder;
        removedSuppliedTrackListeners?: number;
      };
      return {
        dataHandler: browser.activeSuppliedRecorder?.ondataavailable ?? null,
        stopHandler: browser.activeSuppliedRecorder?.onstop ?? null,
        errorHandler: browser.activeSuppliedRecorder?.onerror ?? null,
        removedTrackListeners: browser.removedSuppliedTrackListeners ?? 0,
      };
    }),
  ).toEqual({
    dataHandler: null,
    stopHandler: null,
    errorHandler: null,
    removedTrackListeners: 3,
  });
  await expectSuppliedTrackToBePreserved(page);

  await page.evaluate(() => {
    const browser = globalThis as typeof globalThis & {
      failSuppliedRecorderStart?: boolean;
    };
    browser.failSuppliedRecorderStart = false;
  });
  await startSupplied.click();
  await expect(demo.getByText("Recording", { exact: true }).first()).toBeVisible();
  await demo.getByRole("button", { name: "Cancel recording" }).click();
  await expect
    .poll(async () =>
      (await lifecycle.locator("li").allTextContents()).filter((event) =>
        event.startsWith("Discarded"),
      ).length,
    )
    .toBe(1);
  await expectSuppliedTrackToBePreserved(page);

  await startSupplied.click();
  await expect(demo.getByText("Recording", { exact: true }).first()).toBeVisible();
  await page.evaluate(() => {
    const browser = globalThis as typeof globalThis & {
      activeSuppliedRecorder?: MediaRecorder;
    };
    browser.activeSuppliedRecorder?.dispatchEvent(new Event("error"));
    browser.activeSuppliedRecorder?.stop();
  });
  await expect
    .poll(async () =>
      (await lifecycle.locator("li").allTextContents()).filter((event) =>
        event.startsWith("Failed"),
      ).length,
    )
    .toBe(3);
  await expectSuppliedTrackToBePreserved(page);

  await startSupplied.click();
  await expect(demo.getByText("Recording", { exact: true }).first()).toBeVisible();
  await demo.getByRole("button", { name: "Unmount recorder" }).click();
  await expect(demo.getByText("Recorder unmounted", { exact: true })).toBeVisible();
  await expectSuppliedTrackToBePreserved(page);
});

test("explicit shutdown authority stops the accepted supplied audio track exactly once", async ({
  openRoute,
  page,
}) => {
  await page.addInitScript(() => {
    const browser = globalThis as typeof globalThis & {
      suppliedTrack?: MediaStreamTrack;
      suppliedTrackStopCalls?: number;
      allTrackStopCalls?: number;
      stoppedTrack?: MediaStreamTrack;
      suppliedGetUserMediaCalls?: number;
    };
    const mediaDevices = navigator.mediaDevices;
    const getUserMedia = mediaDevices.getUserMedia.bind(mediaDevices);
    Object.defineProperty(mediaDevices, "getUserMedia", {
      value: async (constraints: MediaStreamConstraints) => {
        const stream = await getUserMedia(constraints);
        browser.suppliedGetUserMediaCalls =
          (browser.suppliedGetUserMediaCalls ?? 0) + 1;
        browser.suppliedTrack = stream.getAudioTracks()[0];
        return stream;
      },
    });
    const stop = MediaStreamTrack.prototype.stop;
    MediaStreamTrack.prototype.stop = function () {
      browser.allTrackStopCalls = (browser.allTrackStopCalls ?? 0) + 1;
      browser.stoppedTrack = this;
      if (this === browser.suppliedTrack) {
        browser.suppliedTrackStopCalls =
          (browser.suppliedTrackStopCalls ?? 0) + 1;
      }
      return stop.call(this);
    };
  });
  await openRoute("/recorder", "Capture, inspect, and replay");
  const demo = page.getByRole("region", { name: "Example demo" });

  await demo
    .getByRole("checkbox", {
      name: "Stop supplied audio track on Recorder cleanup",
    })
    .check();
  await expect(
    demo.getByText("Future supplied source shutdown: StopAudioTracks", {
      exact: true,
    }),
  ).toBeVisible();
  await demo
    .getByRole("button", { name: "Prepare supplied Recording Source" })
    .click();
  await demo
    .getByRole("button", { name: "Start supplied recording" })
    .click();
  await expect(demo.getByText("Recording", { exact: true }).first()).toBeVisible();
  await demo.getByRole("button", { name: "Stop recording" }).click();
  await expect(
    demo.getByRole("status").filter({ hasText: "Recording ready" }),
  ).toBeVisible();

  await expect
    .poll(() =>
      page.evaluate(() => {
        const browser = globalThis as typeof globalThis & {
          suppliedTrack?: MediaStreamTrack;
          suppliedTrackStopCalls?: number;
          allTrackStopCalls?: number;
          stoppedTrack?: MediaStreamTrack;
          suppliedGetUserMediaCalls?: number;
        };
        return {
          readyState: browser.suppliedTrack?.readyState,
          stopCalls: browser.suppliedTrackStopCalls ?? 0,
          allStopCalls: browser.allTrackStopCalls ?? 0,
          stoppedReadyState: browser.stoppedTrack?.readyState,
          getUserMediaCalls: browser.suppliedGetUserMediaCalls ?? 0,
        };
      }),
    )
    .toEqual({
      readyState: "ended",
      stopCalls: 1,
      allStopCalls: 1,
      stoppedReadyState: "ended",
      getUserMediaCalls: 1,
    });
});

test("explicit shutdown authority is exact once across every other terminal path", async ({
  openRoute,
  page,
}) => {
  await page.addInitScript(() => {
    type ShutdownPath =
      | "construction-failure"
      | "start-failure"
      | "discard"
      | "runtime-failure"
      | "finalization-failure"
      | "source-ended-while-paused"
      | "unmount"
      | "pending-start-unmount";
    const browser = globalThis as typeof globalThis & {
      shutdownPath?: ShutdownPath;
      suppliedTrack?: MediaStreamTrack;
      suppliedTrackStopCalls?: number;
      activeSuppliedRecorder?: MediaRecorder;
      suppliedEndedListener?: EventListenerOrEventListenerObject | null;
      failFinalization?: boolean;
    };
    const mediaDevices = navigator.mediaDevices;
    const getUserMedia = mediaDevices.getUserMedia.bind(mediaDevices);
    Object.defineProperty(mediaDevices, "getUserMedia", {
      value: async (constraints: MediaStreamConstraints) => {
        const stream = await getUserMedia(constraints);
        browser.suppliedTrack = stream.getAudioTracks()[0];
        return stream;
      },
    });
    const stopTrack = MediaStreamTrack.prototype.stop;
    MediaStreamTrack.prototype.stop = function () {
      if (this === browser.suppliedTrack) {
        browser.suppliedTrackStopCalls =
          (browser.suppliedTrackStopCalls ?? 0) + 1;
      }
      return stopTrack.call(this);
    };
    const addEventListener = MediaStreamTrack.prototype.addEventListener;
    MediaStreamTrack.prototype.addEventListener = function (
      type,
      listener,
      options,
    ) {
      if (type === "ended") {
        browser.suppliedEndedListener = listener;
      }
      return addEventListener.call(this, type, listener, options);
    };
    const start = MediaRecorder.prototype.start;
    MediaRecorder.prototype.start = function (timeslice?: number) {
      browser.activeSuppliedRecorder = this;
      if (browser.shutdownPath === "start-failure") {
        throw new DOMException("forced supplied start rejection");
      }
      if (browser.shutdownPath === "pending-start-unmount") {
        return;
      }
      if (timeslice === undefined) {
        return start.call(this);
      }
      return start.call(this, timeslice);
    };
    const NativeMediaRecorder = MediaRecorder;
    Object.defineProperty(globalThis, "MediaRecorder", {
      configurable: true,
      value: new Proxy(NativeMediaRecorder, {
        construct(target, argumentsList) {
          if (browser.shutdownPath === "construction-failure") {
            throw new DOMException("forced supplied construction failure");
          }
          return Reflect.construct(target, argumentsList);
        },
      }),
    });
    const arrayBuffer = Blob.prototype.arrayBuffer;
    Blob.prototype.arrayBuffer = function () {
      if (browser.failFinalization) {
        return Promise.reject(
          new DOMException("forced supplied finalization failure"),
        );
      }
      return arrayBuffer.call(this);
    };
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  const paths = [
    "construction-failure",
    "start-failure",
    "discard",
    "runtime-failure",
    "finalization-failure",
    "source-ended-while-paused",
    "unmount",
    "pending-start-unmount",
  ] as const;

  for (const shutdownPath of paths) {
    await openRoute("/recorder", "Capture, inspect, and replay");
    const demo = page.getByRole("region", { name: "Example demo" });
    const lifecycle = demo.getByRole("log", {
      name: "Recording lifecycle events",
    });
    await page.evaluate((path) => {
      const browser = globalThis as typeof globalThis & {
        shutdownPath?: typeof path;
      };
      browser.shutdownPath = path;
    }, shutdownPath);
    await demo
      .getByRole("checkbox", {
        name: "Stop supplied audio track on Recorder cleanup",
      })
      .check();
    await expect(
      demo.getByText("Future supplied source shutdown: StopAudioTracks", {
        exact: true,
      }),
    ).toBeVisible();
    await demo
      .getByRole("button", { name: "Prepare supplied Recording Source" })
      .click();
    await expect(
      demo.getByText("Supplied Recording Source ready", { exact: true }),
    ).toBeVisible();
    await demo
      .getByRole("button", { name: "Start supplied recording" })
      .click();

    if (
      shutdownPath !== "construction-failure" &&
      shutdownPath !== "start-failure" &&
      shutdownPath !== "pending-start-unmount"
    ) {
      await expect(
        demo.getByText("Recording", { exact: true }).first(),
      ).toBeVisible();
    }

    if (shutdownPath === "discard") {
      await demo.getByRole("button", { name: "Cancel recording" }).click();
    }
    if (shutdownPath === "runtime-failure") {
      await page.evaluate(() => {
        const browser = globalThis as typeof globalThis & {
          activeSuppliedRecorder?: MediaRecorder;
        };
        browser.activeSuppliedRecorder?.dispatchEvent(new Event("error"));
        browser.activeSuppliedRecorder?.stop();
      });
    }
    if (shutdownPath === "finalization-failure") {
      await page.evaluate(() => {
        const browser = globalThis as typeof globalThis & {
          failFinalization?: boolean;
        };
        browser.failFinalization = true;
      });
      await demo.getByRole("button", { name: "Stop recording" }).click();
    }
    if (shutdownPath === "source-ended-while-paused") {
      await demo.getByRole("button", { name: "Pause", exact: true }).click();
      await expect(
        demo.getByText("Recording paused", { exact: true }),
      ).toBeVisible();
      await page.evaluate(() => {
        const browser = globalThis as typeof globalThis & {
          suppliedTrack?: MediaStreamTrack;
          suppliedEndedListener?: EventListenerOrEventListenerObject | null;
        };
        const listener = browser.suppliedEndedListener;
        const event = new Event("ended");
        if (typeof listener === "function") {
          listener.call(browser.suppliedTrack, event);
        } else {
          listener?.handleEvent(event);
        }
      });
      await expect(
        demo.getByText("Recording completion cause: SourceEnded", {
          exact: true,
        }),
      ).toBeVisible();
    }
    if (shutdownPath === "unmount") {
      await demo.getByRole("button", { name: "Unmount recorder" }).click();
      await expect(
        demo.getByText("Recorder unmounted", { exact: true }),
      ).toBeVisible();
    }
    if (shutdownPath === "pending-start-unmount") {
      await expect(
        demo.getByText("Preparing", { exact: true }).first(),
      ).toBeVisible();
      await demo.getByRole("button", { name: "Unmount recorder" }).click();
      await expect(
        demo.getByText("Recorder unmounted", { exact: true }),
      ).toBeVisible();
    }

    await expect
      .poll(() =>
        page.evaluate(() => {
          const browser = globalThis as typeof globalThis & {
            suppliedTrack?: MediaStreamTrack;
            suppliedTrackStopCalls?: number;
          };
          return {
            readyState: browser.suppliedTrack?.readyState,
            stopCalls: browser.suppliedTrackStopCalls ?? 0,
          };
        }),
      )
      .toEqual({ readyState: "ended", stopCalls: 1 });

    if (
      shutdownPath === "unmount" ||
      shutdownPath === "pending-start-unmount"
    ) {
      const eventsAtUnmount = await lifecycle.locator("li").count();
      await page.waitForTimeout(300);
      await expect(lifecycle.locator("li")).toHaveCount(eventsAtUnmount);
    }
    await page.waitForTimeout(100);
    expect(
      await page.evaluate(() => {
        const browser = globalThis as typeof globalThis & {
          suppliedTrackStopCalls?: number;
        };
        return browser.suppliedTrackStopCalls ?? 0;
      }),
    ).toBe(1);
  }

  expect(pageErrors).toEqual([]);
});

test("unmount cleans up a supplied Recording while browser startup is pending", async ({
  openRoute,
  page,
}) => {
  await page.addInitScript(() => {
    const browser = globalThis as typeof globalThis & {
      suppliedTrack?: MediaStreamTrack;
      suppliedTrackStopCalls?: number;
      closedRecorderContexts?: number;
    };
    const mediaDevices = navigator.mediaDevices;
    const getUserMedia = mediaDevices.getUserMedia.bind(mediaDevices);
    Object.defineProperty(mediaDevices, "getUserMedia", {
      value: async (constraints: MediaStreamConstraints) => {
        const stream = await getUserMedia(constraints);
        browser.suppliedTrack = stream.getAudioTracks()[0];
        return stream;
      },
    });
    const stop = MediaStreamTrack.prototype.stop;
    MediaStreamTrack.prototype.stop = function () {
      if (this === browser.suppliedTrack) {
        browser.suppliedTrackStopCalls =
          (browser.suppliedTrackStopCalls ?? 0) + 1;
      }
      return stop.call(this);
    };
    const close = AudioContext.prototype.close;
    AudioContext.prototype.close = function () {
      browser.closedRecorderContexts =
        (browser.closedRecorderContexts ?? 0) + 1;
      return close.call(this);
    };
    MediaRecorder.prototype.start = function () {};
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  await openRoute("/recorder", "Capture, inspect, and replay");
  const demo = page.getByRole("region", { name: "Example demo" });

  await demo
    .getByRole("button", { name: "Prepare supplied Recording Source" })
    .click();
  await demo
    .getByRole("button", { name: "Start supplied recording" })
    .click();
  await expect(demo.getByText("Preparing", { exact: true }).first()).toBeVisible();
  await demo.getByRole("button", { name: "Unmount recorder" }).click();
  await expect(demo.getByText("Recorder unmounted", { exact: true })).toBeVisible();

  await expectSuppliedTrackToBePreserved(page);
  await expect
    .poll(() =>
      page.evaluate(() => {
        const browser = globalThis as typeof globalThis & {
          closedRecorderContexts?: number;
        };
        return browser.closedRecorderContexts ?? 0;
      }),
    )
    .toBe(1);
  expect(pageErrors).toEqual([]);
});

async function expectSuppliedTrackToBePreserved(
  page: import("@playwright/test").Page,
) {
  await expect
    .poll(() =>
      page.evaluate(() => {
        const browser = globalThis as typeof globalThis & {
          suppliedTrack?: MediaStreamTrack;
          suppliedTrackStopCalls?: number;
        };
        return {
          readyState: browser.suppliedTrack?.readyState,
          stopCalls: browser.suppliedTrackStopCalls ?? 0,
        };
      }),
    )
    .toEqual({ readyState: "live", stopCalls: 0 });
}
