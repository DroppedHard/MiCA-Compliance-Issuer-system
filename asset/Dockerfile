FROM node:24-bookworm-slim
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci
COPY . .
# Compile inside the image because host artifacts are intentionally excluded
# from the Docker context. Hardhat Node then receives build-info at startup and
# can label calls to ResearchUsdEMT and CaspDepositRouter in its RPC log.
RUN npx hardhat build
EXPOSE 8545
CMD ["npx", "hardhat", "node", "--hostname", "0.0.0.0"]
