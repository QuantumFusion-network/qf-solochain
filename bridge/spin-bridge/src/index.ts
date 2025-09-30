import { ApiPromise, WsProvider, SubmittableExtrinsic } from '@polkadot/api';
import { Keyring } from '@polkadot/keyring';
import { AccountId32 } from '@polkadot/types/interfaces';
import { HexString } from '@polkadot/util/types';
import { cryptoWaitReady } from '@polkadot/util-crypto';
import pino from 'pino';

const FASTCHAIN_WS = process.env.FASTCHAIN_WS ?? 'ws://127.0.0.1:9944';
const PARACHAIN_WS = process.env.PARACHAIN_WS ?? 'ws://127.0.0.1:9988';
const BRIDGE_URI = process.env.BRIDGE_URI ?? '//Alice';
const LOG_LEVEL = process.env.LOG_LEVEL ?? 'info';

const logger = pino({ level: LOG_LEVEL });

type AuthorityTuple = [AccountId32, bigint];

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
    if (idA.toHex() !== idB.toHex() || weightA !== weightB) {
      return false;
    }
  }
  return true;
}

function extractAuthorities(list: any[]): AuthorityTuple[] {
  return list.map((tuple: any) => {
    const [id, weight] = tuple as [AccountId32, { toString: () => string }];
    return [id, BigInt(weight.toString())];
  });
}

async function connectApis(): Promise<BridgeApis> {
  const fastchain = await ApiPromise.create({ provider: new WsProvider(FASTCHAIN_WS) });
  const parachain = await ApiPromise.create({ provider: new WsProvider(PARACHAIN_WS) });
  return { fastchain, parachain };
}

function formatAuthoritiesForParachain(authorities: AuthorityTuple[]) {
  return authorities.map(([authorityId, weight]) => [authorityId.toHex(), weight.toString()]);
}

async function signAndSendAndWait(
  api: ApiPromise,
  extrinsic: SubmittableExtrinsic<'promise'>,
  signer: ReturnType<Keyring['addFromUri']>,
  label: string,
) {
  return new Promise<void>((resolve, reject) => {
    extrinsic.signAndSend(signer, { nonce: -1 }, (result) => {
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
    }).catch(reject);
  });
}

async function ensureAuthoritySet(
  api: ApiPromise,
  signer: ReturnType<Keyring['addFromUri']>,
  setId: number,
  authorities: AuthorityTuple[],
) {
  const current = await api.query.spinPolkadot.authoritySet();
  const desired = formatAuthoritiesForParachain(authorities);

  if (current.isSome) {
    const { setId: existingSetId, authorities: existingAuthorities } = current.unwrap();
    const existingTuples = extractAuthorities(existingAuthorities.toArray());
    if (existingSetId.eq(setId) && authorityTuplesEqual(existingTuples, authorities)) {
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

  let currentSetId = (await fastchain.query.grandpa.currentSetId()).toNumber();
  let currentAuthorities = extractAuthorities((await fastchain.query.grandpa.authorityList()).toArray());
  await ensureAuthoritySet(parachain, bridgeAccount, currentSetId, currentAuthorities);

  let pending = Promise.resolve();
  const enqueue = (task: () => Promise<void>) => {
    pending = pending
      .then(task)
      .catch((err) => logger.error({ err }, 'Bridge task failed'));
  };

  await fastchain.queryMulti([
    fastchain.query.grandpa.currentSetId,
    fastchain.query.grandpa.authorityList,
  ], async ([setId, authorityList]) => {
    const newSetId = setId.toNumber();
    const newAuthorities = extractAuthorities(authorityList.toArray());
    const changed =
      newSetId !== currentSetId || !authorityTuplesEqual(newAuthorities, currentAuthorities);
    if (changed) {
      currentSetId = newSetId;
      currentAuthorities = newAuthorities;
      enqueue(() => ensureAuthoritySet(parachain, bridgeAccount, currentSetId, currentAuthorities));
    }
  });

  await fastchain.rpc.grandpa.subscribeJustifications((raw: HexString) => {
    enqueue(async () => {
      const proofBytes = fastchain.registry.createType('Bytes', raw);
      logger.info({ size: proofBytes.length, setId: currentSetId }, 'Forwarding finality proof');
      const tx = parachain.tx.spinPolkadot.submitFinalityProof(currentSetId, proofBytes);
      await signAndSendAndWait(parachain, tx, bridgeAccount, 'submitFinalityProof');
    });
  });
}

main().catch((err) => {
  logger.error({ err }, 'Bridge crashed');
  process.exit(1);
});
