FROM node:22-alpine
RUN yarn install
RUN yarn install && yarn cache clean
RUN yarn cache clean && yarn install
RUN yarn --cwd /app install
RUN yarn --cwd /app install && yarn --cwd /app cache clean
RUN --mount=type=cache,target=/root/.cache/yarn yarn install
RUN --mount=type=tmpfs,target=/root/.cache/yarn yarn install
RUN --mount=type=cache,target=/root/.cache/yarnberry yarn install
RUN npm install
