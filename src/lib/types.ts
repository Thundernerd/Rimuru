// Mirrors the Rust types in src-tauri/src/phpbb.rs, game.rs and commands.rs.

export type Session = { state: "logged_in"; username: string | null } | { state: "logged_out" };

export type Pagination = {
  start: number;
  next_start: number | null;
  previous_start: number | null;
  last_start: number;
  page: number | null;
  total_pages: number | null;
};

export type TopicKind = "global_announcement" | "announcement" | "sticky" | "normal";

export type TopicSummary = {
  id: number;
  kind: TopicKind;
  title: string;
  name: string;
  prefixes: string[];
  labels: string[];
  url: string;
  author: string | null;
  created: string | null;
  replies: number | null;
  views: number | null;
  last_post_at: string | null;
  last_poster: string | null;
};

export type ForumPage = {
  forum_id: number;
  title: string | null;
  topics: TopicSummary[];
  pagination: Pagination;
};

/** One page of a topic listing: the forum's, or search results. */
export type Listing = { topics: TopicSummary[]; pagination: Pagination; total_matches?: number | null };

export type SearchPage = {
  query: string;
  topics: TopicSummary[];
  /** From "Search found N matches". */
  total_matches: number | null;
  pagination: Pagination;
};

export type Field = { key: string; value: string };

export type GameInfo = {
  title: string | null;
  header_image: string | null;
  store_url: string | null;
  steamdb_url: string | null;
  steam_app_id: number | null;
  genres: string[];
  developer: string | null;
  publisher: string | null;
  release_date: string | null;
  languages: string[];
  platforms: string[];
  features: string[];
  description: string | null;
  system_requirements: { label: string; items: Field[] }[];
  fields: Field[];
  images: string[];
};

export type DownloadLink = { url: string; host: string; label: string; flagged_unsafe: boolean };

export type Release = {
  post_id: number;
  author: string | null;
  posted: string | null;
  title: string | null;
  details: string[];
  version: string | null;
  build: string | null;
  size: string | null;
  tags: string[];
  links: DownloadLink[];
};

export type GameFirstPage = {
  topic_id: number;
  title: string | null;
  info: GameInfo | null;
  /** Releases on page 1, newest first. */
  releases: Release[];
  total_pages: number | null;
  per_page: number | null;
  last_start: number;
};

/** Image URLs for a Steam app; any can be missing. */
export type Artwork = {
  portrait: string | null;
  header: string | null;
  capsule: string | null;
  /** Wide key art without a logo, for backdrops. */
  hero: string | null;
};

export type SteamMatch = { app_id: number; name: string; capsule_image: string | null };

export type SteamApp = {
  app_id: number;
  name: string;
  store_url: string;
  header_image: string | null;
  capsule_image: string | null;
  short_description: string | null;
  developers: string[];
  publishers: string[];
  release_date: string | null;
  coming_soon: boolean;
  platforms: string[];
  genres: string[];
  categories: string[];
  languages: string[];
  screenshots: { thumbnail: string; full: string }[];
  is_free: boolean;
  price: string | null;
  metacritic: number | null;
  website: string | null;
  /** Trailers, the store's highlighted one first. */
  movies: Movie[];
};

export type Movie = { name: string; thumbnail: string | null; hls: string | null; mp4: string | null };

/** What was there when a game topic was last opened. */
export type Visit = {
  replies: number | null;
  release_ids: number[];
  visited_at: number;
  /** The topic as listed at the time (missing on visits from older versions). */
  topic: TopicSummary | null;
};

export type CommandError = { kind: string; message: string };
