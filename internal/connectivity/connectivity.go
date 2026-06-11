// Package connectivity implements the Provider Connectivity Test.
// It sends a real chat completion request to verify a Provider endpoint.
package connectivity

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strings"
	"time"
)

// HTTPClient is an interface so tests can inject a mock.
type HTTPClient interface {
	Do(req *http.Request) (*http.Response, error)
}

// Result holds the outcome of a Connectivity Test.
type Result struct {
	OK        bool
	LatencyMS int64
	Response  string // truncated model response on success
	Error     string // categorized error on failure
}

// Tester performs Connectivity Tests against a Provider endpoint.
type Tester struct {
	client HTTPClient
}

// New returns a Tester with the given HTTP client.
// Pass nil to use the default client with a 30s timeout.
func New(client HTTPClient) *Tester {
	if client == nil {
		client = &http.Client{Timeout: 30 * time.Second}
	}
	return &Tester{client: client}
}

type chatRequest struct {
	Model    string    `json:"model"`
	Messages []message `json:"messages"`
}

type message struct {
	Role    string `json:"role"`
	Content string `json:"content"`
}

type chatResponse struct {
	Choices []struct {
		Message struct {
			Content string `json:"content"`
		} `json:"message"`
	} `json:"choices"`
	Error *apiError `json:"error,omitempty"`
}

type apiError struct {
	Message string `json:"message"`
	Type    string `json:"type"`
	Code    any    `json:"code"`
}

// Test sends a minimal chat completion request and returns a Result.
func (t *Tester) Test(baseURL, apiKey, model string) Result {
	url := strings.TrimRight(baseURL, "/") + "/chat/completions"

	body, _ := json.Marshal(chatRequest{
		Model:    model,
		Messages: []message{{Role: "user", Content: "say hi"}},
	})

	req, err := http.NewRequest(http.MethodPost, url, bytes.NewReader(body))
	if err != nil {
		return Result{Error: fmt.Sprintf("building request: %v", err)}
	}
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Authorization", "Bearer "+apiKey)

	start := time.Now()
	resp, err := t.client.Do(req)
	latency := time.Since(start).Milliseconds()

	if err != nil {
		return Result{LatencyMS: latency, Error: categorizeNetworkError(err)}
	}
	defer resp.Body.Close()

	rawBody, _ := io.ReadAll(resp.Body)

	switch resp.StatusCode {
	case http.StatusUnauthorized, http.StatusForbidden:
		return Result{LatencyMS: latency, Error: "invalid API key (auth error)"}
	case http.StatusNotFound:
		return Result{LatencyMS: latency, Error: "endpoint not found — check base_url"}
	}

	var chatResp chatResponse
	if err := json.Unmarshal(rawBody, &chatResp); err != nil {
		return Result{LatencyMS: latency, Error: fmt.Sprintf("invalid response (status %d): %s", resp.StatusCode, truncate(string(rawBody), 100))}
	}

	if chatResp.Error != nil {
		return Result{LatencyMS: latency, Error: categorizeAPIError(chatResp.Error)}
	}

	if resp.StatusCode != http.StatusOK {
		return Result{LatencyMS: latency, Error: fmt.Sprintf("unexpected status %d: %s", resp.StatusCode, truncate(string(rawBody), 100))}
	}

	if len(chatResp.Choices) == 0 {
		return Result{LatencyMS: latency, Error: "no choices returned"}
	}

	content := chatResp.Choices[0].Message.Content
	return Result{
		OK:        true,
		LatencyMS: latency,
		Response:  truncate(content, 50),
	}
}

func categorizeNetworkError(err error) string {
	msg := err.Error()
	switch {
	case strings.Contains(msg, "no such host") || strings.Contains(msg, "dial"):
		return "unreachable — DNS failure or connection refused"
	case strings.Contains(msg, "timeout") || strings.Contains(msg, "deadline exceeded"):
		return "timeout — server did not respond within 30s"
	case strings.Contains(msg, "connection refused"):
		return "connection refused"
	default:
		return fmt.Sprintf("network error: %v", err)
	}
}

func categorizeAPIError(e *apiError) string {
	msg := strings.ToLower(e.Message)
	switch {
	case strings.Contains(msg, "model") && (strings.Contains(msg, "not found") || strings.Contains(msg, "does not exist")):
		return fmt.Sprintf("model not found: %s", e.Message)
	case strings.Contains(msg, "auth") || strings.Contains(msg, "key") || strings.Contains(msg, "credential"):
		return fmt.Sprintf("auth error: %s", e.Message)
	default:
		return fmt.Sprintf("API error: %s", e.Message)
	}
}

func truncate(s string, n int) string {
	if len(s) <= n {
		return s
	}
	return s[:n] + "…"
}
