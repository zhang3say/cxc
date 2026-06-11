package connectivity

import (
	"encoding/json"
	"io"
	"net/http"
	"strings"
	"testing"
)

// mockHTTPClient implements HTTPClient for tests.
type mockHTTPClient struct {
	statusCode int
	body       string
	err        error
}

func (m *mockHTTPClient) Do(req *http.Request) (*http.Response, error) {
	if m.err != nil {
		return nil, m.err
	}
	return &http.Response{
		StatusCode: m.statusCode,
		Body:       io.NopCloser(strings.NewReader(m.body)),
	}, nil
}

func successBody(content string) string {
	resp := chatResponse{
		Choices: []struct {
			Message struct {
				Content string `json:"content"`
			} `json:"message"`
		}{
			{Message: struct {
				Content string `json:"content"`
			}{Content: content}},
		},
	}
	b, _ := json.Marshal(resp)
	return string(b)
}

func errorBody(msg, typ string) string {
	resp := chatResponse{
		Error: &apiError{Message: msg, Type: typ},
	}
	b, _ := json.Marshal(resp)
	return string(b)
}

func TestTestSuccess(t *testing.T) {
	client := &mockHTTPClient{
		statusCode: 200,
		body:       successBody("Hi there!"),
	}
	tester := New(client)
	result := tester.Test("https://example.com/v1", "sk-test", "gpt-4")

	if !result.OK {
		t.Errorf("expected OK=true, got error: %s", result.Error)
	}
	if result.Response != "Hi there!" {
		t.Errorf("unexpected response: %q", result.Response)
	}
	if result.LatencyMS < 0 {
		t.Error("latency should be non-negative")
	}
}

func TestTestTruncatesLongResponse(t *testing.T) {
	longContent := strings.Repeat("x", 100)
	client := &mockHTTPClient{
		statusCode: 200,
		body:       successBody(longContent),
	}
	tester := New(client)
	result := tester.Test("https://example.com/v1", "sk-test", "gpt-4")

	if !result.OK {
		t.Fatalf("expected success, got: %s", result.Error)
	}
	if len(result.Response) > 53 { // 50 ASCII chars + 3-byte UTF-8 ellipsis
		t.Errorf("response not truncated: len=%d", len(result.Response))
	}
}

func TestTestAuthError401(t *testing.T) {
	client := &mockHTTPClient{statusCode: 401, body: `{"error": "unauthorized"}`}
	tester := New(client)
	result := tester.Test("https://example.com/v1", "sk-bad", "gpt-4")

	if result.OK {
		t.Error("expected failure for 401")
	}
	if !strings.Contains(result.Error, "auth") && !strings.Contains(result.Error, "key") {
		t.Errorf("expected auth error message, got: %s", result.Error)
	}
}

func TestTestAuthError403(t *testing.T) {
	client := &mockHTTPClient{statusCode: 403, body: `{"error": "forbidden"}`}
	tester := New(client)
	result := tester.Test("https://example.com/v1", "sk-bad", "gpt-4")

	if result.OK {
		t.Error("expected failure for 403")
	}
}

func TestTestNotFound(t *testing.T) {
	client := &mockHTTPClient{statusCode: 404, body: `not found`}
	tester := New(client)
	result := tester.Test("https://example.com/v1", "sk-test", "gpt-4")

	if result.OK {
		t.Error("expected failure for 404")
	}
	if !strings.Contains(result.Error, "not found") {
		t.Errorf("expected not found message, got: %s", result.Error)
	}
}

func TestTestNetworkError(t *testing.T) {
	client := &mockHTTPClient{err: &networkError{msg: "no such host"}}
	tester := New(client)
	result := tester.Test("https://nonexistent.invalid/v1", "sk-test", "gpt-4")

	if result.OK {
		t.Error("expected failure for network error")
	}
	if !strings.Contains(result.Error, "unreachable") && !strings.Contains(result.Error, "DNS") && !strings.Contains(result.Error, "network") {
		t.Errorf("expected network error message, got: %s", result.Error)
	}
}

func TestTestModelNotFound(t *testing.T) {
	client := &mockHTTPClient{
		statusCode: 200,
		body:       errorBody("model 'bad-model' not found", "invalid_request_error"),
	}
	tester := New(client)
	result := tester.Test("https://example.com/v1", "sk-test", "bad-model")

	if result.OK {
		t.Error("expected failure for model not found")
	}
	if !strings.Contains(strings.ToLower(result.Error), "model") {
		t.Errorf("expected model error message, got: %s", result.Error)
	}
}

func TestRequestConstruction(t *testing.T) {
	var capturedReq *http.Request
	client := &capturingClient{
		inner: &mockHTTPClient{statusCode: 200, body: successBody("hi")},
		capture: func(r *http.Request) {
			capturedReq = r
		},
	}
	tester := New(client)
	tester.Test("https://example.com/v1", "sk-secret", "gpt-4")

	if capturedReq == nil {
		t.Fatal("no request captured")
	}
	if capturedReq.Header.Get("Authorization") != "Bearer sk-secret" {
		t.Errorf("expected Bearer token, got: %s", capturedReq.Header.Get("Authorization"))
	}
	if capturedReq.Header.Get("Content-Type") != "application/json" {
		t.Errorf("expected application/json content-type")
	}
	if !strings.HasSuffix(capturedReq.URL.Path, "/chat/completions") {
		t.Errorf("expected /chat/completions path, got: %s", capturedReq.URL.Path)
	}
}

// networkError is a simple error for testing.
type networkError struct{ msg string }

func (e *networkError) Error() string { return e.msg }

// capturingClient wraps a client and captures the request.
type capturingClient struct {
	inner   HTTPClient
	capture func(*http.Request)
}

func (c *capturingClient) Do(req *http.Request) (*http.Response, error) {
	c.capture(req)
	return c.inner.Do(req)
}
