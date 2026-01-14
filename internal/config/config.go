package config

import (
	"fmt"
	"os"
	"path/filepath"

	"gopkg.in/yaml.v3"
)

// Config holds the application configuration.
type Config struct {
	Server    ServerConfig `yaml:"server"`
	AgentsDir string       `yaml:"agents_dir"`
	DataDir   string       `yaml:"data_dir"`
}

// ServerConfig holds HTTP server settings.
type ServerConfig struct {
	Port         int `yaml:"port"`
	ReadTimeout  int `yaml:"read_timeout"`
	WriteTimeout int `yaml:"write_timeout"`
}

// Load reads configuration from the given path.
// If path is empty, returns default configuration.
func Load(path string) (*Config, error) {
	cfg := &Config{}

	if path != "" {
		data, err := os.ReadFile(path)
		if err != nil {
			return nil, fmt.Errorf("read config file: %w", err)
		}

		if err := yaml.Unmarshal(data, cfg); err != nil {
			return nil, fmt.Errorf("parse config file: %w", err)
		}
	}

	applyDefaults(cfg)

	return cfg, nil
}

// applyDefaults sets default values for any zero-valued config fields.
func applyDefaults(cfg *Config) {
	// Server defaults.
	if cfg.Server.Port == 0 {
		cfg.Server.Port = 8080
	}
	if cfg.Server.ReadTimeout == 0 {
		cfg.Server.ReadTimeout = 30
	}
	if cfg.Server.WriteTimeout == 0 {
		cfg.Server.WriteTimeout = 30
	}

	// Data directory default.
	if cfg.DataDir == "" {
		cfg.DataDir = "./.agnx"
	}

	// Derived defaults (depend on other config values).
	if cfg.AgentsDir == "" {
		cfg.AgentsDir = filepath.Join(cfg.DataDir, "agents")
	}
}
