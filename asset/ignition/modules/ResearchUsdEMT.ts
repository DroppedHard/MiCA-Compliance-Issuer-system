import { buildModule } from "@nomicfoundation/hardhat-ignition/modules";

export default buildModule("ResearchUsdEMTModule", (m) => {
  const admin = m.getAccount(0);
  const caspHotWallet = m.getAccount(2);
  const token = m.contract("ResearchUsdEMT", [admin]);
  const depositRouter = m.contract("CaspDepositRouter", [token, caspHotWallet]);
  return { token, depositRouter };
});
