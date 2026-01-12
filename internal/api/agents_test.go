package api

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"
)

func TestHandleListAgents(t *testing.T) {
	t.Parallel()

	s := newTestServer(t)
	req := httptest.NewRequest(http.MethodGet, "/api/v1/agents", nil)
	rec := httptest.NewRecorder()

	s.handleListAgents(rec, req)

	if rec.Code != http.StatusOK {
		t.Errorf("status = %d, want %d", rec.Code, http.StatusOK)
	}
	assertJSONContentType(t, rec)

	var resp ListAgentsResponse
	if err := json.NewDecoder(rec.Body).Decode(&resp); err != nil {
		t.Fatalf("decode response: %v", err)
	}

	if resp.Agents == nil {
		t.Error("response.Agents is nil")
	}
	if len(resp.Agents) != 0 {
		t.Errorf("agents length = %d, want 0", len(resp.Agents))
	}
}
