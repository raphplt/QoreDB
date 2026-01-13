import { List, type RowComponentProps } from 'react-window';
import { QueryResult, Row } from '../../lib/tauri';
import './ResultsTable.css';

interface ResultsTableProps {
  result: QueryResult | null;
  height?: number;
}

const ROW_HEIGHT = 32;
const HEADER_HEIGHT = 36;

function formatValue(value: unknown): string {
  if (value === null) return 'NULL';
  if (value === undefined) return '';
  if (typeof value === 'boolean') return value ? 'true' : 'false';
  if (typeof value === 'object') return JSON.stringify(value);
  return String(value);
}

type RowExtraProps = { rows: Row[] };

function RowRenderer({ index, style, rows }: RowComponentProps<RowExtraProps>) {
  const row = rows[index];

  return (
    <div className="results-row" style={style}>
      {row.values.map((value: unknown, colIndex: number) => (
        <div key={colIndex} className="results-cell">
          {formatValue(value)}
        </div>
      ))}
    </div>
  );
}

export function ResultsTable({ result, height = 400 }: ResultsTableProps) {
  if (!result || result.columns.length === 0) {
    if (result?.affected_rows !== undefined) {
      return (
        <div className="results-message">
          <span className="results-success">✓</span>
          {result.affected_rows} row(s) affected in {result.execution_time_ms}ms
        </div>
      );
    }
    return <div className="results-empty">No results to display</div>;
  }

  const { columns, rows } = result;

  return (
    <div className="results-table" style={{ height }}>
      <div className="results-header">
        {columns.map((col, i) => (
          <div key={i} className="results-header-cell" title={col.data_type}>
            {col.name}
          </div>
        ))}
      </div>

      <List
        rowCount={rows.length}
        rowHeight={ROW_HEIGHT}
        rowComponent={RowRenderer}
        rowProps={{ rows }}
        style={{ height: height - HEADER_HEIGHT, width: '100%' }}
      />

      <div className="results-footer">
        {rows.length} row(s) • {result.execution_time_ms}ms
      </div>
    </div>
  );
}
