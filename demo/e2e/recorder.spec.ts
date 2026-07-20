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
    };
    const mediaDevices = navigator.mediaDevices;
    const getUserMedia = mediaDevices.getUserMedia.bind(mediaDevices);
    Object.defineProperty(mediaDevices, "getUserMedia", {
      value: async (constraints: MediaStreamConstraints) => {
        const stream = await getUserMedia(constraints);
        browser.recorderTrack = stream.getAudioTracks()[0];
        return stream;
      },
    });
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
      captureNextDataEvent?: boolean;
    };
    browser.captureNextDataEvent = true;
    browser.recorderTrack?.dispatchEvent(new Event("ended"));
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
    .getByRole("button", { name: "Cancel microphone request" })
    .click();

  await expect(lifecycle.locator("li")).toHaveText(
    /^Discarded \| Recorder \d+ Recording \d+$/,
  );
  await page.waitForTimeout(700);
  await expect(lifecycle.locator("li")).toHaveCount(1);
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
