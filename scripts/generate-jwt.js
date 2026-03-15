#!/usr/bin/env node
// Generate JWT tokens for s1engine relay server authentication.
//
// Usage:
//   node scripts/generate-jwt.js [options]
//
// Options:
//   --secret <secret>    JWT signing secret (or set JWT_SECRET env var)
//   --peer <peerId>      Peer ID (default: random)
//   --name <userName>    User display name (default: "User")
//   --room <roomId>      Room ID (optional, restricts token to specific room)
//   --access <level>     Access level: "edit", "comment", or "view" (default: "edit")
//   --expires <seconds>  Token expiration in seconds (default: 86400 = 24h)
//   --json               Output as JSON object instead of plain token
//
// Examples:
//   # Generate an edit token (uses JWT_SECRET env var)
//   JWT_SECRET=mysecret node scripts/generate-jwt.js --name "Alice" --room "doc-123"
//
//   # Generate a view-only token expiring in 1 hour
//   node scripts/generate-jwt.js --secret mysecret --name "Bob" --access view --expires 3600
//
//   # Generate a comment-only token with JSON output
//   node scripts/generate-jwt.js --secret mysecret --name "Charlie" --access comment --json

const crypto = require('crypto');

// ─── JWT Helpers (same as relay.js) ──────────────────

function base64UrlEncode(buf) {
  return Buffer.from(buf)
    .toString('base64')
    .replace(/\+/g, '-')
    .replace(/\//g, '_')
    .replace(/=+$/, '');
}

function signJWT(payload, secret, expiresInSec) {
  const header = { alg: 'HS256', typ: 'JWT' };
  const now = Math.floor(Date.now() / 1000);
  const body = { ...payload, iat: now };
  if (expiresInSec) body.exp = now + expiresInSec;

  const segments = [
    base64UrlEncode(JSON.stringify(header)),
    base64UrlEncode(JSON.stringify(body)),
  ];
  const signingInput = segments.join('.');
  const signature = crypto
    .createHmac('sha256', secret)
    .update(signingInput)
    .digest();
  segments.push(base64UrlEncode(signature));
  return segments.join('.');
}

// ─── CLI Argument Parsing ────────────────────────────

function getArg(name, defaultValue) {
  const idx = process.argv.indexOf(name);
  if (idx === -1 || idx + 1 >= process.argv.length) return defaultValue;
  return process.argv[idx + 1];
}

function hasFlag(name) {
  return process.argv.includes(name);
}

// ─── Main ────────────────────────────────────────────

const secret = getArg('--secret', process.env.JWT_SECRET || '');
if (!secret) {
  console.error('Error: No secret provided. Use --secret <secret> or set JWT_SECRET env var.');
  process.exit(1);
}

const peerId = getArg('--peer', crypto.randomBytes(8).toString('hex'));
const name = getArg('--name', 'User');
const room = getArg('--room', undefined);
const access = getArg('--access', 'edit');
const expiresIn = parseInt(getArg('--expires', '86400'), 10);
const jsonOutput = hasFlag('--json');

if (!['edit', 'comment', 'view'].includes(access)) {
  console.error(`Error: Invalid access level "${access}". Must be "edit", "comment", or "view".`);
  process.exit(1);
}

const payload = {
  sub: peerId,
  name,
  access,
};
if (room) payload.room = room;

const token = signJWT(payload, secret, expiresIn);

if (jsonOutput) {
  const now = Math.floor(Date.now() / 1000);
  console.log(JSON.stringify({
    token,
    payload: { ...payload, iat: now, exp: now + expiresIn },
    expiresAt: new Date((now + expiresIn) * 1000).toISOString(),
  }, null, 2));
} else {
  console.log(token);
}
