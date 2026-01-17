import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useTheme } from '../../hooks/useTheme';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Settings, Moon, Sun, ChevronDown } from 'lucide-react';
import { clearErrorLogs } from '@/lib/errorLog';
import { clearHistory } from '@/lib/history';
import {
  getDiagnosticsSettings,
  setDiagnosticsSettings,
  DiagnosticsSettings,
} from '@/lib/diagnosticsSettings';
import {
  getSafetyPolicy,
  setSafetyPolicy,
  SafetyPolicy,
} from '@/lib/tauri';

export function SettingsPage() {
  const { t, i18n } = useTranslation();
  const { theme, setTheme } = useTheme();
  const [diagnostics, setDiagnostics] = useState<DiagnosticsSettings>(
    getDiagnosticsSettings()
  );
  const [policy, setPolicy] = useState<SafetyPolicy | null>(null);
  const [policyError, setPolicyError] = useState<string | null>(null);
  const [policySaving, setPolicySaving] = useState(false);

  useEffect(() => {
    let active = true;
    getSafetyPolicy()
      .then(result => {
        if (!active) return;
        if (result.success && result.policy) {
          setPolicy(result.policy);
          setPolicyError(null);
        } else {
          setPolicyError(result.error || t('settings.safetyPolicyError'));
        }
      })
      .catch(() => {
        if (!active) return;
        setPolicyError(t('settings.safetyPolicyError'));
      });

    return () => {
      active = false;
    };
  }, [t]);

  function updateDiagnostics(next: DiagnosticsSettings) {
    setDiagnostics(next);
    setDiagnosticsSettings(next);
    if (!next.storeHistory) {
      clearHistory();
    }
    if (!next.storeErrorLogs) {
      clearErrorLogs();
    }
  }

  async function updatePolicy(next: SafetyPolicy) {
    setPolicy(next);
    setPolicySaving(true);
    setPolicyError(null);

    try {
      const result = await setSafetyPolicy(next);
      if (result.success && result.policy) {
        setPolicy(result.policy);
      } else {
        setPolicyError(result.error || t('settings.safetyPolicyError'));
      }
    } catch {
      setPolicyError(t('settings.safetyPolicyError'));
    } finally {
      setPolicySaving(false);
    }
  }

  return (
    <div className="flex flex-col h-full bg-background p-8 overflow-auto">
      <div className="max-w-2xl mx-auto w-full space-y-8">
        <div className="flex items-center gap-3 mb-8">
          <div className="p-3 rounded-lg bg-primary/10 text-primary">
            <Settings size={32} />
          </div>
          <div>
            <h1 className="text-3xl font-bold tracking-tight">{t('settings.title')}</h1>
          </div>
        </div>

        <div className="grid gap-6">
          <div className="rounded-lg border border-border bg-card text-card-foreground shadow-sm">
            <div className="flex flex-col space-y-1.5 p-6">
              <h3 className="font-semibold leading-none tracking-tight">
                {t('settings.language')}
              </h3>
              <p className="text-sm text-muted-foreground">{t('settings.languageDescription')}</p>
            </div>
            <div className="p-6 pt-0">
              <DropdownMenu>
                <DropdownMenuTrigger asChild>
                  <Button variant="outline" className="w-50 justify-between">
                    {i18n.language.startsWith('fr') ? 'Français' : 'English'}
                    <ChevronDown className="ml-2 h-4 w-4 opacity-50" />
                  </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent className="w-50">
                  <DropdownMenuItem onClick={() => i18n.changeLanguage('en')}>
                    English
                  </DropdownMenuItem>
                  <DropdownMenuItem onClick={() => i18n.changeLanguage('fr')}>
                    Français
                  </DropdownMenuItem>
                </DropdownMenuContent>
              </DropdownMenu>
            </div>
          </div>

          <div className="rounded-lg border border-border bg-card text-card-foreground shadow-sm">
            <div className="flex flex-col space-y-1.5 p-6">
              <h3 className="font-semibold leading-none tracking-tight">{t('settings.theme')}</h3>
              <p className="text-sm text-muted-foreground">{t('settings.themeDescription')}</p>
            </div>
            <div className="p-6 pt-0">
              <DropdownMenu>
                <DropdownMenuTrigger asChild>
                  <Button variant="outline" className="w-50 justify-between">
                    <div className="flex items-center gap-2">
                      {theme === 'dark' ? <Moon size={16} /> : <Sun size={16} />}
                      {theme === 'dark' ? t('settings.themeDark') : t('settings.themeLight')}
                    </div>
                    <ChevronDown className="ml-2 h-4 w-4 opacity-50" />
                  </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent className="w-50">
                  <DropdownMenuItem onClick={() => setTheme('light')}>
                    <div className="flex items-center gap-2">
                      <Sun size={16} />
                      {t('settings.themeLight')}
                    </div>
                  </DropdownMenuItem>
                  <DropdownMenuItem onClick={() => setTheme('dark')}>
                    <div className="flex items-center gap-2">
                      <Moon size={16} />
                      {t('settings.themeDark')}
                    </div>
                  </DropdownMenuItem>
                </DropdownMenuContent>
              </DropdownMenu>
            </div>
          </div>

          <div className="rounded-lg border border-border bg-card text-card-foreground shadow-sm">
            <div className="flex flex-col space-y-1.5 p-6">
              <h3 className="font-semibold leading-none tracking-tight">
                {t('settings.diagnostics')}
              </h3>
              <p className="text-sm text-muted-foreground">
                {t('settings.diagnosticsDescription')}
              </p>
            </div>
            <div className="p-6 pt-0 space-y-4">
              <label className="flex items-start gap-3 text-sm">
                <Checkbox
                  checked={diagnostics.storeHistory}
                  onCheckedChange={checked =>
                    updateDiagnostics({
                      ...diagnostics,
                      storeHistory: !!checked,
                    })
                  }
                />
                <span>
                  <span className="font-medium">{t('settings.storeHistory')}</span>
                  <span className="block text-xs text-muted-foreground">
                    {t('settings.storeHistoryDescription')}
                  </span>
                </span>
              </label>

              <label className="flex items-start gap-3 text-sm">
                <Checkbox
                  checked={diagnostics.storeErrorLogs}
                  onCheckedChange={checked =>
                    updateDiagnostics({
                      ...diagnostics,
                      storeErrorLogs: !!checked,
                    })
                  }
                />
                <span>
                  <span className="font-medium">{t('settings.storeErrorLogs')}</span>
                  <span className="block text-xs text-muted-foreground">
                    {t('settings.storeErrorLogsDescription')}
                  </span>
                </span>
              </label>
            </div>
          </div>

          <div className="rounded-lg border border-border bg-card text-card-foreground shadow-sm">
            <div className="flex flex-col space-y-1.5 p-6">
              <h3 className="font-semibold leading-none tracking-tight">
                {t('settings.safetyPolicy')}
              </h3>
              <p className="text-sm text-muted-foreground">
                {t('settings.safetyPolicyDescription')}
              </p>
            </div>
            <div className="p-6 pt-0 space-y-4">
              <label className="flex items-start gap-3 text-sm">
                <Checkbox
                  checked={policy?.prod_require_confirmation ?? false}
                  disabled={!policy || policySaving}
                  onCheckedChange={checked =>
                    policy &&
                    updatePolicy({
                      ...policy,
                      prod_require_confirmation: !!checked,
                    })
                  }
                />
                <span>
                  <span className="font-medium">
                    {t('settings.safetyPolicyRequireConfirmation')}
                  </span>
                  <span className="block text-xs text-muted-foreground">
                    {t('settings.safetyPolicyRequireConfirmationDescription')}
                  </span>
                </span>
              </label>

              <label className="flex items-start gap-3 text-sm">
                <Checkbox
                  checked={policy?.prod_block_dangerous_sql ?? false}
                  disabled={!policy || policySaving}
                  onCheckedChange={checked =>
                    policy &&
                    updatePolicy({
                      ...policy,
                      prod_block_dangerous_sql: !!checked,
                    })
                  }
                />
                <span>
                  <span className="font-medium">
                    {t('settings.safetyPolicyBlockDangerous')}
                  </span>
                  <span className="block text-xs text-muted-foreground">
                    {t('settings.safetyPolicyBlockDangerousDescription')}
                  </span>
                </span>
              </label>

              <p className="text-xs text-muted-foreground">
                {t('settings.safetyPolicyNote')}
              </p>

              {policyError && (
                <p className="text-xs text-destructive">{policyError}</p>
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
