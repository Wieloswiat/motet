// The IPLD reference codec enforces canonical DAG-CBOR, the PLC wire format.
import { decode, encode } from '@ipld/dag-cbor';
import { isRepositoryKey } from './repository-key.js';

// The operation crosses an untrusted JSON boundary.
/* oxlint-disable anti-slop/no-unsafe-dictionary-type, anti-slop/no-runtime-typeof */
function record(value: unknown): value is Record<string, unknown> {
	return typeof value === 'object' && value !== null && !Array.isArray(value);
}
/* oxlint-enable anti-slop/no-unsafe-dictionary-type, anti-slop/no-runtime-typeof */

function base64url(bytes: Uint8Array): string {
	let binary = '';

	for (const byte of bytes) binary += String.fromCharCode(byte);

	return btoa(binary).replaceAll('+', '-').replaceAll('/', '_').replace(/=+$/, '');
}

function decodeBase64url(value: string): Uint8Array<ArrayBuffer> {
	if (!/^[A-Za-z0-9_-]+$/.test(value) || value.length % 4 === 1)
		throw new Error('Invalid base64url');

	const binary = atob(value.replaceAll('-', '+').replaceAll('_', '/'));
	const bytes = Uint8Array.from(binary, (character) => character.charCodeAt(0));

	if (base64url(bytes) !== value) throw new Error('Noncanonical base64url');

	return bytes;
}

/* oxlint-disable anti-slop/no-runtime-typeof, anti-slop/no-unknown-parameters */
function sameEndpoint(actual: unknown, trusted: string): boolean {
	if (typeof actual !== 'string') return false;

	try {
		return new URL(actual).href.replace(/\/$/, '') === new URL(trusted).href.replace(/\/$/, '');
	} catch {
		return false;
	}
}
/* oxlint-enable anti-slop/no-runtime-typeof, anti-slop/no-unknown-parameters */

/** Derive the PLC DID from canonical DAG-CBOR of a signed genesis operation. */
// The PLC operation is a DAG-CBOR map supplied by the directory.
/* oxlint-disable anti-slop/no-unsafe-dictionary-type */
export async function didOfSignedGenesis(operation: Record<string, unknown>): Promise<string> {
	const bytes = Uint8Array.from(encode(operation));
	const digest = new Uint8Array(await crypto.subtle.digest('SHA-256', bytes));
	const alphabet = 'abcdefghijklmnopqrstuvwxyz234567';
	let bits = 0;
	let buffer = 0;
	let output = '';

	for (const byte of digest) {
		buffer = (buffer << 8) | byte;
		bits += 8;

		while (bits >= 5 && output.length < 24) {
			bits -= 5;
			output += alphabet[(buffer >>> bits) & 31];
		}

		if (output.length === 24) break;
	}

	return `did:plc:${output}`;
}
/* oxlint-enable anti-slop/no-unsafe-dictionary-type */

/** Validate the reservation before signing; return the signed operation and its PLC DID. */
// The reservation is untrusted until every field and its encoded bytes are checked.
/* oxlint-disable anti-slop/no-unknown-parameters, anti-slop/no-unsafe-dictionary-type */
export async function signGenesis(
	operation: unknown,
	signingBytes: string,
	ownerRotationKey: string,
	handle: string,
	tenorUrl: string,
	sign: (bytes: Uint8Array<ArrayBuffer>) => Promise<string>
): Promise<{ signedOperation: Record<string, unknown>; did: string }> {
	if (
		!record(operation) ||
		operation.type !== 'plc_operation' ||
		operation.prev !== null ||
		'sig' in operation
	)
		throw new Error('Invalid PLC genesis');

	if (
		!Array.isArray(operation.rotationKeys) ||
		operation.rotationKeys.length !== 1 ||
		operation.rotationKeys[0] !== ownerRotationKey
	)
		throw new Error('Invalid PLC rotation keys');

	if (
		!Array.isArray(operation.alsoKnownAs) ||
		operation.alsoKnownAs.length !== 1 ||
		operation.alsoKnownAs[0] !== `at://${handle}`
	)
		throw new Error('Invalid PLC handle');

	if (
		!record(operation.verificationMethods) ||
		!isRepositoryKey(operation.verificationMethods.atproto)
	)
		throw new Error('Invalid PLC signing key');

	if (
		!record(operation.services) ||
		!record(operation.services.atproto_pds) ||
		operation.services.atproto_pds.type !== 'AtprotoPersonalDataServer' ||
		!sameEndpoint(operation.services.atproto_pds.endpoint, tenorUrl)
	)
		throw new Error('Invalid PLC PDS endpoint');

	const bytes = decodeBase64url(signingBytes);
	const canonical = encode(operation);

	if (bytes.length !== canonical.length || !bytes.every((byte, index) => byte === canonical[index]))
		throw new Error('PLC signing bytes do not match operation');

	// Preserve the validated operation even if the caller mutates its input during signing.
	const snapshot: unknown = decode(canonical);

	if (!record(snapshot)) throw new Error('Invalid PLC genesis');

	const sig = await sign(bytes);

	if (decodeBase64url(sig).length !== 64) throw new Error('Invalid PLC signature');

	const signedOperation = { ...snapshot, sig };

	return { signedOperation, did: await didOfSignedGenesis(signedOperation) };
}
/* oxlint-enable anti-slop/no-unknown-parameters, anti-slop/no-unsafe-dictionary-type */
