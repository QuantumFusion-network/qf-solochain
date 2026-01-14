import dotenv from 'dotenv';
import { ApiPromise, WsProvider, Keyring } from '@polkadot/api';
import { cryptoWaitReady } from '@polkadot/util-crypto';

dotenv.config({ quiet: true });

const DEFAULT_TIMEOUT_MS = 1000;

/**
 * Loads config from environment variables.
 * 
 * @returns {{apiUrl: string, mnemonic: string, maxAmount: string|number|BigInt, paraId: string|number}}
 */
function getConfig() {
    const apiUrl = process.env.RELAY_WS_URL;
    const timeout = process.env.TIMEOUT_MS || DEFAULT_TIMEOUT_MS;
    const mnemonic = process.env.MNEMONIC;
    const maxAmount = process.env.MAX_AMOUNT;
    const paraId = process.env.PARA_ID;
    const relayBlocksPerParaBlock = parseInt(process.env.RELAY_BLOCKS_PER_PARA_BLOCK || '1', 10);

    if (!apiUrl || !mnemonic || !maxAmount || !paraId) {
        console.error('Missing required env variables.');
        process.exit(1);
    }

    if (!Number.isInteger(relayBlocksPerParaBlock) || relayBlocksPerParaBlock < 1) {
        console.error('RELAY_BLOCKS_PER_PARA_BLOCK must be an integer > 1.');
        process.exit(1);
    }

    return { apiUrl, timeout, mnemonic, maxAmount, paraId, relayBlocksPerParaBlock };
}

// Get config from env and send an on-demand order each block.
(async () => {
    const { apiUrl, timeout, mnemonic, maxAmount, paraId, relayBlocksPerParaBlock } = getConfig();

    await cryptoWaitReady();

    console.log(`Connecting to API at ${apiUrl}...`);
    const wsProvider = new WsProvider(apiUrl, { timeout });
    const api = await ApiPromise.create({ provider: wsProvider });
    await api.isReady;

    const keyring = new Keyring({ type: 'sr25519' });
    const account = keyring.addFromUri(mnemonic);

    let lastBlock = 0;
    let count = 1;

    console.log('Subscribing to new blocks...');
    const unsub = await api.rpc.chain.subscribeNewHeads(async (header) => {
        const blockNumber = header.number.toNumber();
        if (blockNumber === lastBlock) return;
        lastBlock = blockNumber;

        console.log(`${blockNumber} - new relay block, ${count}/${relayBlocksPerParaBlock}...`);
        if (relayBlocksPerParaBlock - count > 0) {
            count++;
            return;
        };
        count = 1;

        try {
            const txHash = await send(api, blockNumber, account, maxAmount, paraId);
            console.log(`${blockNumber} - order sent with tx hash ${txHash}`);
        } catch (err) {
            console.error(`${blockNumber} - error sending the order:`, err);
        }
    });

    const cleanup = async () => {
        try {
            unsub();
            await api.disconnect();
        } catch (e) {
            console.error(e);
        } finally {
            process.exit();
        }
    };

    process.on('SIGINT', cleanup);
    process.on('SIGTERM', cleanup);
})();

/**
 * Sends a placeOrderKeepAlive transaction from the onDemandAssignmentProvider pallet.
 * 
 * @param {ApiPromise} api
 * @param {number} now Current block number
 * @param {object} account
 * @param {string|number|BigInt} maxAmount
 * @param {string|number} paraId
 * @returns {Promise<string>} Transaction hash
 */
async function send(api, now, account, maxAmount, paraId) {
    const tx = (api.tx.onDemand ?? api.tx.onDemandAssignmentProvider).placeOrderKeepAlive(maxAmount, paraId);
    const unsub = await tx.signAndSend(account, ({ status }) => {
        if (status.isInBlock) {
            console.log(`${now} - block order included at blockHash ${status.asInBlock}`);
        } else if (status.isFinalized) {
            console.log(`${now} - block order finalized at blockHash ${status.asFinalized}`);
            unsub();
        }
    });

    return tx.hash.toHex();
}
