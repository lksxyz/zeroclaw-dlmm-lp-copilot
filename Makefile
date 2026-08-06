# ---- help ----------------------------------------------------------------
.PHONY: help
help: ## list targets
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  \033[36m%-18s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST)

# ---- validate ------------------------------------------------------------
.PHONY: validate
validate: ## parse all TOML, check skill frontmatter, test all plugin crates, typecheck worker
	@./scripts/validate-config.sh
	@./scripts/validate-skills.sh
	@cd plugins/dlmm-core && cargo test --locked --quiet
	@cd plugins/dlmm-reader && cargo test --locked --quiet
	@cd plugins/dlmm-builder && cargo test --locked --quiet
	@cd action-endpoint && npx tsc --noEmit

# ---- plugin --------------------------------------------------------------
.PHONY: plugin plugin-build plugin-test
plugin: ## cargo test on all plugin crates (host, no wasm toolchain)
	cd plugins/dlmm-core && cargo test --locked
	cd plugins/dlmm-reader && cargo test --locked
	cd plugins/dlmm-builder && cargo test --locked

plugin-build: ## build reader + builder for wasm32-wasip2 (needs rustup target)
	rustup target add wasm32-wasip2
	cd plugins/dlmm-reader && cargo build --locked --target wasm32-wasip2 --release
	cp plugins/dlmm-reader/target/wasm32-wasip2/release/dlmm_reader.wasm plugins/dlmm-reader/
	cd plugins/dlmm-builder && cargo build --locked --target wasm32-wasip2 --release
	cp plugins/dlmm-builder/target/wasm32-wasip2/release/dlmm_builder.wasm plugins/dlmm-builder/

plugin-test: ## same as `make plugin`
	$(MAKE) plugin

# ---- fixtures ------------------------------------------------------------
.PHONY: fixtures fixtures-check
fixtures: ## regenerate ground-truth fixtures from independent SDK sources
	cd tools && node gen-fixtures.cjs

fixtures-check: ## regenerate and fail if plugins/dlmm-core/tests/fixtures.json drifts
	cd tools && node gen-fixtures.cjs
	cd plugins/dlmm-core && git diff --exit-code -- tests/fixtures.json

# ---- worker (action endpoint) -------------------------------------------
.PHONY: worker-install worker-dev worker-deploy worker-typecheck worker-test
worker-install: ## npm install in the action-endpoint
	cd action-endpoint && npm install

worker-dev: ## wrangler dev — local worker at http://127.0.0.1:8787
	cd action-endpoint && npx wrangler dev

worker-deploy: ## wrangler deploy — needs CLOUDFLARE_API_TOKEN
	cd action-endpoint && npx wrangler deploy

worker-typecheck: ## tsc --noEmit
	cd action-endpoint && npx tsc --noEmit

worker-test: ## relay tests (node --test)
	cd action-endpoint && npm test

# ---- demo ----------------------------------------------------------------
.PHONY: demo
demo: ## run the end-to-end Devnet demo (see showcase/demo-transcript.md)
	@./scripts/devnet-demo.sh

# ---- submit --------------------------------------------------------------
.PHONY: submit-checklist
submit-checklist: ## print the pre-submit checklist
	@./scripts/submit-checklist.sh
