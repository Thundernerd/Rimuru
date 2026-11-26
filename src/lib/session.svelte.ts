import { api, errorMessage } from "./api";
import { setStatus } from "./status.svelte";
import type { Session } from "./types";

type SessionState = { current: Session | null; checking: boolean };

export const session = $state<SessionState>({ current: null, checking: false });

let signingIn = false;

export async function refreshSession() {
  session.checking = true;
  try {
    session.current = await api.session();
    if (session.current.state === "logged_in" && signingIn) {
      signingIn = false;
      await api.hideForumWindow();
    }
  } catch (e) {
    setStatus(`Couldn't check the session: ${errorMessage(e)}`);
  } finally {
    session.checking = false;
  }
}

export function signIn() {
  signingIn = true;
  api.showForumWindow().catch((e) => setStatus(errorMessage(e)));
}
