// SPDX-License-Identifier: Apache-2.0

import { FileText } from 'lucide-react';
import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { LicenseGate } from '@/components/License/LicenseGate';
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import type { AuditLogTab } from '@/lib/stores/modalStore';
import { cn } from '@/lib/utils';
import { AuditLogPanel } from './AuditLogPanel';
import { ProfilingPanel } from './ProfilingPanel';
import { QueryTrendsPanel } from './QueryTrendsPanel';

interface AuditLogModalProps {
  isOpen: boolean;
  initialTab: AuditLogTab;
  onClose: () => void;
}

const TABS: AuditLogTab[] = ['audit', 'profiling', 'trends'];

export function AuditLogModal({ isOpen, initialTab, onClose }: AuditLogModalProps) {
  const { t } = useTranslation();
  const [tab, setTab] = useState<AuditLogTab>(initialTab);

  useEffect(() => {
    if (isOpen) setTab(initialTab);
  }, [isOpen, initialTab]);

  return (
    <Dialog open={isOpen} onOpenChange={open => !open && onClose()}>
      <DialogContent
        disableExitAnimation
        className="max-w-4xl max-h-[85vh] h-[85vh] flex flex-col p-0 gap-0"
      >
        <DialogHeader className="px-4 py-3 border-b border-border">
          <DialogTitle className="flex items-center gap-2 text-base">
            <FileText size={18} />
            {t('interceptor.audit.title')}
          </DialogTitle>
        </DialogHeader>
        <div className="flex gap-1 px-2 py-1.5 border-b border-border">
          {TABS.map(id => (
            <button
              key={id}
              type="button"
              className={cn(
                'px-3 py-1 text-sm rounded transition-colors',
                tab === id ? 'bg-primary text-primary-foreground' : 'hover:bg-muted'
              )}
              onClick={() => setTab(id)}
            >
              {t(`interceptor.audit.tabs.${id}`)}
            </button>
          ))}
        </div>
        <div className="flex-1 min-h-0 overflow-hidden">
          {tab === 'audit' && <AuditLogPanel />}
          {tab === 'profiling' && (
            <LicenseGate feature="profiling">
              <ProfilingPanel />
            </LicenseGate>
          )}
          {tab === 'trends' && <QueryTrendsPanel />}
        </div>
      </DialogContent>
    </Dialog>
  );
}
