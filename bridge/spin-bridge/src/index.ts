import { ApiPromise, WsProvider } from "@polkadot/api";
import type { SubmittableExtrinsic } from "@polkadot/api/types";
import { Keyring } from "@polkadot/keyring";
import { hexToU8a, isHex } from "@polkadot/util";
import { cryptoWaitReady } from "@polkadot/util-crypto";
import pino from "pino";

const FASTCHAIN_WS = process.env.FASTCHAIN_WS ?? "ws://127.0.0.1:9944";
const PARACHAIN_WS = process.env.PARACHAIN_WS ?? "ws://127.0.0.1:9988";
const BRIDGE_URI = process.env.BRIDGE_URI ?? "//Alice";
const LOG_LEVEL = process.env.LOG_LEVEL ?? "info";

const logger = pino({ level: LOG_LEVEL });
const TX_TIMEOUT_MS = Number(process.env.TX_TIMEOUT_MS ?? 120_000);
let shutdownHandler: ((signal?: string) => Promise<void>) | null = null;

type AuthorityTuple = [string, bigint];

type BridgeApis = {
    fastchain: ApiPromise;
    parachain: ApiPromise;
};

type Unsubscribe = () => void;

function normalizeAuthorities(authorities: AuthorityTuple[]): AuthorityTuple[] {
    return [...authorities].sort(([idA], [idB]) => idA.localeCompare(idB));
}

function authorityTuplesEqual(
    a: AuthorityTuple[],
    b: AuthorityTuple[],
): boolean {
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

function decodeAuthorityList(raw: {
    toArray: () => unknown[];
}): AuthorityTuple[] {
    return raw.toArray().map((tuple) => {
        const [id, weight] = tuple as [
            { toHex: () => string },
            { toString: () => string },
        ];
        return [id.toHex(), BigInt(weight.toString())];
    });
}

function normalizeJustificationInput(input: unknown): Uint8Array | null {
    if (!input) {
        return null;
    }
    if (typeof (input as { toU8a?: () => Uint8Array }).toU8a === "function") {
        return (input as { toU8a: () => Uint8Array }).toU8a();
    }
    if (input instanceof Uint8Array) {
        return input;
    }
    if (typeof input === "string" && isHex(input)) {
        return hexToU8a(input);
    }
    return null;
}

function getRpcProvider(api: ApiPromise): {
    subscribe?: (
        section: string,
        method: string,
        params: unknown[],
        cb: (...args: unknown[]) => void,
    ) => Promise<string | number>;
    unsubscribe?: (
        section: string,
        method: string,
        subscriptionId: string | number,
    ) => Promise<boolean>;
    send?: (method: string, params: unknown[]) => Promise<unknown>;
} {
    const withRpcProvider = api as ApiPromise & {
        rpcProvider?: unknown;
        _rpcCore?: { provider?: unknown };
    };
    if (withRpcProvider.rpcProvider) {
        return withRpcProvider.rpcProvider as {
            subscribe?: (
                section: string,
                method: string,
                params: unknown[],
                cb: (...args: unknown[]) => void,
            ) => Promise<string | number>;
            unsubscribe?: (
                section: string,
                method: string,
                subscriptionId: string | number,
            ) => Promise<boolean>;
            send?: (method: string, params: unknown[]) => Promise<unknown>;
        };
    }
    if (withRpcProvider._rpcCore?.provider) {
        return withRpcProvider._rpcCore.provider as {
            subscribe?: (
                section: string,
                method: string,
                params: unknown[],
                cb: (...args: unknown[]) => void,
            ) => Promise<string | number>;
            unsubscribe?: (
                section: string,
                method: string,
                subscriptionId: string | number,
            ) => Promise<boolean>;
            send?: (method: string, params: unknown[]) => Promise<unknown>;
        };
    }
    return {};
}

function extractJustificationPayload(notification: unknown): unknown {
    if (!notification) {
        return notification;
    }
    if (notification instanceof Uint8Array) {
        return notification;
    }
    if (
        typeof notification === "object" &&
        notification !== null &&
        !Array.isArray(notification)
    ) {
        if (
            (notification as { justification?: unknown }).justification !== undefined
        ) {
            return (notification as { justification?: unknown }).justification;
        }
    }
    if (Array.isArray(notification)) {
        return notification[1] ?? notification[0] ?? notification;
    }
    return notification;
}

async function subscribeJustificationStream(
    api: ApiPromise,
    handler: (notification: unknown) => void,
): Promise<Unsubscribe> {
    const decorated = (
        api.rpc as ApiPromise["rpc"] & {
            grandpa?: {
                subscribeJustifications?: (
                    cb: (notification: unknown) => void,
                ) => Promise<Unsubscribe>;
            };
        }
    ).grandpa?.subscribeJustifications;

    if (decorated) {
        return (await decorated(handler)) as unknown as Unsubscribe;
    }

    const provider = getRpcProvider(api);
    if (provider.subscribe && provider.unsubscribe) {
        const subscriptionId = await provider.subscribe(
            "grandpa",
            "subscribeJustifications",
            [],
            handler,
        );
        return () => {
            provider
                .unsubscribe?.(
                    "grandpa",
                    "unsubscribeJustifications",
                    subscriptionId,
                )
                .catch((err) =>
                    logger.warn(
                        { err: formatError(err) },
                        "Failed to unsubscribe from raw justification stream",
                    ),
                );
        };
    }

    throw new Error("grandpa.subscribeJustifications RPC not available");
}

async function fetchFinalityProofBytes(
    api: ApiPromise,
    blockHash: { toHex?: () => string } | string,
): Promise<Uint8Array | null> {
    const proveFinality = (
        api.rpc as ApiPromise["rpc"] & {
            grandpa?: {
                proveFinality?: (hash: unknown) => Promise<{
                    isNone: boolean;
                    unwrap: () => {
                        justification: { toU8a: () => Uint8Array };
                    };
                }>;
            };
        }
    ).grandpa?.proveFinality;

    if (proveFinality) {
        const result = await proveFinality(blockHash);
        if ((result as { isNone?: boolean }).isNone) {
            return null;
        }
        const proof = (
            result as {
                unwrap: () => { justification: { toU8a: () => Uint8Array } };
            }
        ).unwrap();
        return proof.justification.toU8a();
    }

    const provider = getRpcProvider(api);
    if (!provider.send) {
        throw new Error("grandpa.proveFinality RPC not available");
    }

    const hashHex =
        typeof blockHash === "string"
            ? blockHash
            : (blockHash.toHex?.() ?? String(blockHash));
    const raw = (await provider.send("grandpa_proveFinality", [
        hashHex,
    ])) as null | { justification?: unknown };
    if (!raw) {
        return null;
    }

    const candidate = (raw as { justification?: unknown }).justification ?? raw;
    const parsed = normalizeJustificationInput(candidate);
    if (parsed) {
        return parsed;
    }

    try {
        const justification = (candidate as { toU8a?: () => Uint8Array }).toU8a
            ? (candidate as { toU8a: () => Uint8Array }).toU8a()
            : (
                  api.createType("Bytes", candidate) as {
                      toU8a: () => Uint8Array;
                  }
              ).toU8a();
        return justification;
    } catch (err) {
        logger.warn({ err, hash: hashHex }, "Failed to decode finality proof");
        return null;
    }
}

function formatError(error: unknown) {
    if (error instanceof Error) {
        return { message: error.message, stack: error.stack };
    }
    return { message: String(error) };
}

function describeJustification(notification: unknown): unknown {
    if (!notification) {
        return notification;
    }
    if (typeof notification === "string") {
        return notification;
    }
    if (notification instanceof Uint8Array) {
        return `Uint8Array(${notification.length})`;
    }
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
        typeof (notification as { toJSON?: () => unknown }).toJSON === "function"
    ) {
        try {
            return (notification as { toJSON: () => unknown }).toJSON();
        } catch (err) {
            return { error: formatError(err) };
        }
    }
    if (Array.isArray(notification)) {
        return notification.map((item) => describeJustification(item));
    }
    if (
        typeof notification === "object" && notification !== null &&
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

async function fetchAuthorities(api: ApiPromise): Promise<AuthorityTuple[]> {
    const raw = (await api.call.grandpaApi.grandpaAuthorities()) as unknown as {
        toArray: () => unknown[];
    };
    return normalizeAuthorities(decodeAuthorityList(raw));
}

async function connectApis(): Promise<BridgeApis> {
    const fastchain = await ApiPromise.create({
        provider: new WsProvider(FASTCHAIN_WS),
    });
    const parachain = await ApiPromise.create({
        provider: new WsProvider(PARACHAIN_WS),
    });
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
    extrinsic: SubmittableExtrinsic<"promise">,
    signer: ReturnType<Keyring["addFromUri"]>,
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
            current.authorities.map(([idHex, weight]) => [
                idHex,
                BigInt(weight),
            ]),
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
    const { fastchain, parachain } = await connectApis();
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

    const subscriptions: Array<() => void> = [];
    let shuttingDown = false;

    const shutdown = async (signal?: string) => {
        if (shuttingDown) {
            return;
        }
        shuttingDown = true;
        if (signal) {
            logger.info({ signal }, "Received shutdown signal");
        }
        while (subscriptions.length > 0) {
            const unsub = subscriptions.pop();
            if (unsub) {
                try {
                    unsub();
                } catch (err) {
                    logger.warn(
                        { err: formatError(err) },
                        "Failed to unsubscribe",
                    );
                }
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
        (setId: { toString: () => string }) => {
            enqueue(async () => {
                const newSetId = Number(setId.toString());
                const newAuthorities = await fetchAuthorities(fastchain);
                const changed =
                    newSetId !== currentSetId ||
                    !authorityTuplesEqual(newAuthorities, currentAuthorities);
                if (!changed) {
                    return;
                }
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
    )) as unknown as () => void;
    subscriptions.push(unsubSetId);

    let proofSubscriptionEstablished = false;
    try {
        const unsubJustifications = await subscribeJustificationStream(
            fastchain,
            (notification: unknown) => {
                enqueue(async () => {
                    const justification = extractJustificationPayload(notification);
                    const proofU8a = normalizeJustificationInput(justification);
                    if (!proofU8a) {
                        logger.warn(
                            {
                                notification: describeJustification(notification),
                            },
                            "Received justification notification without payload",
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
            (await fastchain.rpc.chain.subscribeFinalizedHeads((header) => {
                enqueue(async () => {
                    if (finalityProofUnavailable) {
                        return;
                    }
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
            })) as unknown as Unsubscribe;
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
