package api

import (
	"net/http"
)

// Agent represents an agent in API responses.
type Agent struct {
	Name        string `json:"name"`
	Description string `json:"description,omitempty"`
	Version     string `json:"version,omitempty"`
}

// ListAgentsResponse is the response for GET /api/v1/agents.
type ListAgentsResponse struct {
	Agents []Agent `json:"agents"`
}

func (s *Server) handleListAgents(w http.ResponseWriter, r *http.Request) {
	// TODO: will be implemented with agent registry
	resp := ListAgentsResponse{
		Agents: []Agent{},
	}
	writeJSON(w, s.logger, http.StatusOK, resp)
}
