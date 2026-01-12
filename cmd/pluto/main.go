package main

import (
	"context"
	"flag"
	"fmt"
	"log/slog"
	"os"
	"os/signal"
	"syscall"

	"github.com/giosakti/pluto/internal/api"
	"github.com/giosakti/pluto/internal/buildinfo"
	"github.com/giosakti/pluto/internal/config"
)

func main() {
	if err := run(); err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		os.Exit(1)
	}
}

func run() error {
	var (
		configPath  string
		showVersion bool
		port        int
	)

	flag.StringVar(&configPath, "config", "", "path to config file")
	flag.BoolVar(&showVersion, "version", false, "show version")
	flag.IntVar(&port, "port", 8080, "server port")
	flag.Parse()

	if showVersion {
		fmt.Printf(
			"pluto %s (commit %s) built %s (go %s)\n",
			buildinfo.Version,
			buildinfo.Commit,
			buildinfo.Date,
			buildinfo.GoVersion(),
		)
		return nil
	}

	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{
		Level: slog.LevelInfo,
	}))
	slog.SetDefault(logger)

	cfg, err := config.Load(configPath)
	if err != nil {
		return fmt.Errorf("load config: %w", err)
	}

	if port != 8080 {
		cfg.Server.Port = port
	}

	ctx, cancel := signal.NotifyContext(context.Background(), syscall.SIGINT, syscall.SIGTERM)
	defer cancel()

	server := api.NewServer(cfg, logger)

	return server.Run(ctx)
}
