import { ApiPromise, WsProvider, Keyring } from '@polkadot/api';
import { cryptoWaitReady } from '@polkadot/util-crypto';

/**
 * Loads config from environment variables.
 * 
 * @returns {{apiUrl: string, mnemonic: string, maxAmount: string|number|BigInt, paraId: string|number}}
 */
function getConfig() {
    const apiUrl = process.env.RELAY_WS_URL;
    const mnemonic = process.env.MNEMONIC;
    const maxAmount = process.env.MAX_AMOUNT;
    const paraId = process.env.PARA_ID;

    if (!apiUrl || !mnemonic || !maxAmount || !paraId) {
        console.error('Missing required env variables.');
        process.exit(1);
    }

    return { apiUrl, mnemonic, maxAmount, paraId };
}

// Get config from env and send an on-demand order each block.
(async () => {
    const { apiUrl, mnemonic, maxAmount, paraId } = getConfig();

    await cryptoWaitReady();

    console.log(`Connecting to API at ${apiUrl}...`);
    const wsProvider = new WsProvider(apiUrl);
    const api = await ApiPromise.create({ wsProvider });
    const keyring = new Keyring({ type: 'sr25519' });
    const account = keyring.addFromUri(mnemonic);

    let lastBlock = 0;

    console.log('Subscribing to new blocks...');
    const unsub = await api.rpc.chain.subscribeNewHeads(async (header) => {
        const blockNumber = header.number.toNumber();
        if (blockNumber === lastBlock) return;
        lastBlock = blockNumber;

        console.log(`${blockNumber} - new relay block`);

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
