import { ApiPromise, WsProvider } from "@polkadot/api";
import type { SubmittableExtrinsic } from "@polkadot/api/types";
import { Keyring } from "@polkadot/keyring";
import { hexToU8a, isHex } from "@polkadot/util";
import type { HexString } from "@polkadot/util/types";
import { cryptoWaitReady } from "@polkadot/util-crypto";
import type {
    ProviderInterface,
    ProviderInterfaceCallback,
} from "@polkadot/rpc-provider/types";
import { Bytes, Option, Struct } from "@polkadot/types-codec";
import type { ITuple } from "@polkadot/types-codec/types";
import type {
    AuthorityId,
    AuthorityWeight,
    AuthorityList,
    Header,
} from "@polkadot/types/interfaces";
import pino from "pino";

const FASTCHAIN_WS = process.env.FASTCHAIN_WS ?? "ws://127.0.0.1:11144";
const PARACHAIN_WS = process.env.PARACHAIN_WS ?? "ws://127.0.0.1:9988";
const BRIDGE_URI = process.env.BRIDGE_URI ?? "//Alice";
const LOG_LEVEL = process.env.LOG_LEVEL ?? "info";
const TX_TIMEOUT_MS = Number(process.env.TX_TIMEOUT_MS ?? 60_000);

const logger = pino({
    base: undefined,
    level: LOG_LEVEL,
    transport: {
        target: "pino-pretty",
        options: {
            colorize: true,
        },
    },
});

let shutdownHandler: ((signal?: string) => Promise<void>) | null = null;

type AuthorityTuple = [authorityIdHex: string, weight: bigint];

type BridgeApis = {
    fastchain: ApiPromise;
    parachain: ApiPromise;
};

type Unsubscribe = () => void;

// ---- helpers: authority comparison ----

function normalizeAuthorities(authorities: AuthorityTuple[]): AuthorityTuple[] {
    return [...authorities].sort(([idA], [idB]) => idA.localeCompare(idB));
}

function authorityTuplesEqual(
    a: AuthorityTuple[],
    b: AuthorityTuple[],
): boolean {
    const A = normalizeAuthorities(a);
    const B = normalizeAuthorities(b);
    if (A.length !== B.length) return false;
    for (let i = 0; i < A.length; i++) {
        if (A[i][0] !== B[i][0] || A[i][1] !== B[i][1]) return false;
    }
    return true;
}

// ---- helpers: GRANDPA authorities decoding (typed) ----

function decodeAuthorityList(raw: AuthorityList): AuthorityTuple[] {
    // AuthorityList = Vec<ITuple<[AuthorityId, AuthorityWeight]>>
    const tuples = raw.toArray() as ITuple<[AuthorityId, AuthorityWeight]>[];
    return tuples.map(([id, weight]) => [
        id.toHex(),
        BigInt(weight.toString()),
    ]);
}

// ---- helpers: provider and justification typing ----

type JustificationLike = Bytes | Uint8Array | HexString;

function pickJustification(input: unknown): JustificationLike | null {
    if (!input) return null;
    if (typeof input === "string") return input as HexString;
    if (input instanceof Uint8Array) return input;
    if (Array.isArray(input)) {
        const [, payload] = input as [unknown, unknown];
        return pickJustification(payload ?? input[0]);
    }
    if (typeof input === "object") {
        const nested = (input as { justification?: unknown }).justification;
        if (nested !== undefined) return pickJustification(nested);
        return input as JustificationLike;
    }
    return null;
}

function justificationToU8a(
    input: JustificationLike | null | undefined,
): Uint8Array | null {
    if (!input) return null;
    if (typeof input === "string") {
        return isHex(input) ? hexToU8a(input) : null;
    }
    if (input instanceof Uint8Array) {
        return input;
    }
    // Bytes
    return (input as Bytes).toU8a();
}

function getRpcProvider(api: ApiPromise): ProviderInterface | null {
    // provider is not part of the public API surface, so we cast narrowly
    const withCore = api as unknown as {
        _rpcCore?: { provider?: ProviderInterface };
    };
    return withCore._rpcCore?.provider ?? null;
}

async function subscribeJustificationStream(
    api: ApiPromise,
    handler: (justification: Bytes | Uint8Array) => void,
): Promise<Unsubscribe> {
    // Preferred, typed path (decorated RPC)
    const decorated = (
        api.rpc.grandpa as unknown as {
            subscribeJustifications?: (
                cb: (j: Bytes) => void,
            ) => Promise<Unsubscribe>;
        }
    ).subscribeJustifications;

    if (decorated) {
        return decorated((j: Bytes) => handler(j));
    }

    // Raw provider fallback (JSON-RPC)
    const provider = getRpcProvider(api);
    if (provider && provider.subscribe && provider.unsubscribe) {
        const onMessage: ProviderInterfaceCallback = (error, result) => {
            if (error) {
                logger.warn(
                    { err: formatError(error) },
                    "Justification raw subscribe error",
                );
                return;
            }
            const u8a = justificationToU8a(pickJustification(result));
            if (u8a) {
                handler(u8a);
            } else {
                logger.warn(
                    { result: describeJustification(result) },
                    "Could not extract justification bytes",
                );
            }
        };

        const subId = await provider.subscribe(
            "grandpa",
            "subscribeJustifications",
            [],
            onMessage,
        );

        return () =>
            provider.unsubscribe!(
                "grandpa",
                "unsubscribeJustifications",
                subId,
            ).catch((err) =>
                logger.warn(
                    { err: formatError(err) },
                    "Failed to unsubscribe from raw justification stream",
                ),
            );
    }

    throw new Error("grandpa.subscribeJustifications RPC not available");
}

async function fetchFinalityProofBytes(
    api: ApiPromise,
    blockHash: HexString | { toHex: () => HexString },
): Promise<Uint8Array | null> {
    const proveFinality = (
        api.rpc.grandpa as unknown as {
            proveFinality: (
                hash: HexString | { toHex: () => HexString },
            ) => Promise<Option<Struct & { justification: Bytes }>>;
        }
    ).proveFinality;

    const res = await proveFinality(blockHash);
    if (res.isNone) return null;
    const unwrapped = res.unwrap();
    return unwrapped.justification.toU8a();
}

// ---- misc helpers ----

function formatError(error: unknown) {
    if (error instanceof Error)
        return { message: error.message, stack: error.stack };
    return { message: String(error) };
}

function describeJustification(notification: unknown): unknown {
    if (!notification) return notification;
    if (typeof notification === "string") return notification;
    if (notification instanceof Uint8Array)
        return `Uint8Array(${notification.length})`;
    if (
        typeof (notification as { toHex?: () => string }).toHex === "function"
    ) {
        try {
            return (notification as { toHex: () => string }).toHex();
        } catch (err) {
            return { error: formatError(err) };
        }
    }
    if (
        typeof (notification as { toJSON?: () => unknown }).toJSON ===
        "function"
    ) {
        try {
            return (notification as { toJSON: () => unknown }).toJSON();
        } catch (err) {
            return { error: formatError(err) };
        }
    }
    if (Array.isArray(notification))
        return notification.map(describeJustification);
    if (
        typeof notification === "object" &&
        notification !== null &&
        !(notification instanceof Error)
    ) {
        const summary: Record<string, unknown> = {};
        for (const [key, value] of Object.entries(notification).slice(0, 10)) {
            summary[key] = describeJustification(value);
        }
        return summary;
    }
    return notification;
}

// ---- chain helpers ----

async function fetchAuthorities(api: ApiPromise): Promise<AuthorityTuple[]> {
    const raw =
        (await api.call.grandpaApi.grandpaAuthorities()) as unknown as AuthorityList;
    return normalizeAuthorities(decodeAuthorityList(raw));
}

function formatAuthoritiesForParachain(authorities: AuthorityTuple[]) {
    return normalizeAuthorities(authorities).map(([authorityId, weight]) => [
        authorityId,
        weight.toString(),
    ]);
}

async function signAndSendAndWait(
    api: ApiPromise,
    extrinsic: SubmittableExtrinsic<"promise">,
    signer: ReturnType<Keyring["addFromUri"]>,
    label: string,
) {
    return new Promise<void>((resolve, reject) => {
        let unsub: (() => void) | undefined;
        const timer = setTimeout(() => {
            if (unsub) unsub();
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
                        const decoded = api.registry.findMetaError(
                            result.dispatchError.asModule,
                        );
                        reject(
                            new Error(
                                `${label} failed: ${decoded.section}.${decoded.name}`,
                            ),
                        );
                    } else {
                        reject(
                            new Error(
                                `${label} failed: ${result.dispatchError.toString()}`,
                            ),
                        );
                    }
                    return;
                }

                if (result.status.isInBlock) {
                    logger.debug(
                        { hash: result.status.asInBlock.toHex() },
                        `${label} included in block`,
                    );
                }

                if (result.status.isFinalized) {
                    if (unsub) {
                        unsub();
                        unsub = undefined;
                    }
                    clearTimeout(timer);
                    logger.info(
                        { hash: result.status.asFinalized.toHex() },
                        `${label} finalized`,
                    );
                    resolve();
                }
            })
            .then((u) => (unsub = u))
            .catch((err) => {
                clearTimeout(timer);
                reject(err);
            });
    });
}

async function ensureAuthoritySet(
    api: ApiPromise,
    signer: ReturnType<Keyring["addFromUri"]>,
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
            current.authorities.map(
                ([idHex, weight]) => [idHex, BigInt(weight)] as AuthorityTuple,
            ),
        );
        if (
            current.setId === setId &&
            authorityTuplesEqual(existingTuples, authorities)
        ) {
            return;
        }
    }

    logger.info(
        { setId, size: authorities.length },
        "Updating parachain authority set",
    );
    const call = api.tx.spinPolkadot.setAuthoritySet(setId, desired);
    const sudoCall = api.tx.sudo.sudo(call);
    await signAndSendAndWait(api, sudoCall, signer, "setAuthoritySet");
}

async function main() {
    await cryptoWaitReady();
    const fastchain = await ApiPromise.create({
        provider: new WsProvider(FASTCHAIN_WS),
    });
    const parachain = await ApiPromise.create({
        provider: new WsProvider(PARACHAIN_WS),
    });
    const keyring = new Keyring({ type: "sr25519" });
    const bridgeAccount = keyring.addFromUri(BRIDGE_URI, {
        name: "spin-bridge",
    });
    logger.info(
        { FASTCHAIN_WS, PARACHAIN_WS, signer: bridgeAccount.address },
        "Bridge starting",
    );

    let currentSetId = Number(
        (await fastchain.query.grandpa.currentSetId()).toString(),
    );
    let currentAuthorities = await fetchAuthorities(fastchain);
    await ensureAuthoritySet(
        parachain,
        bridgeAccount,
        currentSetId,
        currentAuthorities,
    );

    let pending = Promise.resolve();
    const enqueue = (task: () => Promise<void>) => {
        pending = pending
            .then(task)
            .catch((err) =>
                logger.error({ err: formatError(err) }, "Bridge task failed"),
            );
    };

    const subscriptions: Unsubscribe[] = [];
    let shuttingDown = false;

    const shutdown = async (signal?: string) => {
        if (shuttingDown) return;
        shuttingDown = true;
        if (signal) logger.info({ signal }, "Received shutdown signal");

        while (subscriptions.length) {
            const unsub = subscriptions.pop();
            try {
                unsub && unsub();
            } catch (err) {
                logger.warn({ err: formatError(err) }, "Failed to unsubscribe");
            }
        }

        await pending.catch((err) =>
            logger.error(
                { err: formatError(err) },
                "Pending task failed during shutdown",
            ),
        );
        await Promise.allSettled([
            fastchain.disconnect(),
            parachain.disconnect(),
        ]);
    };

    shutdownHandler = shutdown;
    const registerShutdown = (signal: "SIGINT" | "SIGTERM") => {
        process.once(signal, () => {
            shutdown(signal).finally(() => process.exit(0));
        });
    };
    registerShutdown("SIGINT");
    registerShutdown("SIGTERM");

    const unsubSetId = (await fastchain.query.grandpa.currentSetId(
        (setId: { toString(): string }) => {
            enqueue(async () => {
                const newSetId = Number(setId.toString());
                const newAuthorities = await fetchAuthorities(fastchain);
                const changed =
                    newSetId !== currentSetId ||
                    !authorityTuplesEqual(newAuthorities, currentAuthorities);
                if (!changed) return;

                currentSetId = newSetId;
                currentAuthorities = newAuthorities;
                await ensureAuthoritySet(
                    parachain,
                    bridgeAccount,
                    currentSetId,
                    currentAuthorities,
                );
            });
        },
    )) as unknown as Unsubscribe;
    subscriptions.push(unsubSetId);

    let proofSubscriptionEstablished = false;
    try {
        const unsubJustifications = await subscribeJustificationStream(
            fastchain,
            (justification: Bytes | Uint8Array) => {
                enqueue(async () => {
                    const proofU8a = justificationToU8a(justification);
                    if (!proofU8a) {
                        logger.warn(
                            {
                                notification:
                                    describeJustification(justification),
                            },
                            "Empty justification payload",
                        );
                        return;
                    }
                    logger.info(
                        { setId: currentSetId, proofLen: proofU8a.length },
                        "Forwarding finality proof",
                    );
                    const tx = parachain.tx.spinPolkadot.submitFinalityProof(
                        currentSetId,
                        proofU8a,
                    );
                    await signAndSendAndWait(
                        parachain,
                        tx,
                        bridgeAccount,
                        "submitFinalityProof",
                    );
                });
            },
        );
        subscriptions.push(unsubJustifications);
        proofSubscriptionEstablished = true;
        logger.info("Subscribed to GRANDPA justification stream");
    } catch (err) {
        logger.warn(
            { err: formatError(err) },
            "Justification stream unavailable; falling back to proveFinality",
        );
    }

    if (!proofSubscriptionEstablished) {
        if (!fastchain.rpc.chain?.subscribeFinalizedHeads) {
            throw new Error(
                "Finalized heads subscription is unavailable on fastchain RPC",
            );
        }

        let finalityProofUnavailable = false;
        const unsubFinalized =
            (await fastchain.rpc.chain.subscribeFinalizedHeads(
                (header: Header) => {
                    enqueue(async () => {
                        if (finalityProofUnavailable) return;
                        try {
                            const proofU8a = await fetchFinalityProofBytes(
                                fastchain,
                                header.hash,
                            );
                            if (!proofU8a) {
                                logger.warn(
                                    {
                                        block: header.number.toString(),
                                        hash: header.hash.toHex(),
                                    },
                                    "No justification returned for finalized block",
                                );
                                return;
                            }
                            logger.info(
                                {
                                    setId: currentSetId,
                                    proofLen: proofU8a.length,
                                    block: header.number.toString(),
                                },
                                "Forwarding finality proof from proveFinality",
                            );
                            const tx =
                                parachain.tx.spinPolkadot.submitFinalityProof(
                                    currentSetId,
                                    proofU8a,
                                );
                            await signAndSendAndWait(
                                parachain,
                                tx,
                                bridgeAccount,
                                "submitFinalityProof",
                            );
                        } catch (error) {
                            logger.error(
                                { error: formatError(error) },
                                "Failed to forward finality proof for finalized head",
                            );
                            if (
                                !finalityProofUnavailable &&
                                error instanceof Error &&
                                (error.message.includes("Method not found") ||
                                    error.message.includes(
                                        "grandpa.proveFinality RPC not available",
                                    ))
                            ) {
                                finalityProofUnavailable = true;
                                logger.error(
                                    "grandpa_proveFinality RPC not enabled on fastchain; skipping further finality proof attempts",
                                );
                            }
                        }
                    });
                },
            )) as unknown as Unsubscribe;
        subscriptions.push(unsubFinalized);
    }
}

main().catch((err) => {
    logger.error({ err: formatError(err) }, "Bridge crashed");
    const handler = shutdownHandler;
    if (handler) {
        handler().finally(() => process.exit(1));
        return;
    }
    process.exit(1);
});
