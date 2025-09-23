# pallet-spin-anchoring

Secure finality tracking.

Usage example with Polkadot.js. Use in https://portal.qfnetwork.xyz/#/js
```js
function waitForSecureUpTo(targetBlock) {
  return new Promise(async (resolve, reject) => {
    try {
      const unsub = await api.query.spinAnchoring.secureUpTo(async (upTo) => {
        if (upTo.toNumber() >= targetBlock) {
          unsub();
          resolve();
        }
      });
    } catch (e) {
      reject(e);
    }
  });
}

await transfer.signAndSend(ALICE, ({ events = [], status }) => {
    if (status.isFinalized) {
        api.rpc.chain.getHeader(status.asFinalized).then((header) => {
            const includedAt = header.number.toNumber();
            waitForSecureUpTo(includedAt).then(() => {
                console.log("Securely finalized!")
            });
        });
    }
});
```
