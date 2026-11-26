/** Which parts of a topic page show a skeleton while it loads. */
export interface TopicSkeletons {
  /** The About/Features and Details/Languages columns, fed by the forum's first page. */
  details: boolean;
  /** The screenshots strip, fed by Steam, which loads after the first page. */
  screenshots: boolean;
  /** The Releases tab, which has nothing to list until the first page arrives. */
  releases: boolean;
}

/**
 * Skeletons stand in only for data that is still on its way: each goes as soon as its data arrives,
 * and all go once loading ends, so a topic without Steam data doesn't keep an empty block.
 */
export function topicSkeletons(loading: boolean, hasFirst: boolean, hasSteam: boolean): TopicSkeletons {
  return {
    details: loading && !hasFirst,
    screenshots: loading && !hasSteam,
    releases: loading && !hasFirst,
  };
}
