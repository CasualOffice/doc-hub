import { useState } from "react";

import { useAuth } from "../auth/AuthContext.tsx";
import { Sidebar, type NavId } from "../components/Sidebar.tsx";
import { TopBar, type ViewMode } from "../components/TopBar.tsx";
import { EmptyState } from "../components/EmptyState.tsx";
import { Files } from "./Files.tsx";

export function Shell() {
  const { status } = useAuth();
  const username = status.kind === "authed" ? status.me.admin : "admin";
  const [nav, setNav] = useState<NavId>("home");
  const [view, setView] = useState<ViewMode>("grid");
  const [query, setQuery] = useState("");
  const [itemCount, setItemCount] = useState(0);
  const [uploadTick, setUploadTick] = useState(0);
  const [newFolderTick, setNewFolderTick] = useState(0);

  return (
    <div className="h-full w-full flex" style={{ background: "var(--paper)" }}>
      <Sidebar
        current={nav}
        onSelect={setNav}
        itemCount={itemCount}
        onNewFolder={() => setNewFolderTick((t) => t + 1)}
        onUpload={() => setUploadTick((t) => t + 1)}
        username={username}
      />
      <div className="flex-1 flex flex-col" style={{ minWidth: 0 }}>
        <div style={{ padding: "26px 40px 0" }}>
          <TopBar query={query} onQueryChange={setQuery} view={view} onViewChange={setView} />
        </div>
        <main style={{ flex: 1, display: "flex", flexDirection: "column", minHeight: 0 }}>
          {nav === "home" && (
            <Files
              view={view}
              query={query}
              uploadRequested={uploadTick}
              onUploadHandled={() => {}}
              newFolderRequested={newFolderTick}
              onNewFolderHandled={() => {}}
              onItemCount={setItemCount}
            />
          )}
          {nav === "recent" && (
            <div style={{ flex: 1, padding: "40px 0" }}>
              <EmptyState
                title="Nothing recent yet."
                subtitle="Files you open will appear here."
              />
            </div>
          )}
          {nav === "trash" && (
            <div style={{ flex: 1, padding: "40px 0" }}>
              <EmptyState
                title="Trash is empty."
                subtitle="Files you delete will appear here."
              />
            </div>
          )}
        </main>
      </div>
    </div>
  );
}
