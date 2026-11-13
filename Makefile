# Platine — developer shortcuts. Run `make help`.
.PHONY: help bootstrap gen rust-check flutter-check check run test-integration

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## ' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  %-18s %s\n", $$1, $$2}'

bootstrap: ## First setup: macOS runner scaffold, pub get, all codegen
	./tool/bootstrap_macos.sh
	cd app && flutter pub get
	./tool/gen_bridge.sh
	cd app && dart run build_runner build --delete-conflicting-outputs

gen: ## Regenerate bridge glue and Riverpod code
	./tool/gen_bridge.sh
	cd app && dart run build_runner build --delete-conflicting-outputs

rust-check: ## fmt + clippy + tests (Rust)
	cd native && cargo fmt --all --check
	cd native && cargo clippy --workspace --all-targets -- -D warnings
	cd native && cargo test --workspace

flutter-check: ## format + analyze + tests (Flutter)
	cd app && dart format --output=none --set-exit-if-changed lib test integration_test
	cd app && flutter analyze --fatal-infos --fatal-warnings
	cd app && flutter test

check: rust-check flutter-check ## Everything the CI checks, except the macOS build

run: ## Launch the app on macOS
	cd app && flutter run -d macos

test-integration: ## Integration tests against the real Rust engine
	cd app && flutter test integration_test -d macos
