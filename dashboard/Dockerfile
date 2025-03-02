FROM node:22 AS static-builder

WORKDIR /app

RUN \
    --mount=type=bind,source=dashboard/frontend/src,target=src \
    --mount=type=bind,source=dashboard/frontend/index.html,target=index.html \
    --mount=type=bind,source=dashboard/frontend/package-lock.json,target=package-lock.json \
    --mount=type=bind,source=dashboard/frontend/package.json,target=package.json \
    --mount=type=bind,source=dashboard/frontend/postcss.config.js,target=postcss.config.js \
    --mount=type=bind,source=dashboard/frontend/svelte.config.js,target=svelte.config.js \
    --mount=type=bind,source=dashboard/frontend/tailwind.config.js,target=tailwind.config.js \
    --mount=type=bind,source=dashboard/frontend/tsconfig.json,target=tsconfig.json \
    --mount=type=bind,source=dashboard/frontend/tsconfig.node.json,target=tsconfig.node.json \
    --mount=type=bind,source=dashboard/frontend/vite.config.ts,target=vite.config.ts \
    --mount=type=cache,target=/root/.npm \
    --mount=type=cache,target=/app/node_modules \
    <<EOF
npm install
npm run build
EOF

FROM rust AS server-builder

ARG APP_NAME

WORKDIR /app

RUN --mount=type=bind,source=Cargo.toml,target=Cargo.toml \
    --mount=type=bind,source=Cargo.lock,target=Cargo.lock \
    --mount=type=bind,source=client/src,target=client/src \
    --mount=type=bind,source=client/Cargo.toml,target=client/Cargo.toml \
    --mount=type=bind,source=controller/src,target=controller/src \
    --mount=type=bind,source=controller/Cargo.toml,target=controller/Cargo.toml \
    --mount=type=bind,source=devops/src,target=devops/src \
    --mount=type=bind,source=devops/Cargo.toml,target=devops/Cargo.toml \
    --mount=type=bind,source=dashboard/src,target=dashboard/src \
    --mount=type=bind,source=dashboard/Cargo.toml,target=dashboard/Cargo.toml \
    --mount=type=cache,target=/app/target/ \
    --mount=type=cache,target=/usr/local/cargo/registry/ \
    <<EOF
set -e
cargo build --locked --release --bin $APP_NAME
cp ./target/release/$APP_NAME /bin/app
EOF

FROM gcr.io/distroless/cc AS final

COPY --from=static-builder /app/dist /dist
ENV HB_DASHBOARD_DIST_PATH=/dist

COPY --from=server-builder /bin/app /bin/app

ENTRYPOINT ["/bin/app"]
