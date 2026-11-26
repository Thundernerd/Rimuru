import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";

// Mirrors the Rust types in src-tauri/src/phpbb.rs, game.rs and commands.rs.
type Session = { state: "logged_in"; username: string | null } | { state: "logged_out" };
type Pagination = {
  start: number;
  next_start: number | null;
  previous_start: number | null;
  last_start: number;
  page: number | null;
  total_pages: number | null;
};
type TopicSummary = {
  id: number;
  kind: "global_announcement" | "announcement" | "sticky" | "normal";
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
type ForumPage = { forum_id: number; title: string | null; topics: TopicSummary[]; pagination: Pagination };
type Field = { key: string; value: string };
type GameInfo = {
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
type DownloadLink = { url: string; host: string; label: string; flagged_unsafe: boolean };
type Release = {
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
type GameDetail = {
  topic_id: number;
  title: string | null;
  info: GameInfo | null;
  releases: Release[];
  pages_read: number[];
  total_pages: number | null;
};
type CommandError = { kind: string; message: string };

const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;

function el<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  props: { className?: string; text?: string } = {},
  ...children: (Node | null | false)[]
): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  if (props.className) node.className = props.className;
  if (props.text !== undefined) node.textContent = props.text;
  node.append(...children.filter((c): c is Node => !!c));
  return node;
}

/** Resolves on the forum window's next finished page load. */
function nextForumPageLoad(timeoutMs = 20000): Promise<void> {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      unlisten.then((u) => u());
      reject({ kind: "timeout", message: "the forum window did not finish loading" });
    }, timeoutMs);
    const unlisten = listen("forum-page-loaded", () => {
      clearTimeout(timer);
      unlisten.then((u) => u());
      resolve();
    });
  });
}

/**
 * Invokes a forum command. If the site's security check is in the way, the
 * backend has already told the forum window to reload; wait for that and
 * retry once.
 */
async function forum<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    if ((e as CommandError).kind !== "security_check") throw e;
    setStatus("Passing the forum's security check…");
    await nextForumPageLoad();
    return await invoke<T>(cmd, args);
  }
}

function setStatus(text: string) {
  $("status").textContent = text;
}

function showError(e: unknown) {
  const err = e as CommandError;
  setStatus(`Error: ${err?.message ?? String(e)}`);
}

// ---------------------------------------------------------------- session

let signingIn = false;

async function refreshSession() {
  try {
    const session = await forum<Session>("forum_session");
    const label = $("session");
    if (session.state === "logged_in") {
      label.textContent = `Signed in${session.username ? ` as ${session.username}` : ""}`;
      $("sign-in").hidden = true;
      if (signingIn) {
        signingIn = false;
        await invoke("hide_forum_window");
      }
    } else {
      label.textContent = "Not signed in";
      $("sign-in").hidden = false;
    }
  } catch (e) {
    $("session").textContent = "Session unknown";
    showError(e);
  }
}

// ---------------------------------------------------------------- forum list

let forumPage: ForumPage | null = null;
let forumStart = 0;
let currentTopic: number | null = null;

async function loadForum(start = 0) {
  const forumId = Number($<HTMLInputElement>("forum-id").value);
  setStatus(`Loading forum ${forumId}…`);
  try {
    forumPage = await forum<ForumPage>("fetch_forum", { forumId, start });
    forumStart = start;
    renderForum(forumPage);
    setStatus(`Loaded ${forumPage.topics.length} topics.`);
  } catch (e) {
    showError(e);
  }
}

function pageLabel(p: Pagination): string {
  return p.page && p.total_pages ? ` — page ${p.page} of ${p.total_pages}` : "";
}

function renderForum(page: ForumPage) {
  const { pagination: p } = page;
  $("forum-title").textContent = (page.title ?? `Forum ${page.forum_id}`) + pageLabel(p);
  $<HTMLButtonElement>("prev").disabled = p.previous_start === null;
  $<HTMLButtonElement>("next").disabled = p.next_start === null;

  $("topics").replaceChildren(
    ...page.topics.map((topic) => {
      const title = el("button", { className: "topic", text: topic.name });
      title.type = "button";
      title.title = topic.title;
      title.addEventListener("click", () => loadGame(topic.id));
      const tags = [...topic.prefixes, ...topic.labels].map((t) => el("span", { className: "tag", text: t }));
      const meta = [
        topic.author && `by ${topic.author}`,
        topic.created,
        topic.replies !== null && `${topic.replies} replies`,
        topic.last_post_at && `last post ${topic.last_post_at}`,
      ]
        .filter(Boolean)
        .join(" · ");
      return el("li", {}, title, ...tags, el("small", { text: meta }));
    }),
  );
}

// ---------------------------------------------------------------- game view

async function loadGame(topicId: number) {
  setStatus(`Loading topic ${topicId}…`);
  try {
    const game = await forum<GameDetail>("fetch_game", { topicId });
    currentTopic = topicId;
    $<HTMLButtonElement>("snapshot-topic").disabled = false;
    renderGame(game);
    const pages = game.total_pages ? ` (read ${game.pages_read.length} of ${game.total_pages} pages)` : "";
    setStatus(`Found ${game.releases.length} releases${pages}.`);
  } catch (e) {
    showError(e);
  }
}

function renderGame(game: GameDetail) {
  $("topic-title").textContent = game.info?.title ?? game.title ?? `Topic ${game.topic_id}`;
  const releases = el(
    "section",
    { className: "releases" },
    el("h3", { text: `Releases (${game.releases.length})` }),
    ...(game.releases.length
      ? game.releases.map(renderRelease)
      : [el("p", { className: "muted", text: "No posts with download links on the pages read." })]),
  );
  $("game").replaceChildren(...[game.info && renderInfo(game.info), releases].filter((n): n is HTMLElement => !!n));
}

function row(label: string, value: string | null | undefined | string[]): HTMLElement | null {
  const text = Array.isArray(value) ? value.join(", ") : value;
  if (!text) return null;
  return el("div", { className: "row" }, el("dt", { text: label }), el("dd", { text }));
}

function renderInfo(info: GameInfo): HTMLElement {
  const header = info.header_image ? Object.assign(el("img", { className: "header-image" }), { src: info.header_image, alt: "" }) : null;

  const links = el("p", { className: "info-links" });
  if (info.store_url) links.append(linkButton("Steam store", info.store_url));
  if (info.steamdb_url) links.append(linkButton("SteamDB", info.steamdb_url));

  const details = el(
    "dl",
    { className: "details" },
    row("Developer", info.developer),
    row("Publisher", info.publisher),
    row("Release date", info.release_date),
    row("Genres", info.genres),
    row("Platforms", info.platforms),
    row("Languages", info.languages),
    row("Steam app id", info.steam_app_id?.toString()),
    ...info.fields.map((f) => row(f.key, f.value)),
  );

  const extra = el("details", {}, el("summary", { text: "More" }));
  if (info.features.length) {
    extra.append(el("h4", { text: "Features" }), el("ul", {}, ...info.features.map((f) => el("li", { text: f }))));
  }
  if (info.description) extra.append(el("h4", { text: "About" }), el("p", { className: "prewrap", text: info.description }));
  for (const set of info.system_requirements) {
    extra.append(
      el("h4", { text: `System requirements${set.label ? ` — ${set.label}` : ""}` }),
      el("dl", { className: "details" }, ...set.items.map((f) => row(f.key, f.value))),
    );
  }

  return el("section", { className: "info card" }, header, links, details, extra.childElementCount > 1 && extra);
}

function renderRelease(release: Release): HTMLElement {
  const facts = [
    release.version && `v${release.version.replace(/^v/i, "")}`,
    release.build && `build ${release.build}`,
    release.size,
  ].filter(Boolean) as string[];
  const header = el(
    "header",
    {},
    el("strong", { text: release.title ?? "Release" }),
    ...release.tags.map((t) => el("span", { className: "tag", text: t })),
    ...facts.map((f) => el("span", { className: "fact", text: f })),
  );
  const byline = el("small", {
    text: [release.author && `by ${release.author}`, release.posted].filter(Boolean).join(" · "),
  });

  const links = el(
    "ul",
    { className: "links" },
    ...release.links.map((link) =>
      el(
        "li",
        {},
        el("span", { className: "host", text: link.host }),
        link.label !== link.host && el("span", { text: link.label }),
        link.flagged_unsafe && el("span", { className: "unsafe", text: "⚠ flagged unsafe by the forum" }),
        linkButton("Open", link.url, link.flagged_unsafe),
      ),
    ),
  );

  const details = el(
    "details",
    {},
    el("summary", { text: "Post text" }),
    el("p", { className: "prewrap", text: release.details.join("\n") }),
  );
  return el("article", { className: "card" }, header, byline, links, details);
}

/**
 * Opens a URL in the system browser. Links the forum flags as unsafe need a
 * second click.
 */
function linkButton(label: string, url: string, needsConfirm = false): HTMLButtonElement {
  const button = el("button", { className: "open", text: label });
  button.type = "button";
  button.title = url;
  let armed = false;
  button.addEventListener("click", () => {
    if (needsConfirm && !armed) {
      armed = true;
      button.textContent = "Click again to open";
      return;
    }
    openUrl(url).catch(showError);
  });
  return button;
}

// ---------------------------------------------------------------- snapshots

async function snapshot(path: string) {
  try {
    const file = await forum<string>("save_snapshot", { path });
    setStatus(`Saved ${file}`);
  } catch (e) {
    showError(e);
  }
}

window.addEventListener("DOMContentLoaded", () => {
  $("sign-in").addEventListener("click", () => {
    signingIn = true;
    invoke("show_forum_window").catch(showError);
  });
  $("show-forum").addEventListener("click", () => {
    invoke("show_forum_window", { path: "index.php" }).catch(showError);
  });
  $("forum-form").addEventListener("submit", (e) => {
    e.preventDefault();
    loadForum();
  });
  $("prev").addEventListener("click", () => loadForum(forumPage?.pagination.previous_start ?? 0));
  $("next").addEventListener("click", () => loadForum(forumPage?.pagination.next_start ?? 0));
  $("snapshot-forum").addEventListener("click", () =>
    snapshot(`viewforum.php?f=${$<HTMLInputElement>("forum-id").value}&start=${forumStart}`),
  );
  $("snapshot-topic").addEventListener("click", () => {
    if (currentTopic !== null) snapshot(`viewtopic.php?t=${currentTopic}&start=0`);
  });

  // Every page the forum window finishes loading may have changed the
  // session (security check passed, signed in, signed out).
  listen("forum-page-loaded", () => refreshSession());
});
