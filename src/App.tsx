import { useState, useEffect } from 'react';
import { Sidebar } from './components/Sidebar/Sidebar';
import { TabBar } from './components/Tabs/TabBar';
import { GlobalSearch } from './components/Search/GlobalSearch';
import { QueryPanel } from './components/Query/QueryPanel';
import { ConnectionModal } from './components/Connection/ConnectionModal';
import './index.css';
import './App.css';

type Driver = 'postgres' | 'mysql' | 'mongodb';

function App() {
  const [searchOpen, setSearchOpen] = useState(false);
  const [connectionModalOpen, setConnectionModalOpen] = useState(false);
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [driver, setDriver] = useState<Driver>('postgres');

  // Global keyboard shortcuts
  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault();
        setSearchOpen(true);
      }
      // Cmd+N: New connection
      if ((e.metaKey || e.ctrlKey) && e.key === 'n') {
        e.preventDefault();
        setConnectionModalOpen(true);
      }
    }

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, []);

  function handleConnected(newSessionId: string, newDriver: string) {
    setSessionId(newSessionId);
    setDriver(newDriver as Driver);
  }

  return (
    <>
      <div className="layout">
        <Sidebar
          onNewConnection={() => setConnectionModalOpen(true)}
          onConnected={handleConnected}
          connectedSessionId={sessionId}
        />
        <main className="layout-main">
          <TabBar />
          <div className="layout-content">
            {sessionId ? (
              <QueryPanel sessionId={sessionId} dialect={driver} />
            ) : (
              <div className="welcome-screen">
                <h2>Welcome to QoreDB</h2>
                <p className="text-muted">
                  Add a connection in the sidebar to get started.
                </p>
                <button 
                  className="welcome-btn"
                  onClick={() => setConnectionModalOpen(true)}
                >
                  + New Connection
                </button>
                <p className="text-hint">
                  or press <kbd>Cmd+N</kbd>
                </p>
              </div>
            )}
          </div>
        </main>
      </div>

      <ConnectionModal
        isOpen={connectionModalOpen}
        onClose={() => setConnectionModalOpen(false)}
        onConnected={handleConnected}
      />

      <GlobalSearch
        isOpen={searchOpen}
        onClose={() => setSearchOpen(false)}
      />
    </>
  );
}

export default App;
