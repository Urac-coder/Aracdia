import { useEffect, useRef, useState } from "react";
import {
  CheckCircle2,
  Circle,
  CloudDownload,
  Download,
  Gamepad2,
  LogOut,
  Newspaper,
  Package,
  Play,
  Power,
  RefreshCw,
  Server,
  Settings,
  Square,
  XCircle,
} from "lucide-react";
import { Card } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import type { PlayerProfile } from "@/lib/profile";
import {
  describePhase,
  fetchEngineRelease,
  formatBytes,
  getEngineStatus,
  installEngine,
  listenToInstall,
  type EngineRelease,
  type EngineStatus,
  type InstallProgress,
} from "@/lib/engine";
import {
  fetchContentRelease,
  getContentStatus,
  installContent,
  listenToContentInstall,
  type ContentRelease,
  type ContentStatus,
  type InstallProgress as ContentInstallProgress,
} from "@/lib/content";
import {
  currentSession,
  launchEngine,
  listenToLaunch,
  stopEngine,
  type RunningSession,
} from "@/lib/launch";
import {
  getServerStatus,
  listenToServer,
  startServer,
  stopServer,
  type ServerSession,
  type ServerStatus,
} from "@/lib/server";

interface HomeScreenProps {
  profile: PlayerProfile;
  onLogout: () => void;
  onOpenSettings: () => void;
}

const LAUNCHER_VERSION = "0.1.0";

type Flow =
  | { kind: "idle" }
  | { kind: "fetchingManifest" }
  | { kind: "installing"; progress: InstallProgress | null; release: EngineRelease }
  | { kind: "starting" }
  | { kind: "running"; session: RunningSession }
  | { kind: "error"; message: string };

export function HomeScreen({ profile, onLogout, onOpenSettings }: HomeScreenProps) {
  const [status, setStatus] = useState<EngineStatus | null>(null);
  const [flow, setFlow] = useState<Flow>({ kind: "idle" });
  const installInFlight = useRef(false);

  // Initial engine status fetch + reconcile a possibly running engine session
  // (e.g. WebView reload, or relaunch of the launcher after a crash that left
  // the engine subprocess alive).
  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const [s, session] = await Promise.all([
          getEngineStatus(),
          currentSession(),
        ]);
        if (cancelled) return;
        setStatus(s);
        if (session) {
          setFlow({ kind: "running", session });
        }
      } catch (err) {
        console.error("Failed to read engine state", err);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  // Subscribe to install events
  useEffect(() => {
    let unlisten: (() => void) | null = null;
    listenToInstall({
      onProgress: (progress) => {
        setFlow((current) =>
          current.kind === "installing" ? { ...current, progress } : current,
        );
      },
      onComplete: ({ version }) => {
        setStatus({ kind: "installed", version, path: "" });
        setFlow({ kind: "idle" });
        installInFlight.current = false;
      },
      onError: ({ message }) => {
        setFlow({ kind: "error", message });
        installInFlight.current = false;
      },
    }).then((fn) => {
      unlisten = fn;
    });
    return () => {
      unlisten?.();
    };
  }, []);

  // Subscribe to launch lifecycle events
  useEffect(() => {
    let unlisten: (() => void) | null = null;
    listenToLaunch({
      onStarted: (session) => {
        setFlow({ kind: "running", session });
      },
      onExited: ({ exitCode, success }) => {
        setFlow(
          success
            ? { kind: "idle" }
            : {
                kind: "error",
                message: `Le moteur s'est arrêté avec le code ${exitCode ?? "?"}.`,
              },
        );
      },
    }).then((fn) => {
      unlisten = fn;
    });
    return () => {
      unlisten?.();
    };
  }, []);

  async function handlePlay() {
    if (installInFlight.current) return;
    if (flow.kind === "running" || flow.kind === "starting") return;

    if (status?.kind === "installed") {
      try {
        setFlow({ kind: "starting" });
        await launchEngine();
        // The "started" event will move us to the running state.
      } catch (err) {
        setFlow({
          kind: "error",
          message: err instanceof Error ? err.message : String(err),
        });
      }
      return;
    }

    // Engine not installed yet → trigger install pipeline
    installInFlight.current = true;
    try {
      setFlow({ kind: "fetchingManifest" });
      const release = await fetchEngineRelease();
      setFlow({ kind: "installing", progress: null, release });
      await installEngine(release);
    } catch (err) {
      setFlow({
        kind: "error",
        message: err instanceof Error ? err.message : String(err),
      });
      installInFlight.current = false;
    }
  }

  async function handleStop() {
    try {
      await stopEngine();
    } catch (err) {
      console.error("Failed to stop engine", err);
    }
  }

  return (
    <div className="flex h-full w-full flex-col">
      <header className="drag-region flex h-14 items-center justify-between border-b border-[var(--color-border-subtle)] px-6">
        <div className="flex items-center gap-3">
          <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-gradient-to-br from-indigo-500 to-purple-600 shadow-md shadow-indigo-900/40">
            <span className="font-display text-sm font-black text-white">A</span>
          </div>
          <div className="flex items-baseline gap-2">
            <span className="font-display text-base font-semibold tracking-tight">Aracdia</span>
            <span className="text-xs text-[var(--color-text-muted)]">v{LAUNCHER_VERSION}</span>
          </div>
        </div>

        <div className="no-drag flex items-center gap-2">
          <Button variant="ghost" size="sm" onClick={onOpenSettings} aria-label="Paramètres">
            <Settings className="h-4 w-4" />
          </Button>
          <Button variant="ghost" size="sm" onClick={onLogout} aria-label="Déconnexion">
            <LogOut className="h-4 w-4" />
          </Button>
        </div>
      </header>

      <main className="grid flex-1 grid-cols-1 gap-6 overflow-auto p-6 lg:grid-cols-3">
        <section className="lg:col-span-2">
          <Card className="h-full p-6">
            <div className="mb-4 flex items-center gap-2 text-[var(--color-text-secondary)]">
              <Newspaper className="h-4 w-4" />
              <h2 className="text-sm font-medium uppercase tracking-wider">Actualités</h2>
            </div>
            <div className="flex h-full min-h-[300px] flex-col items-center justify-center text-center">
              <div className="mb-3 text-3xl">🚧</div>
              <p className="font-display text-lg font-semibold">
                Aracdia est en cours de construction
              </p>
              <p className="mt-1 max-w-md text-sm text-[var(--color-text-secondary)]">
                Le launcher peut télécharger, installer et lancer le moteur Aracdia.
                Le contenu de jeu (mods Lua) arrive dans les prochaines étapes.
              </p>
            </div>
          </Card>
        </section>

        <aside className="flex flex-col gap-6">
          <Card className="p-5">
            <div className="flex items-center gap-3">
              <div className="flex h-12 w-12 items-center justify-center rounded-xl bg-gradient-to-br from-indigo-500 to-purple-600 text-lg font-bold text-white shadow-lg shadow-indigo-900/30">
                {profile.username.charAt(0).toUpperCase()}
              </div>
              <div className="min-w-0 flex-1">
                <p className="truncate font-display text-base font-semibold">{profile.username}</p>
                <p className="truncate font-mono text-[10px] text-[var(--color-text-muted)]">
                  {profile.id}
                </p>
              </div>
            </div>
          </Card>

          <Card className="flex flex-1 flex-col p-5">
            <PlayPanel
              status={status}
              flow={flow}
              onPlay={handlePlay}
              onStop={handleStop}
              onRetry={() => setFlow({ kind: "idle" })}
            />
          </Card>

          <ServerCard />
          <ContentCard />
        </aside>
      </main>
    </div>
  );
}

function ServerCard() {
  const [status, setStatus] = useState<ServerStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    let interval: number | null = null;

    const refresh = async () => {
      try {
        const s = await getServerStatus();
        if (!cancelled) setStatus(s);
      } catch (err) {
        console.error("Failed to read server status", err);
      }
    };

    refresh();
    // Poll every 5 s to catch external changes (server crash, manual kill).
    interval = window.setInterval(refresh, 5_000);

    return () => {
      cancelled = true;
      if (interval !== null) clearInterval(interval);
    };
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    listenToServer({
      onStarted: (session) => {
        const s: ServerStatus = { kind: "running", ...session };
        setStatus(s);
        setError(null);
      },
      onStopped: () => {
        setStatus({ kind: "stopped" });
      },
    }).then((fn) => {
      unlisten = fn;
    });
    return () => {
      unlisten?.();
    };
  }, []);

  async function handleToggle() {
    if (busy) return;
    setBusy(true);
    setError(null);
    try {
      if (status?.kind === "running") {
        await stopServer();
      } else {
        await startServer();
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

  const running = status?.kind === "running";

  return (
    <Card className="p-5">
      <div className="mb-3 flex items-center justify-between gap-2">
        <div className="flex items-center gap-2 text-[var(--color-text-secondary)]">
          <Server className="h-4 w-4" />
          <h2 className="text-xs font-medium uppercase tracking-wider">Serveur Aracdia</h2>
        </div>
        <ServerStatusBadge running={running} />
      </div>

      {running ? (
        <RunningServerInfo session={status as ServerSession & { kind: "running" }} />
      ) : (
        <p className="text-sm text-[var(--color-text-secondary)]">
          Le serveur est arrêté. Démarre-le pour permettre à tes amis de te rejoindre, même quand tu ne joues pas.
        </p>
      )}

      {error && (
        <p className="mt-3 text-xs text-[var(--color-danger-500)]">{error}</p>
      )}

      <Button
        size="sm"
        variant={running ? "secondary" : "primary"}
        className="mt-4 w-full"
        onClick={handleToggle}
        disabled={busy}
      >
        <Power className="h-4 w-4" />
        {running ? "Arrêter le serveur" : "Démarrer le serveur"}
      </Button>
    </Card>
  );
}

function ServerStatusBadge({ running }: { running: boolean }) {
  if (running) {
    return (
      <span className="inline-flex items-center gap-1.5 rounded-full bg-[var(--color-success-500)]/10 px-2 py-0.5 text-[10px] font-medium uppercase tracking-wider text-[var(--color-success-500)]">
        <span className="relative flex h-1.5 w-1.5">
          <span className="absolute inset-0 animate-ping rounded-full bg-[var(--color-success-500)] opacity-60" />
          <span className="relative inline-block h-1.5 w-1.5 rounded-full bg-[var(--color-success-500)]" />
        </span>
        En ligne
      </span>
    );
  }
  return (
    <span className="inline-flex items-center gap-1.5 rounded-full bg-[var(--color-bg-overlay)] px-2 py-0.5 text-[10px] font-medium uppercase tracking-wider text-[var(--color-text-muted)]">
      <Circle className="h-1.5 w-1.5 fill-current" />
      Arrêté
    </span>
  );
}

function RunningServerInfo({
  session,
}: {
  session: ServerSession & { kind: "running" };
}) {
  const exposed = session.bind === "0.0.0.0";
  return (
    <div className="space-y-2 text-sm">
      <div className="grid grid-cols-[max-content_1fr] items-baseline gap-x-3 gap-y-1.5 text-xs">
        <span className="text-[var(--color-text-muted)]">Bind</span>
        <span className="font-mono">
          {session.bind}:{session.port}
        </span>
        <span className="text-[var(--color-text-muted)]">PID</span>
        <span className="font-mono">{session.pid}</span>
        <span className="text-[var(--color-text-muted)]">Démarré</span>
        <span>{formatStartedAt(session.startedAt)}</span>
      </div>

      {exposed && (
        <p className="rounded-md bg-[var(--color-bg-overlay)] p-2 text-[11px] leading-snug text-[var(--color-text-secondary)]">
          Tes amis peuvent te rejoindre via <span className="font-mono">&lt;ton-ip-publique&gt;:{session.port}</span>.
          {" "}
          Forward le port <span className="font-mono">{session.port}/UDP</span> sur ton routeur pour Internet, sinon ils accéderont seulement via le LAN.
        </p>
      )}
    </div>
  );
}

type ContentFlow =
  | { kind: "idle" }
  | { kind: "checking" }
  | { kind: "installing"; progress: ContentInstallProgress | null; release: ContentRelease }
  | { kind: "uptoDate"; version: string }
  | { kind: "available"; release: ContentRelease }
  | { kind: "error"; message: string };

function ContentCard() {
  const [status, setStatus] = useState<ContentStatus | null>(null);
  const [flow, setFlow] = useState<ContentFlow>({ kind: "idle" });
  const inFlight = useRef(false);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const s = await getContentStatus();
        if (!cancelled) setStatus(s);
      } catch (err) {
        console.error("Failed to read content status", err);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    listenToContentInstall({
      onProgress: (progress) => {
        setFlow((current) =>
          current.kind === "installing" ? { ...current, progress } : current,
        );
      },
      onComplete: ({ version }) => {
        setStatus({ kind: "installed", version, path: "" });
        setFlow({ kind: "uptoDate", version });
        inFlight.current = false;
      },
      onError: ({ message }) => {
        setFlow({ kind: "error", message });
        inFlight.current = false;
      },
    }).then((fn) => {
      unlisten = fn;
    });
    return () => {
      unlisten?.();
    };
  }, []);

  async function handleCheck() {
    if (inFlight.current) return;
    inFlight.current = true;
    setFlow({ kind: "checking" });
    try {
      const release = await fetchContentRelease();
      const installedVersion = status?.kind === "installed" ? status.version : null;
      if (installedVersion === release.version) {
        setFlow({ kind: "uptoDate", version: release.version });
        inFlight.current = false;
      } else {
        setFlow({ kind: "available", release });
        inFlight.current = false;
      }
    } catch (err) {
      setFlow({
        kind: "error",
        message: err instanceof Error ? err.message : String(err),
      });
      inFlight.current = false;
    }
  }

  async function handleInstall(release: ContentRelease) {
    if (inFlight.current) return;
    inFlight.current = true;
    setFlow({ kind: "installing", progress: null, release });
    try {
      await installContent(release);
      // Completion handled by the event listener above.
    } catch (err) {
      setFlow({
        kind: "error",
        message: err instanceof Error ? err.message : String(err),
      });
      inFlight.current = false;
    }
  }

  const isInstalled = status?.kind === "installed";

  return (
    <Card className="p-5">
      <div className="mb-3 flex items-center gap-2 text-[var(--color-text-secondary)]">
        <Package className="h-4 w-4" />
        <h2 className="text-xs font-medium uppercase tracking-wider">Contenu</h2>
      </div>

      {flow.kind === "checking" && (
        <p className="text-sm text-[var(--color-text-secondary)]">Recherche d'une mise à jour…</p>
      )}

      {flow.kind === "installing" && (
        <ContentInstallProgressView flow={flow} />
      )}

      {flow.kind === "uptoDate" && (
        <div className="flex items-center gap-2 text-sm">
          <CheckCircle2 className="h-3.5 w-3.5 text-[var(--color-success-500)]" />
          <span className="text-[var(--color-text-secondary)]">
            Contenu à jour · v{flow.version}
          </span>
        </div>
      )}

      {flow.kind === "available" && (
        <div className="flex flex-col gap-3">
          <div className="text-sm">
            <span className="text-[var(--color-text-secondary)]">Mise à jour disponible : </span>
            <span className="font-display font-semibold">v{flow.release.version}</span>
            <span className="ml-1 text-[var(--color-text-muted)]">
              ({formatBytes(flow.release.asset.sizeBytes)})
            </span>
          </div>
          <Button size="sm" onClick={() => handleInstall(flow.release)}>
            <CloudDownload className="h-4 w-4" />
            Télécharger et installer
          </Button>
        </div>
      )}

      {flow.kind === "error" && (
        <div className="flex flex-col gap-3">
          <p className="text-sm text-[var(--color-danger-500)]">{flow.message}</p>
          <Button size="sm" variant="secondary" onClick={() => setFlow({ kind: "idle" })}>
            Fermer
          </Button>
        </div>
      )}

      {flow.kind === "idle" && (
        <div className="flex flex-col gap-3">
          <div className="flex items-center gap-2 text-sm">
            {isInstalled ? (
              <>
                <CheckCircle2 className="h-3.5 w-3.5 text-[var(--color-success-500)]" />
                <span className="text-[var(--color-text-secondary)]">
                  Contenu installé · v{status!.version}
                </span>
              </>
            ) : (
              <>
                <Package className="h-3.5 w-3.5 text-[var(--color-text-muted)]" />
                <span className="text-[var(--color-text-secondary)]">Contenu intégré au launcher</span>
              </>
            )}
          </div>
          <Button size="sm" variant="secondary" onClick={handleCheck}>
            <RefreshCw className="h-4 w-4" />
            Vérifier les mises à jour
          </Button>
        </div>
      )}
    </Card>
  );
}

function ContentInstallProgressView({
  flow,
}: {
  flow: { progress: ContentInstallProgress | null; release: ContentRelease };
}) {
  const progress = flow.progress;
  const phase = progress ? describePhase(progress.phase) : "Préparation";
  const total = progress?.bytesTotal ?? flow.release.asset.sizeBytes;
  const done = progress?.bytesDone ?? 0;
  const ratio = total > 0 ? Math.min(1, done / total) : 0;

  return (
    <div className="flex flex-col gap-2 text-sm">
      <div className="flex items-baseline justify-between gap-2">
        <span className="font-display font-semibold">
          {phase} · v{flow.release.version}
        </span>
        <span className="text-xs text-[var(--color-text-muted)]">
          {formatBytes(done)} / {formatBytes(total)}
        </span>
      </div>
      <div className="h-1.5 w-full overflow-hidden rounded-full bg-[var(--color-bg-overlay)]">
        <div
          className="h-full rounded-full bg-[var(--color-accent-500)] transition-[width] duration-200"
          style={{ width: `${(ratio * 100).toFixed(1)}%` }}
        />
      </div>
    </div>
  );
}

interface PlayPanelProps {
  status: EngineStatus | null;
  flow: Flow;
  onPlay: () => void;
  onStop: () => void;
  onRetry: () => void;
}

function PlayPanel({ status, flow, onPlay, onStop, onRetry }: PlayPanelProps) {
  const isInstalled = status?.kind === "installed";

  if (flow.kind === "fetchingManifest") {
    return (
      <PanelLayout title="Récupération du manifest" subtitle="Recherche de la dernière version disponible…">
        <Spinner />
      </PanelLayout>
    );
  }

  if (flow.kind === "installing") {
    const progress = flow.progress;
    const phase = progress ? describePhase(progress.phase) : "Préparation";
    const total = progress?.bytesTotal ?? flow.release.asset.sizeBytes;
    const done = progress?.bytesDone ?? 0;
    const ratio = total > 0 ? Math.min(1, done / total) : 0;

    return (
      <PanelLayout
        title={`${phase} · Aracdia Engine ${flow.release.version}`}
        subtitle={`${formatBytes(done)} / ${formatBytes(total)}`}
      >
        <div className="h-2 w-full overflow-hidden rounded-full bg-[var(--color-bg-overlay)]">
          <div
            className="h-full rounded-full bg-[var(--color-accent-500)] transition-[width] duration-200"
            style={{ width: `${(ratio * 100).toFixed(1)}%` }}
          />
        </div>
      </PanelLayout>
    );
  }

  if (flow.kind === "starting") {
    return (
      <PanelLayout title="Démarrage du moteur" subtitle="Préparation du processus…">
        <Spinner />
      </PanelLayout>
    );
  }

  if (flow.kind === "running") {
    const { session } = flow;
    const startedAt = formatStartedAt(session.startedAt);
    return (
      <PanelLayout
        icon={<Gamepad2 className="h-5 w-5 text-[var(--color-success-500)]" />}
        title="Jeu en cours"
        subtitle={`PID ${session.pid} · démarré ${startedAt}`}
      >
        <Button variant="danger" className="w-full" onClick={onStop}>
          <Square className="h-4 w-4" />
          Quitter le jeu
        </Button>
      </PanelLayout>
    );
  }

  if (flow.kind === "error") {
    return (
      <PanelLayout
        icon={<XCircle className="h-5 w-5 text-[var(--color-danger-500)]" />}
        title="Échec"
        subtitle={flow.message}
      >
        <Button variant="secondary" className="w-full" onClick={onRetry}>
          Fermer
        </Button>
      </PanelLayout>
    );
  }

  // Idle states (engine installed or not)
  return (
    <>
      <div className="mb-4">
        <h3 className="text-xs font-medium uppercase tracking-wider text-[var(--color-text-muted)]">
          Prêt à jouer
        </h3>
        <p className="mt-1 font-display text-lg font-semibold">Aracdia · monde principal</p>
        <div className="mt-2 flex items-center gap-2 text-xs">
          {isInstalled ? (
            <>
              <CheckCircle2 className="h-3.5 w-3.5 text-[var(--color-success-500)]" />
              <span className="text-[var(--color-text-secondary)]">
                Moteur installé · {status!.version}
              </span>
            </>
          ) : (
            <>
              <Download className="h-3.5 w-3.5 text-[var(--color-text-muted)]" />
              <span className="text-[var(--color-text-secondary)]">
                Moteur non installé · sera téléchargé au lancement
              </span>
            </>
          )}
        </div>
      </div>

      <div className="mt-auto">
        <Button size="lg" className="w-full" onClick={onPlay}>
          {isInstalled ? <Play className="h-5 w-5" /> : <Download className="h-5 w-5" />}
          {isInstalled ? "JOUER" : "INSTALLER ET JOUER"}
        </Button>
      </div>
    </>
  );
}

function PanelLayout({
  title,
  subtitle,
  icon,
  children,
}: {
  title: string;
  subtitle?: string;
  icon?: React.ReactNode;
  children?: React.ReactNode;
}) {
  return (
    <>
      <div className="mb-4 flex items-start gap-2">
        {icon}
        <div className="min-w-0 flex-1">
          <h3 className="font-display text-base font-semibold tracking-tight">{title}</h3>
          {subtitle ? (
            <p className="mt-1 truncate text-xs text-[var(--color-text-secondary)]">{subtitle}</p>
          ) : null}
        </div>
      </div>
      <div className="mt-auto">{children}</div>
    </>
  );
}

function Spinner() {
  return (
    <div className="flex h-12 items-center justify-center">
      <div className="h-6 w-6 animate-spin rounded-full border-2 border-[var(--color-border-strong)] border-t-[var(--color-accent-500)]" />
    </div>
  );
}

const RELATIVE_FORMATTER = new Intl.RelativeTimeFormat("fr", { numeric: "auto" });

function formatStartedAt(iso: string): string {
  const t = Date.parse(iso);
  if (Number.isNaN(t)) return "à l'instant";
  const diffSec = Math.round((t - Date.now()) / 1000);
  const abs = Math.abs(diffSec);
  if (abs < 60) return RELATIVE_FORMATTER.format(diffSec, "second");
  if (abs < 3600) return RELATIVE_FORMATTER.format(Math.round(diffSec / 60), "minute");
  if (abs < 86_400) return RELATIVE_FORMATTER.format(Math.round(diffSec / 3600), "hour");
  return RELATIVE_FORMATTER.format(Math.round(diffSec / 86_400), "day");
}
