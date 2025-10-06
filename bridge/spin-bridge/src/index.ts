import { ApiPromise, WsProvider } from '@polkadot/api';
import type { SubmittableExtrinsic } from '@polkadot/api/types';
import { Keyring } from '@polkadot/keyring';
import { cryptoWaitReady } from '@polkadot/util-crypto';
import pino from 'pino';

const FASTCHAIN_WS = process.env.FASTCHAIN_WS ?? 'ws://127.0.0.1:9944';
const PARACHAIN_WS = process.env.PARACHAIN_WS ?? 'ws://127.0.0.1:9988';
const BRIDGE_URI = process.env.BRIDGE_URI ?? '//Alice';
const LOG_LEVEL = process.env.LOG_LEVEL ?? 'info';

const logger = pino({ level: LOG_LEVEL });

type AuthorityTuple = [string, bigint];

type BridgeApis = {
  fastchain: ApiPromise;
  parachain: ApiPromise;
};

function authorityTuplesEqual(a: AuthorityTuple[], b: AuthorityTuple[]): boolean {
  if (a.length !== b.length) {
    return false;
  }
  for (let i = 0; i < a.length; i += 1) {
    const [idA, weightA] = a[i];
    const [idB, weightB] = b[i];
    if (idA !== idB || weightA !== weightB) {
      return false;
    }
  }
  return true;
}

function decodeAuthorityList(
  raw: { toArray: () => unknown[] },
): AuthorityTuple[] {
  return raw.toArray().map((tuple) => {
    const [id, weight] = tuple as [{ toHex: () => string }, { toString: () => string }];
    return [id.toHex(), BigInt(weight.toString())];
  });
}

async function fetchAuthorities(api: ApiPromise): Promise<AuthorityTuple[]> {
  const raw = (await api.call.grandpaApi.grandpaAuthorities()) as unknown as {
    toArray: () => unknown[];
  };
  return decodeAuthorityList(raw);
}

async function connectApis(): Promise<BridgeApis> {
  const fastchain = await ApiPromise.create({ provider: new WsProvider(FASTCHAIN_WS) });
  const parachain = await ApiPromise.create({ provider: new WsProvider(PARACHAIN_WS) });
  return { fastchain, parachain };
}

function formatAuthoritiesForParachain(authorities: AuthorityTuple[]) {
  return authorities.map(([authorityId, weight]) => [authorityId, weight.toString()]);
}

async function signAndSendAndWait(
  api: ApiPromise,
  extrinsic: SubmittableExtrinsic<'promise'>,
  signer: ReturnType<Keyring['addFromUri']>,
  label: string,
) {
  return new Promise<void>((resolve, reject) => {
    extrinsic
      .signAndSend(signer, { nonce: -1 }, (result) => {
        if (result.dispatchError) {
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
          logger.info({ hash: result.status.asFinalized.toHex() }, `${label} finalized`);
          resolve();
        }
      })
      .catch(reject);
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
    const existingTuples: AuthorityTuple[] = current.authorities.map(([idHex, weight]) => [
      idHex,
      BigInt(weight),
    ]);
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

  await fastchain.query.grandpa.currentSetId((setId: { toString: () => string }) => {
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
  });

  await fastchain.rpc.grandpa.subscribeJustifications((notification: any) => {
    enqueue(async () => {
      const justification = notification?.justification ?? notification?.[1];
      if (!justification) {
        logger.warn('Received justification notification without payload');
        return;
      }
      const proofHex = justification.toHex();
      logger.info({ setId: currentSetId, proofLen: justification.length }, 'Forwarding finality proof');
      const tx = parachain.tx.spinPolkadot.submitFinalityProof(currentSetId, proofHex);
      await signAndSendAndWait(parachain, tx, bridgeAccount, 'submitFinalityProof');
    });
  });
}

main().catch((err) => {
  logger.error({ err }, 'Bridge crashed');
  process.exit(1);
});
