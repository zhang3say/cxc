use std::io;
use std::time::{Duration, SystemTime};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Cell, Paragraph, Row, Table, TableState},
    Terminal,
};
use anyhow::{Context, Result};
use crate::config::{self, Config, Provider};
use crate::target::{TargetAdapter, TargetConfig, codex::CodexAdapter};

// ── Theme ───────────────────────────────────────────────────────────────────

struct Theme {
    title_bg: Color,
    title_fg: Color,
    active: Color,
    selected_bg: Color,
    selected_fg: Color,
    dim: Color,
    success: Color,
    error: Color,
    warn: Color,
    help: Color,
    header: Color,
    remark: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            title_bg: Color::Rgb(0x5E, 0x81, 0xAC), // Nord 10
            title_fg: Color::Rgb(0xEC, 0xEF, 0xF4), // Nord 6
            active: Color::Rgb(0xA3, 0xBE, 0x8C),   // Nord 14
            selected_bg: Color::Rgb(0x3B, 0x42, 0x52), // Nord 1
            selected_fg: Color::Rgb(0xEC, 0xEF, 0xF4), // Nord 6
            dim: Color::Rgb(0x4C, 0x56, 0x6A),      // Nord 3
            success: Color::Rgb(0xA3, 0xBE, 0x8C),  // Nord 14
            error: Color::Rgb(0xBF, 0x61, 0x6A),    // Nord 11
            warn: Color::Rgb(0xEB, 0xCB, 0x8B),     // Nord 13
            help: Color::Rgb(0x61, 0x6E, 0x88),     // Nord 3-ish/blue
            header: Color::Rgb(0x81, 0xA1, 0xC1),   // Nord 9
            remark: Color::Rgb(0xB4, 0x8E, 0xAD),   // Nord 15
        }
    }
}

// ── View Modes ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ViewMode {
    List,
    Add,
    Confirm,
    Edit,
    ModelPicker,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConfirmAction {
    Switch,
    Remove,
}

struct ConfirmState {
    action: ConfirmAction,
    subject: String, // provider name
}

struct AddFormState {
    field: usize,
    values: [String; 5],
    cursor: usize,
}

impl Default for AddFormState {
    fn default() -> Self {
        Self {
            field: 0,
            values: [
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            ],
            cursor: 0,
        }
    }
}

struct EditFormState {
    field: usize,
    values: [String; 5],
    old_name: String,
    cursor: usize,
}

impl Default for EditFormState {
    fn default() -> Self {
        Self {
            field: 0,
            values: [
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            ],
            old_name: String::new(),
            cursor: 0,
        }
    }
}

// ── Events ──────────────────────────────────────────────────────────────────

enum TuiEvent {
    Key(KeyEvent),
    Tick,
    TestResult {
        name: String,
        ok: bool,
        latency_ms: Option<u64>,
        error: Option<String>,
    },
    ModelsFetched {
        /// Which form triggered the fetch: Add or Edit
        target: ViewMode,
        result: std::result::Result<Vec<String>, String>,
    },
}

// ── App State ───────────────────────────────────────────────────────────────

struct TuiApp {
    cfg: Config,
    cursor: usize,
    mode: ViewMode,
    add_form: AddFormState,
    edit_form: EditFormState,
    confirm: Option<ConfirmState>,
    status: String,
    status_err: bool,
    testing: std::collections::HashSet<String>,
    test_all_active: bool,
    // Model Discovery state
    fetching_models: bool,
    /// Which form (Add/Edit) spawned the model fetch — to return to after picking
    model_picker_origin: ViewMode,
    model_list: Vec<String>,
    model_picker_cursor: usize,
}

impl TuiApp {
    fn new() -> Result<Self> {
        let cfg = config::load().context("Failed to load configuration")?;
        Ok(Self {
            cfg,
            cursor: 0,
            mode: ViewMode::List,
            add_form: AddFormState::default(),
            edit_form: EditFormState::default(),
            confirm: None,
            status: String::new(),
            status_err: false,
            testing: std::collections::HashSet::new(),
            test_all_active: false,
            fetching_models: false,
            model_picker_origin: ViewMode::Add,
            model_list: Vec::new(),
            model_picker_cursor: 0,
        })
    }

    fn submit_add(&mut self) -> Result<()> {
        let name = self.add_form.values[0].trim().to_string();
        let base_url = self.add_form.values[1].trim().to_string();
        let api_key = self.add_form.values[2].trim().to_string();
        let model = self.add_form.values[3].trim().to_string();
        let remark = self.add_form.values[4].trim().to_string();

        if let Err(e) = validate_input_fields(&name, &base_url, &api_key, &model) {
            self.status = format!("Validation failed: {}", e);
            self.status_err = true;
            return Ok(());
        }

        let p = Provider {
            name: name.clone(),
            base_url,
            api_key,
            model,
            wire_api: "responses".to_string(),
            remark: if remark.is_empty() { None } else { Some(remark) },
            last_test: None,
            latency_ms: None,
            last_ok: None,
        };

        match config::add_provider(&mut self.cfg, p) {
            Ok(_) => {
                self.status = format!("✓ Provider \"{}\" added", name);
                self.status_err = false;
                self.mode = ViewMode::List;
                if let Some(pos) = self.cfg.providers.iter().position(|prov| prov.name == name) {
                    self.cursor = pos;
                }
            }
            Err(e) => {
                self.status = format!("✗ Add failed: {}", e);
                self.status_err = true;
            }
        }
        Ok(())
    }

    fn submit_edit(&mut self) -> Result<()> {
        let name = self.edit_form.values[0].trim().to_string();
        let base_url = self.edit_form.values[1].trim().to_string();
        let api_key = self.edit_form.values[2].trim().to_string();
        let model = self.edit_form.values[3].trim().to_string();
        let remark = self.edit_form.values[4].trim().to_string();

        if let Err(e) = validate_input_fields(&name, &base_url, &api_key, &model) {
            self.status = format!("Validation failed: {}", e);
            self.status_err = true;
            return Ok(());
        }

        let updated = Provider {
            name: name.clone(),
            base_url,
            api_key,
            model,
            wire_api: "responses".to_string(),
            remark: if remark.is_empty() { None } else { Some(remark) },
            last_test: None,
            latency_ms: None,
            last_ok: None,
        };

        let old_name = self.edit_form.old_name.clone();
        match config::edit_provider(&mut self.cfg, &old_name, updated.clone()) {
            Ok(_) => {
                self.status = format!("✓ Provider \"{}\" updated", old_name);
                self.status_err = false;
                self.mode = ViewMode::List;

                if self.cfg.active == name {
                    if let Ok(adapter) = CodexAdapter::new() {
                        let tc = TargetConfig {
                            base_url: updated.base_url.clone(),
                            api_key: updated.api_key.clone(),
                            model: updated.model.clone(),
                            wire_api: "responses".to_string(),
                        };
                        let _ = adapter.write(&tc);
                    }
                }

                if let Some(pos) = self.cfg.providers.iter().position(|prov| prov.name == name) {
                    self.cursor = pos;
                }
            }
            Err(e) => {
                self.status = format!("✗ Edit failed: {}", e);
                self.status_err = true;
            }
        }
        Ok(())
    }

    fn handle_list_key(&mut self, key: KeyEvent, tx: &tokio::sync::mpsc::Sender<TuiEvent>) -> Result<bool> {
        let providers = &self.cfg.providers;

        if key.modifiers.contains(KeyModifiers::CONTROL) {
            if let KeyCode::Char('c') | KeyCode::Char('C') = key.code {
                return Ok(true);
            }
            return Ok(false);
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                return Ok(true);
            }
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                if !providers.is_empty() && self.cursor < providers.len() - 1 {
                    self.cursor += 1;
                }
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                self.mode = ViewMode::Add;
                self.add_form = AddFormState::default();
                self.status.clear();
                self.status_err = false;
            }
            KeyCode::Char('e') | KeyCode::Char('E') => {
                if !providers.is_empty() {
                    let p = &providers[self.cursor];
                    self.mode = ViewMode::Edit;
                    self.edit_form = EditFormState {
                        field: 0,
                        values: [
                            p.name.clone(),
                            p.base_url.clone(),
                            p.api_key.clone(),
                            p.model.clone(),
                            p.remark.as_deref().unwrap_or("").to_string(),
                        ],
                        old_name: p.name.clone(),
                        cursor: p.name.chars().count(),
                    };
                    self.status.clear();
                    self.status_err = false;
                }
            }
            KeyCode::Char('t') => {
                if !providers.is_empty() {
                    let p = &providers[self.cursor];
                    self.testing.insert(p.name.clone());
                    self.status = format!("Testing \"{}\"…", p.name);
                    self.status_err = false;

                    let tx = tx.clone();
                    let name = p.name.clone();
                    let base_url = p.base_url.clone();
                    let api_key = p.api_key.clone();
                    let model = p.model.clone();

                    tokio::spawn(async move {
                        let tester = crate::connectivity::Tester::new();
                        let res = tester.test(&base_url, &api_key, &model).await;
                        let _ = tx.send(TuiEvent::TestResult {
                            name,
                            ok: res.ok,
                            latency_ms: if res.ok { Some(res.latency_ms as u64) } else { None },
                            error: if res.ok { None } else { Some(res.error) },
                        }).await;
                    });
                }
            }
            KeyCode::Char('T') => {
                if !providers.is_empty() {
                    self.status = format!("Testing all {} providers concurrently…", providers.len());
                    self.status_err = false;
                    self.test_all_active = true;

                    for p in providers {
                        self.testing.insert(p.name.clone());

                        let tx = tx.clone();
                        let name = p.name.clone();
                        let base_url = p.base_url.clone();
                        let api_key = p.api_key.clone();
                        let model = p.model.clone();

                        tokio::spawn(async move {
                            let tester = crate::connectivity::Tester::new();
                            let res = tester.test(&base_url, &api_key, &model).await;
                            let _ = tx.send(TuiEvent::TestResult {
                                name,
                                ok: res.ok,
                                latency_ms: if res.ok { Some(res.latency_ms as u64) } else { None },
                                error: if res.ok { None } else { Some(res.error) },
                            }).await;
                        });
                    }
                }
            }
            KeyCode::Enter | KeyCode::Char('s') | KeyCode::Char('S') => {
                if !providers.is_empty() {
                    let p = &providers[self.cursor];
                    if p.name == self.cfg.active {
                        self.status = format!("Provider \"{}\" is already active.", p.name);
                        self.status_err = false;
                    } else {
                        self.mode = ViewMode::Confirm;
                        self.confirm = Some(ConfirmState {
                            action: ConfirmAction::Switch,
                            subject: p.name.clone(),
                        });
                    }
                }
            }
            KeyCode::Delete | KeyCode::Char('d') | KeyCode::Char('D') => {
                if !providers.is_empty() {
                    let p = &providers[self.cursor];
                    self.mode = ViewMode::Confirm;
                    self.confirm = Some(ConfirmState {
                        action: ConfirmAction::Remove,
                        subject: p.name.clone(),
                    });
                }
            }
            _ => {}
        }
        Ok(false)
    }

    fn handle_add_key(&mut self, key: KeyEvent, tx: &tokio::sync::mpsc::Sender<TuiEvent>) -> Result<()> {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    if !self.fetching_models {
                        self.submit_add()?;
                    }
                }
                KeyCode::Char('a') | KeyCode::Char('A') => {
                    self.add_form.cursor = 0;
                }
                KeyCode::Char('e') | KeyCode::Char('E') => {
                    let val = &self.add_form.values[self.add_form.field];
                    self.add_form.cursor = val.chars().count();
                }
                KeyCode::Char('l') | KeyCode::Char('L') => {
                    self.trigger_fetch_models(ViewMode::Add, tx);
                }
                _ => {}
            }
            return Ok(());
        }

        match key.code {
            KeyCode::Esc => {
                self.mode = ViewMode::List;
                self.status.clear();
                self.status_err = false;
            }
            KeyCode::Enter => {
                if self.add_form.field < 4 {
                    self.add_form.field += 1;
                    let val = &self.add_form.values[self.add_form.field];
                    self.add_form.cursor = val.chars().count();
                } else {
                    self.submit_add()?;
                }
            }
            KeyCode::Up => {
                if self.add_form.field > 0 {
                    self.add_form.field -= 1;
                    let val = &self.add_form.values[self.add_form.field];
                    self.add_form.cursor = val.chars().count();
                }
            }
            KeyCode::Down => {
                if self.add_form.field < 4 {
                    self.add_form.field += 1;
                    let val = &self.add_form.values[self.add_form.field];
                    self.add_form.cursor = val.chars().count();
                }
            }
            KeyCode::Left => {
                if self.add_form.cursor > 0 {
                    self.add_form.cursor -= 1;
                }
            }
            KeyCode::Right => {
                let val = &self.add_form.values[self.add_form.field];
                if self.add_form.cursor < val.chars().count() {
                    self.add_form.cursor += 1;
                }
            }
            KeyCode::Home => {
                self.add_form.cursor = 0;
            }
            KeyCode::End => {
                let val = &self.add_form.values[self.add_form.field];
                self.add_form.cursor = val.chars().count();
            }
            KeyCode::Backspace => {
                let val = &self.add_form.values[self.add_form.field];
                let (new_val, new_cursor) = delete_char(val, self.add_form.cursor);
                self.add_form.values[self.add_form.field] = new_val;
                self.add_form.cursor = new_cursor;
            }
            KeyCode::Char(c) => {
                let val = &self.add_form.values[self.add_form.field];
                let (new_val, new_cursor) = insert_char(val, self.add_form.cursor, c);
                self.add_form.values[self.add_form.field] = new_val;
                self.add_form.cursor = new_cursor;
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_edit_key(&mut self, key: KeyEvent, tx: &tokio::sync::mpsc::Sender<TuiEvent>) -> Result<()> {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    if !self.fetching_models {
                        self.submit_edit()?;
                    }
                }
                KeyCode::Char('a') | KeyCode::Char('A') => {
                    self.edit_form.cursor = 0;
                }
                KeyCode::Char('e') | KeyCode::Char('E') => {
                    let val = &self.edit_form.values[self.edit_form.field];
                    self.edit_form.cursor = val.chars().count();
                }
                KeyCode::Char('l') | KeyCode::Char('L') => {
                    self.trigger_fetch_models(ViewMode::Edit, tx);
                }
                _ => {}
            }
            return Ok(());
        }

        match key.code {
            KeyCode::Esc => {
                self.mode = ViewMode::List;
                self.status.clear();
                self.status_err = false;
            }
            KeyCode::Enter => {
                if self.edit_form.field < 4 {
                    self.edit_form.field += 1;
                    let val = &self.edit_form.values[self.edit_form.field];
                    self.edit_form.cursor = val.chars().count();
                } else {
                    self.submit_edit()?;
                }
            }
            KeyCode::Up => {
                if self.edit_form.field > 0 {
                    self.edit_form.field -= 1;
                    let val = &self.edit_form.values[self.edit_form.field];
                    self.edit_form.cursor = val.chars().count();
                }
            }
            KeyCode::Down => {
                if self.edit_form.field < 4 {
                    self.edit_form.field += 1;
                    let val = &self.edit_form.values[self.edit_form.field];
                    self.edit_form.cursor = val.chars().count();
                }
            }
            KeyCode::Left => {
                if self.edit_form.cursor > 0 {
                    self.edit_form.cursor -= 1;
                }
            }
            KeyCode::Right => {
                let val = &self.edit_form.values[self.edit_form.field];
                if self.edit_form.cursor < val.chars().count() {
                    self.edit_form.cursor += 1;
                }
            }
            KeyCode::Home => {
                self.edit_form.cursor = 0;
            }
            KeyCode::End => {
                let val = &self.edit_form.values[self.edit_form.field];
                self.edit_form.cursor = val.chars().count();
            }
            KeyCode::Backspace => {
                let val = &self.edit_form.values[self.edit_form.field];
                let (new_val, new_cursor) = delete_char(val, self.edit_form.cursor);
                self.edit_form.values[self.edit_form.field] = new_val;
                self.edit_form.cursor = new_cursor;
            }
            KeyCode::Char(c) => {
                let val = &self.edit_form.values[self.edit_form.field];
                let (new_val, new_cursor) = insert_char(val, self.edit_form.cursor, c);
                self.edit_form.values[self.edit_form.field] = new_val;
                self.edit_form.cursor = new_cursor;
            }
            _ => {}
        }
        Ok(())
    }

    fn trigger_fetch_models(&mut self, origin: ViewMode, tx: &tokio::sync::mpsc::Sender<TuiEvent>) {
        // Read base_url and api_key from whichever form is active
        let (base_url, api_key) = match origin {
            ViewMode::Add => (
                self.add_form.values[1].trim().to_string(),
                self.add_form.values[2].trim().to_string(),
            ),
            ViewMode::Edit => (
                self.edit_form.values[1].trim().to_string(),
                self.edit_form.values[2].trim().to_string(),
            ),
            _ => return,
        };

        if base_url.is_empty() || api_key.is_empty() {
            self.status = "⚠ Fill in Base URL and API Key first".to_string();
            self.status_err = true;
            return;
        }

        self.fetching_models = true;
        self.model_picker_origin = origin;
        self.status = "⟳ Fetching models…".to_string();
        self.status_err = false;

        let tx = tx.clone();
        tokio::spawn(async move {
            let result = crate::connectivity::fetch_models(&base_url, &api_key).await
                .map_err(|e| e.to_string());
            let _ = tx.send(TuiEvent::ModelsFetched { target: origin, result }).await;
        });
    }

    fn handle_model_picker_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.mode = self.model_picker_origin;
                self.status = "Model selection cancelled".to_string();
                self.status_err = false;
            }
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                if self.model_picker_cursor > 0 {
                    self.model_picker_cursor -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                if !self.model_list.is_empty() && self.model_picker_cursor < self.model_list.len() - 1 {
                    self.model_picker_cursor += 1;
                }
            }
            KeyCode::Enter => {
                if let Some(selected) = self.model_list.get(self.model_picker_cursor) {
                    let selected = selected.clone();
                    match self.model_picker_origin {
                        ViewMode::Add => {
                            self.add_form.values[3] = selected.clone();
                            self.add_form.cursor = selected.chars().count();
                        }
                        ViewMode::Edit => {
                            self.edit_form.values[3] = selected.clone();
                            self.edit_form.cursor = selected.chars().count();
                        }
                        _ => {}
                    }
                    self.status = format!("✓ Selected model: {}", selected);
                    self.status_err = false;
                    self.mode = self.model_picker_origin;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_confirm_key(&mut self, key: KeyEvent) -> Result<()> {
        let confirm = match &self.confirm {
            Some(c) => c,
            None => {
                self.mode = ViewMode::List;
                return Ok(());
            }
        };

        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                let action = confirm.action;
                let subject = confirm.subject.clone();
                self.confirm = None;

                match action {
                    ConfirmAction::Switch => {
                        if let Some(p) = config::get_provider(&self.cfg, &subject) {
                            let p = p.clone();
                            match CodexAdapter::new() {
                                Ok(adapter) => {
                                    let tc = TargetConfig {
                                        base_url: p.base_url.clone(),
                                        api_key: p.api_key.clone(),
                                        model: p.model.clone(),
                                        wire_api: if p.wire_api.is_empty() { "responses".to_string() } else { p.wire_api.clone() },
                                    };
                                    match adapter.write(&tc) {
                                        Ok(_) => {
                                            match config::set_active(&mut self.cfg, &subject) {
                                                Ok(_) => {
                                                    self.status = format!("✓ Switched to \"{}\"", subject);
                                                    self.status_err = false;
                                                }
                                                Err(e) => {
                                                    self.status = format!("✗ Switch failed: {}", e);
                                                    self.status_err = true;
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            self.status = format!("✗ Switch failed: {}", e);
                                            self.status_err = true;
                                        }
                                    }
                                }
                                Err(e) => {
                                    self.status = format!("✗ Switch failed: {}", e);
                                    self.status_err = true;
                                }
                            }
                        }
                    }
                    ConfirmAction::Remove => {
                        match config::remove_provider(&mut self.cfg, &subject) {
                            Ok(_) => {
                                self.status = format!("✓ Provider \"{}\" removed", subject);
                                self.status_err = false;
                                if self.cursor > 0 && self.cursor >= self.cfg.providers.len() {
                                    self.cursor = self.cfg.providers.len().saturating_sub(1);
                                }
                            }
                            Err(e) => {
                                self.status = format!("✗ Remove failed: {}", e);
                                self.status_err = true;
                            }
                        }
                    }
                }
                self.mode = ViewMode::List;
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.confirm = None;
                self.mode = ViewMode::List;
                self.status = "Cancelled".to_string();
                self.status_err = false;
            }
            _ => {}
        }
        Ok(())
    }
}

// ── Helper functions for text manipulation ──────────────────────────────────

fn insert_char(s: &str, idx: usize, c: char) -> (String, usize) {
    let mut chars: Vec<char> = s.chars().collect();
    let idx = idx.min(chars.len());
    chars.insert(idx, c);
    (chars.into_iter().collect(), idx + 1)
}

fn delete_char(s: &str, idx: usize) -> (String, usize) {
    let mut chars: Vec<char> = s.chars().collect();
    if idx == 0 || chars.is_empty() {
        return (s.to_string(), idx);
    }
    let delete_idx = (idx - 1).min(chars.len() - 1);
    chars.remove(delete_idx);
    (chars.into_iter().collect(), delete_idx)
}

fn truncate(s: &str, n: usize) -> String {
    let w = unicode_width::UnicodeWidthStr::width(s);
    if w <= n {
        return s.to_string();
    }
    let mut current_w = 0;
    let mut chars = Vec::new();
    for c in s.chars() {
        let cw = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
        if current_w + cw > n.saturating_sub(1) {
            break;
        }
        current_w += cw;
        chars.push(c);
    }
    chars.push('…');
    chars.into_iter().collect()
}

fn validate_input_fields(name: &str, base_url: &str, api_key: &str, model: &str) -> Result<()> {
    if name.trim().is_empty() {
        return Err(anyhow::anyhow!("name cannot be empty"));
    }
    if !base_url.starts_with("http://") && !base_url.starts_with("https://") {
        return Err(anyhow::anyhow!("invalid base URL: must start with http:// or https://"));
    }
    url::Url::parse(base_url).map_err(|_| anyhow::anyhow!("invalid base URL"))?;
    if api_key.trim().is_empty() {
        return Err(anyhow::anyhow!("API key cannot be empty"));
    }
    if model.trim().is_empty() {
        return Err(anyhow::anyhow!("model cannot be empty"));
    }
    Ok(())
}

// ── Rendering ───────────────────────────────────────────────────────────────

fn draw(frame: &mut ratatui::Frame, app: &mut TuiApp, theme: &Theme) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Title bar
            Constraint::Length(1), // Spacer
            Constraint::Min(3),    // Main content (Table or Form or Confirm)
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Remark
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Status line
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Help bar
        ])
        .split(frame.size());

    // 1. Title bar
    let title_spans = vec![
        Span::styled(
            "  CXC — Codex Cross-Connect  ",
            Style::default().fg(theme.title_fg).bg(theme.title_bg).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(
            format!("v{}", env!("CARGO_PKG_VERSION")),
            Style::default().fg(theme.dim),
        ),
    ];
    let title = Paragraph::new(Line::from(title_spans));
    frame.render_widget(title, chunks[0]);

    // 2. Main content area
    match app.mode {
        ViewMode::List => {
            let providers = &app.cfg.providers;
            if providers.is_empty() {
                let no_providers = Paragraph::new("  No providers saved. Press [a] to add one.")
                    .style(Style::default().fg(theme.dim));
                frame.render_widget(no_providers, chunks[2]);
            } else {
                // We render Header + Separator + Table rows
                // Constrain the horizontal width of the list to exactly 107 characters (left-aligned)
                let horizontal_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Length(107),
                        Constraint::Min(0),
                    ])
                    .split(chunks[2]);

                let layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(2), // Header + Separator
                        Constraint::Min(1),    // Scrollable Table body
                    ])
                    .split(horizontal_chunks[0]);

                // Header & Separator Table
                let header_table = Table::new(
                    vec![
                        Row::new(vec![
                            Cell::from(""),
                            Cell::from("NAME").style(Style::default().fg(theme.header).add_modifier(Modifier::BOLD)),
                            Cell::from("BASE URL").style(Style::default().fg(theme.header).add_modifier(Modifier::BOLD)),
                            Cell::from("MODEL").style(Style::default().fg(theme.header).add_modifier(Modifier::BOLD)),
                            Cell::from("LATENCY").style(Style::default().fg(theme.header).add_modifier(Modifier::BOLD)),
                            Cell::from("LAST TEST").style(Style::default().fg(theme.header).add_modifier(Modifier::BOLD)),
                        ]),
                        Row::new(vec![
                            Cell::from(""),
                            Cell::from("────────────────"),
                            Cell::from("────────────────────────────────────────"),
                            Cell::from("────────────────────"),
                            Cell::from("────────────"),
                            Cell::from("────────────"),
                        ]).style(Style::default().fg(theme.dim)),
                    ],
                    [
                        Constraint::Length(2),  // Active star
                        Constraint::Length(16), // Name
                        Constraint::Length(40), // Base URL
                        Constraint::Length(20), // Model
                        Constraint::Length(12), // Latency
                        Constraint::Length(12), // Last Test
                    ]
                )
                .column_spacing(1);
                frame.render_widget(header_table, layout[0]);

                // Table rows
                let rows: Vec<Row> = providers.iter().map(|p| {
                    let is_active = p.name == app.cfg.active;

                    let active_cell = if is_active {
                        Cell::from("★ ").style(Style::default().fg(theme.active).add_modifier(Modifier::BOLD))
                    } else {
                        Cell::from("  ")
                    };

                    let name_cell = if is_active {
                        Cell::from(p.name.clone()).style(Style::default().fg(theme.active).add_modifier(Modifier::BOLD))
                    } else {
                        Cell::from(p.name.clone())
                    };

                    let url_cell = Cell::from(truncate(&p.base_url, 38));
                    let model_cell = Cell::from(truncate(&p.model, 18));

                    let latency_cell = if app.testing.contains(&p.name) {
                        Cell::from("⟳ testing…").style(Style::default().fg(theme.warn))
                    } else if let Some(lat_ms) = p.latency_ms {
                        let lat_str = format!("{}ms", lat_ms);
                        if p.last_ok == Some(true) {
                            Cell::from(format!("✓ {}", lat_str)).style(Style::default().fg(theme.success))
                        } else {
                            Cell::from(format!("✗ {}", lat_str)).style(Style::default().fg(theme.error))
                        }
                    } else {
                        Cell::from("-").style(Style::default().fg(theme.dim))
                    };

                    let last_test_text = if let Some(last_test_time) = p.last_test {
                        last_test_time.format("%H:%M:%S").to_string()
                    } else {
                        "-".to_string()
                    };
                    let last_test_cell = if p.last_test.is_none() {
                        Cell::from(last_test_text).style(Style::default().fg(theme.dim))
                    } else {
                        Cell::from(last_test_text)
                    };

                    Row::new(vec![
                        active_cell,
                        name_cell,
                        url_cell,
                        model_cell,
                        latency_cell,
                        last_test_cell,
                    ])
                }).collect();

                let table = Table::new(rows, [
                    Constraint::Length(2),  // Active star
                    Constraint::Length(16), // Name
                    Constraint::Length(40), // Base URL
                    Constraint::Length(20), // Model
                    Constraint::Length(12), // Latency
                    Constraint::Length(12), // Last Test
                ])
                .column_spacing(1)
                .highlight_style(Style::default().bg(theme.selected_bg).fg(theme.selected_fg));

                let mut table_state = TableState::default();
                table_state.select(Some(app.cursor));
                frame.render_stateful_widget(table, layout[1], &mut table_state);
            }
        }
        ViewMode::Add => {
            let labels = ["Name", "Base URL", "API Key", "Model", "Remark"];
            let placeholders = [
                "e.g. my-relay",
                "e.g. https://api.example.com/v1",
                "e.g. sk-...",
                "e.g. gpt-4",
                "e.g. backup proxy (optional)",
            ];

            let show_cursor = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| (d.as_millis() % 1000) < 500)
                .unwrap_or(true);
            let cursor_char = if show_cursor { "█" } else { " " };

            let mut lines = vec![
                Line::from(Span::styled("  Add Provider", Style::default().fg(theme.header).add_modifier(Modifier::BOLD))),
                Line::from(""),
            ];

            for (idx, label) in labels.iter().enumerate() {
                let val = &app.add_form.values[idx];
                let prefix = if idx == app.add_form.field { "> " } else { "  " };

                if idx == app.add_form.field {
                    let mut spans = vec![
                        Span::styled(format!("{}{}: ", prefix, label), Style::default().fg(theme.warn).add_modifier(Modifier::BOLD)),
                    ];

                    let chars: Vec<char> = val.chars().collect();
                    let cursor_idx = app.add_form.cursor.min(chars.len());
                    let left: String = chars[..cursor_idx].iter().collect();
                    let right: String = chars[cursor_idx..].iter().collect();

                    if val.is_empty() {
                        spans.push(Span::raw(cursor_char));
                        spans.push(Span::styled(placeholders[idx].to_string(), Style::default().fg(theme.dim)));
                    } else {
                        spans.push(Span::raw(left));
                        spans.push(Span::raw(cursor_char));
                        spans.push(Span::raw(right));
                    }
                    lines.push(Line::from(spans));
                } else if idx < app.add_form.field {
                    // For the Model field (index 3), show fetching indicator if in progress
                    let display_val = if idx == 3 && app.fetching_models {
                        Span::styled("⟳ fetching…".to_string(), Style::default().fg(theme.warn))
                    } else {
                        Span::styled(val.clone(), Style::default().fg(theme.success))
                    };
                    lines.push(Line::from(vec![
                        Span::styled(format!("{}{}: ", prefix, label), Style::default().fg(theme.dim)),
                        display_val,
                    ]));
                } else {
                    lines.push(Line::from(vec![
                        Span::styled(format!("{}{}: ", prefix, label), Style::default().fg(theme.dim)),
                        Span::styled(placeholders[idx].to_string(), Style::default().fg(theme.dim)),
                    ]));
                }
            }


            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  ↳ 按 Ctrl+L 可从 base_url 拉取模型列表",
                Style::default().fg(theme.dim),
            )));
            frame.render_widget(Paragraph::new(lines), chunks[2]);
        }
        ViewMode::Edit => {
            let labels = ["Name", "Base URL", "API Key", "Model", "Remark"];

            let show_cursor = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| (d.as_millis() % 1000) < 500)
                .unwrap_or(true);
            let cursor_char = if show_cursor { "█" } else { " " };

            let mut lines = vec![
                Line::from(Span::styled(
                    format!("  Edit Provider: {}", app.edit_form.old_name),
                    Style::default().fg(theme.header).add_modifier(Modifier::BOLD)
                )),
                Line::from(""),
            ];

            for (idx, label) in labels.iter().enumerate() {
                let val = &app.edit_form.values[idx];
                let prefix = if idx == app.edit_form.field { "> " } else { "  " };

                if idx == app.edit_form.field {
                    let mut spans = vec![
                        Span::styled(format!("{}{}: ", prefix, label), Style::default().fg(theme.warn).add_modifier(Modifier::BOLD)),
                    ];

                    let chars: Vec<char> = val.chars().collect();
                    let cursor_idx = app.edit_form.cursor.min(chars.len());
                    let left: String = chars[..cursor_idx].iter().collect();
                    let right: String = chars[cursor_idx..].iter().collect();

                    spans.push(Span::raw(left));
                    spans.push(Span::raw(cursor_char));
                    spans.push(Span::raw(right));
                    lines.push(Line::from(spans));
                } else if idx < app.edit_form.field {
                    let display_val = if idx == 3 && app.fetching_models {
                        Span::styled("⟳ fetching…".to_string(), Style::default().fg(theme.warn))
                    } else {
                        Span::styled(val.clone(), Style::default().fg(theme.success))
                    };
                    lines.push(Line::from(vec![
                        Span::styled(format!("{}{}: ", prefix, label), Style::default().fg(theme.dim)),
                        display_val,
                    ]));
                } else {
                    lines.push(Line::from(vec![
                        Span::styled(format!("{}{}: ", prefix, label), Style::default().fg(theme.dim)),
                        Span::styled(val.clone(), Style::default().fg(theme.dim)),
                    ]));
                }
            }


            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  ↳ 按 Ctrl+L 可从 base_url 拉取模型列表",
                Style::default().fg(theme.dim),
            )));
            frame.render_widget(Paragraph::new(lines), chunks[2]);
        }
        ViewMode::Confirm => {
            if let Some(confirm) = &app.confirm {
                let action_text = match confirm.action {
                    ConfirmAction::Switch => format!("Switch to \"{}\"? This will modify Codex config.", confirm.subject),
                    ConfirmAction::Remove => format!("Remove provider \"{}\"?", confirm.subject),
                };

                let confirm_lines = vec![
                    Line::from(Span::styled(format!("  {}", action_text), Style::default().fg(theme.warn).add_modifier(Modifier::BOLD))),
                    Line::from(""),
                    Line::from(vec![
                        Span::raw("  "),
                        Span::styled("[y]", Style::default().fg(theme.success).add_modifier(Modifier::BOLD)),
                        Span::raw(" yes   "),
                        Span::styled("[n]", Style::default().fg(theme.error).add_modifier(Modifier::BOLD)),
                        Span::raw(" cancel"),
                    ]),
                ];
                frame.render_widget(Paragraph::new(confirm_lines), chunks[2]);
            }
        }
        ViewMode::ModelPicker => {
            let (op, name) = match app.model_picker_origin {
                ViewMode::Add => (
                    "添加 Provider",
                    app.add_form.values[0].trim().to_string(),
                ),
                ViewMode::Edit => (
                    "编辑 Provider",
                    app.edit_form.old_name.trim().to_string(),
                ),
                _ => ("", String::new()),
            };
            let title = if name.is_empty() {
                "  Select Model".to_string()
            } else {
                format!("  {} : {} — Select Model", op, name)
            };

            let mut lines = vec![
                Line::from(Span::styled(
                    title,
                    Style::default().fg(theme.header).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
            ];

            if app.model_list.is_empty() {
                lines.push(Line::from(Span::styled(
                    "  (no models)",
                    Style::default().fg(theme.dim),
                )));
            } else {
                for (idx, model_id) in app.model_list.iter().enumerate() {
                    let is_selected = idx == app.model_picker_cursor;
                    let prefix = if is_selected { "> " } else { "  " };
                    let style = if is_selected {
                        Style::default().fg(theme.selected_fg).bg(theme.selected_bg).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    };
                    lines.push(Line::from(Span::styled(
                        format!("{}  {}", prefix, model_id),
                        style,
                    )));
                }
            }

            frame.render_widget(Paragraph::new(lines), chunks[2]);
        }
    }

    // 3. Remark line
    if app.mode == ViewMode::List && !app.cfg.providers.is_empty() && app.cursor < app.cfg.providers.len() {
        let p = &app.cfg.providers[app.cursor];
        let remark_text = p.remark.as_deref().unwrap_or("(none)");
        let remark_line = Line::from(vec![
            Span::styled("  Remark: ", Style::default().fg(theme.header).add_modifier(Modifier::BOLD)),
            Span::styled(remark_text, Style::default().fg(theme.remark).add_modifier(Modifier::ITALIC)),
        ]);
        frame.render_widget(Paragraph::new(remark_line), chunks[4]);
    } else {
        frame.render_widget(Paragraph::new(""), chunks[4]);
    }

    // 4. Status line
    if !app.status.is_empty() {
        let status_style = if app.status_err {
            Style::default().fg(theme.error).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.success).add_modifier(Modifier::BOLD)
        };
        let status_p = Paragraph::new(Line::from(Span::styled(format!("  {}", app.status), status_style)));
        frame.render_widget(status_p, chunks[6]);
    } else {
        frame.render_widget(Paragraph::new(""), chunks[6]);
    }

    // 5. Help bar
    let help_text = match app.mode {
        ViewMode::List => "  ↑/↓ navigate  ·  a add  ·  e edit  ·  t test  ·  T test all  ·  Enter/s switch  ·  d/Del remove  ·  q quit",
        ViewMode::Add | ViewMode::Edit => "  [Enter] next  ·  [↑/↓] navigate  ·  [Ctrl+L] fetch models  ·  [Ctrl+S] save  ·  [Esc] cancel",
        ViewMode::ModelPicker => "  [↑/↓] navigate  ·  [Enter] select  ·  [Esc] cancel",
        _ => "",
    };
    if !help_text.is_empty() {
        let help_p = Paragraph::new(help_text).style(Style::default().fg(theme.help));
        frame.render_widget(help_p, chunks[8]);
    } else {
        frame.render_widget(Paragraph::new(""), chunks[8]);
    }
}

// ── Main Event Loop ──────────────────────────────────────────────────────────

async fn run_tui_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    tx: &tokio::sync::mpsc::Sender<TuiEvent>,
    rx: &mut tokio::sync::mpsc::Receiver<TuiEvent>,
) -> Result<()> {
    let mut app = TuiApp::new()?;
    let theme = Theme::default();

    loop {
        terminal.draw(|f| draw(f, &mut app, &theme))?;

        if let Some(event) = rx.recv().await {
            match event {
                TuiEvent::Tick => {
                    // Just tick to update blinking cursor
                }
                TuiEvent::Key(key) => {
                    match app.mode {
                        ViewMode::List => {
                            if app.handle_list_key(key, tx)? {
                                break; // Exit app
                            }
                        }
                        ViewMode::Add => {
                            app.handle_add_key(key, tx)?;
                        }
                        ViewMode::Edit => {
                            app.handle_edit_key(key, tx)?;
                        }
                        ViewMode::Confirm => {
                            app.handle_confirm_key(key)?;
                        }
                        ViewMode::ModelPicker => {
                            app.handle_model_picker_key(key)?;
                        }
                    }
                }
                TuiEvent::TestResult { name, ok, latency_ms, error } => {
                    app.testing.remove(&name);

                    let latency_val = latency_ms.unwrap_or(0);
                    if let Err(e) = config::update_test_result(&mut app.cfg, &name, latency_val as i64, ok) {
                        app.status = format!("✗ Failed to update config: {}", e);
                        app.status_err = true;
                    } else {
                        if ok {
                            app.status = format!("✓ {}: connected in {}ms", name, latency_val);
                            app.status_err = false;
                        } else {
                            app.status = format!("✗ {}: {}", name, error.as_deref().unwrap_or("unknown error"));
                            app.status_err = true;
                        }
                    }

                    if app.testing.is_empty() && app.test_all_active {
                        app.status = "✓ All tests completed".to_string();
                        app.test_all_active = false;
                    }

                    if let Ok(new_cfg) = config::load() {
                        app.cfg = new_cfg;
                    }
                }
                TuiEvent::ModelsFetched { target, result } => {
                    app.fetching_models = false;
                    match result {
                        Ok(models) if models.is_empty() => {
                            app.status = "⚠ No models returned".to_string();
                            app.status_err = true;
                        }
                        Ok(models) => {
                            app.model_list = models;
                            app.model_picker_cursor = 0;
                            app.model_picker_origin = target;
                            app.mode = ViewMode::ModelPicker;
                            app.status = format!("✓ {} models fetched — use ↑/↓ to select", app.model_list.len());
                            app.status_err = false;
                        }
                        Err(e) => {
                            app.status = format!("✗ {}", e);
                            app.status_err = true;
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

pub async fn run() -> Result<()> {
    // Setup panic hook to restore terminal on panic
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original_hook(panic_info);
    }));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let (tx, mut rx) = tokio::sync::mpsc::channel::<TuiEvent>(100);

    // Event thread
    let tx_input = tx.clone();
    std::thread::spawn(move || {
        loop {
            match event::read() {
                Ok(Event::Key(key)) => {
                    if key.kind != event::KeyEventKind::Release {
                        if tx_input.blocking_send(TuiEvent::Key(key)).is_err() {
                            break;
                        }
                    }
                }
                Ok(Event::Resize(_, _)) => {
                    // Force a tick event on resize to trigger redraw immediately
                    let _ = tx_input.blocking_send(TuiEvent::Tick);
                }
                _ => {}
            }
        }
    });

    // Ticker thread
    let tx_tick = tx.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(250));
        loop {
            interval.tick().await;
            if tx_tick.send(TuiEvent::Tick).await.is_err() {
                break;
            }
        }
    });

    let res = run_tui_loop(&mut terminal, &tx, &mut rx).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_manipulation() {
        // Insert char
        let (s1, c1) = insert_char("hello", 2, 'x');
        assert_eq!(s1, "hexllo");
        assert_eq!(c1, 3);

        let (s2, c2) = insert_char("你好", 1, 'x');
        assert_eq!(s2, "你x好");
        assert_eq!(c2, 2);

        // Delete char
        let (s3, c3) = delete_char("hexllo", 3);
        assert_eq!(s3, "hello");
        assert_eq!(c3, 2);

        let (s4, c4) = delete_char("你x好", 2);
        assert_eq!(s4, "你好");
        assert_eq!(c4, 1);
    }

    #[test]
    fn test_validation() {
        assert!(validate_input_fields("p1", "https://api.openai.com/v1", "sk-key", "gpt-4").is_ok());
        assert!(validate_input_fields("", "https://api.openai.com/v1", "sk-key", "gpt-4").is_err());
        assert!(validate_input_fields("p1", "ftp://api.openai.com", "sk-key", "gpt-4").is_err());
        assert!(validate_input_fields("p1", "https://api.openai.com/v1", "", "gpt-4").is_err());
        assert!(validate_input_fields("p1", "https://api.openai.com/v1", "sk-key", "").is_err());
    }
}
