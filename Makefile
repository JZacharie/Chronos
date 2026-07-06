.PHONY: all build release check test test-watch clean run fmt lint ci-win ci-linux install-desktop

CARGO = cargo
RUSTFLAGS ?=

all: check test build

# ── Build ───────────────────────────────────────────────────────
build:
	$(CARGO) build

release:
	$(CARGO) build --release

# ── Quality ──────────────────────────────────────────────────────
check:
	$(CARGO) check

fmt:
	$(CARGO) fmt --all --check

fmt-fix:
	$(CARGO) fmt --all

lint:
	$(CARGO) clippy --workspace --all-targets -- -D warnings

# ── Test ─────────────────────────────────────────────────────────
test:
	$(CARGO) test

test-watch:
	$(CARGO) watch -x test

# ── Clean ────────────────────────────────────────────────────────
clean:
	$(CARGO) clean
	rm -rf target/

# ── Cross-compilation ────────────────────────────────────────────
ci-win:
	$(CARGO) build --release --target x86_64-pc-windows-gnu

ci-linux:
	./local-ci.sh

# ── Install ──────────────────────────────────────────────────────
install-desktop: release
	@echo "Installing .desktop file..."
	@mkdir -p $(HOME)/.local/share/applications
	@mkdir -p $(HOME)/.local/share/icons/hicolor/scalable/apps
	@cp resources/chronos.svg $(HOME)/.local/share/icons/hicolor/scalable/apps/chronos.svg
	@sed "s|@@EXEC@@|$(shell pwd)/target/release/chronos|" \
		resources/chronos.desktop.in > $(HOME)/.local/share/applications/chronos.desktop
	@update-desktop-database $(HOME)/.local/share/applications 2>/dev/null || true
	@gtk-update-icon-cache $(HOME)/.local/share/icons/hicolor 2>/dev/null || true
	@echo "Done. You may need to log out and back in for changes to take effect."

# ── Run ──────────────────────────────────────────────────────────
run:
	RUST_LOG=info $(CARGO) run

run-release:
	RUST_LOG=info $(CARGO) run --release

# ── Help ─────────────────────────────────────────────────────────
help:
	@echo "Usage: make <target>"
	@echo ""
	@echo "Build:"
	@echo "  all           Default: check + test + build"
	@echo "  build         Debug build"
	@echo "  release       Release build"
	@echo ""
	@echo "Quality:"
	@echo "  check         cargo check"
	@echo "  fmt           Check formatting"
	@echo "  fmt-fix       Fix formatting"
	@echo "  lint          Clippy lint check"
	@echo ""
	@echo "Test:"
	@echo "  test          Run all tests"
	@echo "  test-watch    Auto-rerun tests on change (requires cargo-watch)"
	@echo ""
	@echo "Cross-compile:"
	@echo "  ci-win        Build Windows binary"
	@echo "  ci-linux      Run local CI (fmt + lint + test + build)"
	@echo ""
	@echo "Install:"
	@echo "  install-desktop  Install Linux .desktop file"
	@echo ""
	@echo "Run:"
	@echo "  run           cargo run (debug)"
	@echo "  run-release   cargo run --release"
	@echo ""
	@echo "Clean:"
	@echo "  clean         Remove build artifacts"
