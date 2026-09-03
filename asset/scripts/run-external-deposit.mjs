import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const client = process.argv[2] ?? "alice";
const amount = process.argv[3] ?? "100";
const hardhatCli = fileURLToPath(
  new URL("../node_modules/hardhat/dist/src/cli.js", import.meta.url),
);

const result = spawnSync(
  process.execPath,
  [hardhatCli, "run", "scripts/external-deposit.ts", "--network", "localhost"],
  {
    cwd: fileURLToPath(new URL("..", import.meta.url)),
    env: {
      ...process.env,
      CASP_CLIENT_ID: client,
      DEPOSIT_AMOUNT_RUSD: amount,
    },
    stdio: "inherit",
  },
);

if (result.error !== undefined) {
  throw result.error;
}
process.exitCode = result.status ?? 1;
