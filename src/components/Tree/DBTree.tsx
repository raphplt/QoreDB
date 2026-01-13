import { useState, useEffect } from 'react';
import { Namespace, Collection, listNamespaces, listCollections } from '../../lib/tauri';
import './DBTree.css';

interface DBTreeProps {
  connectionId: string;
}

export function DBTree({ connectionId }: DBTreeProps) {
  const [namespaces, setNamespaces] = useState<Namespace[]>([]);
  const [expandedNs, setExpandedNs] = useState<string | null>(null);
  const [collections, setCollections] = useState<Collection[]>([]);
  const [loading, setLoading] = useState(false);

  // TODO: Get sessionId from connection - for now this is a placeholder
  const sessionId = connectionId;

  useEffect(() => {
    loadNamespaces();
  }, [connectionId]);

  async function loadNamespaces() {
    try {
      setLoading(true);
      const result = await listNamespaces(sessionId);
      if (result.success && result.namespaces) {
        setNamespaces(result.namespaces);
      }
    } catch (err) {
      console.error('Failed to load namespaces:', err);
    } finally {
      setLoading(false);
    }
  }

  async function handleExpandNamespace(ns: Namespace) {
    const key = `${ns.database}:${ns.schema || ''}`;
    
    if (expandedNs === key) {
      setExpandedNs(null);
      setCollections([]);
      return;
    }

    setExpandedNs(key);
    
    try {
      const result = await listCollections(sessionId, ns);
      if (result.success && result.collections) {
        setCollections(result.collections);
      }
    } catch (err) {
      console.error('Failed to load collections:', err);
    }
  }

  function getNsKey(ns: Namespace): string {
    return `${ns.database}:${ns.schema || ''}`;
  }

  if (loading) {
    return <div className="tree-loading">Loading...</div>;
  }

  return (
    <div className="db-tree">
      {namespaces.map(ns => (
        <div key={getNsKey(ns)} className="tree-namespace">
          <button
            className="tree-node"
            onClick={() => handleExpandNamespace(ns)}
          >
            <span className="tree-icon">
              {expandedNs === getNsKey(ns) ? '📂' : '📁'}
            </span>
            <span className="tree-label truncate">
              {ns.schema ? `${ns.database}.${ns.schema}` : ns.database}
            </span>
          </button>
          
          {expandedNs === getNsKey(ns) && (
            <div className="tree-children">
              {collections.map(col => (
                <button
                  key={col.name}
                  className="tree-node tree-leaf"
                >
                  <span className="tree-icon">
                    {col.collection_type === 'View' ? '👁️' : '📄'}
                  </span>
                  <span className="tree-label truncate">{col.name}</span>
                </button>
              ))}
            </div>
          )}
        </div>
      ))}
    </div>
  );
}
