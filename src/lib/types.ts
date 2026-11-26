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

export type GameDetail = {
  topic_id: number;
  title: string | null;
  info: GameInfo | null;
  releases: Release[];
  pages_read: number[];
  total_pages: number | null;
};

export type CommandError = { kind: string; message: string };
