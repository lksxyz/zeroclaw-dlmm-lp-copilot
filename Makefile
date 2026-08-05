# ---- help ----------------------------------------------------------------
.PHONY: help
help: ## list targets
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  \033[36m%-18s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST)

# ---- validate ------------------------------------------------------------
.PHONY: validate
validate: ## parse all TOML, check skill frontmatter, run plugin tests
	@./scripts/validate-config.sh
	@./scripts/validate-skills.sh
	@cd plugins/dlmm-reader && cargo test --locked --quiet

# ---- plugin --------------------------------------------------------------
.PHONY: plugin plugin-build plugin-test
plugin: ## cargo test on the dlmm-reader plugin (host, no wasm toolchain)
	cd plugins/dlmm-reader && cargo test --locked

plugin-build: ## build dlmm-reader for wasm32-wasip2 (needs rustup target)
	rustup target add wasm32-wasip2
	cd plugins/dlmm-reader && cargo build --locked --target wasm32-wasip2 --release
	cp plugins/dlmm-reader/target/wasm32-wasip2/release/dlmm_reader.wasm plugins/dlmm-reader/

plugin-test: ## same as `make plugin`
	$(MAKE) plugin

# ---- worker (action endpoint) -------------------------------------------
.PHONY: worker-install worker-dev worker-deploy worker-typecheck
worker-install: ## npm install in the action-endpoint
	cd action-endpoint && npm install

worker-dev: ## wrangler dev — local worker at http://127.0.0.1:8787
	cd action-endpoint && npx wrangler dev

worker-deploy: ## wrangler deploy — needs CLOUDFLARE_API_TOKEN
	cd action-endpoint && npx wrangler deploy

worker-typecheck: ## tsc --noEmit
	cd action-endpoint && npx tsc --noEmit

# ---- demo ----------------------------------------------------------------
.PHONY: demo
demo: ## run the end-to-end Devnet demo (see showcase/demo-transcript.md)
	@./scripts/devnet-demo.sh

# ---- submit --------------------------------------------------------------
.PHONY: submit-checklist
submit-checklist: ## print the pre-submit checklist
	@./scripts/submit-checklist.sh
