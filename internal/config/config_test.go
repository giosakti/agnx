package config

import (
	"os"
	"path/filepath"
	"testing"
)

func TestLoad_EmptyPath(t *testing.T) {
	t.Parallel()

	cfg, err := Load("")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	// Check defaults are applied.
	if cfg.Server.Port != 8080 {
		t.Errorf("Server.Port = %d, want 8080", cfg.Server.Port)
	}
	if cfg.Server.ReadTimeout != 30 {
		t.Errorf("Server.ReadTimeout = %d, want 30", cfg.Server.ReadTimeout)
	}
	if cfg.Server.WriteTimeout != 30 {
		t.Errorf("Server.WriteTimeout = %d, want 30", cfg.Server.WriteTimeout)
	}
	if cfg.DataDir != "./.pluto" {
		t.Errorf("DataDir = %q, want %q", cfg.DataDir, "./.pluto")
	}
	wantAgentsDir := filepath.Join("./.pluto", "agents")
	if cfg.AgentsDir != wantAgentsDir {
		t.Errorf("AgentsDir = %q, want %q", cfg.AgentsDir, wantAgentsDir)
	}
}

func TestLoad_ValidConfig(t *testing.T) {
	t.Parallel()

	content := `
server:
  port: 9000
  read_timeout: 60
  write_timeout: 120
agents_dir: /custom/agents
data_dir: /custom/data
`
	path := writeTempFile(t, "config.yaml", content)

	cfg, err := Load(path)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if cfg.Server.Port != 9000 {
		t.Errorf("Server.Port = %d, want 9000", cfg.Server.Port)
	}
	if cfg.Server.ReadTimeout != 60 {
		t.Errorf("Server.ReadTimeout = %d, want 60", cfg.Server.ReadTimeout)
	}
	if cfg.Server.WriteTimeout != 120 {
		t.Errorf("Server.WriteTimeout = %d, want 120", cfg.Server.WriteTimeout)
	}
	if cfg.AgentsDir != "/custom/agents" {
		t.Errorf("AgentsDir = %q, want %q", cfg.AgentsDir, "/custom/agents")
	}
	if cfg.DataDir != "/custom/data" {
		t.Errorf("DataDir = %q, want %q", cfg.DataDir, "/custom/data")
	}
}

func TestLoad_PartialConfig(t *testing.T) {
	t.Parallel()

	// Only set port; other fields should use defaults.
	content := `
server:
  port: 3000
`
	path := writeTempFile(t, "partial.yaml", content)

	cfg, err := Load(path)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if cfg.Server.Port != 3000 {
		t.Errorf("Server.Port = %d, want 3000", cfg.Server.Port)
	}
	// Defaults should be applied.
	if cfg.Server.ReadTimeout != 30 {
		t.Errorf("Server.ReadTimeout = %d, want 30", cfg.Server.ReadTimeout)
	}
	if cfg.DataDir != "./.pluto" {
		t.Errorf("DataDir = %q, want %q", cfg.DataDir, "./.pluto")
	}
}

func TestLoad_InvalidYAML(t *testing.T) {
	t.Parallel()

	content := `server: [invalid`
	path := writeTempFile(t, "invalid.yaml", content)

	_, err := Load(path)
	if err == nil {
		t.Fatal("expected error for invalid YAML")
	}
}

func TestLoad_NonExistentFile(t *testing.T) {
	t.Parallel()

	_, err := Load("/nonexistent/config.yaml")
	if err == nil {
		t.Fatal("expected error for non-existent file")
	}
}

func TestLoad_AgentsDirDerivedFromDataDir(t *testing.T) {
	t.Parallel()

	// Set data_dir but not agents_dir; agents_dir should be derived.
	content := `
data_dir: /my/data
`
	path := writeTempFile(t, "derived.yaml", content)

	cfg, err := Load(path)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	want := filepath.Join("/my/data", "agents")
	if cfg.AgentsDir != want {
		t.Errorf("AgentsDir = %q, want %q", cfg.AgentsDir, want)
	}
}

func writeTempFile(t *testing.T, name, content string) string {
	t.Helper()
	dir := t.TempDir()
	path := filepath.Join(dir, name)
	if err := os.WriteFile(path, []byte(content), 0644); err != nil {
		t.Fatalf("write temp file: %v", err)
	}
	return path
}
