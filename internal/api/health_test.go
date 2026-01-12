package api

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"
)

func TestHandleLivez(t *testing.T) {
	t.Parallel()

	s := newTestServer(t)
	req := httptest.NewRequest(http.MethodGet, "/livez", nil)
	rec := httptest.NewRecorder()

	s.handleLivez(rec, req)

	if rec.Code != http.StatusOK {
		t.Errorf("status = %d, want %d", rec.Code, http.StatusOK)
	}
	if body := rec.Body.String(); body != "ok" {
		t.Errorf("body = %q, want %q", body, "ok")
	}
}

func TestHandleReadyz(t *testing.T) {
	t.Parallel()

	s := newTestServer(t)
	req := httptest.NewRequest(http.MethodGet, "/readyz", nil)
	rec := httptest.NewRecorder()

	s.handleReadyz(rec, req)

	if rec.Code != http.StatusOK {
		t.Errorf("status = %d, want %d", rec.Code, http.StatusOK)
	}
	if body := rec.Body.String(); body != "ok" {
		t.Errorf("body = %q, want %q", body, "ok")
	}
}

func TestHandleVersion(t *testing.T) {
	t.Parallel()

	s := newTestServer(t)
	req := httptest.NewRequest(http.MethodGet, "/version", nil)
	rec := httptest.NewRecorder()

	s.handleVersion(rec, req)

	if rec.Code != http.StatusOK {
		t.Errorf("status = %d, want %d", rec.Code, http.StatusOK)
	}
	assertJSONContentType(t, rec)

	var resp VersionResponse
	if err := json.NewDecoder(rec.Body).Decode(&resp); err != nil {
		t.Fatalf("decode response: %v", err)
	}

	if resp.Version == "" {
		t.Error("response.Version is empty")
	}
	if resp.Go == "" {
		t.Error("response.Go is empty")
	}
}
