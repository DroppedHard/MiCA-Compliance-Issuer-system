import { buildModule } from "@nomicfoundation/hardhat-ignition/modules";

export default buildModule("ResearchEuroEMTModule", (m) => {
  const admin = m.getAccount(0);
  const token = m.contract("ResearchEuroEMT", [admin]);
  return { token };
});
