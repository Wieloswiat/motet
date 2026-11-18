function modularPower(base: bigint, exponent: bigint, modulus: bigint): bigint {
	let result = 1n;

	while (exponent > 0n) {
		if (exponent & 1n) result = (result * base) % modulus;

		base = (base * base) % modulus;
		exponent >>= 1n;
	}

	return result;
}

// PLC signing keys use compressed secp256k1 or P-256 multicodec points.
/* oxlint-disable anti-slop/no-runtime-typeof, anti-slop/no-unknown-parameters */
export function isRepositoryKey(value: unknown): boolean {
	if (typeof value !== 'string' || !value.startsWith('did:key:z')) return false;

	const encoded = value.slice(9);

	if (encoded.length === 0 || encoded.length > 50) return false;

	const alphabet = '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz';
	let number = 0n;

	for (const character of encoded) {
		const digit = alphabet.indexOf(character);

		if (digit < 0) return false;

		number = number * 58n + BigInt(digit);
	}

	const bytes: number[] = [];

	while (number > 0n) {
		bytes.unshift(Number(number & 255n));
		number >>= 8n;
	}

	for (const character of encoded) {
		if (character !== '1') break;

		bytes.unshift(0);
	}

	if (bytes.length !== 35 || (bytes[2] !== 2 && bytes[2] !== 3)) return false;

	const secp = bytes[0] === 0xe7 && bytes[1] === 0x01;
	const p256 = bytes[0] === 0x80 && bytes[1] === 0x24;

	if (!secp && !p256) return false;

	const p = secp
		? 0xfffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc2fn
		: 0xffffffff00000001000000000000000000000000ffffffffffffffffffffffffn;

	const a = secp ? 0n : p - 3n;
	const b = secp ? 7n : 0x5ac635d8aa3a93e7b3ebbd55769886bc651d06b0cc53b0f63bce3c3e27d2604bn;
	let x = 0n;

	for (const byte of bytes.slice(3)) x = (x << 8n) | BigInt(byte);

	if (x >= p) return false;

	const ySquared = (x * x * x + a * x + b) % p;

	return ySquared !== 0n && modularPower(ySquared, (p - 1n) / 2n, p) === 1n;
}
/* oxlint-enable anti-slop/no-runtime-typeof, anti-slop/no-unknown-parameters */
