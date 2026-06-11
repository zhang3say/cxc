// Package tui implements the Bubble Tea TUI for CXC.
// It is launched when `cxc` is run with no subcommand.
package tui

import (
	"fmt"
	"net/url"
	"strings"
	"time"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/zhang3say/cxc/internal/config"
	"github.com/zhang3say/cxc/internal/connectivity"
	"github.com/zhang3say/cxc/internal/target"
	codexadapter "github.com/zhang3say/cxc/internal/target/codex"
)

// ── Styles ──────────────────────────────────────────────────────────────────

var (
	styleTitle = lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#ECEFF4")).
			Background(lipgloss.Color("#5E81AC")).
			Padding(0, 2)

	styleActive = lipgloss.NewStyle().
			Foreground(lipgloss.Color("#A3BE8C")).
			Bold(true)

	styleSelected = lipgloss.NewStyle().
			Background(lipgloss.Color("#3B4252")).
			Foreground(lipgloss.Color("#ECEFF4"))

	styleDim = lipgloss.NewStyle().
			Foreground(lipgloss.Color("#4C566A"))

	styleSuccess = lipgloss.NewStyle().
			Foreground(lipgloss.Color("#A3BE8C"))

	styleError = lipgloss.NewStyle().
			Foreground(lipgloss.Color("#BF616A"))

	styleWarn = lipgloss.NewStyle().
			Foreground(lipgloss.Color("#EBCB8B"))

	styleHelp = lipgloss.NewStyle().
			Foreground(lipgloss.Color("#616E88"))

	styleHeader = lipgloss.NewStyle().
			Foreground(lipgloss.Color("#81A1C1")).
			Bold(true)
)

// ── View modes ───────────────────────────────────────────────────────────────

type viewMode int

const (
	modeList    viewMode = iota
	modeAdd              // sequential add form
	modeConfirm          // confirm switch/remove
	modeEdit             // sequential edit form
)

// ── Messages ─────────────────────────────────────────────────────────────────

type testResultMsg struct {
	name   string
	result connectivity.Result
}

type switchDoneMsg struct{ err error }
type removeDoneMsg struct{ err error }
type addDoneMsg struct{ err error }
type editDoneMsg struct{ err error }
type configReloadedMsg struct{ cfg *config.Config }

// ── Add form state ────────────────────────────────────────────────────────────

type addField int

const (
	fieldName addField = iota
	fieldBaseURL
	fieldAPIKey
	fieldModel
	fieldRemark
	fieldDone
)

type addFormState struct {
	field  addField
	values [5]string
	cursor int
}

// ── Edit form state ───────────────────────────────────────────────────────────

type editField int

const (
	editFieldName editField = iota
	editFieldBaseURL
	editFieldAPIKey
	editFieldModel
	editFieldRemark
	editFieldDone
)

type editFormState struct {
	field   editField
	values  [5]string
	oldName string
	cursor  int
}

// ── Confirm state ─────────────────────────────────────────────────────────────

type confirmAction int

const (
	confirmSwitch confirmAction = iota
	confirmRemove
)

type confirmState struct {
	action  confirmAction
	subject string // provider name
}

// ── Model ─────────────────────────────────────────────────────────────────────

type model struct {
	cfg       *config.Config
	cursor    int
	mode      viewMode
	addForm   addFormState
	editForm  editFormState
	confirm   confirmState
	status    string
	statusErr bool
	testing   string // provider name being tested, "" if none
	width     int
	height    int
}

func initialModel() (model, error) {
	cfg, err := config.Load()
	if err != nil {
		return model{}, err
	}
	return model{cfg: cfg}, nil
}

// ── Init ──────────────────────────────────────────────────────────────────────

func (m model) Init() tea.Cmd {
	return nil
}

// ── Update ────────────────────────────────────────────────────────────────────

func (m model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {

	case tea.WindowSizeMsg:
		m.width = msg.Width
		m.height = msg.Height
		return m, nil

	case testResultMsg:
		m.testing = ""
		_ = config.UpdateTestResult(m.cfg, msg.name, msg.result.LatencyMS, msg.result.OK)
		if msg.result.OK {
			m.status = fmt.Sprintf("✓ %s: connected in %dms — %q", msg.name, msg.result.LatencyMS, msg.result.Response)
			m.statusErr = false
		} else {
			m.status = fmt.Sprintf("✗ %s: %s", msg.name, msg.result.Error)
			m.statusErr = true
		}
		return m, reloadConfig()

	case configReloadedMsg:
		if msg.cfg != nil {
			m.cfg = msg.cfg
		}
		return m, nil

	case switchDoneMsg:
		if msg.err != nil {
			m.status = "✗ Switch failed: " + msg.err.Error()
			m.statusErr = true
		} else {
			m.status = "✓ Switched successfully"
			m.statusErr = false
		}
		m.mode = modeList
		return m, reloadConfig()

	case removeDoneMsg:
		if msg.err != nil {
			m.status = "✗ Remove failed: " + msg.err.Error()
			m.statusErr = true
		} else {
			m.status = "✓ Provider removed"
			m.statusErr = false
		}
		m.mode = modeList
		if m.cursor > 0 && m.cursor >= len(m.cfg.Providers)-1 {
			m.cursor--
		}
		return m, reloadConfig()

	case addDoneMsg:
		if msg.err != nil {
			m.status = "✗ Add failed: " + msg.err.Error()
			m.statusErr = true
		} else {
			m.status = "✓ Provider added"
			m.statusErr = false
		}
		m.mode = modeList
		m.addForm = addFormState{}
		return m, reloadConfig()

	case editDoneMsg:
		if msg.err != nil {
			m.status = "✗ Edit failed: " + msg.err.Error()
			m.statusErr = true
		} else {
			m.status = "✓ Provider updated"
			m.statusErr = false
		}
		m.mode = modeList
		m.editForm = editFormState{}
		return m, reloadConfig()

	case tickMsg:
		if m.mode == modeAdd || m.mode == modeEdit {
			return m, tick()
		}
		return m, nil

	case tea.KeyMsg:
		switch m.mode {
		case modeList:
			return m.updateList(msg)
		case modeAdd:
			return m.updateAdd(msg)
		case modeConfirm:
			return m.updateConfirm(msg)
		case modeEdit:
			return m.updateEdit(msg)
		}
	}

	return m, nil
}

func (m model) updateList(msg tea.KeyMsg) (tea.Model, tea.Cmd) {
	providers := m.cfg.Providers

	switch msg.String() {
	case "q", "ctrl+c", "esc":
		return m, tea.Quit

	case "up", "k":
		if m.cursor > 0 {
			m.cursor--
		}

	case "down", "j":
		if m.cursor < len(providers)-1 {
			m.cursor++
		}

	case "a":
		m.mode = modeAdd
		m.addForm = addFormState{}
		return m, tick()

	case "e":
		if len(providers) == 0 {
			break
		}
		p := providers[m.cursor]
		m.mode = modeEdit
		m.editForm = editFormState{
			field:   editFieldName,
			oldName: p.Name,
			values: [5]string{
				p.Name,
				p.BaseURL,
				p.APIKey,
				p.Model,
				p.Remark,
			},
		}
		return m, tick()

	case "t":
		if len(providers) == 0 {
			break
		}
		p := providers[m.cursor]
		m.testing = p.Name
		m.status = fmt.Sprintf("Testing %q…", p.Name)
		m.statusErr = false
		return m, runTest(p.Name, p.BaseURL, p.APIKey, p.Model)

	case "enter", "s":
		if len(providers) == 0 {
			break
		}
		p := providers[m.cursor]
		if p.Name == m.cfg.Active {
			m.status = fmt.Sprintf("Provider %q is already active.", p.Name)
			m.statusErr = false
			break
		}
		m.mode = modeConfirm
		m.confirm = confirmState{action: confirmSwitch, subject: p.Name}

	case "d", "delete":
		if len(providers) == 0 {
			break
		}
		p := providers[m.cursor]
		m.mode = modeConfirm
		m.confirm = confirmState{action: confirmRemove, subject: p.Name}
	}

	return m, nil
}

func (m model) updateAdd(msg tea.KeyMsg) (tea.Model, tea.Cmd) {
	idx := int(m.addForm.field)
	valRunes := []rune("")
	if idx < 5 {
		valRunes = []rune(m.addForm.values[idx])
	}

	switch msg.String() {
	case "esc":
		m.mode = modeList
		m.addForm = addFormState{}
		return m, nil

	case "enter":
		if m.addForm.field < fieldDone {
			m.addForm.field++
			if m.addForm.field == fieldDone {
				// All fields collected — submit
				return m, submitAdd(m.addForm.values)
			}
			m.addForm.cursor = len([]rune(m.addForm.values[m.addForm.field]))
		}

	case "left":
		if m.addForm.cursor > 0 {
			m.addForm.cursor--
		}
	case "right":
		if m.addForm.cursor < len(valRunes) {
			m.addForm.cursor++
		}
	case "home", "ctrl+a":
		m.addForm.cursor = 0
	case "end", "ctrl+e":
		m.addForm.cursor = len(valRunes)

	case "backspace":
		if idx < 5 {
			m.addForm.values[idx], m.addForm.cursor = deleteAtRune(m.addForm.values[idx], m.addForm.cursor)
		}

	default:
		if idx < 5 && len(msg.Runes) > 0 {
			m.addForm.values[idx] = insertAtRune(m.addForm.values[idx], m.addForm.cursor, string(msg.Runes))
			m.addForm.cursor += len(msg.Runes)
		}
	}

	return m, nil
}

func (m model) updateEdit(msg tea.KeyMsg) (tea.Model, tea.Cmd) {
	idx := int(m.editForm.field)
	valRunes := []rune("")
	if idx < 5 {
		valRunes = []rune(m.editForm.values[idx])
	}

	switch msg.String() {
	case "esc":
		m.mode = modeList
		m.editForm = editFormState{}
		return m, nil

	case "enter":
		if m.editForm.field < editFieldDone {
			m.editForm.field++
			if m.editForm.field == editFieldDone {
				// All fields collected — submit
				return m, submitEdit(m.editForm.oldName, m.editForm.values)
			}
			m.editForm.cursor = len([]rune(m.editForm.values[m.editForm.field]))
		}

	case "left":
		if m.editForm.cursor > 0 {
			m.editForm.cursor--
		}
	case "right":
		if m.editForm.cursor < len(valRunes) {
			m.editForm.cursor++
		}
	case "home", "ctrl+a":
		m.editForm.cursor = 0
	case "end", "ctrl+e":
		m.editForm.cursor = len(valRunes)

	case "backspace":
		if idx < 5 {
			m.editForm.values[idx], m.editForm.cursor = deleteAtRune(m.editForm.values[idx], m.editForm.cursor)
		}

	default:
		if idx < 5 && len(msg.Runes) > 0 {
			m.editForm.values[idx] = insertAtRune(m.editForm.values[idx], m.editForm.cursor, string(msg.Runes))
			m.editForm.cursor += len(msg.Runes)
		}
	}

	return m, nil
}

func (m model) updateConfirm(msg tea.KeyMsg) (tea.Model, tea.Cmd) {
	switch msg.String() {
	case "y", "Y":
		switch m.confirm.action {
		case confirmSwitch:
			return m, runSwitch(m.cfg, m.confirm.subject)
		case confirmRemove:
			return m, runRemove(m.cfg, m.confirm.subject)
		}
	case "n", "N", "esc":
		m.mode = modeList
		m.status = "Cancelled"
		m.statusErr = false
	}
	return m, nil
}

// ── View ──────────────────────────────────────────────────────────────────────

func (m model) View() string {
	var sb strings.Builder

	// Title bar
	sb.WriteString(styleTitle.Render("  CXC — Codex Cross-Connect  "))
	sb.WriteString("\n\n")

	switch m.mode {
	case modeList:
		sb.WriteString(m.viewList())
	case modeAdd:
		sb.WriteString(m.viewAdd())
	case modeConfirm:
		sb.WriteString(m.viewConfirm())
	case modeEdit:
		sb.WriteString(m.viewEdit())
	}

	// Status line
	sb.WriteString("\n")
	if m.status != "" {
		if m.statusErr {
			sb.WriteString(styleError.Render(m.status))
		} else {
			sb.WriteString(styleSuccess.Render(m.status))
		}
		sb.WriteString("\n")
	}

	// Help bar
	sb.WriteString("\n")
	sb.WriteString(m.viewHelp())

	return sb.String()
}

func (m model) viewList() string {
	var sb strings.Builder

	if len(m.cfg.Providers) == 0 {
		sb.WriteString(styleDim.Render("  No providers saved. Press [a] to add one."))
		return sb.String()
	}

	// Header
	header := fmt.Sprintf("  %-16s %-40s %-20s %-10s %-12s",
		"NAME", "BASE URL", "MODEL", "LATENCY", "LAST TEST")
	sb.WriteString(styleHeader.Render(header))
	sb.WriteString("\n")
	sb.WriteString(styleDim.Render("  " + strings.Repeat("─", 100)))
	sb.WriteString("\n")

	for i, p := range m.cfg.Providers {
		active := "  "
		name := p.Name
		if p.Name == m.cfg.Active {
			active = "★ "
			name = styleActive.Render(name)
		}

		baseURL := truncate(p.BaseURL, 38)
		model := truncate(p.Model, 18)

		latency := styleDim.Render("-")
		if p.LatencyMS != nil {
			lat := fmt.Sprintf("%dms", *p.LatencyMS)
			if p.LastOK != nil && *p.LastOK {
				latency = styleSuccess.Render("✓ " + lat)
			} else {
				latency = styleError.Render("✗ " + lat)
			}
		}
		if m.testing == p.Name {
			latency = styleWarn.Render("⟳ testing…")
		}

		lastTest := styleDim.Render("-")
		if p.LastTest != nil {
			lastTest = p.LastTest.Format("15:04:05")
		}

		row := fmt.Sprintf("%s%-16s %-40s %-20s %-10s %-12s",
			active, name, baseURL, model, latency, lastTest)

		if i == m.cursor {
			sb.WriteString(styleSelected.Render(row))
		} else {
			sb.WriteString(row)
		}
		sb.WriteString("\n")
	}

	// Show remark of selected provider
	sb.WriteString("\n")
	if m.cursor >= 0 && m.cursor < len(m.cfg.Providers) {
		p := m.cfg.Providers[m.cursor]
		remarkText := p.Remark
		if remarkText == "" {
			remarkText = "(none)"
		}
		sb.WriteString(styleHeader.Render("  Remark: ") + styleWarn.Render(remarkText) + "\n")
	}

	return sb.String()
}

func (m model) viewAdd() string {
	labels := []string{"Name", "Base URL", "API Key", "Model", "Remark"}
	placeholders := []string{
		"e.g. my-relay",
		"e.g. https://api.example.com/v1",
		"e.g. sk-...",
		"e.g. gpt-4",
		"e.g. backup proxy (optional)",
	}

	var sb strings.Builder
	sb.WriteString(styleHeader.Render("  Add Provider") + "\n\n")

	for i, label := range labels {
		f := addField(i)
		val := m.addForm.values[i]

		prefix := "  "
		if f == m.addForm.field {
			prefix = "> "
			cursor := "█"
			if time.Now().UnixMilli()%1000 < 500 {
				cursor = " "
			}
			runes := []rune(val)
			cursorIdx := m.addForm.cursor
			if cursorIdx < 0 {
				cursorIdx = 0
			}
			if cursorIdx > len(runes) {
				cursorIdx = len(runes)
			}
			left := string(runes[:cursorIdx])
			right := string(runes[cursorIdx:])
			if val == "" {
				sb.WriteString(styleWarn.Render(prefix+label+": ") + cursor + styleDim.Render(placeholders[i]) + "\n")
			} else {
				sb.WriteString(styleWarn.Render(prefix+label+": ") + left + cursor + right + "\n")
			}
		} else if f < m.addForm.field {
			sb.WriteString(styleDim.Render(prefix+label+": ") + styleSuccess.Render(val) + "\n")
		} else {
			sb.WriteString(styleDim.Render(prefix+label+": ") + styleDim.Render(placeholders[i]) + "\n")
		}
	}

	sb.WriteString("\n" + styleDim.Render("  [Enter] next field/submit  [Esc] cancel"))
	return sb.String()
}

func (m model) viewEdit() string {
	labels := []string{"Name", "Base URL", "API Key", "Model", "Remark"}
	var sb strings.Builder
	sb.WriteString(styleHeader.Render(fmt.Sprintf("  Edit Provider: %s", m.editForm.oldName)) + "\n\n")

	for i, label := range labels {
		f := editField(i)
		val := m.editForm.values[i]

		prefix := "  "
		if f == m.editForm.field {
			prefix = "> "
			cursor := "█"
			if time.Now().UnixMilli()%1000 < 500 {
				cursor = " "
			}
			runes := []rune(val)
			cursorIdx := m.editForm.cursor
			if cursorIdx < 0 {
				cursorIdx = 0
			}
			if cursorIdx > len(runes) {
				cursorIdx = len(runes)
			}
			left := string(runes[:cursorIdx])
			right := string(runes[cursorIdx:])
			sb.WriteString(styleWarn.Render(prefix+label+": ") + left + cursor + right + "\n")
		} else if f < m.editForm.field {
			sb.WriteString(styleDim.Render(prefix+label+": ") + styleSuccess.Render(val) + "\n")
		} else {
			sb.WriteString(styleDim.Render(prefix+label+": ") + styleDim.Render(val) + "\n")
		}
	}

	sb.WriteString("\n" + styleDim.Render("  [Enter] next field/submit  [Esc] cancel"))
	return sb.String()
}

func (m model) viewConfirm() string {
	var action string
	switch m.confirm.action {
	case confirmSwitch:
		action = fmt.Sprintf("Switch to %q? This will modify Codex config.", m.confirm.subject)
	case confirmRemove:
		action = fmt.Sprintf("Remove provider %q?", m.confirm.subject)
	}
	return styleWarn.Render("  "+action) + "\n\n  " +
		styleSuccess.Render("[y]") + " yes   " +
		styleError.Render("[n]") + " cancel"
}

func (m model) viewHelp() string {
	if m.mode != modeList {
		return ""
	}
	keys := []string{
		"↑/↓ navigate",
		"a add",
		"e edit",
		"t test",
		"Enter/s switch",
		"d/Del remove",
		"q quit",
	}
	return styleHelp.Render("  " + strings.Join(keys, "  ·  "))
}

// ── Commands ──────────────────────────────────────────────────────────────────

func runTest(name, baseURL, apiKey, model string) tea.Cmd {
	return func() tea.Msg {
		tester := connectivity.New(nil)
		result := tester.Test(baseURL, apiKey, model)
		return testResultMsg{name: name, result: result}
	}
}

func runSwitch(cfg *config.Config, name string) tea.Cmd {
	return func() tea.Msg {
		p, ok := config.GetProvider(cfg, name)
		if !ok {
			return switchDoneMsg{err: fmt.Errorf("provider %q not found", name)}
		}
		adapter := codexadapter.New()
		wireAPI := p.WireAPI
		if wireAPI == "" {
			wireAPI = "responses"
		}
		tc := target.Config{
			BaseURL: p.BaseURL,
			APIKey:  p.APIKey,
			Model:   p.Model,
			WireAPI: wireAPI,
		}
		if err := adapter.Write(&tc); err != nil {
			return switchDoneMsg{err: err}
		}
		if err := config.SetActive(cfg, name); err != nil {
			return switchDoneMsg{err: err}
		}
		return switchDoneMsg{}
	}
}

func runRemove(cfg *config.Config, name string) tea.Cmd {
	return func() tea.Msg {
		if err := config.RemoveProvider(cfg, name); err != nil {
			return removeDoneMsg{err: err}
		}
		return removeDoneMsg{}
	}
}

func submitAdd(values [5]string) tea.Cmd {
	return func() tea.Msg {
		cfg, err := config.Load()
		if err != nil {
			return addDoneMsg{err: err}
		}
		p := config.Provider{
			Name:    strings.TrimSpace(values[0]),
			BaseURL: strings.TrimSpace(values[1]),
			APIKey:  strings.TrimSpace(values[2]),
			Model:   strings.TrimSpace(values[3]),
			Remark:  strings.TrimSpace(values[4]),
			WireAPI: "responses",
		}
		if err := config.AddProvider(cfg, p); err != nil {
			return addDoneMsg{err: err}
		}
		return addDoneMsg{}
	}
}

func submitEdit(oldName string, values [5]string) tea.Cmd {
	return func() tea.Msg {
		cfg, err := config.Load()
		if err != nil {
			return editDoneMsg{err: err}
		}

		name := strings.TrimSpace(values[0])
		baseURL := strings.TrimSpace(values[1])
		apiKey := strings.TrimSpace(values[2])
		model := strings.TrimSpace(values[3])
		remark := strings.TrimSpace(values[4])

		// Validate
		if name == "" {
			return editDoneMsg{err: fmt.Errorf("name cannot be empty")}
		}
		if _, err := url.ParseRequestURI(baseURL); err != nil || !strings.HasPrefix(baseURL, "http") {
			return editDoneMsg{err: fmt.Errorf("invalid base URL")}
		}
		if apiKey == "" {
			return editDoneMsg{err: fmt.Errorf("API key cannot be empty")}
		}
		if model == "" {
			return editDoneMsg{err: fmt.Errorf("model cannot be empty")}
		}

		updated := config.Provider{
			Name:    name,
			BaseURL: baseURL,
			APIKey:  apiKey,
			Model:   model,
			Remark:  remark,
			WireAPI: "responses",
		}

		if err := config.EditProvider(cfg, oldName, updated); err != nil {
			return editDoneMsg{err: err}
		}

		// If the updated provider is active, update Codex config as well
		if cfg.Active == name {
			adapter := codexadapter.New()
			tc := target.Config{
				BaseURL: baseURL,
				APIKey:  apiKey,
				Model:   model,
				WireAPI: "responses",
			}
			if err := adapter.Write(&tc); err != nil {
				return editDoneMsg{err: err}
			}
		}

		return editDoneMsg{}
	}
}

type tickMsg time.Time

func tick() tea.Cmd {
	return tea.Tick(time.Millisecond*250, func(t time.Time) tea.Msg {
		return tickMsg(t)
	})
}

func reloadConfig() tea.Cmd {
	return func() tea.Msg {
		cfg, err := config.Load()
		if err != nil {
			return configReloadedMsg{}
		}
		return configReloadedMsg{cfg: cfg}
	}
}

// ── Helper ────────────────────────────────────────────────────────────────────

// insertAtRune inserts a string into an existing string at rune index idx.
func insertAtRune(s string, idx int, insert string) string {
	runes := []rune(s)
	if idx < 0 {
		idx = 0
	}
	if idx > len(runes) {
		idx = len(runes)
	}
	result := make([]rune, 0, len(runes)+len([]rune(insert)))
	result = append(result, runes[:idx]...)
	result = append(result, []rune(insert)...)
	result = append(result, runes[idx:]...)
	return string(result)
}

// deleteAtRune deletes the character immediately before rune index idx (backspace).
func deleteAtRune(s string, idx int) (string, int) {
	runes := []rune(s)
	if idx <= 0 {
		return s, 0
	}
	if idx > len(runes) {
		idx = len(runes)
	}
	result := make([]rune, 0, len(runes)-1)
	result = append(result, runes[:idx-1]...)
	result = append(result, runes[idx:]...)
	return string(result), idx - 1
}

func truncate(s string, n int) string {
	if len(s) <= n {
		return s
	}
	return s[:n-1] + "…"
}

// Run launches the Bubble Tea TUI.
func Run() error {
	m, err := initialModel()
	if err != nil {
		return err
	}
	p := tea.NewProgram(m, tea.WithAltScreen())
	_, err = p.Run()
	return err
}
