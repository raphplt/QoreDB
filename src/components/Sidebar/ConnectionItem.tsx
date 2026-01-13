import { SavedConnection } from '../../lib/tauri';
import { Loader2, ChevronRight, ChevronDown } from 'lucide-react';
import { cn } from '@/lib/utils';

interface ConnectionItemProps {
  connection: SavedConnection;
  isSelected: boolean;
  isExpanded: boolean;
  isConnected?: boolean;
  isConnecting?: boolean;
  onSelect: () => void;
}

const DRIVER_ICONS: Record<string, string> = {
  postgres: '🐘',
  mysql: '🐬',
  mongodb: '🍃',
};

export function ConnectionItem({ 
  connection, 
  isSelected, 
  isExpanded, 
  isConnected,
  isConnecting,
  onSelect 
}: ConnectionItemProps) {
  const icon = DRIVER_ICONS[connection.driver] || '📦';

  return (
    <button
      className={cn(
        "group w-full flex items-center gap-2 px-2 py-1.5 text-sm rounded-md transition-all select-none",
        "hover:bg-accent/10 hover:text-accent-foreground",
        isSelected && !isConnected && "bg-muted text-foreground",
        isSelected && isConnected && "bg-accent/15 text-accent font-medium",
        !isSelected && "text-muted-foreground"
      )}
      onClick={onSelect}
      disabled={isConnecting}
    >
      <span className="shrink-0 text-base opacity-80 group-hover:opacity-100">
        {icon}
      </span>
      
      <span className="flex-1 truncate text-left">
        {connection.name}
      </span>
      
      {isConnecting ? (
        <Loader2 size={14} className="animate-spin text-muted-foreground" />
      ) : isConnected && !isConnecting ? (
        <span className="w-2 h-2 rounded-full bg-success shadow-sm shadow-success/50" />
      ) : null}
      
      <div className={cn("text-muted-foreground/50", isExpanded && "transform rotate-90")}>
        {isExpanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
      </div>
    </button>
  );
}
