import { describe, expect, it } from "vitest";

import { formatBytes, formatProgress, isArchive } from "./download";

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
