import { useState } from 'react';
import './JSONViewer.css';

interface JSONViewerProps {
  data: unknown;
  initialExpanded?: boolean;
  maxDepth?: number;
}

export function JSONViewer({ data, initialExpanded = true, maxDepth = 5 }: JSONViewerProps) {
  return (
    <div className="json-viewer">
      <JSONNode value={data} depth={0} initialExpanded={initialExpanded} maxDepth={maxDepth} />
    </div>
  );
}

interface JSONNodeProps {
  value: unknown;
  keyName?: string;
  depth: number;
  initialExpanded: boolean;
  maxDepth: number;
}

function JSONNode({ value, keyName, depth, initialExpanded, maxDepth }: JSONNodeProps) {
  const [expanded, setExpanded] = useState(initialExpanded && depth < 2);

  if (value === null) {
    return (
      <div className="json-line">
        {keyName && <span className="json-key">"{keyName}": </span>}
        <span className="json-null">null</span>
      </div>
    );
  }

  if (typeof value === 'boolean') {
    return (
      <div className="json-line">
        {keyName && <span className="json-key">"{keyName}": </span>}
        <span className="json-boolean">{value ? 'true' : 'false'}</span>
      </div>
    );
  }

  if (typeof value === 'number') {
    return (
      <div className="json-line">
        {keyName && <span className="json-key">"{keyName}": </span>}
        <span className="json-number">{value}</span>
      </div>
    );
  }

  if (typeof value === 'string') {
    return (
      <div className="json-line">
        {keyName && <span className="json-key">"{keyName}": </span>}
        <span className="json-string">"{value}"</span>
      </div>
    );
  }

  if (Array.isArray(value)) {
    if (depth >= maxDepth) {
      return (
        <div className="json-line">
          {keyName && <span className="json-key">"{keyName}": </span>}
          <span className="json-collapsed">[...{value.length} items]</span>
        </div>
      );
    }

    return (
      <div className="json-node">
        <div className="json-line json-expandable" onClick={() => setExpanded(!expanded)}>
          <span className="json-toggle">{expanded ? '▼' : '▶'}</span>
          {keyName && <span className="json-key">"{keyName}": </span>}
          <span className="json-bracket">[</span>
          {!expanded && <span className="json-collapsed">...{value.length} items</span>}
          {!expanded && <span className="json-bracket">]</span>}
        </div>
        {expanded && (
          <div className="json-children">
            {value.map((item, i) => (
              <JSONNode
                key={i}
                value={item}
                depth={depth + 1}
                initialExpanded={initialExpanded}
                maxDepth={maxDepth}
              />
            ))}
            <div className="json-line"><span className="json-bracket">]</span></div>
          </div>
        )}
      </div>
    );
  }

  if (typeof value === 'object') {
    const entries = Object.entries(value as Record<string, unknown>);
    
    if (depth >= maxDepth) {
      return (
        <div className="json-line">
          {keyName && <span className="json-key">"{keyName}": </span>}
          <span className="json-collapsed">{'{...}'}</span>
        </div>
      );
    }

    return (
      <div className="json-node">
        <div className="json-line json-expandable" onClick={() => setExpanded(!expanded)}>
          <span className="json-toggle">{expanded ? '▼' : '▶'}</span>
          {keyName && <span className="json-key">"{keyName}": </span>}
          <span className="json-bracket">{'{'}</span>
          {!expanded && <span className="json-collapsed">...{entries.length} keys</span>}
          {!expanded && <span className="json-bracket">{'}'}</span>}
        </div>
        {expanded && (
          <div className="json-children">
            {entries.map(([k, v]) => (
              <JSONNode
                key={k}
                keyName={k}
                value={v}
                depth={depth + 1}
                initialExpanded={initialExpanded}
                maxDepth={maxDepth}
              />
            ))}
            <div className="json-line"><span className="json-bracket">{'}'}</span></div>
          </div>
        )}
      </div>
    );
  }

  return <div className="json-line">{String(value)}</div>;
}
