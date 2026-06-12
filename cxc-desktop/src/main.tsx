import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";

// Mock Tauri IPC for browser preview
if (typeof window !== "undefined" && !(window as any).__TAURI_INTERNALS__) {
  console.log("Initializing Tauri Browser Mocks for CXC Desktop UI Preview");
  
  const mockConfig = {
    active: "primary-relay",
    providers: [
      {
        name: "primary-relay",
        base_url: "https://api.openai.com/v1",
        api_key: "sk-proj-primarykey1234567890",
        model: "gpt-4o",
        wire_api: "responses",
        remark: "Fast primary inference",
        last_test: "2026-06-12T15:10:00.000Z",
        latency_ms: 85,
        last_ok: true
      },
      {
        name: "fallback-relay",
        base_url: "https://api.anthropic.com/v1",
        api_key: "sk-ant-fallbackkey1234567890",
        model: "claude-3-5-sonnet",
        wire_api: "responses",
        remark: "Backup anthropic endpoint",
        last_test: "2026-06-12T15:08:00.000Z",
        latency_ms: 240,
        last_ok: true
      },
      {
        name: "local-ollama",
        base_url: "http://localhost:11434/v1",
        api_key: "ollama-key-ignored",
        model: "llama3",
        wire_api: "responses",
        remark: "Offline dev test",
        last_test: "2026-06-12T15:05:00.000Z",
        latency_ms: 0,
        last_ok: false
      }
    ],
    codex_source: "app",
    codex_custom_dir: ""
  };

  (window as any).__TAURI_INTERNALS__ = {
    transformCallback: (callback: any, once: boolean) => {
      console.log("Mock transformCallback registered", callback, once);
      return Math.floor(Math.random() * 1000000);
    },
    invoke: async (cmd: string, args: any) => {
      console.log("Mock Tauri IPC invoke:", cmd, args);
      if (cmd === "get_config") {
        return mockConfig;
      }
      if (cmd === "switch_provider") {
        mockConfig.active = args.name;
        return { ...mockConfig };
      }
      if (cmd === "test_provider") {
        const p = mockConfig.providers.find(x => x.name === args.name);
        if (p) {
          p.last_test = new Date().toISOString();
          p.latency_ms = Math.floor(Math.random() * 200) + 30;
          p.last_ok = true;
        }
        return { ...mockConfig };
      }
      if (cmd === "test_all_providers") {
        for (const p of mockConfig.providers) {
          p.last_test = new Date().toISOString();
          p.latency_ms = Math.floor(Math.random() * 200) + 30;
          p.last_ok = Math.random() > 0.15;
        }
        return { ...mockConfig };
      }
      if (cmd === "delete_provider") {
        mockConfig.providers = mockConfig.providers.filter(x => x.name !== args.name);
        return { ...mockConfig };
      }
      if (cmd === "add_provider") {
        mockConfig.providers.push(args.provider);
        return { ...mockConfig };
      }
      if (cmd === "edit_provider") {
        const idx = mockConfig.providers.findIndex(x => x.name === args.oldName);
        if (idx !== -1) {
          mockConfig.providers[idx] = args.updated;
        }
        return { ...mockConfig };
      }
      if (cmd === "fetch_models") {
        await new Promise(r => setTimeout(r, 800));
        return ["gpt-4o", "gpt-4-turbo", "gpt-3.5-turbo", "claude-3-5-sonnet", "llama3"];
      }
      if (cmd === "save_settings") {
        (mockConfig as any).codex_source = args.source;
        (mockConfig as any).codex_custom_dir = args.customDir;
        return { ...mockConfig };
      }
      return null;
    },
    // Mock the plugin mechanism for event listeners
    plugins: {
      event: {
        listen: async (event: string, handler: any) => {
          console.log("Mock Tauri event listen registered:", event, handler);
          return () => console.log("Mock Tauri event unlisten called");
        }
      }
    }
  };

  // Mock Tauri v2 event listen module export mapping if needed
  (window as any).__TAURI__ = {
    event: {
      listen: async (event: string, handler: any) => {
        console.log("Mock Tauri event listen:", event, handler);
        return () => console.log("Mock Tauri event unlisten");
      }
    }
  };
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
