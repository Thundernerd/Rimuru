import { describe, expect, it } from "vitest";

import { keepsShown, topicSkeletons } from "./skeleton";

const none = { details: false, screenshots: false, releases: false, latest: false };

describe("topicSkeletons", () => {
  it("shows every skeleton while nothing has arrived", () => {
    expect(topicSkeletons(true, false, false)).toEqual({ details: true, screenshots: true, releases: true, latest: true });
  });

  it("keeps the screenshots skeleton once the first page is in but Steam is not", () => {
    expect(topicSkeletons(true, true, false)).toEqual({ details: false, screenshots: true, releases: false, latest: true });
  });

  it("drops each skeleton as its data arrives", () => {
    expect(topicSkeletons(true, true, true)).toEqual({ details: false, screenshots: false, releases: false, latest: true });
  });

  it("shows none once loading is over, even with data missing", () => {
    expect(topicSkeletons(false, false, false)).toEqual(none);
  });

  it("shows none while a Refresh reloads data already on screen", () => {
    expect(topicSkeletons(true, true, true, true)).toEqual(none);
  });

  it("shows none on a Refresh of a topic without Steam data", () => {
    expect(topicSkeletons(true, true, false, true)).toEqual(none);
  });
});

describe("keepsShown", () => {
  it("keeps the data on a reload of the topic on screen", () => {
    expect(keepsShown(7, 7)).toBe(true);
  });

  it("starts from empty on a first load or a switch to another topic", () => {
    expect(keepsShown(null, 7)).toBe(false);
    expect(keepsShown(3, 7)).toBe(false);
  });
});
