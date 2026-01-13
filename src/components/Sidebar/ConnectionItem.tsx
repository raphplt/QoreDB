import { SavedConnection } from '../../lib/tauri';
import './ConnectionItem.css';

interface ConnectionItemProps {
  connection: SavedConnection;
  isSelected: boolean;
  isExpanded: boolean;
  onSelect: () => void;
}

const DRIVER_ICONS: Record<string, string> = {
  postgres: '🐘',
  mysql: '🐬',
  mongodb: '🍃',
};

export function ConnectionItem({ connection, isSelected, isExpanded, onSelect }: ConnectionItemProps) {
  const icon = DRIVER_ICONS[connection.driver] || '📦';

  return (
    <button
      className={`connection-item ${isSelected ? 'selected' : ''}`}
      onClick={onSelect}
    >
      <span className="connection-icon">{icon}</span>
      <span className="connection-name truncate">{connection.name}</span>
      <span className="connection-chevron">{isExpanded ? '▼' : '▶'}</span>
    </button>
  );
}
