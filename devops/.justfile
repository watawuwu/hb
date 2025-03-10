up-all:
    docker compose up

up *args:
    docker compose up {{ args }}

create-cluster:
    kind create cluster --name=perses-dev --config=kind.yaml

delete-cluster:
    kind delete cluster --name=perses-dev

http-server:
    cargo run --release --bin test-server

dev-dashboard:
    docker compose up -d prometheus
    just http-server

