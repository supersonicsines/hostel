.PHONY: dev install uninstall clean version

BINARY = hostel
INSTALL_DIR = $(HOME)/.local/bin

dev:
	cargo build
	./target/debug/localhostel

install:
	cargo build --release
	@mkdir -p $(INSTALL_DIR)
	cp target/release/localhostel $(INSTALL_DIR)/$(BINARY)
	@echo "Installed $(BINARY) to $(INSTALL_DIR)"
	@$(INSTALL_DIR)/$(BINARY) --version
	@echo "Run 'hostel' from anywhere"

uninstall:
	rm -f $(INSTALL_DIR)/$(BINARY)
	@echo "Uninstalled $(BINARY)"

clean:
	cargo clean

version:
	cargo run --quiet -- --version
