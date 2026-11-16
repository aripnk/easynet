set dotenv-load

scope-check:
    bash scripts/check-card-scope.sh

check: scope-check
    cargo fmt --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo nextest run --workspace

lab-up:
    docker compose -f docker/nat-lab.yml up -d --build

lab-test:
    sh docker/run-nat-tests.sh

lab-down:
    docker compose -f docker/nat-lab.yml down -v

deny:
    cargo deny check
