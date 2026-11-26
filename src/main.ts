import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

// Mirrors the Rust types in src-tauri/src/phpbb.rs.
type Session = { state: "logged_in"; username: string | null } | { state: "logged_out" };
type Pagination = {
  start: number;
  next_start: number | null;
  previous_start: number | null;
  last_start: number;
};
type TopicSummary = {
  id: number;
  title: string;
  url: string;
  author: string | null;
  last_poster: string | null;
  replies: number | null;
  views: number | null;
};
type ForumPage = { forum_id: number; title: string | null; topics: TopicSummary[]; pagination: Pagination };
type Post = {
  id: number | null;
  author: string | null;
  posted: string | null;
  content_html: string;
  content_text: string;
};
type TopicPage = { topic_id: number; title: string | null; posts: Post[]; pagination: Pagination };
type CommandError = { kind: string; message: string };

const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;

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

let forumPage: ForumPage | null = null;
let currentTopic: number | null = null;

async function loadForum(start = 0) {
  const forumId = Number($<HTMLInputElement>("forum-id").value);
  setStatus(`Loading forum ${forumId}…`);
  try {
    forumPage = await forum<ForumPage>("fetch_forum", { forumId, start });
    renderForum(forumPage);
    setStatus(`Loaded ${forumPage.topics.length} topics.`);
  } catch (e) {
    showError(e);
  }
}

function renderForum(page: ForumPage) {
  $("forum-title").textContent = page.title ?? `Forum ${page.forum_id}`;
  $<HTMLButtonElement>("prev").disabled = page.pagination.previous_start === null;
  $<HTMLButtonElement>("next").disabled = page.pagination.next_start === null;

  const list = $("topics");
  list.replaceChildren(
    ...page.topics.map((topic) => {
      const li = document.createElement("li");
      const title = document.createElement("button");
      title.type = "button";
      title.className = "topic";
      title.textContent = topic.title;
      title.addEventListener("click", () => loadTopic(topic.id));
      const meta = document.createElement("small");
      meta.textContent = [
        topic.author && `by ${topic.author}`,
        topic.replies !== null && `${topic.replies} replies`,
        topic.views !== null && `${topic.views} views`,
      ]
        .filter(Boolean)
        .join(" · ");
      li.append(title, meta);
      return li;
    }),
  );
}

async function loadTopic(topicId: number, start = 0) {
  setStatus(`Loading topic ${topicId}…`);
  try {
    const page = await forum<TopicPage>("fetch_topic", { topicId, start });
    currentTopic = topicId;
    $<HTMLButtonElement>("snapshot-topic").disabled = false;
    renderTopic(page);
    setStatus(`Loaded ${page.posts.length} posts.`);
  } catch (e) {
    showError(e);
  }
}

function renderTopic(page: TopicPage) {
  $("topic-title").textContent = page.title ?? `Topic ${page.topic_id}`;
  // Plain text only for now: the forum's HTML is untrusted and must be
  // sanitised before it is ever rendered as markup.
  $("posts").replaceChildren(
    ...page.posts.map((post) => {
      const article = document.createElement("article");
      const header = document.createElement("header");
      header.textContent = [post.author ?? "Unknown", post.posted].filter(Boolean).join(" · ");
      const body = document.createElement("p");
      body.textContent = post.content_text;
      article.append(header, body);
      return article;
    }),
  );
}

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
    snapshot(`viewforum.php?f=${$<HTMLInputElement>("forum-id").value}`),
  );
  $("snapshot-topic").addEventListener("click", () => {
    if (currentTopic !== null) snapshot(`viewtopic.php?t=${currentTopic}`);
  });

  // Every page the forum window finishes loading may have changed the
  // session (security check passed, signed in, signed out).
  listen("forum-page-loaded", () => refreshSession());
});
