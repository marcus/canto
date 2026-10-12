# Canto: build, test, and install the CLI and the macOS app.
APP_DIR ?= $(HOME)/Applications/Canto.app
BIN_DIR ?= $(HOME)/.local/bin

.PHONY: build release test run app install install-cli clean

build:
	cargo build --workspace

release:
	cargo build --workspace --release

test:
	cargo test --workspace

run:
	cargo run -p canto-app

# Bundle the release app so it launches from Finder, Spotlight and the Dock.
app: release
	rm -rf "$(APP_DIR)"
	mkdir -p "$(APP_DIR)/Contents/MacOS" "$(APP_DIR)/Contents/Resources"
	cp packaging/Info.plist "$(APP_DIR)/Contents/Info.plist"
	cp target/release/canto-app "$(APP_DIR)/Contents/MacOS/canto-app"
	[ -f packaging/Canto.icns ] && cp packaging/Canto.icns "$(APP_DIR)/Contents/Resources/" || true
	codesign --force --sign - "$(APP_DIR)" >/dev/null
	@echo "installed $(APP_DIR)"

install-cli: release
	mkdir -p "$(BIN_DIR)"
	install -m 755 target/release/canto "$(BIN_DIR)/canto"
	@echo "installed $(BIN_DIR)/canto"

install: app install-cli

clean:
	cargo clean
