# ---------- Build stage ----------
FROM node:20-slim AS builder

RUN corepack enable && corepack prepare pnpm@9 --activate

WORKDIR /app

COPY package.json pnpm-lock.yaml tsconfig.json ./
RUN pnpm install --frozen-lockfile

COPY src ./src
RUN pnpm exec tsc -p .

RUN pnpm prune --prod

# ---------- Runtime stage ----------
FROM node:20-slim AS runner

ENV NODE_ENV=production \
    LOG_LEVEL=info \
    FASTCHAIN_WS=ws://127.0.0.1:11144 \
    PARACHAIN_WS=ws://127.0.0.1:9988 \
    RELAYER_URI=//Alice \
    TX_TIMEOUT_MS=30000

WORKDIR /app
USER node

COPY --from=builder --chown=node:node /app/dist ./dist
COPY --from=builder --chown=node:node /app/node_modules ./node_modules
COPY --chown=node:node package.json ./

CMD ["node", "--enable-source-maps", "dist/index.js"]
