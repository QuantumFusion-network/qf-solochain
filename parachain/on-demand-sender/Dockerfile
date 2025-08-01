# docker build --build-arg NODE_VERSION=$(cat .nvmrc) -t qfnetwork/on-demand-sender:local .
# docker run --rm -it --env-file .env.example --network host qfnetwork/on-demand-sender:local
ARG NODE_VERSION=lts

FROM node:${NODE_VERSION}

WORKDIR /app

COPY package*.json ./
RUN npm install

COPY index.js ./

CMD ["node", "index.js"]
