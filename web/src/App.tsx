import { Toaster } from "sonner";

import { AuthProvider, useAuth } from "./auth/AuthContext.tsx";
import { Setup } from "./pages/Setup.tsx";
import { SignIn } from "./pages/SignIn.tsx";
import { Shell } from "./pages/Shell.tsx";

function Router() {
  const { status } = useAuth();
  if (status.kind === "loading") {
    return (
      <div
        className="h-full w-full flex items-center justify-center"
        style={{ background: "var(--paper)" }}
      />
    );
  }
  if (status.kind === "needs-setup") return <Setup />;
  return status.kind === "authed" ? <Shell /> : <SignIn />;
}

export function App() {
  return (
    <AuthProvider>
      <Router />
      <Toaster
        position="bottom-center"
        toastOptions={{
          style: {
            background: "var(--ink)",
            color: "var(--paper)",
            border: "none",
            borderRadius: 13,
            fontFamily: "var(--font-sans)",
            fontSize: "var(--text-sm)",
            fontWeight: 500,
            padding: "12px 18px",
            boxShadow: "0 10px 30px rgba(26,26,30,.3)",
          },
        }}
      />
    </AuthProvider>
  );
}
