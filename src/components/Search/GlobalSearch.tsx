import { useState, useEffect, useRef } from 'react';
import './GlobalSearch.css';

interface GlobalSearchProps {
  isOpen: boolean;
  onClose: () => void;
  onSelect?: (result: SearchResult) => void;
}

interface SearchResult {
  type: 'connection' | 'table' | 'query';
  id: string;
  label: string;
  sublabel?: string;
}

export function GlobalSearch({ isOpen, onClose, onSelect }: GlobalSearchProps) {
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<SearchResult[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (isOpen) {
      inputRef.current?.focus();
      setQuery('');
      setResults([]);
      setSelectedIndex(0);
    }
  }, [isOpen]);

  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if (!isOpen) return;

      if (e.key === 'Escape') {
        onClose();
      } else if (e.key === 'ArrowDown') {
        e.preventDefault();
        setSelectedIndex(i => Math.min(i + 1, results.length - 1));
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        setSelectedIndex(i => Math.max(i - 1, 0));
      } else if (e.key === 'Enter' && results[selectedIndex]) {
        onSelect?.(results[selectedIndex]);
        onClose();
      }
    }

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, results, selectedIndex, onClose, onSelect]);

  // TODO: Implement actual search
  function handleSearch(value: string) {
    setQuery(value);
    setSelectedIndex(0);
    
    // Placeholder results
    if (value.trim()) {
      setResults([
        { type: 'connection', id: '1', label: 'Production DB', sublabel: 'postgres' },
        { type: 'table', id: '2', label: 'users', sublabel: 'public.users' },
      ]);
    } else {
      setResults([]);
    }
  }

  if (!isOpen) return null;

  return (
    <div className="search-overlay" onClick={onClose}>
      <div className="search-modal" onClick={e => e.stopPropagation()}>
        <input
          ref={inputRef}
          className="search-input"
          type="text"
          placeholder="Search connections, tables..."
          value={query}
          onChange={e => handleSearch(e.target.value)}
        />
        
        {results.length > 0 && (
          <div className="search-results">
            {results.map((result, i) => (
              <button
                key={result.id}
                className={`search-result ${i === selectedIndex ? 'selected' : ''}`}
                onClick={() => {
                  onSelect?.(result);
                  onClose();
                }}
              >
                <span className="search-result-icon">
                  {result.type === 'connection' ? '🔌' : '📄'}
                </span>
                <span className="search-result-label">{result.label}</span>
                {result.sublabel && (
                  <span className="search-result-sublabel">{result.sublabel}</span>
                )}
              </button>
            ))}
          </div>
        )}
        
        <div className="search-hints">
          <span>↑↓ Navigate</span>
          <span>↵ Select</span>
          <span>esc Close</span>
        </div>
      </div>
    </div>
  );
}
