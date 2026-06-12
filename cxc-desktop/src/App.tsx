import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

interface Provider {
  name: string;
  base_url: string;
  api_key: string;
  model: string;
  wire_api: string;
  remark?: string;
  last_test?: string;
  latency_ms?: number;
  last_ok?: boolean;
}

interface Config {
  active: string;
  providers: Provider[];
}

function App() {
  const [config, setConfig] = useState<Config | null>(null);
  const [loading, setLoading] = useState<boolean>(true);
  const [switching, setSwitching] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    loadConfig();
  }, []);

  async function loadConfig() {
    try {
      setLoading(true);
      setError(null);
      const cfg = await invoke<Config>("get_config");
      setConfig(cfg);
    } catch (e: any) {
      setError(e.toString());
    } finally {
      setLoading(false);
    }
  }

  async function handleSwitch(name: string) {
    try {
      setSwitching(name);
      setError(null);
      const updatedCfg = await invoke<Config>("switch_provider", { name });
      setConfig(updatedCfg);
    } catch (e: any) {
      setError(e.toString());
    } finally {
      setSwitching(null);
    }
  }

  return (
    <div className="app-container">
      <header className="app-header">
        <div className="header-logo">
          <span className="logo-c">C</span>
          <span className="logo-xc">XC</span>
          <span className="badge">Desktop</span>
        </div>
        <div className="header-actions">
          <button className="btn btn-secondary btn-icon" onClick={loadConfig} disabled={loading}>
            {loading ? "⟳" : "↻"}
          </button>
        </div>
      </header>

      <main className="app-main">
        {error && (
          <div className="alert alert-error">
            <span className="alert-icon">⚠</span>
            <span className="alert-message">{error}</span>
            <button className="alert-close" onClick={() => setError(null)}>×</button>
          </div>
        )}

        {loading && !config ? (
          <div className="loading-state">
            <div className="spinner"></div>
            <p>Loading configuration...</p>
          </div>
        ) : (
          <section className="section">
            <div className="section-header">
              <h2>Saved Providers</h2>
              <span className="count-badge">{config?.providers?.length || 0}</span>
            </div>

            <div className="providers-grid">
              {config?.providers?.map((p) => {
                const isActive = config.active === p.name;
                const isThisSwitching = switching === p.name;
                return (
                  <div key={p.name} className={`provider-card ${isActive ? "active" : ""}`}>
                    <div className="card-header">
                      <div className="title-wrapper">
                        <h3 className="provider-name">{p.name}</h3>
                        {isActive && <span className="active-badge">Active</span>}
                      </div>
                      {p.remark && <p className="provider-remark">{p.remark}</p>}
                    </div>

                    <div className="card-body">
                      <div className="info-row">
                        <span className="info-label">Base URL:</span>
                        <span className="info-value code">{p.base_url}</span>
                      </div>
                      <div className="info-row">
                        <span className="info-label">Model:</span>
                        <span className="info-value code">{p.model}</span>
                      </div>
                      <div className="info-row">
                        <span className="info-label">API Key:</span>
                        <span className="info-value code">
                          {p.api_key.substring(0, 8)}••••••••
                        </span>
                      </div>
                      {p.latency_ms !== undefined && p.last_ok !== undefined && (
                        <div className="info-row">
                          <span className="info-label">Latency:</span>
                          <span className={`info-value ${p.last_ok ? "text-success" : "text-error"}`}>
                            {p.last_ok ? `${p.latency_ms} ms` : "Failed"}
                          </span>
                        </div>
                      )}
                    </div>

                    <div className="card-actions">
                      {!isActive ? (
                        <button
                          className="btn btn-primary btn-block"
                          onClick={() => handleSwitch(p.name)}
                          disabled={switching !== null}
                        >
                          {isThisSwitching ? "Switching..." : "Switch to Provider"}
                        </button>
                      ) : (
                        <button className="btn btn-success btn-block" disabled>
                          ✓ Currently Active
                        </button>
                      )}
                    </div>
                  </div>
                );
              })}

              {config?.providers?.length === 0 && (
                <div className="empty-state">
                  <p>No providers configured yet.</p>
                </div>
              )}
            </div>
          </section>
        )}
      </main>
    </div>
  );
}

export default App;
