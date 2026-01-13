import { useState, useEffect } from 'react';
import { Sidebar } from './components/Sidebar/Sidebar';
import { TabBar } from './components/Tabs/TabBar';
import { GlobalSearch } from './components/Search/GlobalSearch';
import { QueryPanel } from './components/Query/QueryPanel';
import { ConnectionModal } from './components/Connection/ConnectionModal';
import { Button } from './components/ui/button';
import { Search, Database } from 'lucide-react';
import './index.css';

type Driver = 'postgres' | 'mysql' | 'mongodb';

function App() {
  const [searchOpen, setSearchOpen] = useState(false);
  const [connectionModalOpen, setConnectionModalOpen] = useState(false);
  const [sessionId, setSessionId] = useState<string | null>(null);
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
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
      <div className="flex h-screen w-screen overflow-hidden bg-background text-foreground font-sans">
        <Sidebar
          onNewConnection={() => setConnectionModalOpen(true)}
          onConnected={handleConnected}
          connectedSessionId={sessionId}
        />
        <main className="flex-1 flex flex-col min-w-0 bg-background">
          <TabBar />
          <div className="flex-1 overflow-auto p-4">
            {sessionId ? (
              <QueryPanel sessionId={sessionId} dialect={driver} />
            ) : (
              <div className="flex flex-col items-center justify-center h-full text-center space-y-4">
                <div className="p-4 rounded-full bg-accent/10 text-accent mb-2">
                  <Database size={48} strokeWidth={1.5} />
                </div>
                <h2 className="text-2xl font-semibold tracking-tight">Welcome to QoreDB</h2>
                <p className="text-muted-foreground max-w-[400px]">
                  Add a connection in the sidebar to get started, or search for existing resources.
                </p>
                <div className="flex flex-col gap-2 min-w-[200px]">
                  <Button 
                    onClick={() => setConnectionModalOpen(true)}
                    className="w-full"
                  >
                    + New Connection
                  </Button>
                  <Button 
                    variant="outline" 
                    onClick={() => setSearchOpen(true)}
                    className="w-full text-muted-foreground"
                  >
                    <Search className="mr-2 h-4 w-4" />
                    Search <kbd className="ml-auto pointer-events-none inline-flex h-5 select-none items-center gap-1 rounded border bg-muted px-1.5 font-mono text-[10px] font-medium text-muted-foreground opacity-100"><span className="text-xs">⌘</span>K</kbd>
                  </Button>
                </div>
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
