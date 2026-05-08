# writer-service Dockerfile (Docker Compose ready)
# Placed in novel-net bridge, reached by n8n at http://writer-service:8787

FROM node:22-alpine

WORKDIR /app

# Only need express + dotenv (no claude-code CLI — server.js calls DeepSeek API directly)
COPY writer-service/package.json writer-service/package-lock.json* ./
RUN npm install --production

COPY writer-service/server.js ./

RUN mkdir -p /data/paper && chown -R node:node /app /data

EXPOSE 8787

HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
  CMD node -e "require('http').get('http://localhost:8787/health',r=>{process.exit(r.statusCode===200?0:1)})"

USER node

ENV WRITER_SERVICE_PROMPTS_DIR=/app/prompts

CMD ["node", "server.js"]
