package api

import (
	"context"
	"fmt"
	"log/slog"
	"net/http"
	"time"

	"github.com/giosakti/pluto/internal/config"
)

// Server is the HTTP API server.
type Server struct {
	cfg    *config.Config
	logger *slog.Logger
	server *http.Server
}

// NewServer creates a new API server.
func NewServer(cfg *config.Config, logger *slog.Logger) *Server {
	return &Server{
		cfg:    cfg,
		logger: logger,
	}
}

// Run starts the server and blocks until context is cancelled.
func (s *Server) Run(ctx context.Context) error {
	mux := http.NewServeMux()
	s.registerRoutes(mux)

	s.server = &http.Server{
		Addr:         fmt.Sprintf(":%d", s.cfg.Server.Port),
		Handler:      mux,
		ReadTimeout:  time.Duration(s.cfg.Server.ReadTimeout) * time.Second,
		WriteTimeout: time.Duration(s.cfg.Server.WriteTimeout) * time.Second,
	}

	errCh := make(chan error, 1)
	go func() {
		s.logger.Info("starting server", "port", s.cfg.Server.Port)
		if err := s.server.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			errCh <- err
		}
		close(errCh)
	}()

	select {
	case err := <-errCh:
		return fmt.Errorf("server error: %w", err)
	case <-ctx.Done():
		s.logger.Info("shutting down server")
		shutdownCtx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
		defer cancel()
		if err := s.server.Shutdown(shutdownCtx); err != nil {
			return fmt.Errorf("shutdown: %w", err)
		}
		return nil
	}
}

func (s *Server) registerRoutes(mux *http.ServeMux) {
	// Health endpoints
	mux.HandleFunc("GET /livez", s.handleLivez)
	mux.HandleFunc("GET /readyz", s.handleReadyz)
	mux.HandleFunc("GET /version", s.handleVersion)

	// Agent endpoints
	mux.HandleFunc("GET /api/v1/agents", s.handleListAgents)
}
