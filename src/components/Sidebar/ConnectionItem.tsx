import { SavedConnection } from '../../lib/tauri';
import './ConnectionItem.css';

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
      className={`connection-item ${isSelected ? 'selected' : ''} ${isConnected ? 'connected' : ''}`}
      onClick={onSelect}
      disabled={isConnecting}
    >
      <span className="connection-icon">{icon}</span>
      <span className="connection-name truncate">{connection.name}</span>
      {isConnecting && <span className="connection-status">⏳</span>}
      {isConnected && !isConnecting && <span className="connection-status connected">●</span>}
      <span className="connection-chevron">{isExpanded ? '▼' : '▶'}</span>
    </button>
  );
}
