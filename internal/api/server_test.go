package api

import (
	"io"
	"log/slog"
	"net/http/httptest"
	"testing"

	"github.com/giosakti/pluto/internal/config"
)

func newTestServer(t *testing.T) *Server {
	t.Helper()
	cfg, _ := config.Load("")
	logger := slog.New(slog.NewTextHandler(io.Discard, nil))
	return NewServer(cfg, logger)
}

func newDiscardLogger() *slog.Logger {
	return slog.New(slog.NewTextHandler(io.Discard, nil))
}

func assertJSONContentType(t *testing.T, rec *httptest.ResponseRecorder) {
	t.Helper()
	want := "application/json; charset=utf-8"
	if ct := rec.Header().Get("Content-Type"); ct != want {
		t.Errorf("Content-Type = %q, want %q", ct, want)
	}
}
