# syntax=docker/dockerfile:1

FROM node:22 AS builder

WORKDIR /app

RUN \
    --mount=type=bind,source=public,target=public \
    --mount=type=bind,source=src,target=src \
    --mount=type=bind,source=index.html,target=index.html \
    --mount=type=bind,source=package-lock.json,target=package-lock.json \
    --mount=type=bind,source=package.json,target=package.json \
    --mount=type=bind,source=postcss.config.js,target=postcss.config.js \
    --mount=type=bind,source=svelte.config.js,target=svelte.config.js \
    --mount=type=bind,source=tsconfig.json,target=tsconfig.json \
    --mount=type=bind,source=tsconfig.node.json,target=tsconfig.node.json \
    --mount=type=bind,source=vite.config.ts,target=vite.config.ts \
    --mount=type=cache,target=/root/.npm \
    --mount=type=cache,target=/app/node_modules \
    <<EOF
npm install
npm run build
EOF

FROM openresty/openresty

COPY --from=builder /app/dist /usr/local/openresty/nginx/html

COPY nginx.conf /usr/local/openresty/nginx/conf/nginx.conf.template

EXPOSE 80

ADD docker-entrypoint.sh /docker-entrypoint.sh
RUN chmod +x /docker-entrypoint.sh

CMD ["/docker-entrypoint.sh"]
