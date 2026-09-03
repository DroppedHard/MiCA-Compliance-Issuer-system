export type MintBlockCode =
  | "TOKEN_WIND_DOWN"
  | "TOKEN_PAUSED"
  | "ACTIVITY_THRESHOLD_BLOCK"
  | "RESERVE_COVERAGE_BLOCK";

export type MintStateSnapshot = {
  windDown: boolean;
  paused: boolean;
  activityBlocked: boolean;
  activityEvidence: `0x${string}`;
  reserveState: number;
  reserveEvidence: `0x${string}`;
};

export class MintBlockedError extends Error {
  readonly name = "MintBlockedError";

  constructor(
    readonly code: MintBlockCode,
    readonly reason: string,
    readonly evidenceHash?: `0x${string}`,
    options?: ErrorOptions,
  ) {
    super(
      `Mint rUSD jest zablokowany [${code}]. Powód: ${reason}${evidenceHash === undefined ? "" : ` Dowód on-chain: ${evidenceHash}.`}`,
      options,
    );
  }
}

type MintStateReader = {
  read: {
    windDown: () => Promise<boolean>;
    paused: () => Promise<boolean>;
    issuanceBlocked: () => Promise<boolean>;
    issuanceBlockEvidence: () => Promise<`0x${string}`>;
    reserveState: () => Promise<number>;
    reserveStateEvidence: () => Promise<`0x${string}`>;
  };
};

export async function executeMintWithContext<T>(
  token: MintStateReader,
  mint: () => Promise<T>,
): Promise<T> {
  const before = mintBlockedError(await readSnapshot(token));
  if (before !== undefined) throw before;

  try {
    return await mint();
  } catch (cause) {
    // The state may have changed after the preflight read and before submission.
    const current = mintBlockedError(await readSnapshot(token), cause);
    if (current !== undefined) throw current;
    throw cause;
  }
}

export function mintBlockedError(
  state: MintStateSnapshot,
  cause?: unknown,
): MintBlockedError | undefined {
  if (state.windDown) {
    return new MintBlockedError(
      "TOKEN_WIND_DOWN",
      "token znajduje się w nieodwracalnym stanie wygaszania; dozwolone pozostaje wyłącznie spalanie związane z wykupem",
      undefined,
      { cause },
    );
  }
  if (state.paused) {
    return new MintBlockedError(
      "TOKEN_PAUSED",
      "globalna pauza kontraktu blokuje wszystkie zmiany sald, w tym emisję",
      undefined,
      { cause },
    );
  }
  if (state.activityBlocked) {
    return new MintBlockedError(
      "ACTIVITY_THRESHOLD_BLOCK",
      "emisja została wstrzymana przez kontrolę progów aktywności",
      state.activityEvidence,
      { cause },
    );
  }
  if (state.reserveState === 2) {
    return new MintBlockedError(
      "RESERVE_COVERAGE_BLOCK",
      "stan rezerw nie pozwala na emisję (pokrycie poniżej 100% albo brak wiarygodnych danych o rezerwie)",
      state.reserveEvidence,
      { cause },
    );
  }
  return undefined;
}

async function readSnapshot(token: MintStateReader): Promise<MintStateSnapshot> {
  const [windDown, paused, activityBlocked, activityEvidence, reserveState, reserveEvidence] =
    await Promise.all([
      token.read.windDown(),
      token.read.paused(),
      token.read.issuanceBlocked(),
      token.read.issuanceBlockEvidence(),
      token.read.reserveState(),
      token.read.reserveStateEvidence(),
    ]);
  return {
    windDown,
    paused,
    activityBlocked,
    activityEvidence,
    reserveState: Number(reserveState),
    reserveEvidence,
  };
}
