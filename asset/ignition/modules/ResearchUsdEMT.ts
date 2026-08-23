import { buildModule } from "@nomicfoundation/hardhat-ignition/modules";

export default buildModule("ResearchUsdEMTModule", (m) => {
  const admin = m.getAccount(0);
  const token = m.contract("ResearchUsdEMT", [admin]);
  return { token };
});
