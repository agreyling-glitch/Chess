ARG COMPANION_IMAGE=ironwood-companion:validation
FROM ${COMPANION_IMAGE} AS companion
FROM node:22-bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*
WORKDIR /validation
COPY deploy/ironwood-server/validation-tools/package*.json ./
RUN npm ci --ignore-scripts --omit=optional --no-audit --no-fund
COPY --from=companion /usr/local/bin/ironwood-server /validation/ironwood-server
COPY tests ./tests
COPY web/*.js ./web/
COPY scripts/tablebase.test.mjs scripts/run-companion-platform-checks.mjs scripts/validate-companion-container-api.mjs ./scripts/
RUN mkdir -p /validation/tmp && chown -R 10001:10001 /validation
USER 10001:10001
ENV COMPANION_VALIDATION_ENVIRONMENT=container-toolbox
ENV COMPANION_BINARY=/validation/ironwood-server
CMD ["node", "scripts/run-companion-platform-checks.mjs"]
