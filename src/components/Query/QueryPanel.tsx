import { useState, useCallback } from 'react';
import { SQLEditor } from '../Editor/SQLEditor';
import { MongoEditor, MONGO_TEMPLATES } from '../Editor/MongoEditor';
import { ResultsTable } from '../Results/ResultsTable';
import { JSONViewer } from '../Results/JSONViewer';
import { executeQuery, cancelQuery, QueryResult } from '../../lib/tauri';
import './QueryPanel.css';

interface QueryPanelProps {
  sessionId: string | null;
  dialect?: 'postgres' | 'mysql' | 'mongodb';
}

export function QueryPanel({ sessionId, dialect = 'postgres' }: QueryPanelProps) {
  const isMongo = dialect === 'mongodb';
  const defaultQuery = isMongo ? MONGO_TEMPLATES.find : 'SELECT 1;';
  
  const [query, setQuery] = useState(defaultQuery);
  const [result, setResult] = useState<QueryResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [cancelling, setCancelling] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleExecute = useCallback(async (queryText?: string) => {
    if (!sessionId) {
      setError('No connection selected');
      return;
    }

    const queryToRun = queryText || query;
    if (!queryToRun.trim()) return;

    setLoading(true);
    setError(null);
    setResult(null);

    try {
      const response = await executeQuery(sessionId, queryToRun);
      
      if (response.success && response.result) {
        setResult(response.result);
      } else {
        setError(response.error || 'Query failed');
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Unknown error');
    } finally {
      setLoading(false);
    }
  }, [sessionId, query]);

  const handleCancel = useCallback(async () => {
    if (!sessionId || !loading) return;

    setCancelling(true);
    try {
      await cancelQuery(sessionId);
    } catch (err) {
      console.error('Failed to cancel:', err);
    } finally {
      setCancelling(false);
      setLoading(false);
    }
  }, [sessionId, loading]);

  return (
    <div className="query-panel">
      {/* Toolbar */}
      <div className="query-toolbar">
        <button
          className="query-btn primary"
          onClick={() => handleExecute()}
          disabled={loading || !sessionId}
        >
          {loading ? '⏳ Running...' : '▶ Run'}
        </button>

        {loading && (
          <button
            className="query-btn danger"
            onClick={handleCancel}
            disabled={cancelling}
          >
            {cancelling ? 'Stopping...' : '⏹ Stop'}
          </button>
        )}

        {isMongo && (
          <select
            className="query-template-select"
            onChange={(e) => setQuery(MONGO_TEMPLATES[e.target.value as keyof typeof MONGO_TEMPLATES] || query)}
            defaultValue=""
          >
            <option value="" disabled>Templates...</option>
            <option value="find">find()</option>
            <option value="findOne">findOne()</option>
            <option value="aggregate">aggregate()</option>
            <option value="insertOne">insertOne()</option>
            <option value="updateOne">updateOne()</option>
            <option value="deleteOne">deleteOne()</option>
          </select>
        )}

        <span className="query-hint">
          Cmd+Enter to run{!isMongo && ' • Select text to run partial'}
        </span>

        {!sessionId && (
          <span className="query-warning">⚠ No connection</span>
        )}
      </div>

      {/* Editor - SQL or MongoDB */}
      <div className="query-editor">
        {isMongo ? (
          <MongoEditor
            value={query}
            onChange={setQuery}
            onExecute={() => handleExecute()}
            readOnly={loading}
          />
        ) : (
          <SQLEditor
            value={query}
            onChange={setQuery}
            onExecute={() => handleExecute()}
            onExecuteSelection={(selection) => handleExecute(selection)}
            dialect={dialect as 'postgres' | 'mysql'}
            readOnly={loading}
          />
        )}
      </div>

      {/* Results / Error */}
      <div className="query-results">
        {error && (
          <div className="query-error">
            <span className="error-icon">✕</span>
            {error}
          </div>
        )}

        {!error && result && (
          isMongo ? (
            <JSONViewer data={result.rows.map(r => r.values[0])} />
          ) : (
            <ResultsTable result={result} height={300} />
          )
        )}

        {!error && !result && (
          <div className="query-empty">
            Run a query to see results
          </div>
        )}
      </div>
    </div>
  );
}
