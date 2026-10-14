FROM node:22-bookworm-slim AS builder
WORKDIR /app
ARG NEXT_PUBLIC_API_PORT=3099
ENV NEXT_PUBLIC_API_PORT=${NEXT_PUBLIC_API_PORT}
COPY package.json package-lock.json ./
RUN npm ci
COPY . .
RUN npm run build

FROM node:22-bookworm-slim
WORKDIR /app
ENV NODE_ENV=production
ENV PORT=3089
ENV HOSTNAME=0.0.0.0
ENV INTERNAL_API_BASE_URL=http://backend:3099
ENV NEXT_PUBLIC_API_PORT=3099
COPY --from=builder /app/.next/standalone ./
COPY --from=builder /app/.next/static ./.next/static
COPY --from=builder /app/public ./public

EXPOSE 3089
CMD ["node", "server.js"]
