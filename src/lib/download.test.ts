import { describe, expect, it } from "vitest";

import {
  baseName,
  clearFinished,
  failEntry,
  finishEntry,
  formatBytes,
  formatProgress,
  isArchive,
  orderEntries,
  progressEntry,
  runningCount,
  startEntry,
} from "./download";

describe("formatBytes", () => {
  it("keeps small counts in bytes and scales larger ones", () => {
    expect(formatBytes(512)).toBe("512 B");
    expect(formatBytes(1536)).toBe("1.5 KB");
    expect(formatBytes(25 * 1024 * 1024)).toBe("25 MB");
  });
});

describe("formatProgress", () => {
  it("shows a percentage when the size is known", () => {
    expect(formatProgress({ received: 50, total: 200 })).toBe("25%");
    expect(formatProgress({ received: 200, total: 200 })).toBe("100%");
  });

  it("shows the bytes so far when the size is unknown", () => {
    expect(formatProgress({ received: 2048, total: null })).toBe("2.0 KB");
  });
});

describe("isArchive", () => {
  it("recognises the archives Rimuru extracts", () => {
    expect(isArchive("/home/me/Games/Game v1.2.ZIP")).toBe(true);
    expect(isArchive("C:\\Games\\game.tar.gz")).toBe(true);
    expect(isArchive("/home/me/Games/setup.exe")).toBe(false);
  });
});

describe("download entries", () => {
  const base = { url: "u", host: "h", label: "l", topic: null };
  const two = () => startEntry(startEntry([], { ...base, id: 1 }), { ...base, id: 2 });

  it("records start, progress and finish by id", () => {
    let e = progressEntry(two(), 2, { received: 5, total: 10 });
    e = finishEntry(e, 1, "/d/a.zip");
    expect(e[0]).toMatchObject({ state: "done", path: "/d/a.zip" });
    expect(e[1]).toMatchObject({ state: "running", progress: { received: 5, total: 10 } });
  });

  it("tells a cancel from a failure", () => {
    const e = failEntry(failEntry(two(), 1, "boom", false), 2, "cancelled", true);
    expect(e.map((x) => x.state)).toEqual(["failed", "cancelled"]);
    expect(e[0].error).toBe("boom");
  });

  it("orders running first, then newest first", () => {
    const e = finishEntry(two(), 2, "/p");
    expect(orderEntries(e).map((x) => x.id)).toEqual([1, 2]);
    const f = startEntry(e, { ...base, id: 3 });
    expect(orderEntries(f).map((x) => x.id)).toEqual([3, 1, 2]);
    expect(runningCount(f)).toBe(2);
  });

  it("clears everything but running entries", () => {
    expect(clearFinished(finishEntry(two(), 1, "/p")).map((x) => x.id)).toEqual([2]);
  });

  it("takes a file name from a path", () => {
    expect(baseName("C:\\d\\a.zip")).toBe("a.zip");
    expect(baseName("/d/a.zip")).toBe("a.zip");
  });
});
