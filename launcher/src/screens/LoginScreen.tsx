import { useState, type FormEvent } from "react";
import { Card } from "@/components/ui/Card";
import { Input } from "@/components/ui/Input";
import { Button } from "@/components/ui/Button";
import { saveProfile, validateUsername, type PlayerProfile } from "@/lib/profile";

interface LoginScreenProps {
  onLoggedIn: (profile: PlayerProfile) => void;
}

export function LoginScreen({ onLoggedIn }: LoginScreenProps) {
  const [username, setUsername] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const validationError = validateUsername(username);
    if (validationError) {
      setError(validationError);
      return;
    }
    setError(null);
    setSubmitting(true);
    try {
      const profile = await saveProfile(username.trim());
      onLoggedIn(profile);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Erreur inconnue");
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div className="flex h-full w-full items-center justify-center px-6">
      <Card className="w-full max-w-md p-8">
        <header className="mb-6 text-center">
          <div className="mx-auto mb-4 flex h-12 w-12 items-center justify-center rounded-xl bg-gradient-to-br from-indigo-500 to-purple-600 shadow-lg shadow-indigo-900/40">
            <span className="font-display text-xl font-black text-white">A</span>
          </div>
          <h1 className="font-display text-2xl font-bold tracking-tight">
            Bienvenue sur Aracdia
          </h1>
          <p className="mt-1 text-sm text-[var(--color-text-secondary)]">
            Choisis ton pseudo pour commencer
          </p>
        </header>

        <form onSubmit={handleSubmit} className="space-y-4">
          <div className="space-y-2">
            <label
              htmlFor="username"
              className="block text-xs font-medium uppercase tracking-wider text-[var(--color-text-muted)]"
            >
              Pseudo
            </label>
            <Input
              id="username"
              autoFocus
              autoComplete="off"
              spellCheck={false}
              maxLength={16}
              placeholder="Aragorn_42"
              value={username}
              invalid={!!error}
              onChange={(e) => {
                setUsername(e.currentTarget.value);
                if (error) setError(null);
              }}
            />
            {error ? (
              <p className="text-xs text-[var(--color-danger-500)]">{error}</p>
            ) : (
              <p className="text-xs text-[var(--color-text-muted)]">
                3–16 caractères, lettres, chiffres et underscore.
              </p>
            )}
          </div>

          <Button
            type="submit"
            size="lg"
            className="w-full"
            disabled={submitting || username.trim().length === 0}
          >
            {submitting ? "Création…" : "Continuer"}
          </Button>
        </form>

        <p className="mt-6 text-center text-xs text-[var(--color-text-muted)]">
          Mode hors-ligne · Aucune donnée envoyée à un serveur
        </p>
      </Card>
    </div>
  );
}
