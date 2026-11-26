/** Which parts of a topic page show a skeleton while it loads. */
export interface TopicSkeletons {
  /** The About/Features and Details/Languages columns, fed by the forum's first page. */
  details: boolean;
  /** The screenshots strip, fed by Steam, which loads after the first page. */
  screenshots: boolean;
  /** The Releases tab, which has nothing to list until the first page arrives. */
  releases: boolean;
  /** The Overview's Latest release card, which waits for the first release found. */
  latest: boolean;
}

/**
 * Skeletons stand in only for data that is still on its way: each goes as soon as its data arrives,
 * and all go once loading ends, so a topic without Steam data doesn't keep an empty block.
 * While the topic on screen reloads, such as on a Refresh, its content stays and no skeleton shows,
 * even for parts it never had, like screenshots for a topic without Steam data.
 */
export function topicSkeletons(
  loading: boolean,
  hasFirst: boolean,
  hasSteam: boolean,
  reloading = false,
): TopicSkeletons {
  const waiting = loading && !reloading;
  return {
    details: waiting && !hasFirst,
    screenshots: waiting && !hasSteam,
    releases: waiting && !hasFirst,
    latest: waiting,
  };
}

/** Whether a load of topic `id` keeps the data on screen: only when that data is the same topic's. */
export function keepsShown(loadedId: number | null, id: number): boolean {
  return loadedId === id;
}
