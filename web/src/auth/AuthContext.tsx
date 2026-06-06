import { createContext, useCallback, useContext, useEffect, useState, type ReactNode } from "react";

import * as api from "../api/client.ts";

export type AuthStatus =
  | { kind: "loading" }
  | { kind: "needs-setup" }
  | { kind: "anonymous" }
  | { kind: "authed"; me: api.Me };

interface AuthCtx {
  status: AuthStatus;
  signIn: (username: string, password: string) => Promise<void>;
  signOut: () => Promise<void>;
  /** Called after the wizard completes — the response already minted a
   * session, so this just refreshes the bootstrap state. */
  refresh: () => Promise<void>;
}

const Ctx = createContext<AuthCtx | null>(null);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [status, setStatus] = useState<AuthStatus>({ kind: "loading" });

  // Bootstrap order: setup-status first (so a fresh install renders the
  // wizard rather than a sign-in card the operator can't satisfy), then
  // /api/me to decide between anonymous and authed.
  const bootstrap = useCallback(async () => {
    try {
      const setup = await api.setupStatus();
      if (setup.needs_setup) {
        setStatus({ kind: "needs-setup" });
        return;
      }
    } catch {
      // Older backends without the setup endpoint fall through — treat as
      // already-initialized and go straight to the /api/me check.
    }
    try {
      const me = await api.me();
      setStatus({ kind: "authed", me });
    } catch {
      setStatus({ kind: "anonymous" });
    }
  }, []);

  useEffect(() => {
    void bootstrap();
  }, [bootstrap]);

  const signIn = useCallback(async (username: string, password: string) => {
    await api.signIn(username, password);
    const me = await api.me();
    setStatus({ kind: "authed", me });
  }, []);

  const signOut = useCallback(async () => {
    try {
      await api.signOut();
    } finally {
      setStatus({ kind: "anonymous" });
    }
  }, []);

  return (
    <Ctx.Provider value={{ status, signIn, signOut, refresh: bootstrap }}>{children}</Ctx.Provider>
  );
}

export function useAuth() {
  const v = useContext(Ctx);
  if (!v) throw new Error("useAuth() outside AuthProvider");
  return v;
}
