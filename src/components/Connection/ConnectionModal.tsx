import { useState, useEffect } from 'react';
import { 
  testConnection, 
  connect, 
  saveConnection, 
  ConnectionConfig 
} from '../../lib/tauri';
import './ConnectionModal.css';

interface ConnectionModalProps {
  isOpen: boolean;
  onClose: () => void;
  onConnected: (sessionId: string, driver: string) => void;
}

type Driver = 'postgres' | 'mysql' | 'mongodb';

interface FormData {
  name: string;
  driver: Driver;
  host: string;
  port: number;
  username: string;
  password: string;
  database: string;
  ssl: boolean;
}

const DEFAULT_PORTS: Record<Driver, number> = {
  postgres: 5432,
  mysql: 3306,
  mongodb: 27017,
};

const DRIVER_LABELS: Record<Driver, string> = {
  postgres: 'PostgreSQL',
  mysql: 'MySQL / MariaDB',
  mongodb: 'MongoDB',
};

const DRIVER_ICONS: Record<Driver, string> = {
  postgres: '🐘',
  mysql: '🐬',
  mongodb: '🍃',
};

const initialFormData: FormData = {
  name: '',
  driver: 'postgres',
  host: 'localhost',
  port: 5432,
  username: '',
  password: '',
  database: '',
  ssl: false,
};

export function ConnectionModal({ isOpen, onClose, onConnected }: ConnectionModalProps) {
  const [formData, setFormData] = useState<FormData>(initialFormData);
  const [testing, setTesting] = useState(false);
  const [connecting, setConnecting] = useState(false);
  const [testResult, setTestResult] = useState<'success' | 'error' | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Reset form when modal opens
  useEffect(() => {
    if (isOpen) {
      setFormData(initialFormData);
      setTestResult(null);
      setError(null);
    }
  }, [isOpen]);

  // Update port when driver changes
  function handleDriverChange(driver: Driver) {
    setFormData(prev => ({
      ...prev,
      driver,
      port: DEFAULT_PORTS[driver],
    }));
    setTestResult(null);
    setError(null);
  }

  function handleChange(field: keyof FormData, value: string | number | boolean) {
    setFormData(prev => ({ ...prev, [field]: value }));
    setTestResult(null);
    setError(null);
  }

  async function handleTestConnection() {
    setTesting(true);
    setTestResult(null);
    setError(null);

    try {
      const config: ConnectionConfig = {
        driver: formData.driver,
        host: formData.host,
        port: formData.port,
        username: formData.username,
        password: formData.password,
        database: formData.database || undefined,
        ssl: formData.ssl,
      };

      const result = await testConnection(config);
      
      if (result.success) {
        setTestResult('success');
      } else {
        setTestResult('error');
        setError(result.error || 'Connection failed');
      }
    } catch (err) {
      setTestResult('error');
      setError(err instanceof Error ? err.message : 'Unknown error');
    } finally {
      setTesting(false);
    }
  }

  async function handleSaveAndConnect() {
    setConnecting(true);
    setError(null);

    try {
      const config: ConnectionConfig = {
        driver: formData.driver,
        host: formData.host,
        port: formData.port,
        username: formData.username,
        password: formData.password,
        database: formData.database || undefined,
        ssl: formData.ssl,
      };

      // Save to vault
      const connectionId = `conn_${Date.now()}`;
      await saveConnection({
        id: connectionId,
        name: formData.name || `${formData.host}:${formData.port}`,
        driver: formData.driver,
        host: formData.host,
        port: formData.port,
        username: formData.username,
        password: formData.password,
        database: formData.database || undefined,
        ssl: formData.ssl,
        project_id: 'default',
      });

      // Connect
      const connectResult = await connect(config);
      
      if (connectResult.success && connectResult.session_id) {
        onConnected(connectResult.session_id, formData.driver);
        onClose();
      } else {
        setError(connectResult.error || 'Failed to connect');
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Unknown error');
    } finally {
      setConnecting(false);
    }
  }

  if (!isOpen) return null;

  const isValid = formData.host && formData.username && formData.password;

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-content" onClick={e => e.stopPropagation()}>
        <header className="modal-header">
          <h2>New Connection</h2>
          <button className="modal-close" onClick={onClose}>×</button>
        </header>

        <div className="modal-body">
          {/* Driver Selection */}
          <div className="driver-select">
            {(Object.keys(DRIVER_LABELS) as Driver[]).map(driver => (
              <button
                key={driver}
                className={`driver-btn ${formData.driver === driver ? 'active' : ''}`}
                onClick={() => handleDriverChange(driver)}
              >
                <span className="driver-icon">{DRIVER_ICONS[driver]}</span>
                <span className="driver-label">{DRIVER_LABELS[driver]}</span>
              </button>
            ))}
          </div>

          {/* Connection Name */}
          <div className="form-field">
            <label>Connection Name</label>
            <input
              type="text"
              placeholder="My Database"
              value={formData.name}
              onChange={e => handleChange('name', e.target.value)}
            />
          </div>

          {/* Host & Port */}
          <div className="form-row">
            <div className="form-field flex-2">
              <label>Host *</label>
              <input
                type="text"
                placeholder="localhost"
                value={formData.host}
                onChange={e => handleChange('host', e.target.value)}
              />
            </div>
            <div className="form-field flex-1">
              <label>Port</label>
              <input
                type="number"
                value={formData.port}
                onChange={e => handleChange('port', parseInt(e.target.value) || 0)}
              />
            </div>
          </div>

          {/* Username & Password */}
          <div className="form-row">
            <div className="form-field">
              <label>Username *</label>
              <input
                type="text"
                placeholder="user"
                value={formData.username}
                onChange={e => handleChange('username', e.target.value)}
              />
            </div>
            <div className="form-field">
              <label>Password *</label>
              <input
                type="password"
                placeholder="••••••••"
                value={formData.password}
                onChange={e => handleChange('password', e.target.value)}
              />
            </div>
          </div>

          {/* Database */}
          <div className="form-field">
            <label>Database</label>
            <input
              type="text"
              placeholder={formData.driver === 'postgres' ? 'postgres' : ''}
              value={formData.database}
              onChange={e => handleChange('database', e.target.value)}
            />
          </div>

          {/* SSL Toggle */}
          <div className="form-field-inline">
            <input
              type="checkbox"
              id="ssl"
              checked={formData.ssl}
              onChange={e => handleChange('ssl', e.target.checked)}
            />
            <label htmlFor="ssl">Use SSL/TLS</label>
          </div>

          {/* Error / Success Message */}
          {error && (
            <div className="form-message error">
              <span>✕</span> {error}
            </div>
          )}
          {testResult === 'success' && (
            <div className="form-message success">
              <span>✓</span> Connection successful!
            </div>
          )}
        </div>

        <footer className="modal-footer">
          <button className="btn-secondary" onClick={onClose}>
            Cancel
          </button>
          <button
            className="btn-secondary"
            onClick={handleTestConnection}
            disabled={!isValid || testing}
          >
            {testing ? 'Testing...' : 'Test Connection'}
          </button>
          <button
            className="btn-primary"
            onClick={handleSaveAndConnect}
            disabled={!isValid || connecting}
          >
            {connecting ? 'Connecting...' : 'Save & Connect'}
          </button>
        </footer>
      </div>
    </div>
  );
}
