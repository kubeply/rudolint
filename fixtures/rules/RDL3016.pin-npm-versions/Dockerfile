FROM node:22-alpine
RUN npm ci
RUN npm install lodash
RUN npm install lodash@4.17.21
RUN npm install @scope/tool
RUN npm install @scope/tool@1.2.3
RUN npm install --save-dev eslint@9.17.0 prettier
RUN npm install --tag latest lodash@4.17.21
RUN npm --registry https://registry.npmjs.org install lodash@4.17.21
RUN npm --workspace packages/app install lodash@4.17.21
RUN npm --workspaces install lodash
RUN npm --include-workspace-root install lodash@4.17.21
RUN npm --tag install view lodash
RUN echo npm install lodash
