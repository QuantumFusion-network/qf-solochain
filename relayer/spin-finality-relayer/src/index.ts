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
import type { RegistryTypes } from "@polkadot/types/types";
import type { Compact, u64 } from "@polkadot/types-codec";
import pino from "pino";

const FASTCHAIN_WS = process.env.FASTCHAIN_WS ?? "ws://127.0.0.1:9944";
const PARACHAIN_WS = process.env.PARACHAIN_WS ?? "ws://127.0.0.1:40767";
const FASTCHAIN_SIGNER_URI = process.env.FASTCHAIN_SIGNER_URI ?? "//Bob";
const PARACHAIN_SIGNER_URI = process.env.PARACHAIN_SIGNER_URI ?? "//Alice";
const LOG_LEVEL = process.env.LOG_LEVEL ?? "debug";
const TX_TIMEOUT_MS = Number(process.env.TX_TIMEOUT_MS ?? 60_000);
const API_CONNECT_TIMEOUT_MS = Number(process.env.API_CONNECT_TIMEOUT_MS ?? 10_000);

// Retry knobs (safe defaults)
const TX_RETRY_MAX_ATTEMPTS = Number(process.env.TX_RETRY_MAX_ATTEMPTS ?? 2);
const TX_RETRY_BASE_DELAY_MS = Number(
    process.env.TX_RETRY_BASE_DELAY_MS ?? 1_500,
);
const TX_RETRY_MAX_DELAY_MS = Number(
    process.env.TX_RETRY_MAX_DELAY_MS ?? 20_000,
);
const RECONNECT_BASE_DELAY_MS = Number(
    process.env.RECONNECT_BASE_DELAY_MS ?? 2_000,
);
const RECONNECT_MAX_DELAY_MS = Number(process.env.RECONNECT_MAX_DELAY_MS ?? 30_000);

const logger = pino({
    base: undefined,
    level: LOG_LEVEL,
    transport: {
        target: "pino-pretty",
        options: { colorize: true },
    },
});

let shutdownHandler: ((signal?: string) => Promise<void>) | null = null;
let stopRequested = false;

type AuthorityTuple = [authorityIdHex: string, weight: bigint];
type Unsubscribe = () => void;
type FastchainHeader = Omit<Header, "number"> & { number: Compact<u64> };
type DecodedGrandpaJustification = {
    commit: {
        targetNumber: { toString: () => string };
        targetHash: { toHex: () => string };
        precommits?: { length?: number };
    };
};

// ----------------------------------
// helpers
// ----------------------------------

function sleep(ms: number) {
    return new Promise<void>((res) => setTimeout(res, ms));
}

function withTimeout<T>(
    promise: Promise<T>,
    timeoutMs: number,
    message: string,
): Promise<T> {
    let timer: ReturnType<typeof setTimeout> | null = null;

    const timeout = new Promise<never>((_, reject) => {
        timer = setTimeout(() => reject(new Error(message)), timeoutMs);
    });

    return Promise.race([promise, timeout]).finally(() => {
        if (timer !== null) {
            clearTimeout(timer);
        }
    });
}

function errorMessage(err: unknown): string {
    if (err instanceof Error) return err.message;
    const maybe = err as { message?: unknown };
    if (
        typeof err === "object" &&
        err !== null &&
        typeof maybe.message === "string"
    ) {
        return maybe.message;
    }
    return String(err);
}

function isAuthoritySetMismatch(err: unknown): boolean {
    const msg = errorMessage(err);
    // matches: "submitFinalityProof-XXXX failed: spinPolkadot.AuthoritySetMismatch"
    return msg.includes("AuthoritySetMismatch");
}

function isPriorityTooLow(err: unknown): boolean {
    const msg = errorMessage(err);
    return (
        msg.includes("1014") &&
        (msg.includes("Priority is too low") ||
            msg.includes(
                "too low priority to replace another transaction already in the pool",
            ))
    );
}

function isRetryableRpcError(err: unknown): boolean {
    // Keep retries only for tx-pool replace races.
    // Network/transport errors should restart the whole session instead.
    return isPriorityTooLow(err);
}

async function withRetry<T>(
    label: string,
    fn: () => Promise<T>,
    opts?: {
        maxAttempts?: number;
        baseDelayMs?: number;
        maxDelayMs?: number;
        retryIf?: (err: unknown) => boolean;
    },
): Promise<T> {
    const maxAttempts = opts?.maxAttempts ?? TX_RETRY_MAX_ATTEMPTS;
    const retryIf = opts?.retryIf ?? isRetryableRpcError;

    let attempt = 1;
    let delay = opts?.baseDelayMs ?? TX_RETRY_BASE_DELAY_MS;
    const maxDelay = opts?.maxDelayMs ?? TX_RETRY_MAX_DELAY_MS;

    // eslint-disable-next-line no-constant-condition
    while (true) {
        try {
            return await fn();
        } catch (err) {
            const canRetry = retryIf(err);
            if (!canRetry || attempt >= maxAttempts) {
                throw err;
            }

            logger.warn(
                {
                    attempt,
                    maxAttempts,
                    nextDelayMs: delay,
                    err: formatError(err),
                },
                `${label} failed; retrying`,
            );

            await sleep(delay);
            delay = Math.min(delay * 2, maxDelay);
            attempt += 1;
        }
    }
}

// Serial queue for tx submissions (prevents nonce collisions)
function makeSerialQueue() {
    let tail: Promise<unknown> = Promise.resolve();

    const run = <T>(task: () => Promise<T>): Promise<T> => {
        const next = tail.then(task, task);
        // Keep tail alive even if a task fails
        tail = next.then(
            () => undefined,
            () => undefined,
        );
        return next;
    };

    const drain = () => tail;

    return { run, drain };
}

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
    header: FastchainHeader,
): Promise<Uint8Array | null> {
    const proveFinality = (
        api.rpc.grandpa as unknown as {
            proveFinality: (
                blockNumber: bigint | number | Compact<u64>,
            ) => Promise<Option<Struct & { justification: Bytes }>>;
        }
    ).proveFinality;

    const res = await proveFinality(header.number.unwrap().toBigInt());
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

async function fetchAuthoritiesAt(
    api: ApiPromise,
    atHash: HexString,
): Promise<AuthorityTuple[]> {
    const apiAt = await api.at(atHash);
    const raw =
        (await apiAt.call.grandpaApi.grandpaAuthorities()) as unknown as AuthorityList;
    return normalizeAuthorities(decodeAuthorityList(raw));
}

async function fetchSetIdAt(
    api: ApiPromise,
    atHash: HexString,
): Promise<bigint> {
    const setIdCodec = await api.query.grandpa.currentSetId.at(atHash);
    return BigInt(setIdCodec.toString());
}

function formatAuthoritiesForParachain(authorities: AuthorityTuple[]) {
    return normalizeAuthorities(authorities).map(([authorityId, weight]) => [
        authorityId,
        weight.toString(),
    ]);
}

// Note: explicit nonce + fail-fast on dropped/usurped/invalid
async function signAndSendAndWait(
    api: ApiPromise,
    extrinsic: SubmittableExtrinsic<"promise">,
    signer: ReturnType<Keyring["addFromUri"]>,
    label: string,
) {
    // IMPORTANT: explicit accountNextIndex avoids races better than nonce:-1 in concurrent contexts.
    const nonce = await api.rpc.system.accountNextIndex(signer.address);

    return new Promise<void>((resolve, reject) => {
        let unsub: (() => void) | undefined;

        const clearSub = (logOnError: boolean) => {
            if (!unsub) return;
            try {
                unsub();
            } catch {
                if (logOnError) {
                    logger.error(
                        { label },
                        "Failed to unsubscribe after finalized (probably already unsubscribed)",
                    );
                }
            }
            unsub = undefined;
        };

        const timer = setTimeout(() => {
            clearSub(false);
            reject(
                new Error(
                    `${label} timed out after ${TX_TIMEOUT_MS}ms`,
                ),
            );
        }, TX_TIMEOUT_MS);

        const finish = (err?: Error) => {
            clearSub(false);
            clearTimeout(timer);
            if (err) reject(err);
        };

        extrinsic
            .signAndSend(signer, { nonce }, (result) => {
                if (result.dispatchError) {
                    if (result.dispatchError.isModule) {
                        const decoded = api.registry.findMetaError(
                            result.dispatchError.asModule,
                        );
                        finish(
                            new Error(
                                `${label} failed: ${decoded.section}.${decoded.name}`,
                            ),
                        );
                    } else {
                        finish(
                            new Error(
                                `${label} failed: ${result.dispatchError.toString()}`,
                            ),
                        );
                    }
                    return;
                }

                // Fail-fast statuses (prevents waiting until timeout)
                if (result.status.isInvalid) {
                    finish(new Error(`${label} invalid (tx pool rejected)`));
                    return;
                }
                if (result.status.isDropped) {
                    finish(new Error(`${label} dropped from tx pool`));
                    return;
                }
                if (result.status.isUsurped) {
                    finish(
                        new Error(
                            `${label} usurped (nonce replaced by another tx)`,
                        ),
                    );
                    return;
                }

                if (result.status.isFinalized) {
                    clearTimeout(timer);
                    clearSub(true);
                    logger.info(
                        { hash: result.status.asFinalized.toHex() },
                        `${label} finalized`,
                    );
                    resolve();
                }

                if (result.status.isInBlock) {
                    logger.debug(
                        { hash: result.status.asInBlock.toHex() },
                        `${label} included in block`,
                    );
                }
            })
            .then((u) => (unsub = u))
            .catch((err) =>
                finish(err instanceof Error ? err : new Error(String(err))),
            );
    });
}

async function ensureAuthoritySet(
    api: ApiPromise,
    signer: ReturnType<Keyring["addFromUri"]>,
    setId: bigint,
    authorities: AuthorityTuple[],
) {
    const currentRaw = await api.query.spinPolkadot.fastchainAuthoritySet();
    const current = currentRaw.toJSON() as null | {
        setId: number | string;
        authorities: [string, string][];
    };

    const desired = formatAuthoritiesForParachain(authorities);

    if (current) {
        const currentSetId = BigInt(current.setId);
        const existingTuples = normalizeAuthorities(
            current.authorities.map(
                ([idHex, weight]) => [idHex, BigInt(weight)] as AuthorityTuple,
            ),
        );
        if (
            currentSetId === setId &&
            authorityTuplesEqual(existingTuples, authorities)
        ) {
            return;
        }
    }

    logger.info(
        { setId: setId.toString(), size: authorities.length },
        "Updating parachain authority set",
    );

    const call = api.tx.spinPolkadot.setAuthoritySet(setId.toString(), desired);
    const sudoCall = api.tx.sudo.sudo(call);

    await withRetry(
        `setAuthoritySet-${setId.toString()}`,
        () => signAndSendAndWait(api, sudoCall, signer, "setAuthoritySet"),
        { retryIf: isRetryableRpcError },
    );
}

type Task = () => Promise<void>;

// Runs at most 1 task at a time. While running, only keeps the latest enqueued task.
function makeLatestRunner() {
    let running = false;
    let latest: Task | null = null;

    let drainResolve: (() => void) | null = null;
    let drainPromise: Promise<void> | null = null;

    const ensureDrainPromise = () => {
        if (!drainPromise) {
            drainPromise = new Promise<void>((res) => (drainResolve = res));
        }
        return drainPromise;
    };

    const maybeResolveDrain = () => {
        if (!running && !latest && drainResolve) {
            drainResolve();
            drainResolve = null;
            drainPromise = null;
        }
    };

    const start = () => {
        if (running) return;
        if (!latest) return;
        running = true;
        ensureDrainPromise();

        void (async () => {
            while (latest) {
                const task = latest;
                latest = null;
                try {
                    await task();
                } catch (err) {
                    logger.error(
                        { err: formatError(err) },
                        "Relayer task failed",
                    );
                }
            }
            running = false;
            maybeResolveDrain();
        })();
    };

    const enqueue = (task: Task) => {
        latest = task;
        start();
    };

    const drain = () =>
        running || latest ? ensureDrainPromise() : Promise.resolve();

    return { enqueue, drain };
}

const fastchainCustomTypes = {
    GrandpaJustification: {
        round: "u64",
        commit: "GrandpaCommit",
        votesAncestries: "Vec<Header>",
    },
    GrandpaCommit: {
        targetHash: "H256",
        targetNumber: "u64", // <-- KEY: must be u64, not u32
        precommits: "Vec<GrandpaSignedPrecommit>",
    },
    GrandpaSignedPrecommit: {
        precommit: "GrandpaPrecommit",
        signature: "[u8; 64]", // Ed25519 signature
        id: "[u8; 32]", // AuthorityId
    },
    GrandpaPrecommit: {
        targetHash: "H256",
        targetNumber: "u64", // <-- Also u64
    },
};

type ChainName = "fastchain" | "parachain";

type SessionBreak = {
    chain: ChainName;
    reason: "disconnected" | "error";
    err?: unknown;
};

async function connectApiWithRetry(
    chain: ChainName,
    endpoint: string,
    opts?: { types?: RegistryTypes },
): Promise<ApiPromise> {
    let delay = RECONNECT_BASE_DELAY_MS;

    while (!stopRequested) {
        let api: ApiPromise | null = null;
        const provider = new WsProvider(endpoint);
        try {
            const createOpts: { provider: WsProvider; types?: RegistryTypes } = {
                provider,
            };
            if (opts?.types) {
                createOpts.types = opts.types;
            }

            api = await withTimeout(
                ApiPromise.create(createOpts),
                API_CONNECT_TIMEOUT_MS,
                `${chain} create timed out after ${API_CONNECT_TIMEOUT_MS}ms`,
            );

            await withTimeout(
                api.isReady,
                API_CONNECT_TIMEOUT_MS,
                `${chain} connection timed out after ${API_CONNECT_TIMEOUT_MS}ms`,
            );

            logger.info({ chain, endpoint }, "API connected");
            return api;
        } catch (err) {
            if (api) {
                await api.disconnect().catch(() => undefined);
            } else {
                await provider.disconnect().catch(() => undefined);
            }
            if (stopRequested) {
                throw err;
            }

            logger.warn(
                {
                    chain,
                    endpoint,
                    retryInMs: delay,
                    err: formatError(err),
                },
                "API connection failed; retrying",
            );

            await sleep(delay);
            delay = Math.min(delay * 2, RECONNECT_MAX_DELAY_MS);
        }
    }

    throw new Error(`Stop requested while connecting ${chain}`);
}

function waitForApiBreak(api: ApiPromise, chain: ChainName): Promise<SessionBreak> {
    return new Promise((resolve) => {
        api.once("disconnected", () => {
            resolve({ chain, reason: "disconnected" });
        });
        api.once("error", (err: unknown) => {
            resolve({ chain, reason: "error", err });
        });
    });
}

async function runRelayerSession() {
    await cryptoWaitReady();

    const fastchain = await connectApiWithRetry("fastchain", FASTCHAIN_WS, {
        types: fastchainCustomTypes,
    });
    const parachain = await connectApiWithRetry("parachain", PARACHAIN_WS);

    const keyring = new Keyring({ type: "sr25519" });
    const fastchainAccount = keyring.addFromUri(FASTCHAIN_SIGNER_URI, {
        name: "spin-finality-relayer-fastchain",
    });
    const parachainAccount = keyring.addFromUri(PARACHAIN_SIGNER_URI, {
        name: "spin-finality-relayer-parachain",
    });

    logger.info(
        {
            FASTCHAIN_WS,
            PARACHAIN_WS,
            fastchainSigner: fastchainAccount.address,
            parachainSigner: parachainAccount.address,
        },
        "Relayer starting",
    );

    const proofRunner = makeLatestRunner();

    // Serialize txs per chain/signer to prevent nonce collisions (fixes 1014 pool replacement)
    const parachainTxQ = makeSerialQueue();
    const fastchainTxQ = makeSerialQueue();

    // Cache authorities by setId (authorities are stable within a set)
    const authorityCache = new Map<bigint, AuthorityTuple[]>();

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

        await Promise.allSettled([
            proofRunner.drain(),
            parachainTxQ.drain(),
            fastchainTxQ.drain(),
        ]);

        await Promise.allSettled([
            fastchain.disconnect(),
            parachain.disconnect(),
        ]);

        if (shutdownHandler === shutdown) {
            shutdownHandler = null;
        }
    };

    shutdownHandler = shutdown;

    // Best-effort: prime parachain authority set to current fastchain set.
    // (If it fails transiently, we continue; the next proof will fix it anyway.)
    try {
        const tipSetId = BigInt(
            (await fastchain.query.grandpa.currentSetId()).toString(),
        );
        const tipAuthorities = await fetchAuthorities(fastchain);
        authorityCache.set(tipSetId, tipAuthorities);

        await parachainTxQ.run(async () => {
            await ensureAuthoritySet(
                parachain,
                parachainAccount,
                tipSetId,
                tipAuthorities,
            );
        });
    } catch (err) {
        logger.warn(
            { err: formatError(err) },
            "Initial authority set sync failed (will retry on next proof)",
        );
    }

    // Unified proof forwarding pipeline:
    // - derive setId AT the target block hash
    // - ensure parachain has that authority set
    // - submit proof
    // - then noteAnchorVerified on fastchain
    const forwardProof = async (args: {
        upTo: bigint;
        targetHash: HexString;
        proofU8a: Uint8Array;
    }) => {
        const { upTo, targetHash, proofU8a } = args;

        // Derive the correct setId for THIS proof (fixes AuthoritySetMismatch)
        const setIdAtTarget = await fetchSetIdAt(fastchain, targetHash);

        let authorities = authorityCache.get(setIdAtTarget);
        if (!authorities) {
            authorities = await fetchAuthoritiesAt(fastchain, targetHash);
            authorityCache.set(setIdAtTarget, authorities);
        }

        logger.info(
            {
                upTo: upTo.toString(),
                targetHash,
                setId: setIdAtTarget.toString(),
                proofLen: proofU8a.length,
            },
            "Forwarding finality proof",
        );

        // Everything on parachain must be serialized to avoid nonce collisions
        await parachainTxQ.run(async () => {
            // Ensure authority set matches what this proof requires
            await ensureAuthoritySet(
                parachain,
                parachainAccount,
                setIdAtTarget,
                authorities!,
            );

            const label = `submitFinalityProof-${upTo.toString()}`;
            const sendProof = async (sid: bigint) => {
                const tx = parachain.tx.spinPolkadot.submitFinalityProof(
                    sid.toString(),
                    proofU8a,
                );

                // Diagnostics: compare decoded proof shape in both registries.
                // If these lengths differ or are zero, it's usually an endpoint/type mismatch.
                try {
                    const paraJust = tx.method.args[1] as unknown as {
                        commit?: { precommits?: { length?: number }; targetNumber?: { toString?: () => string } };
                    };
                    const paraPrecommitsLen = paraJust.commit?.precommits?.length ?? -1;
                    const paraTargetNumber = paraJust.commit?.targetNumber?.toString?.();

                    let fastPrecommitsLen = -1;
                    let fastTargetNumber: string | undefined;
                    try {
                        const fastJust = fastchain.registry.createType(
                            "GrandpaJustification",
                            proofU8a,
                        ) as unknown as {
                            commit?: {
                                precommits?: { length?: number };
                                targetNumber?: { toString?: () => string };
                            };
                        };
                        fastPrecommitsLen = fastJust.commit?.precommits?.length ?? -1;
                        fastTargetNumber = fastJust.commit?.targetNumber?.toString?.();
                    } catch (decodeErr) {
                        logger.warn(
                            { err: formatError(decodeErr) },
                            "Failed to decode proof as GrandpaJustification in fastchain registry",
                        );
                    }

                    logger.info(
                        {
                            upTo: upTo.toString(),
                            sid: sid.toString(),
                            paraPrecommitsLen,
                            paraTargetNumber,
                            fastPrecommitsLen,
                            fastTargetNumber,
                        },
                        "Proof decode diagnostics before submitFinalityProof",
                    );
                } catch (diagErr) {
                    logger.warn(
                        { err: formatError(diagErr) },
                        "Failed to build proof diagnostics",
                    );
                }

                await withRetry(
                    label,
                    () =>
                        signAndSendAndWait(
                            parachain,
                            tx,
                            parachainAccount,
                            label,
                        ),
                    { retryIf: isRetryableRpcError },
                );
            };

            try {
                await sendProof(setIdAtTarget);
            } catch (err) {
                if (!isAuthoritySetMismatch(err)) {
                    throw err;
                }

                // Recovery path for edge cases:
                // 1) refresh authorities at targetHash and retry
                logger.warn(
                    {
                        err: formatError(err),
                        setId: setIdAtTarget.toString(),
                        targetHash,
                    },
                    "AuthoritySetMismatch: refreshing authority set at target hash and retrying once",
                );

                const freshSetId = await fetchSetIdAt(fastchain, targetHash);
                const freshAuthorities = await fetchAuthoritiesAt(
                    fastchain,
                    targetHash,
                );
                authorityCache.set(freshSetId, freshAuthorities);

                await ensureAuthoritySet(
                    parachain,
                    parachainAccount,
                    freshSetId,
                    freshAuthorities,
                );

                try {
                    await sendProof(freshSetId);
                    return;
                } catch (err2) {
                    if (!isAuthoritySetMismatch(err2)) throw err2;

                    // 2) last resort: try the parent hash set (transition boundary edge case)
                    const header =
                        await fastchain.rpc.chain.getHeader(targetHash);
                    const parentHash = header.parentHash.toHex() as HexString;

                    logger.warn(
                        { err: formatError(err2), parentHash, targetHash },
                        "AuthoritySetMismatch persists: trying parent hash authority set once",
                    );

                    const parentSetId = await fetchSetIdAt(
                        fastchain,
                        parentHash,
                    );
                    const parentAuthorities = await fetchAuthoritiesAt(
                        fastchain,
                        parentHash,
                    );
                    authorityCache.set(parentSetId, parentAuthorities);

                    await ensureAuthoritySet(
                        parachain,
                        parachainAccount,
                        parentSetId,
                        parentAuthorities,
                    );
                    await sendProof(parentSetId);
                }
            }
        });

        // Fastchain txs also serialized (not strictly necessary here, but safe)
        await fastchainTxQ.run(async () => {
            const label = `noteAnchorVerified-${upTo.toString()}`;
            const tx = fastchain.tx.spinAnchoring.noteAnchorVerified(
                upTo.toString(),
            );
            await withRetry(
                label,
                () =>
                    signAndSendAndWait(fastchain, tx, fastchainAccount, label),
                { retryIf: isRetryableRpcError },
            );
        });
    };

    // Prefer justification stream. If unavailable, fallback to finalized heads + proveFinality.
    try {
        const unsubJustifications = await subscribeJustificationStream(
            fastchain,
            (justification: Bytes | Uint8Array) => {
                const proofU8a = justificationToU8a(justification);
                if (!proofU8a) {
                    logger.warn(
                        { notification: describeJustification(justification) },
                        "Empty justification payload",
                    );
                    return;
                }

                let typedJustification: DecodedGrandpaJustification;
                try {
                    typedJustification = fastchain.registry.createType(
                        "GrandpaJustification",
                        justification,
                    ) as unknown as DecodedGrandpaJustification;
                } catch (err) {
                    logger.warn(
                        {
                            err: formatError(err),
                            notification: describeJustification(justification),
                        },
                        "Failed to decode GrandpaJustification",
                    );
                    return;
                }

                const upTo = BigInt(
                    typedJustification.commit.targetNumber.toString(),
                );
                const targetHash =
                    typedJustification.commit.targetHash.toHex() as HexString;

                logger.info({ upTo: upTo.toString() }, "upTo");

                proofRunner.enqueue(async () => {
                    await forwardProof({ upTo, targetHash, proofU8a });
                });
            },
        );

        subscriptions.push(unsubJustifications);
        logger.info("Subscribed to GRANDPA justification stream");
    } catch (err) {
        logger.warn(
            { err: formatError(err) },
            "Justification stream unavailable; falling back to finalized heads + proveFinality",
        );
        const unsubHeads = (await fastchain.rpc.chain.subscribeFinalizedHeads(
            (header: FastchainHeader) => {
                proofRunner.enqueue(async () => {
                    const upTo = BigInt(header.number.toString());
                    const targetHash = (
                        await fastchain.rpc.chain.getBlockHash(
                            header.number.unwrap().toBigInt(),
                        )
                    ).toHex() as HexString;

                    const proof = await fetchFinalityProofBytes(
                        fastchain,
                        header,
                    );
                    if (!proof) return;

                    logger.info(
                        { upTo: upTo.toString() },
                        "upTo (proveFinality)",
                    );
                    await forwardProof({ upTo, targetHash, proofU8a: proof });
                });
            },
        )) as unknown as Unsubscribe;

        subscriptions.push(unsubHeads);
        logger.info("Subscribed to finalized heads (proveFinality fallback)");
    }

    const sessionBreak = await Promise.race([
        waitForApiBreak(fastchain, "fastchain"),
        waitForApiBreak(parachain, "parachain"),
    ]);

    if (!shuttingDown && !stopRequested) {
        logger.warn(
            {
                chain: sessionBreak.chain,
                reason: sessionBreak.reason,
                err: sessionBreak.err ? formatError(sessionBreak.err) : undefined,
            },
            "API connection lost; restarting relayer session",
        );
    }

    await shutdown();

    if (!stopRequested) {
        throw new Error(
            `${sessionBreak.chain} connection ${sessionBreak.reason}; restarting session`,
        );
    }
}

async function main() {
    const registerShutdown = (signal: "SIGINT" | "SIGTERM") => {
        process.once(signal, () => {
            stopRequested = true;
            const handler = shutdownHandler;
            if (!handler) {
                process.exit(0);
                return;
            }
            handler(signal).finally(() => process.exit(0));
        });
    };
    registerShutdown("SIGINT");
    registerShutdown("SIGTERM");

    let restartDelayMs = RECONNECT_BASE_DELAY_MS;
    while (!stopRequested) {
        try {
            await runRelayerSession();
            restartDelayMs = RECONNECT_BASE_DELAY_MS;
        } catch (err) {
            if (stopRequested) break;
            logger.warn(
                {
                    retryInMs: restartDelayMs,
                    err: formatError(err),
                },
                "Relayer session stopped; restarting",
            );
            await sleep(restartDelayMs);
            restartDelayMs = Math.min(restartDelayMs * 2, RECONNECT_MAX_DELAY_MS);
        }
    }
}

main().catch((err) => {
    logger.error({ err: formatError(err) }, "Relayer crashed");
    const handler = shutdownHandler;
    if (handler) {
        handler().finally(() => process.exit(1));
        return;
    }
    process.exit(1);
});
