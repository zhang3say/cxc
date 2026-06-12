import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
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

const initialFormValues = {
  name: "",
  base_url: "",
  api_key: "",
  model: "",
  wire_api: "responses",
  remark: "",
};

function App() {
  const [config, setConfig] = useState<Config | null>(null);
  const [loading, setLoading] = useState<boolean>(true);
  const [switching, setSwitching] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Form State
  const [showForm, setShowForm] = useState<"add" | "edit" | null>(null);
  const [editingName, setEditingName] = useState<string | null>(null);
  const [formValues, setFormValues] = useState(initialFormValues);

  // Model Discovery State
  const [fetchingModels, setFetchingModels] = useState<boolean>(false);
  const [fetchedModels, setFetchedModels] = useState<string[]>([]);
  const [fetchError, setFetchError] = useState<string | null>(null);

  // Connectivity Test State
  const [testingProvider, setTestingProvider] = useState<string | null>(null);
  const [testingAll, setTestingAll] = useState<boolean>(false);

  useEffect(() => {
    loadConfig();

    let unlisten: (() => void) | undefined;
    const setupListener = async () => {
      unlisten = await listen<Config>("config-updated", (event) => {
        setConfig(event.payload);
      });
    };
    setupListener();

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
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

  function openAddForm() {
    setFormValues(initialFormValues);
    setFetchedModels([]);
    setFetchError(null);
    setShowForm("add");
    setEditingName(null);
  }

  function openEditForm(p: Provider) {
    setFormValues({
      name: p.name,
      base_url: p.base_url,
      api_key: p.api_key,
      model: p.model,
      wire_api: p.wire_api,
      remark: p.remark || "",
    });
    setFetchedModels([]);
    setFetchError(null);
    setShowForm("edit");
    setEditingName(p.name);
  }

  async function handleFetchModels() {
    if (!formValues.base_url || !formValues.api_key) {
      setFetchError("Please fill in Base URL and API Key first");
      return;
    }
    try {
      setFetchingModels(true);
      setFetchError(null);
      const models = await invoke<string[]>("fetch_models", {
        baseUrl: formValues.base_url,
        apiKey: formValues.api_key,
      });
      if (models.length === 0) {
        setFetchError("No models returned from endpoint");
      } else {
        setFetchedModels(models);
      }
    } catch (e: any) {
      setFetchError(e.toString());
    } finally {
      setFetchingModels(false);
    }
  }

  async function handleSubmitForm(e: React.FormEvent) {
    e.preventDefault();
    if (!formValues.name || !formValues.base_url || !formValues.api_key || !formValues.model) {
      setError("Please fill in all required fields (Name, Base URL, API Key, Model)");
      return;
    }

    try {
      setError(null);
      let updatedCfg: Config;
      if (showForm === "add") {
        updatedCfg = await invoke<Config>("add_provider", { provider: formValues });
      } else {
        updatedCfg = await invoke<Config>("edit_provider", {
          oldName: editingName,
          updated: formValues,
        });
      }
      setConfig(updatedCfg);
      setShowForm(null);
    } catch (e: any) {
      setError(e.toString());
    }
  }

  async function handleDeleteProvider(name: string) {
    if (!confirm(`Are you sure you want to remove provider "${name}"?`)) {
      return;
    }
    try {
      setError(null);
      const updatedCfg = await invoke<Config>("delete_provider", { name });
      setConfig(updatedCfg);
    } catch (e: any) {
      setError(e.toString());
    }
  }

  async function handleTestProvider(name: string) {
    try {
      setTestingProvider(name);
      setError(null);
      const updatedCfg = await invoke<Config>("test_provider", { name });
      setConfig(updatedCfg);
    } catch (e: any) {
      setError(e.toString());
    } finally {
      setTestingProvider(null);
    }
  }

  async function handleTestAllProviders() {
    try {
      setTestingAll(true);
      setError(null);
      const updatedCfg = await invoke<Config>("test_all_providers");
      setConfig(updatedCfg);
    } catch (e: any) {
      setError(e.toString());
    } finally {
      setTestingAll(false);
    }
  }

  function formatDate(isoStr?: string) {
    if (!isoStr) return "";
    try {
      const d = new Date(isoStr);
      return d.toLocaleString();
    } catch {
      return isoStr;
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
          <button className="btn btn-primary" onClick={openAddForm} style={{ marginRight: "0.5rem" }}>
            + Add Provider
          </button>
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

        {showForm && (
          <div className="modal-backdrop">
            <div className="modal-content">
              <div className="modal-header">
                <h2>{showForm === "add" ? "Add New Provider" : `Edit Provider: ${editingName}`}</h2>
                <button className="modal-close-btn" onClick={() => setShowForm(null)}>×</button>
              </div>

              <form onSubmit={handleSubmitForm} className="modal-form">
                <div className="form-group">
                  <label htmlFor="form-name">Provider Name *</label>
                  <input
                    id="form-name"
                    type="text"
                    required
                    value={formValues.name}
                    onChange={(e) => setFormValues({ ...formValues, name: e.target.value })}
                    placeholder="e.g. fast-relay"
                  />
                </div>

                <div className="form-group">
                  <label htmlFor="form-url">Base URL *</label>
                  <input
                    id="form-url"
                    type="url"
                    required
                    value={formValues.base_url}
                    onChange={(e) => setFormValues({ ...formValues, base_url: e.target.value })}
                    placeholder="https://api.example.com/v1"
                  />
                </div>

                <div className="form-group">
                  <label htmlFor="form-key">API Key *</label>
                  <input
                    id="form-key"
                    type="password"
                    required
                    value={formValues.api_key}
                    onChange={(e) => setFormValues({ ...formValues, api_key: e.target.value })}
                    placeholder="sk-••••••••••••"
                  />
                </div>

                <div className="form-group">
                  <div className="model-label-row">
                    <label htmlFor="form-model">Model *</label>
                    <button
                      type="button"
                      className="btn btn-secondary btn-xs"
                      onClick={handleFetchModels}
                      disabled={fetchingModels || !formValues.base_url || !formValues.api_key}
                    >
                      {fetchingModels ? "Fetching..." : "🔍 Discover Models"}
                    </button>
                  </div>

                  {fetchedModels.length > 0 ? (
                    <select
                      id="form-model-select"
                      value={formValues.model}
                      onChange={(e) => setFormValues({ ...formValues, model: e.target.value })}
                    >
                      <option value="">-- Select a model --</option>
                      {fetchedModels.map((m) => (
                        <option key={m} value={m}>
                          {m}
                        </option>
                      ))}
                    </select>
                  ) : (
                    <input
                      id="form-model"
                      type="text"
                      required
                      value={formValues.model}
                      onChange={(e) => setFormValues({ ...formValues, model: e.target.value })}
                      placeholder="e.g. gpt-4o (or fetch list above)"
                    />
                  )}
                  {fetchError && <span className="field-error">{fetchError}</span>}
                </div>

                <div className="form-group">
                  <label htmlFor="form-wire">Wire API</label>
                  <input
                    id="form-wire"
                    type="text"
                    value={formValues.wire_api}
                    onChange={(e) => setFormValues({ ...formValues, wire_api: e.target.value })}
                    placeholder="responses"
                  />
                </div>

                <div className="form-group">
                  <label htmlFor="form-remark">Remark / Description</label>
                  <input
                    id="form-remark"
                    type="text"
                    value={formValues.remark}
                    onChange={(e) => setFormValues({ ...formValues, remark: e.target.value })}
                    placeholder="e.g. Backup endpoint"
                  />
                </div>

                <div className="modal-actions">
                  <button type="button" className="btn btn-secondary" onClick={() => setShowForm(null)}>
                    Cancel
                  </button>
                  <button type="submit" className="btn btn-primary">
                    {showForm === "add" ? "Create Provider" : "Save Changes"}
                  </button>
                </div>
              </form>
            </div>
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
              {config && config.providers.length > 0 && (
                <button
                  className="btn btn-secondary btn-sm"
                  onClick={handleTestAllProviders}
                  disabled={testingAll || testingProvider !== null}
                  style={{ marginLeft: "auto" }}
                >
                  {testingAll ? "Testing All..." : "⚡ Test All Connections"}
                </button>
              )}
            </div>

            <div className="providers-grid">
              {config?.providers?.map((p) => {
                const isActive = config.active === p.name;
                const isThisSwitching = switching === p.name;
                const isTesting = testingProvider === p.name || (testingAll && !isActive);

                return (
                  <div key={p.name} className={`provider-card ${isActive ? "active" : ""} ${isTesting ? "testing" : ""}`}>
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

                      {isTesting ? (
                        <div className="info-row testing-indicator">
                          <span className="info-label">Latency:</span>
                          <span className="info-value text-warn">
                            <span className="mini-spinner"></span> testing...
                          </span>
                        </div>
                      ) : p.latency_ms !== undefined && p.last_ok !== undefined ? (
                        <>
                          <div className="info-row">
                            <span className="info-label">Latency:</span>
                            <span className={`info-value ${p.last_ok ? "text-success" : "text-error"}`}>
                              {p.last_ok ? `${p.latency_ms} ms` : "Failed"}
                            </span>
                          </div>
                          {p.last_test && (
                            <div className="info-row">
                              <span className="info-label">Last Tested:</span>
                              <span className="info-value text-dim">{formatDate(p.last_test)}</span>
                            </div>
                          )}
                        </>
                      ) : null}
                    </div>

                    <div className="card-actions-row">
                      <div className="switch-wrapper">
                        {!isActive ? (
                          <button
                            className="btn btn-primary btn-sm btn-block"
                            onClick={() => handleSwitch(p.name)}
                            disabled={switching !== null || testingAll || testingProvider !== null}
                          >
                            {isThisSwitching ? "Switching..." : "Switch"}
                          </button>
                        ) : (
                          <button className="btn btn-success btn-sm btn-block" disabled>
                            ✓ Active
                          </button>
                        )}
                      </div>
                      <div className="manage-buttons">
                        <button
                          className="btn btn-secondary btn-sm"
                          onClick={() => handleTestProvider(p.name)}
                          disabled={testingProvider !== null || testingAll || switching !== null}
                          title="Test connectivity"
                        >
                          {testingProvider === p.name ? "⏳" : "⚡"}
                        </button>
                        <button className="btn btn-secondary btn-sm" onClick={() => openEditForm(p)}>
                          Edit
                        </button>
                        <button
                          className="btn btn-danger btn-sm"
                          onClick={() => handleDeleteProvider(p.name)}
                          disabled={isActive || switching !== null || testingAll || testingProvider !== null}
                          title={isActive ? "Cannot delete active provider" : "Delete provider"}
                        >
                          Delete
                        </button>
                      </div>
                    </div>
                  </div>
                );
              })}

              {config?.providers?.length === 0 && (
                <div className="empty-state">
                  <p>No providers configured yet. Click "+ Add Provider" to create one.</p>
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
