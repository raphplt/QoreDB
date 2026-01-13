import { useState, useEffect } from 'react';
import { ConnectionItem } from './ConnectionItem';
import { DBTree } from '../Tree/DBTree';
import { listSavedConnections, connect, getConnectionCredentials, SavedConnection, ConnectionConfig } from '../../lib/tauri';
import { useTheme } from '../../hooks/useTheme';
import './Sidebar.css';

const DEFAULT_PROJECT = 'default';

interface SidebarProps {
  onNewConnection: () => void;
  onConnected: (sessionId: string, driver: string) => void;
  connectedSessionId: string | null;
}

export function Sidebar({ onNewConnection, onConnected, connectedSessionId }: SidebarProps) {
  const [connections, setConnections] = useState<SavedConnection[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [connecting, setConnecting] = useState<string | null>(null);
  const { isDark, toggleTheme } = useTheme();

  useEffect(() => {
    loadConnections();
  }, []);

  useEffect(() => {
    loadConnections();
  }, [connectedSessionId]);

  async function loadConnections() {
    try {
      const saved = await listSavedConnections(DEFAULT_PROJECT);
      setConnections(saved);
    } catch (err) {
      console.error('Failed to load connections:', err);
    }
  }

  async function handleConnect(conn: SavedConnection) {
    setConnecting(conn.id);
    setSelectedId(conn.id);

    try {
      const credsResult = await getConnectionCredentials('default', conn.id);
      
      if (!credsResult.success || !credsResult.password) {
        console.error('Failed to get credentials:', credsResult.error);
        return;
      }

      const config: ConnectionConfig = {
        driver: conn.driver,
        host: conn.host,
        port: conn.port,
        username: conn.username,
        password: credsResult.password,
        database: conn.database,
        ssl: conn.ssl,
      };

      const result = await connect(config);
      
      if (result.success && result.session_id) {
        onConnected(result.session_id, conn.driver);
        setExpandedId(conn.id);
      } else {
        console.error('Connection failed:', result.error);
      }
    } catch (err) {
      console.error('Connection error:', err);
    } finally {
      setConnecting(null);
    }
  }

  function handleSelect(conn: SavedConnection) {
    if (connectedSessionId && selectedId === conn.id) {
      setExpandedId(expandedId === conn.id ? null : conn.id);
    } else {
      handleConnect(conn);
    }
  }

  return (
    <aside className="sidebar">
      <header className="sidebar-header">
        <h1 className="sidebar-title">QoreDB</h1>
        <button
          className="sidebar-theme-toggle"
          onClick={toggleTheme}
          title={isDark ? 'Switch to light mode' : 'Switch to dark mode'}
        >
          {isDark ? '☀️' : '🌙'}
        </button>
      </header>

      <section className="sidebar-section">
        <h2 className="sidebar-section-title">Connections</h2>
        <div className="sidebar-connections">
          {connections.length === 0 ? (
            <p className="sidebar-empty">No saved connections</p>
          ) : (
            connections.map(conn => (
              <div key={conn.id}>
                <ConnectionItem
                  connection={conn}
                  isSelected={selectedId === conn.id}
                  isExpanded={expandedId === conn.id}
                  isConnected={connectedSessionId !== null && selectedId === conn.id}
                  isConnecting={connecting === conn.id}
                  onSelect={() => handleSelect(conn)}
                />
                {expandedId === conn.id && connectedSessionId && (
                  <DBTree connectionId={conn.id} />
                )}
              </div>
            ))
          )}
        </div>
      </section>

      <footer className="sidebar-footer">
        <button className="sidebar-add-btn" onClick={onNewConnection}>
          + New Connection
        </button>
      </footer>
    </aside>
  );
}
