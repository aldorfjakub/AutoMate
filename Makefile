
help:
	@echo "Targets:"
	@echo "  make up           start Redis (docker compose up -d redis)"
	@echo "  make runner-image build the sandbox image used for matches"
	@echo "  make test         run all Rust tests"
	@echo "  make check        run frontend typecheck (svelte-check)"
	@echo "  make lint         check formatting + clippy lint"
	@echo "  make down         stop Redis"

up:
	docker compose up -d redis

down:
	docker compose down

# The orchestrator launches containers with the image name "python-runner"
runner-image:
	docker build -t python-runner orchestrator/python-runner

test:
	cd backend && cargo test
	cd orchestrator && cargo test

check:
	cd frontend && npm run check

lint:
	cd backend && cargo fmt -- --check && cargo clippy
	cd orchestrator && cargo fmt -- --check && cargo clippy