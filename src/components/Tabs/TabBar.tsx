import './TabBar.css';

export interface TabItem {
  id: string;
  title: string;
  type: 'query' | 'table' | 'settings';
}

interface TabBarProps {
  tabs?: TabItem[];
  activeId?: string;
  onSelect?: (id: string) => void;
  onClose?: (id: string) => void;
  onNew?: () => void;
}

export function TabBar({ 
  tabs = [], 
  activeId, 
  onSelect, 
  onClose, 
  onNew 
}: TabBarProps) {
  return (
    <div className="tab-bar">
      <div className="tab-list">
        {tabs.map(tab => (
          <button
            key={tab.id}
            className={`tab ${activeId === tab.id ? 'active' : ''}`}
            onClick={() => onSelect?.(tab.id)}
          >
            <span className="tab-title truncate">{tab.title}</span>
            <span
              className="tab-close"
              onClick={(e) => {
                e.stopPropagation();
                onClose?.(tab.id);
              }}
            >
              ×
            </span>
          </button>
        ))}
      </div>
      <button className="tab-new" onClick={onNew} title="New Query (Cmd+T)">
        +
      </button>
    </div>
  );
}
