import { ApiPromise, WsProvider } from '@polkadot/api';
import type { SubmittableExtrinsic } from '@polkadot/api/types';
import { Keyring } from '@polkadot/keyring';
import { hexToU8a, isHex } from '@polkadot/util';
import { cryptoWaitReady } from '@polkadot/util-crypto';
import pino from 'pino';

const FASTCHAIN_WS = process.env.FASTCHAIN_WS ?? 'ws://127.0.0.1:9944';
const PARACHAIN_WS = process.env.PARACHAIN_WS ?? 'ws://127.0.0.1:9988';
const BRIDGE_URI = process.env.BRIDGE_URI ?? '//Alice';
const LOG_LEVEL = process.env.LOG_LEVEL ?? 'info';

const logger = pino({ level: LOG_LEVEL });
const TX_TIMEOUT_MS = Number(process.env.TX_TIMEOUT_MS ?? 120_000);
let shutdownHandler: ((signal?: string) => Promise<void>) | null = null;

type AuthorityTuple = [string, bigint];

type BridgeApis = {
  fastchain: ApiPromise;
  parachain: ApiPromise;
};

function normalizeAuthorities(authorities: AuthorityTuple[]): AuthorityTuple[] {
  return [...authorities].sort(([idA], [idB]) => idA.localeCompare(idB));
}

function authorityTuplesEqual(a: AuthorityTuple[], b: AuthorityTuple[]): boolean {
  const normalizedA = normalizeAuthorities(a);
  const normalizedB = normalizeAuthorities(b);
  if (normalizedA.length !== normalizedB.length) {
    return false;
  }
  for (let i = 0; i < normalizedA.length; i += 1) {
    const [idA, weightA] = normalizedA[i];
    const [idB, weightB] = normalizedB[i];
    if (idA !== idB || weightA !== weightB) {
      return false;
    }
  }
  return true;
}

function decodeAuthorityList(raw: { toArray: () => unknown[] }): AuthorityTuple[] {
  return raw.toArray().map((tuple) => {
    const [id, weight] = tuple as [{ toHex: () => string }, { toString: () => string }];
    return [id.toHex(), BigInt(weight.toString())];
  });
}

async function fetchAuthorities(api: ApiPromise): Promise<AuthorityTuple[]> {
  const raw = (await api.call.grandpaApi.grandpaAuthorities()) as unknown as {
    toArray: () => unknown[];
  };
  return normalizeAuthorities(decodeAuthorityList(raw));
}

async function connectApis(): Promise<BridgeApis> {
  const fastchain = await ApiPromise.create({ provider: new WsProvider(FASTCHAIN_WS) });
  const parachain = await ApiPromise.create({ provider: new WsProvider(PARACHAIN_WS) });
  return { fastchain, parachain };
}

function formatAuthoritiesForParachain(authorities: AuthorityTuple[]) {
  return normalizeAuthorities(authorities).map(([authorityId, weight]) => [
    authorityId,
    weight.toString(),
  ]);
}

async function signAndSendAndWait(
  api: ApiPromise,
  extrinsic: SubmittableExtrinsic<'promise'>,
  signer: ReturnType<Keyring['addFromUri']>,
  label: string,
) {
  return new Promise<void>((resolve, reject) => {
    let unsub: (() => void) | undefined;
    const timer = setTimeout(() => {
      if (unsub) {
        unsub();
      }
      reject(new Error(`${label} timed out after ${TX_TIMEOUT_MS}ms`));
    }, TX_TIMEOUT_MS);

    extrinsic
      .signAndSend(signer, { nonce: -1 }, (result) => {
        if (result.dispatchError) {
          if (unsub) {
            unsub();
            unsub = undefined;
          }
          clearTimeout(timer);
          if (result.dispatchError.isModule) {
            const decoded = api.registry.findMetaError(result.dispatchError.asModule);
            reject(new Error(`${label} failed: ${decoded.section}.${decoded.name}`));
          } else {
            reject(new Error(`${label} failed: ${result.dispatchError.toString()}`));
          }
          return;
        }

        if (result.status.isInBlock) {
          logger.debug({ hash: result.status.asInBlock.toHex() }, `${label} included in block`);
        }

        if (result.status.isFinalized) {
          if (unsub) {
            unsub();
            unsub = undefined;
          }
          clearTimeout(timer);
          logger.info({ hash: result.status.asFinalized.toHex() }, `${label} finalized`);
          resolve();
        }
      })
      .then((unsubFn) => {
        unsub = unsubFn;
      })
      .catch((err) => {
        clearTimeout(timer);
        reject(err);
      });
  });
}

async function ensureAuthoritySet(
  api: ApiPromise,
  signer: ReturnType<Keyring['addFromUri']>,
  setId: number,
  authorities: AuthorityTuple[],
) {
  const currentRaw = await api.query.spinPolkadot.authoritySet();
  const current = currentRaw.toJSON() as null | {
    setId: number;
    authorities: [string, string][];
  };
  const desired = formatAuthoritiesForParachain(authorities);

  if (current) {
    const existingTuples = normalizeAuthorities(
      current.authorities.map(([idHex, weight]) => [idHex, BigInt(weight)]),
    );
    if (current.setId === setId && authorityTuplesEqual(existingTuples, authorities)) {
      return;
    }
  }

  logger.info({ setId, size: authorities.length }, 'Updating parachain authority set');
  const call = api.tx.spinPolkadot.setAuthoritySet(setId, desired);
  const sudoCall = api.tx.sudo.sudo(call);
  await signAndSendAndWait(api, sudoCall, signer, 'setAuthoritySet');
}

async function main() {
  await cryptoWaitReady();
  const { fastchain, parachain } = await connectApis();
  const keyring = new Keyring({ type: 'sr25519' });
  const bridgeAccount = keyring.addFromUri(BRIDGE_URI, { name: 'spin-bridge' });
  logger.info({ FASTCHAIN_WS, PARACHAIN_WS, signer: bridgeAccount.address }, 'Bridge starting');

  let currentSetId = Number((await fastchain.query.grandpa.currentSetId()).toString());
  let currentAuthorities = await fetchAuthorities(fastchain);
  await ensureAuthoritySet(parachain, bridgeAccount, currentSetId, currentAuthorities);

  let pending = Promise.resolve();
  const enqueue = (task: () => Promise<void>) => {
    pending = pending
      .then(task)
      .catch((err) => logger.error({ err }, 'Bridge task failed'));
  };

  const subscriptions: Array<() => void> = [];
  let shuttingDown = false;

  const shutdown = async (signal?: string) => {
    if (shuttingDown) {
      return;
    }
    shuttingDown = true;
    if (signal) {
      logger.info({ signal }, 'Received shutdown signal');
    }
    while (subscriptions.length > 0) {
      const unsub = subscriptions.pop();
      if (unsub) {
        try {
          unsub();
        } catch (err) {
          logger.warn({ err }, 'Failed to unsubscribe');
        }
      }
    }
    await pending.catch((err) => logger.error({ err }, 'Pending task failed during shutdown'));
    await Promise.allSettled([fastchain.disconnect(), parachain.disconnect()]);
  };

  shutdownHandler = shutdown;

  const registerShutdown = (signal: 'SIGINT' | 'SIGTERM') => {
    process.once(signal, () => {
      shutdown(signal).finally(() => process.exit(0));
    });
  };

  registerShutdown('SIGINT');
  registerShutdown('SIGTERM');

  const unsubSetId = (await fastchain.query.grandpa.currentSetId((setId: { toString: () => string }) => {
    enqueue(async () => {
      const newSetId = Number(setId.toString());
      const newAuthorities = await fetchAuthorities(fastchain);
      const changed =
        newSetId !== currentSetId || !authorityTuplesEqual(newAuthorities, currentAuthorities);
      if (!changed) {
        return;
      }
      currentSetId = newSetId;
      currentAuthorities = newAuthorities;
      await ensureAuthoritySet(parachain, bridgeAccount, currentSetId, currentAuthorities);
    });
  })) as unknown as () => void;
  subscriptions.push(unsubSetId);

  const unsubJustifications = (await fastchain.rpc.grandpa.subscribeJustifications((notification: any) => {
    enqueue(async () => {
      const justification = notification?.justification ?? notification?.[1];
      if (!justification) {
        logger.warn('Received justification notification without payload');
        return;
      }

      let proofU8a: Uint8Array;
      try {
        if (typeof justification.toU8a === 'function') {
          proofU8a = justification.toU8a();
        } else if (justification instanceof Uint8Array) {
          proofU8a = justification;
        } else if (typeof justification === 'string' && isHex(justification)) {
          proofU8a = hexToU8a(justification);
        } else {
          throw new Error('Unsupported justification payload');
        }
      } catch (err) {
        logger.warn({ err }, 'Failed to parse justification payload');
        return;
      }

      logger.info({ setId: currentSetId, proofLen: proofU8a.length }, 'Forwarding finality proof');
      const tx = parachain.tx.spinPolkadot.submitFinalityProof(currentSetId, proofU8a);
      await signAndSendAndWait(parachain, tx, bridgeAccount, 'submitFinalityProof');
    });
  })) as unknown as () => void;
  subscriptions.push(unsubJustifications);
}

main().catch((err) => {
  logger.error({ err }, 'Bridge crashed');
  const handler = shutdownHandler;
  if (handler) {
    handler().finally(() => process.exit(1));
    return;
  }
  process.exit(1);
});
