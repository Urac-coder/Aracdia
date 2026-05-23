import { useEffect, useState } from "react";
import { LoginScreen } from "@/screens/LoginScreen";
import { HomeScreen } from "@/screens/HomeScreen";
import { clearProfile, loadProfile, type PlayerProfile } from "@/lib/profile";

type AppState =
  | { kind: "loading" }
  | { kind: "login" }
  | { kind: "home"; profile: PlayerProfile };

export default function App() {
  const [state, setState] = useState<AppState>({ kind: "loading" });

  // Bootstrap: try to load the persisted profile on mount
  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const profile = await loadProfile();
        if (cancelled) return;
        setState(profile ? { kind: "home", profile } : { kind: "login" });
      } catch (err) {
        console.error("Failed to load profile", err);
        if (!cancelled) setState({ kind: "login" });
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  async function handleLogout() {
    try {
      await clearProfile();
    } catch (err) {
      console.error("Failed to clear profile", err);
    }
    setState({ kind: "login" });
  }

  if (state.kind === "loading") {
    return (
      <div className="flex h-full w-full items-center justify-center">
        <div className="h-8 w-8 animate-spin rounded-full border-2 border-[var(--color-border-strong)] border-t-[var(--color-accent-500)]" />
      </div>
    );
  }

  if (state.kind === "login") {
    return (
      <LoginScreen
        onLoggedIn={(profile) => setState({ kind: "home", profile })}
      />
    );
  }

  return <HomeScreen profile={state.profile} onLogout={handleLogout} />;
}
