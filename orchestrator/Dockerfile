FROM node:22-alpine
WORKDIR /app
COPY orchestrator/package.json orchestrator/package-lock.json* ./
RUN npm install --production
COPY orchestrator/scheduler.js ./
RUN mkdir -p /data && chown -R node:node /app /data
EXPOSE 3001
HEALTHCHECK --interval=60s --timeout=5s --retries=3 \
  CMD node -e "require('http').get('http://localhost:3001/status',r=>{process.exit(r.statusCode===200?0:1)})"
USER node
CMD ["node", "scheduler.js"]
