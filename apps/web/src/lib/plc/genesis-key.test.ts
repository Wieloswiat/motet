import { ECDH } from 'node:crypto';
import { encode } from '@ipld/dag-cbor';
import fc from 'fast-check';
import { base58btc } from 'multiformats/bases/base58';
import { describe, expect, it, vi } from 'vitest';
import { signGenesis } from './index.js';

const handle = 'world.motet.test';
const tenor = 'https://tenor.motet.test';
const rotationKey = 'did:key:zQ3shhCGUqDKjStzuDxPkTxN6ujddP4RkEKJJouJGRRkaLGbg';
const compactSignature = Buffer.alloc(64, 1).toString('base64url');
const limit = 2n ** 256n;

const curves = [
	// SEC 2 defines the field sizes; the multicodec table defines the two key tags.
	{
		name: 'prime256v1',
		field: limit - 2n ** 224n + 2n ** 192n + 2n ** 96n - 1n,
		tag: [0x80, 0x24]
	},
	{ name: 'secp256k1', field: limit - 2n ** 32n - 977n, tag: [0xe7, 0x01] }
];

// OpenSSL decodes SEC1 points without relying on Motet's modular arithmetic.
function isPublicPoint(point: string, curve: string): boolean {
	try {
		return ECDH.convertKey(point, curve, 'hex', 'hex', 'compressed') === point;
	} catch {
		return false;
	}
}

const settings: Pick<fc.Parameters, 'numRuns' | 'seed' | 'path'> = { numRuns: 128 };

if (process.env.FC_SEED !== undefined) settings.seed = Number(process.env.FC_SEED);

if (process.env.FC_PATH !== undefined) settings.path = process.env.FC_PATH;

describe('PLC genesis repository keys', () => {
	// A sign request must not bind the World to a point that cannot verify repo commits.
	// Checking only the multicodec and length would sign genesis with off-curve points.
	it.each(curves)(
		'allows signing for exactly the valid compressed $name points',
		async ({ name, field, tag }) => {
			const xCoordinate = fc.oneof(
				fc.bigInt({ min: 0n, max: limit - 1n }),
				fc.constantFrom(0n, 1n, field - 1n, field, field + 1n, limit - 1n)
			);

			await fc.assert(
				fc.asyncProperty(xCoordinate, fc.constantFrom('02', '03'), async (x, prefix) => {
					const point = `${prefix}${x.toString(16).padStart(64, '0')}`;

					const atproto = `did:key:${base58btc.encode(Buffer.concat([Buffer.from(tag), Buffer.from(point, 'hex')]))}`;

					const operation = {
						type: 'plc_operation',
						prev: null,
						rotationKeys: [rotationKey],
						alsoKnownAs: [`at://${handle}`],
						verificationMethods: { atproto },
						services: { atproto_pds: { type: 'AtprotoPersonalDataServer', endpoint: tenor } }
					};

					const signer = vi.fn(async () => compactSignature);

					const result = signGenesis(
						operation,
						Buffer.from(encode(operation)).toString('base64url'),
						rotationKey,
						handle,
						tenor,
						signer
					);

					if (isPublicPoint(point, name)) {
						await expect(result).resolves.toMatchObject({
							signedOperation: { verificationMethods: { atproto } }
						});
						expect(signer).toHaveBeenCalledOnce();
					} else {
						await expect(result).rejects.toThrow('Invalid PLC signing key');
						expect(signer).not.toHaveBeenCalled();
					}
				}),
				{
					...settings,
					examples: [
						[0n, '02'],
						[1n, '02'],
						[field, '03']
					]
				}
			);
		}
	);
});
