import { ECDH } from 'node:crypto';
import { encode } from '@ipld/dag-cbor';
import fc from 'fast-check';
import { base58btc } from 'multiformats/bases/base58';
import { describe, expect, it, vi } from 'vitest';
import { signGenesis } from './index.js';
import { isRepositoryKey } from './repository-key.js';

const handle = 'world.motet.test';
const tenor = 'https://tenor.motet.test';
const rotationKey = 'did:key:zQ3shhCGUqDKjStzuDxPkTxN6ujddP4RkEKJJouJGRRkaLGbg';
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

function repositoryDidKey(tag: readonly number[], point: string): string {
	return `did:key:${base58btc.encode(Buffer.concat([Buffer.from(tag), Buffer.from(point, 'hex')]))}`;
}

describe('PLC genesis repository keys', () => {
	// Checking only the multicodec and length would accept off-curve points.
	it.each(curves)('accepts exactly the valid compressed $name points', ({ name, field, tag }) => {
		const xCoordinate = fc.oneof(
			fc.bigInt({ min: 0n, max: limit - 1n }),
			fc.constantFrom(0n, 1n, field - 1n, field, field + 1n, limit - 1n)
		);

		fc.assert(
			fc.property(xCoordinate, fc.constantFrom('02', '03'), (x, prefix) => {
				const point = `${prefix}${x.toString(16).padStart(64, '0')}`;

				expect(isRepositoryKey(repositoryDidKey(tag, point))).toBe(isPublicPoint(point, name));
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
	});

	// Validation must gate the signer, or it could bind a World to a key that cannot verify commits.
	it('refuses a well-formed off-curve key before asking the owner to sign', async () => {
		// x=0 does not have a corresponding y on secp256k1.
		const atproto = repositoryDidKey([0xe7, 0x01], `02${'00'.repeat(32)}`);

		const operation = {
			type: 'plc_operation',
			prev: null,
			rotationKeys: [rotationKey],
			alsoKnownAs: [`at://${handle}`],
			verificationMethods: { atproto },
			services: { atproto_pds: { type: 'AtprotoPersonalDataServer', endpoint: tenor } }
		};

		const signer = vi.fn(async () => 'unused');

		await expect(
			signGenesis(
				operation,
				Buffer.from(encode(operation)).toString('base64url'),
				rotationKey,
				handle,
				tenor,
				signer
			)
		).rejects.toThrow('Invalid PLC signing key');
		expect(signer).not.toHaveBeenCalled();
	});
});
