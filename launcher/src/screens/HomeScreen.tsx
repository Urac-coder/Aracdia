import { LogOut, Play, Settings, Newspaper } from "lucide-react";
import { Card } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import type { PlayerProfile } from "@/lib/profile";

interface HomeScreenProps {
  profile: PlayerProfile;
  onLogout: () => void;
  onOpenSettings: () => void;
}

const LAUNCHER_VERSION = "0.1.0";

export function HomeScreen({ profile, onLogout, onOpenSettings }: HomeScreenProps) {
  function handlePlay() {
    // TODO: trigger the download + launch pipeline (etapes 3–5)
    console.log("Play clicked — launch pipeline not implemented yet");
  }

  return (
    <div className="flex h-full w-full flex-col">
      {/* Top bar */}
      <header className="drag-region flex h-14 items-center justify-between border-b border-[var(--color-border-subtle)] px-6">
        <div className="flex items-center gap-3">
          <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-gradient-to-br from-indigo-500 to-purple-600 shadow-md shadow-indigo-900/40">
            <span className="font-display text-sm font-black text-white">A</span>
          </div>
          <div className="flex items-baseline gap-2">
            <span className="font-display text-base font-semibold tracking-tight">
              Aracdia
            </span>
            <span className="text-xs text-[var(--color-text-muted)]">
              v{LAUNCHER_VERSION}
            </span>
          </div>
        </div>

        <div className="no-drag flex items-center gap-2">
          <Button
            variant="ghost"
            size="sm"
            onClick={onOpenSettings}
            aria-label="Paramètres"
          >
            <Settings className="h-4 w-4" />
          </Button>
          <Button variant="ghost" size="sm" onClick={onLogout} aria-label="Déconnexion">
            <LogOut className="h-4 w-4" />
          </Button>
        </div>
      </header>

      {/* Main content */}
      <main className="grid flex-1 grid-cols-1 gap-6 overflow-auto p-6 lg:grid-cols-3">
        {/* News column */}
        <section className="lg:col-span-2">
          <Card className="h-full p-6">
            <div className="mb-4 flex items-center gap-2 text-[var(--color-text-secondary)]">
              <Newspaper className="h-4 w-4" />
              <h2 className="text-sm font-medium uppercase tracking-wider">
                Actualités
              </h2>
            </div>

            <div className="flex h-full min-h-[300px] flex-col items-center justify-center text-center">
              <div className="mb-3 text-3xl">🚧</div>
              <p className="font-display text-lg font-semibold">
                Aracdia est en cours de construction
              </p>
              <p className="mt-1 max-w-md text-sm text-[var(--color-text-secondary)]">
                Le launcher est prêt. Le moteur de jeu et le téléchargement
                arrivent dans les prochaines étapes.
              </p>
            </div>
          </Card>
        </section>

        {/* Profile + Play column */}
        <aside className="flex flex-col gap-6">
          <Card className="p-5">
            <div className="flex items-center gap-3">
              <div className="flex h-12 w-12 items-center justify-center rounded-xl bg-gradient-to-br from-indigo-500 to-purple-600 text-lg font-bold text-white shadow-lg shadow-indigo-900/30">
                {profile.username.charAt(0).toUpperCase()}
              </div>
              <div className="min-w-0 flex-1">
                <p className="truncate font-display text-base font-semibold">
                  {profile.username}
                </p>
                <p className="truncate font-mono text-[10px] text-[var(--color-text-muted)]">
                  {profile.id}
                </p>
              </div>
            </div>
          </Card>

          <Card className="flex flex-1 flex-col p-5">
            <div className="mb-4">
              <h3 className="text-xs font-medium uppercase tracking-wider text-[var(--color-text-muted)]">
                Prêt à jouer
              </h3>
              <p className="mt-1 font-display text-lg font-semibold">
                Aracdia · monde principal
              </p>
            </div>

            <div className="mt-auto">
              <Button size="lg" className="w-full" onClick={handlePlay}>
                <Play className="h-5 w-5" />
                JOUER
              </Button>
              <p className="mt-3 text-center text-xs text-[var(--color-text-muted)]">
                Aracdia Engine non installé · le téléchargement démarrera au lancement
              </p>
            </div>
          </Card>
        </aside>
      </main>
    </div>
  );
}
