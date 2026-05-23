import { useEffect, useState } from "react";
import { ArrowLeft, RotateCcw, Save, ServerCog, Cpu, Globe } from "lucide-react";
import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";
import { Input } from "@/components/ui/Input";
import {
  DEFAULT_SETTINGS,
  hasErrors,
  loadSettings,
  resetSettings,
  saveSettings,
  SETTINGS_RULES,
  validateSettings,
  type LauncherSettings,
} from "@/lib/settings";

interface SettingsScreenProps {
  onBack: () => void;
}

type Status = { kind: "idle" } | { kind: "saving" } | { kind: "saved" } | { kind: "error"; message: string };

export function SettingsScreen({ onBack }: SettingsScreenProps) {
  const [settings, setSettings] = useState<LauncherSettings>(DEFAULT_SETTINGS);
  const [initialSettings, setInitialSettings] = useState<LauncherSettings>(DEFAULT_SETTINGS);
  const [loading, setLoading] = useState(true);
  const [status, setStatus] = useState<Status>({ kind: "idle" });

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const loaded = await loadSettings();
        if (cancelled) return;
        setSettings(loaded);
        setInitialSettings(loaded);
      } catch (err) {
        console.error("Failed to load settings", err);
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  const errors = validateSettings(settings);
  const isDirty = JSON.stringify(settings) !== JSON.stringify(initialSettings);
  const canSave = !hasErrors(errors) && isDirty && status.kind !== "saving";

  function update<K extends keyof LauncherSettings>(key: K, value: LauncherSettings[K]) {
    setSettings((prev) => ({ ...prev, [key]: value }));
    if (status.kind !== "idle") setStatus({ kind: "idle" });
  }

  async function handleSave() {
    if (!canSave) return;
    setStatus({ kind: "saving" });
    try {
      const persisted = await saveSettings(settings);
      setSettings(persisted);
      setInitialSettings(persisted);
      setStatus({ kind: "saved" });
    } catch (err) {
      setStatus({
        kind: "error",
        message: err instanceof Error ? err.message : "Échec de la sauvegarde.",
      });
    }
  }

  async function handleReset() {
    setStatus({ kind: "saving" });
    try {
      const fresh = await resetSettings();
      setSettings(fresh);
      setInitialSettings(fresh);
      setStatus({ kind: "saved" });
    } catch (err) {
      setStatus({
        kind: "error",
        message: err instanceof Error ? err.message : "Échec de la réinitialisation.",
      });
    }
  }

  if (loading) {
    return (
      <div className="flex h-full w-full items-center justify-center">
        <div className="h-8 w-8 animate-spin rounded-full border-2 border-[var(--color-border-strong)] border-t-[var(--color-accent-500)]" />
      </div>
    );
  }

  return (
    <div className="flex h-full w-full flex-col">
      {/* Header */}
      <header className="drag-region flex h-14 items-center justify-between border-b border-[var(--color-border-subtle)] px-6">
        <div className="no-drag flex items-center gap-3">
          <Button variant="ghost" size="sm" onClick={onBack} aria-label="Retour">
            <ArrowLeft className="h-4 w-4" />
          </Button>
          <h1 className="font-display text-base font-semibold tracking-tight">Paramètres</h1>
        </div>

        <div className="no-drag flex items-center gap-3">
          <StatusIndicator status={status} />
          <Button variant="secondary" size="sm" onClick={handleReset} disabled={status.kind === "saving"}>
            <RotateCcw className="h-4 w-4" />
            Par défaut
          </Button>
          <Button size="sm" onClick={handleSave} disabled={!canSave}>
            <Save className="h-4 w-4" />
            Enregistrer
          </Button>
        </div>
      </header>

      {/* Content */}
      <main className="flex-1 overflow-auto p-6">
        <div className="mx-auto flex max-w-3xl flex-col gap-6">
          <SettingsSection
            icon={<Cpu className="h-4 w-4" />}
            title="Performances"
            description="Réglages liés au moteur de jeu."
          >
            <Field
              label="Mémoire allouée"
              hint={`${SETTINGS_RULES.memoryMin}–${SETTINGS_RULES.memoryMax} Mio · 1024 Mio = 1 Gio`}
              error={errors.memoryMb}
            >
              <div className="flex items-center gap-3">
                <Input
                  type="number"
                  inputMode="numeric"
                  min={SETTINGS_RULES.memoryMin}
                  max={SETTINGS_RULES.memoryMax}
                  step={256}
                  invalid={!!errors.memoryMb}
                  value={settings.memoryMb}
                  onChange={(e) => update("memoryMb", Number(e.currentTarget.value))}
                  className="max-w-[160px]"
                />
                <span className="text-sm text-[var(--color-text-secondary)]">Mio</span>
              </div>
            </Field>
          </SettingsSection>

          <SettingsSection
            icon={<ServerCog className="h-4 w-4" />}
            title="Serveur par défaut"
            description="Adresse utilisée par le bouton JOUER."
          >
            <Field label="Adresse" error={errors.serverAddress}>
              <Input
                placeholder="play.aracdia.example (laisser vide = solo)"
                invalid={!!errors.serverAddress}
                value={settings.serverAddress}
                onChange={(e) => update("serverAddress", e.currentTarget.value)}
                maxLength={SETTINGS_RULES.serverAddressMax}
              />
            </Field>

            <Field label="Port" hint={`${SETTINGS_RULES.portMin}–${SETTINGS_RULES.portMax}`} error={errors.serverPort}>
              <Input
                type="number"
                inputMode="numeric"
                min={SETTINGS_RULES.portMin}
                max={SETTINGS_RULES.portMax}
                invalid={!!errors.serverPort}
                value={settings.serverPort}
                onChange={(e) => update("serverPort", Number(e.currentTarget.value))}
                className="max-w-[160px]"
              />
            </Field>

            <Toggle
              label="Connexion automatique"
              description="Au lancement, se connecter directement au serveur configuré."
              checked={settings.autoConnect}
              onChange={(checked) => update("autoConnect", checked)}
            />
          </SettingsSection>

          <SettingsSection
            icon={<Globe className="h-4 w-4" />}
            title="Installation"
            description="Emplacement des fichiers téléchargés (moteur, contenu de jeu)."
          >
            <Field
              label="Dossier d'installation personnalisé"
              hint="Vide = emplacement par défaut du système d'exploitation."
            >
              <Input
                placeholder="/Users/<vous>/Aracdia (laisser vide pour le défaut OS)"
                value={settings.installDir ?? ""}
                onChange={(e) => {
                  const value = e.currentTarget.value;
                  update("installDir", value.length === 0 ? null : value);
                }}
              />
            </Field>
          </SettingsSection>
        </div>
      </main>
    </div>
  );
}

function StatusIndicator({ status }: { status: Status }) {
  if (status.kind === "saving") {
    return <span className="text-xs text-[var(--color-text-secondary)]">Enregistrement…</span>;
  }
  if (status.kind === "saved") {
    return <span className="text-xs text-[var(--color-success-500)]">Enregistré</span>;
  }
  if (status.kind === "error") {
    return <span className="text-xs text-[var(--color-danger-500)]">{status.message}</span>;
  }
  return null;
}

interface SectionProps {
  icon: React.ReactNode;
  title: string;
  description?: string;
  children: React.ReactNode;
}

function SettingsSection({ icon, title, description, children }: SectionProps) {
  return (
    <Card className="p-6">
      <div className="mb-5 flex items-start gap-3">
        <span className="mt-0.5 inline-flex h-7 w-7 items-center justify-center rounded-md bg-[var(--color-bg-elevated)] text-[var(--color-text-secondary)] ring-1 ring-inset ring-[var(--color-border-subtle)]">
          {icon}
        </span>
        <div>
          <h2 className="font-display text-base font-semibold tracking-tight">{title}</h2>
          {description ? (
            <p className="mt-0.5 text-xs text-[var(--color-text-secondary)]">{description}</p>
          ) : null}
        </div>
      </div>
      <div className="space-y-5">{children}</div>
    </Card>
  );
}

interface FieldProps {
  label: string;
  hint?: string;
  error?: string;
  children: React.ReactNode;
}

function Field({ label, hint, error, children }: FieldProps) {
  return (
    <div className="space-y-1.5">
      <label className="block text-xs font-medium uppercase tracking-wider text-[var(--color-text-muted)]">
        {label}
      </label>
      {children}
      {error ? (
        <p className="text-xs text-[var(--color-danger-500)]">{error}</p>
      ) : hint ? (
        <p className="text-xs text-[var(--color-text-muted)]">{hint}</p>
      ) : null}
    </div>
  );
}

interface ToggleProps {
  label: string;
  description?: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
}

function Toggle({ label, description, checked, onChange }: ToggleProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      onClick={() => onChange(!checked)}
      className="flex w-full items-center justify-between gap-4 rounded-lg p-3 text-left transition-colors hover:bg-white/[0.03]"
    >
      <div>
        <div className="text-sm font-medium text-[var(--color-text-primary)]">{label}</div>
        {description ? (
          <div className="mt-0.5 text-xs text-[var(--color-text-secondary)]">{description}</div>
        ) : null}
      </div>
      <span
        className={
          "relative inline-block h-6 w-11 shrink-0 rounded-full transition-colors " +
          (checked ? "bg-[var(--color-accent-600)]" : "bg-[var(--color-bg-overlay)]")
        }
      >
        <span
          className={
            "absolute top-0.5 left-0.5 h-5 w-5 rounded-full bg-white shadow transition-transform " +
            (checked ? "translate-x-5" : "translate-x-0")
          }
        />
      </span>
    </button>
  );
}
