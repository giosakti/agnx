package buildinfo

import "runtime"

// Build info (injected via -ldflags).
//
// Example:
//
//	go build -ldflags "-X github.com/giosakti/agnx/internal/buildinfo.Version=v0.1.0 -X github.com/giosakti/agnx/internal/buildinfo.Commit=$(git rev-parse --short HEAD) -X github.com/giosakti/agnx/internal/buildinfo.Date=$(date -u +%Y-%m-%dT%H:%M:%SZ)" ./cmd/agnx
var (
	Version = "dev"
	Commit  = "none"
	Date    = "unknown"
)

func GoVersion() string {
	return runtime.Version()
}
