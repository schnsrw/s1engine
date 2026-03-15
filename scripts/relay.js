#!/usr/bin/env node
// WebSocket relay server for s1engine real-time collaboration.
//
// Usage:
//   node scripts/relay.js [--port 8787]
//
// Environment Variables:
//   PORT=8787              Server port (overridden by --port flag)
//   PUBLIC_DIR=            Static file directory (optional, enables file serving)
//   JWT_SECRET=            JWT signing secret (optional, disables auth if empty)
//   RATE_LIMIT=100         Messages per second per peer
//   MAX_PEERS_PER_ROOM=50  Max peers per room
//   ROOM_TTL_DAYS=30       Days to keep inactive room data on disk
//   DATA_DIR=./data        Persistence directory for room state
//   ADMIN_TOKEN=           Admin API access token
//
// Protocol:
//   All messages are JSON. Each message has a "type" field.
//
//   Client -> Server:
//     { type: "join",      room: "abc", userName: "Alice", userColor: "#e06c75", token: "jwt..." }
//     { type: "op",        room: "abc", data: <serialized CRDT op JSON> }
//     { type: "awareness", room: "abc", data: <serialized awareness update JSON> }
//     { type: "sync-req",  room: "abc", stateVector: <JSON state vector> }
//     { type: "leave",     room: "abc" }
//
//   Server -> Client:
//     { type: "joined",    room: "abc", peerId: "...", peers: [...], access: "edit" }
//     { type: "peer-join", room: "abc", peerId: "...", userName: "...", userColor: "..." }
//     { type: "peer-leave",room: "abc", peerId: "..." }
//     { type: "op",        room: "abc", peerId: "...", data: <CRDT op JSON> }
//     { type: "awareness", room: "abc", peerId: "...", data: <awareness JSON> }
//     { type: "sync-resp", room: "abc", ops: [<array of CRDT op JSONs>] }
//     { type: "error",     message: "..." }
//
// Zero external dependencies — uses only Node.js built-ins (http, crypto, fs, path, zlib).

const http = require('http');
const crypto = require('crypto');
const fs = require('fs');
const path = require('path');
const zlib = require('zlib');

// ─── Configuration ───────────────────────────────────

const PORT = parseInt(
  process.argv.find((_, i, a) => a[i - 1] === '--port') ||
  process.env.PORT ||
  '8787', 10
);
const PUBLIC_DIR = process.env.PUBLIC_DIR || null;
const JWT_SECRET = process.env.JWT_SECRET || '';
const RATE_LIMIT = parseInt(process.env.RATE_LIMIT || '100', 10);
const MAX_PEERS_PER_ROOM = parseInt(process.env.MAX_PEERS_PER_ROOM || '50', 10);
const ROOM_TTL_DAYS = parseInt(process.env.ROOM_TTL_DAYS || '30', 10);
const DATA_DIR = process.env.DATA_DIR || path.join(process.cwd(), 'data');
const ADMIN_TOKEN = process.env.ADMIN_TOKEN || '';

const ROOMS_DIR = path.join(DATA_DIR, 'rooms');
const OP_LOG_CAP = 10000;
const OP_LOG_TRIM = 5000;
const PERSIST_INTERVAL = 100; // Save every N ops
const ROOM_CLEANUP_TIMEOUT_MS = 5 * 60 * 1000; // 5 minutes
const startTime = Date.now();

// ─── Logging ─────────────────────────────────────────

function log(msg) {
  console.log(`${new Date().toISOString()} [relay] ${msg}`);
}

function logError(msg) {
  console.error(`${new Date().toISOString()} [relay] ERROR: ${msg}`);
}

// ─── JWT Helpers (HMAC-SHA256, zero dependencies) ────

function base64UrlEncode(buf) {
  return Buffer.from(buf)
    .toString('base64')
    .replace(/\+/g, '-')
    .replace(/\//g, '_')
    .replace(/=+$/, '');
}

function base64UrlDecode(str) {
  str = str.replace(/-/g, '+').replace(/_/g, '/');
  while (str.length % 4 !== 0) str += '=';
  return Buffer.from(str, 'base64');
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

function verifyJWT(token, secret) {
  if (!token || !secret) return null;
  const parts = token.split('.');
  if (parts.length !== 3) return null;

  try {
    const signingInput = parts[0] + '.' + parts[1];
    const expectedSig = crypto
      .createHmac('sha256', secret)
      .update(signingInput)
      .digest();
    const actualSig = base64UrlDecode(parts[2]);

    // Constant-time comparison
    if (expectedSig.length !== actualSig.length) return null;
    if (!crypto.timingSafeEqual(expectedSig, actualSig)) return null;

    const payload = JSON.parse(base64UrlDecode(parts[1]).toString('utf8'));

    // Check expiration
    if (payload.exp) {
      const now = Math.floor(Date.now() / 1000);
      if (now > payload.exp) return null;
    }

    return payload;
  } catch (_) {
    return null;
  }
}

// ─── Persistence ─────────────────────────────────────

function ensureDataDir() {
  try {
    fs.mkdirSync(ROOMS_DIR, { recursive: true });
  } catch (e) {
    if (e.code !== 'EEXIST') logError(`Failed to create data dir: ${e.message}`);
  }
}

function roomFilePath(roomId) {
  // Sanitize roomId to prevent path traversal
  const safe = roomId.replace(/[^a-zA-Z0-9_-]/g, '_');
  return path.join(ROOMS_DIR, `${safe}.json`);
}

function saveRoom(roomId, room) {
  try {
    const filePath = roomFilePath(roomId);
    const data = JSON.stringify({
      opLog: room.opLog,
      createdAt: room.createdAt || new Date().toISOString(),
      lastActivity: new Date().toISOString(),
    });
    fs.writeFileSync(filePath, data, 'utf8');
  } catch (e) {
    logError(`Failed to save room ${roomId}: ${e.message}`);
  }
}

function loadRoom(roomId) {
  try {
    const filePath = roomFilePath(roomId);
    if (!fs.existsSync(filePath)) return null;
    const raw = fs.readFileSync(filePath, 'utf8');
    const data = JSON.parse(raw);
    return data;
  } catch (e) {
    logError(`Failed to load room ${roomId}: ${e.message}`);
    return null;
  }
}

function cleanupOldRooms() {
  try {
    if (!fs.existsSync(ROOMS_DIR)) return;
    const files = fs.readdirSync(ROOMS_DIR);
    const cutoff = Date.now() - ROOM_TTL_DAYS * 24 * 60 * 60 * 1000;
    let cleaned = 0;

    for (const file of files) {
      if (!file.endsWith('.json')) continue;
      const filePath = path.join(ROOMS_DIR, file);
      try {
        const raw = fs.readFileSync(filePath, 'utf8');
        const data = JSON.parse(raw);
        const lastActivity = data.lastActivity ? new Date(data.lastActivity).getTime() : 0;
        if (lastActivity < cutoff) {
          fs.unlinkSync(filePath);
          cleaned++;
        }
      } catch (_) {
        // Skip corrupt files
      }
    }
    if (cleaned > 0) log(`Cleaned up ${cleaned} room(s) older than ${ROOM_TTL_DAYS} days`);
  } catch (e) {
    logError(`Failed to cleanup old rooms: ${e.message}`);
  }
}

// ─── Rate Limiting ───────────────────────────────────

// Track per-peer message counts: Map<peerId, { count, windowStart }>
const rateLimitState = new Map();

function checkRateLimit(peerId) {
  const now = Date.now();
  let state = rateLimitState.get(peerId);

  if (!state || now - state.windowStart >= 1000) {
    // New window
    state = { count: 1, windowStart: now };
    rateLimitState.set(peerId, state);
    return true;
  }

  state.count++;
  if (state.count > RATE_LIMIT) {
    return false;
  }
  return true;
}

function clearRateLimitState(peerId) {
  rateLimitState.delete(peerId);
}

// ─── Metrics Tracking ────────────────────────────────

let totalOpsProcessed = 0;
let opsInLastSecond = 0;
let opsPerSecond = 0;

setInterval(() => {
  opsPerSecond = opsInLastSecond;
  opsInLastSecond = 0;
}, 1000);

// ─── Room State ──────────────────────────────────────

// rooms: Map<roomId, {
//   peers: Map<peerId, { ws, userName, userColor, access }>,
//   opLog: string[],
//   createdAt: string,
//   opsSinceSave: number,
//   _cleanupTimer: Timeout|null
// }>
const rooms = new Map();

function getOrCreateRoom(roomId) {
  if (!rooms.has(roomId)) {
    // Try to load from disk first
    const persisted = loadRoom(roomId);
    const room = {
      peers: new Map(),
      opLog: persisted ? persisted.opLog : [],
      createdAt: persisted ? persisted.createdAt : new Date().toISOString(),
      opsSinceSave: 0,
      _cleanupTimer: null,
    };
    rooms.set(roomId, room);
    if (persisted) {
      log(`Loaded room ${roomId} from disk (${room.opLog.length} ops)`);
    }
  }
  return rooms.get(roomId);
}

function cleanupRoom(roomId) {
  const room = rooms.get(roomId);
  if (room && room.peers.size === 0) {
    // Persist before starting cleanup timer
    saveRoom(roomId, room);

    // Keep room alive for 5 minutes after last peer leaves (for reconnection)
    room._cleanupTimer = setTimeout(() => {
      if (room.peers.size === 0) {
        saveRoom(roomId, room);
        rooms.delete(roomId);
        log(`Room ${roomId} removed from memory (persisted to disk)`);
      }
    }, ROOM_CLEANUP_TIMEOUT_MS);
  }
}

// ─── Peer Management ─────────────────────────────────

function generatePeerId() {
  return crypto.randomBytes(8).toString('hex');
}

function broadcast(room, message, excludePeerId) {
  const json = JSON.stringify(message);
  for (const [pid, peer] of room.peers) {
    if (pid !== excludePeerId && peer.ws.readyState === 1) {
      try { peer.ws.send(json); } catch (_) { /* ignore send errors */ }
    }
  }
}

function sendTo(ws, message) {
  if (ws.readyState === 1) {
    try { ws.send(JSON.stringify(message)); } catch (_) {}
  }
}

// ─── Permission Helpers ──────────────────────────────

function canSendOps(access) {
  return access === 'edit';
}

function canSendComments(access) {
  return access === 'edit' || access === 'comment';
}

function isCommentOp(data) {
  // Heuristic: check if the op data contains comment-related type markers
  if (typeof data === 'string') {
    try {
      const parsed = JSON.parse(data);
      return parsed.type === 'comment' || parsed.opType === 'comment';
    } catch (_) {}
  }
  return false;
}

// ─── Message Handlers ────────────────────────────────

function handleJoin(ws, peerId, msg) {
  const roomId = msg.room;
  if (!roomId || typeof roomId !== 'string') {
    sendTo(ws, { type: 'error', message: 'Missing room ID' });
    return;
  }

  // ── JWT Authentication ──
  let access = 'edit'; // Default: full access (backwards compatible)
  let authenticatedName = null;

  if (JWT_SECRET) {
    if (msg.token) {
      const payload = verifyJWT(msg.token, JWT_SECRET);
      if (!payload) {
        sendTo(ws, { type: 'error', message: 'Invalid or expired token' });
        return;
      }

      // Verify room claim if present
      if (payload.room && payload.room !== roomId) {
        sendTo(ws, { type: 'error', message: 'Token not valid for this room' });
        return;
      }

      access = payload.access || 'edit';
      if (!['edit', 'comment', 'view'].includes(access)) {
        access = 'view';
      }
      authenticatedName = payload.name || null;
    }
    // If JWT_SECRET is set but no token provided, allow anonymous (backwards compatible)
  }

  // ── Room size limit ──
  const room = getOrCreateRoom(roomId);
  if (room.peers.size >= MAX_PEERS_PER_ROOM) {
    sendTo(ws, { type: 'error', message: `Room is full (max ${MAX_PEERS_PER_ROOM} peers)` });
    return;
  }

  if (room._cleanupTimer) {
    clearTimeout(room._cleanupTimer);
    room._cleanupTimer = null;
  }

  const userName = authenticatedName || msg.userName || 'Anonymous';
  const userColor = msg.userColor || '#999999';

  room.peers.set(peerId, { ws, userName, userColor, access });
  ws._peerId = peerId;
  ws._roomId = roomId;

  // Send join confirmation with current peer list
  const peers = [];
  for (const [pid, peer] of room.peers) {
    if (pid !== peerId) {
      peers.push({ peerId: pid, userName: peer.userName, userColor: peer.userColor });
    }
  }
  sendTo(ws, { type: 'joined', room: roomId, peerId, peers, access });

  // Notify existing peers
  broadcast(room, {
    type: 'peer-join',
    room: roomId,
    peerId,
    userName,
    userColor,
  }, peerId);

  // Send buffered op log to new peer for catch-up
  if (room.opLog.length > 0) {
    sendTo(ws, { type: 'sync-resp', room: roomId, ops: room.opLog });
  }

  log(`[${roomId}] ${userName} joined (${room.peers.size} peers, access=${access})`);
}

function handleOp(ws, peerId, msg) {
  const roomId = msg.room;
  const room = rooms.get(roomId);
  if (!room || !room.peers.has(peerId)) {
    sendTo(ws, { type: 'error', message: 'Not in room' });
    return;
  }

  // Permission check
  const peer = room.peers.get(peerId);
  if (!canSendOps(peer.access)) {
    if (canSendComments(peer.access) && isCommentOp(msg.data)) {
      // Allow comment ops for comment-access peers
    } else {
      sendTo(ws, { type: 'error', message: 'Insufficient permissions to send ops' });
      return;
    }
  }

  // Store op in room log (cap at OP_LOG_CAP to prevent memory issues)
  if (typeof msg.data === 'string') {
    room.opLog.push(msg.data);
    if (room.opLog.length > OP_LOG_CAP) {
      room.opLog.splice(0, room.opLog.length - OP_LOG_TRIM);
    }

    // Track ops for metrics
    totalOpsProcessed++;
    opsInLastSecond++;

    // Periodic persistence
    room.opsSinceSave = (room.opsSinceSave || 0) + 1;
    if (room.opsSinceSave >= PERSIST_INTERVAL) {
      room.opsSinceSave = 0;
      saveRoom(roomId, room);
    }
  }

  // Broadcast to all other peers
  broadcast(room, {
    type: 'op',
    room: roomId,
    peerId,
    data: msg.data,
  }, peerId);
}

function handleAwareness(ws, peerId, msg) {
  const roomId = msg.room;
  const room = rooms.get(roomId);
  if (!room || !room.peers.has(peerId)) return;

  broadcast(room, {
    type: 'awareness',
    room: roomId,
    peerId,
    data: msg.data,
  }, peerId);
}

function handleSyncReq(ws, peerId, msg) {
  const roomId = msg.room;
  const room = rooms.get(roomId);
  if (!room) {
    sendTo(ws, { type: 'sync-resp', room: roomId, ops: [] });
    return;
  }

  // State vector filtering: if client provides a stateVector, only send missing ops
  if (msg.stateVector && typeof msg.stateVector === 'object') {
    // stateVector is { replicaId: lastSeenCounter, ... }
    // Filter ops that the client hasn't seen yet
    const missing = [];
    for (const opStr of room.opLog) {
      try {
        const op = JSON.parse(opStr);
        const replicaId = op.replicaId || op.replica_id || op.peerId;
        const counter = op.counter || op.seq || 0;
        if (replicaId && msg.stateVector[replicaId] !== undefined) {
          if (counter > msg.stateVector[replicaId]) {
            missing.push(opStr);
          }
        } else {
          // Unknown replica — send it (client's CRDT will deduplicate)
          missing.push(opStr);
        }
      } catch (_) {
        // Can't parse op — send it anyway for safety
        missing.push(opStr);
      }
    }
    sendTo(ws, { type: 'sync-resp', room: roomId, ops: missing });
  } else {
    // No state vector — return full op log (CRDT handles deduplication)
    sendTo(ws, { type: 'sync-resp', room: roomId, ops: room.opLog });
  }
}

function handleLeave(ws, peerId, msg) {
  const roomId = msg.room || ws._roomId;
  const room = rooms.get(roomId);
  if (!room) return;

  const peer = room.peers.get(peerId);
  const userName = peer ? peer.userName : 'Unknown';

  room.peers.delete(peerId);
  clearRateLimitState(peerId);

  broadcast(room, {
    type: 'peer-leave',
    room: roomId,
    peerId,
  });

  log(`[${roomId}] ${userName} left (${room.peers.size} peers)`);
  cleanupRoom(roomId);
}

function handleDisconnect(ws) {
  const peerId = ws._peerId;
  const roomId = ws._roomId;
  if (peerId && roomId) {
    handleLeave(ws, peerId, { room: roomId });
  }
}

// ─── WebSocket Server (minimal, no dependencies) ─────

function acceptWebSocket(req, socket, head) {
  const key = req.headers['sec-websocket-key'];
  if (!key) { socket.destroy(); return; }

  const accept = crypto
    .createHash('sha1')
    .update(key + '258EAFA5-E914-47DA-95CA-5AB5A0085CC1')
    .digest('base64');

  socket.write(
    'HTTP/1.1 101 Switching Protocols\r\n' +
    'Upgrade: websocket\r\n' +
    'Connection: Upgrade\r\n' +
    `Sec-WebSocket-Accept: ${accept}\r\n` +
    '\r\n'
  );

  const ws = createWsWrapper(socket);
  const peerId = generatePeerId();
  ws._peerId = peerId;

  ws.on('message', (data) => {
    // Rate limiting
    if (!checkRateLimit(peerId)) {
      sendTo(ws, { type: 'error', message: 'Rate limit exceeded' });
      return;
    }

    try {
      const msg = JSON.parse(data);
      switch (msg.type) {
        case 'join':      handleJoin(ws, peerId, msg); break;
        case 'op':        handleOp(ws, peerId, msg); break;
        case 'awareness': handleAwareness(ws, peerId, msg); break;
        case 'sync-req':  handleSyncReq(ws, peerId, msg); break;
        case 'leave':     handleLeave(ws, peerId, msg); break;
        default:
          sendTo(ws, { type: 'error', message: `Unknown message type: ${msg.type}` });
      }
    } catch (e) {
      sendTo(ws, { type: 'error', message: 'Invalid JSON' });
    }
  });

  ws.on('close', () => handleDisconnect(ws));
}

// Minimal WebSocket frame parser/writer
function createWsWrapper(socket) {
  const events = {};
  let closed = false;

  const ws = {
    readyState: 1,
    _peerId: null,
    _roomId: null,

    on(event, handler) {
      if (!events[event]) events[event] = [];
      events[event].push(handler);
    },

    send(data) {
      if (closed) return;
      const buf = Buffer.from(data, 'utf8');
      let header;
      if (buf.length < 126) {
        header = Buffer.alloc(2);
        header[0] = 0x81; // FIN + text
        header[1] = buf.length;
      } else if (buf.length < 65536) {
        header = Buffer.alloc(4);
        header[0] = 0x81;
        header[1] = 126;
        header.writeUInt16BE(buf.length, 2);
      } else {
        header = Buffer.alloc(10);
        header[0] = 0x81;
        header[1] = 127;
        header.writeBigUInt64BE(BigInt(buf.length), 2);
      }
      socket.write(Buffer.concat([header, buf]));
    },
  };

  let buffer = Buffer.alloc(0);

  socket.on('data', (chunk) => {
    buffer = Buffer.concat([buffer, chunk]);

    while (buffer.length >= 2) {
      const opcode = buffer[0] & 0x0f;
      const masked = (buffer[1] & 0x80) !== 0;
      let payloadLen = buffer[1] & 0x7f;
      let offset = 2;

      if (payloadLen === 126) {
        if (buffer.length < 4) return;
        payloadLen = buffer.readUInt16BE(2);
        offset = 4;
      } else if (payloadLen === 127) {
        if (buffer.length < 10) return;
        payloadLen = Number(buffer.readBigUInt64BE(2));
        offset = 10;
      }

      const maskLen = masked ? 4 : 0;
      const totalLen = offset + maskLen + payloadLen;
      if (buffer.length < totalLen) return;

      const mask = masked ? buffer.slice(offset, offset + maskLen) : null;
      const payload = buffer.slice(offset + maskLen, totalLen);

      if (masked && mask) {
        for (let i = 0; i < payload.length; i++) {
          payload[i] ^= mask[i % 4];
        }
      }

      buffer = buffer.slice(totalLen);

      if (opcode === 0x08) {
        // Close frame
        closed = true;
        ws.readyState = 3;
        socket.end();
        (events['close'] || []).forEach(h => h());
        return;
      }

      if (opcode === 0x09) {
        // Ping -> Pong
        const pong = Buffer.alloc(2);
        pong[0] = 0x8a;
        pong[1] = 0;
        socket.write(pong);
        continue;
      }

      if (opcode === 0x01 || opcode === 0x02) {
        const text = payload.toString('utf8');
        (events['message'] || []).forEach(h => h(text));
      }
    }
  });

  socket.on('close', () => {
    if (!closed) {
      closed = true;
      ws.readyState = 3;
      (events['close'] || []).forEach(h => h());
    }
  });

  socket.on('error', () => {
    if (!closed) {
      closed = true;
      ws.readyState = 3;
      (events['close'] || []).forEach(h => h());
    }
  });

  return ws;
}

// ─── Static File Serving ─────────────────────────────

const MIME_TYPES = {
  '.html': 'text/html; charset=utf-8',
  '.js':   'application/javascript; charset=utf-8',
  '.mjs':  'application/javascript; charset=utf-8',
  '.css':  'text/css; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.wasm': 'application/wasm',
  '.png':  'image/png',
  '.jpg':  'image/jpeg',
  '.jpeg': 'image/jpeg',
  '.gif':  'image/gif',
  '.svg':  'image/svg+xml',
  '.ico':  'image/x-icon',
  '.woff': 'font/woff',
  '.woff2': 'font/woff2',
  '.ttf':  'font/ttf',
  '.otf':  'font/otf',
  '.txt':  'text/plain; charset=utf-8',
  '.map':  'application/json',
};

// Extensions eligible for gzip/brotli compression
const COMPRESSIBLE = new Set(['.html', '.js', '.mjs', '.css', '.json', '.svg', '.txt', '.map']);

// Hashed asset extensions get long-lived cache headers
const IMMUTABLE_CACHE = 'public, max-age=31536000, immutable';
const NO_CACHE = 'no-cache, no-store, must-revalidate';

function serveStatic(req, res) {
  if (!PUBLIC_DIR) return false;

  // Parse URL (strip query string / hash)
  let urlPath = (req.url || '/').split('?')[0].split('#')[0];

  // Prevent directory traversal
  urlPath = path.normalize(urlPath).replace(/^(\.\.(\/|\\|$))+/, '');

  let filePath = path.join(PUBLIC_DIR, urlPath);

  // If path is a directory, try index.html
  try {
    const stat = fs.statSync(filePath);
    if (stat.isDirectory()) {
      filePath = path.join(filePath, 'index.html');
    }
  } catch (_) {
    // File does not exist — try SPA fallback below
  }

  // Check if file exists
  if (!fs.existsSync(filePath) || fs.statSync(filePath).isDirectory()) {
    // SPA fallback: serve index.html for paths without file extensions
    const ext = path.extname(urlPath);
    if (!ext) {
      const indexPath = path.join(PUBLIC_DIR, 'index.html');
      if (fs.existsSync(indexPath)) {
        serveFile(indexPath, '.html', req, res, false);
        return true;
      }
    }
    return false;
  }

  const ext = path.extname(filePath).toLowerCase();
  // Hashed assets (e.g., main-abc123.js) get immutable cache
  const isHashed = /[-\.][a-f0-9]{8,}\./.test(path.basename(filePath));
  serveFile(filePath, ext, req, res, isHashed);
  return true;
}

// Security headers applied to HTML responses
const SECURITY_HEADERS = {
  'Content-Security-Policy': [
    "default-src 'self'",
    "script-src 'self' 'wasm-unsafe-eval' https://cdn.jsdelivr.net",
    "style-src 'self' 'unsafe-inline' https://cdn.jsdelivr.net https://fonts.googleapis.com",
    "font-src 'self' https://fonts.gstatic.com",
    "connect-src 'self' ws: wss:",
    "img-src 'self' data: blob:",
  ].join('; '),
  'X-Frame-Options': 'SAMEORIGIN',
  'Referrer-Policy': 'strict-origin-when-cross-origin',
};

function serveFile(filePath, ext, req, res, isHashed) {
  const contentType = MIME_TYPES[ext] || 'application/octet-stream';
  const cacheControl = (ext === '.html' || !isHashed) ? NO_CACHE : IMMUTABLE_CACHE;

  const headers = {
    'Content-Type': contentType,
    'Cache-Control': cacheControl,
    'X-Content-Type-Options': 'nosniff',
  };

  // Add security headers for HTML responses
  if (ext === '.html') {
    Object.assign(headers, SECURITY_HEADERS);
  }

  // Check if we can compress the response
  const acceptEncoding = (req.headers['accept-encoding'] || '');
  const canCompress = COMPRESSIBLE.has(ext);

  if (canCompress && acceptEncoding.includes('br')) {
    headers['Content-Encoding'] = 'br';
    headers['Vary'] = 'Accept-Encoding';
    res.writeHead(200, headers);
    fs.createReadStream(filePath).pipe(zlib.createBrotliCompress()).pipe(res);
  } else if (canCompress && acceptEncoding.includes('gzip')) {
    headers['Content-Encoding'] = 'gzip';
    headers['Vary'] = 'Accept-Encoding';
    res.writeHead(200, headers);
    fs.createReadStream(filePath).pipe(zlib.createGzip()).pipe(res);
  } else {
    const stat = fs.statSync(filePath);
    headers['Content-Length'] = stat.size;
    res.writeHead(200, headers);
    fs.createReadStream(filePath).pipe(res);
  }
}

// ─── HTTP Server ─────────────────────────────────────

const server = http.createServer((req, res) => {
  const corsHeaders = {
    'Access-Control-Allow-Origin': '*',
    'Access-Control-Allow-Headers': 'Authorization, Content-Type',
    'Access-Control-Allow-Methods': 'GET, OPTIONS',
  };

  // Handle CORS preflight
  if (req.method === 'OPTIONS') {
    res.writeHead(204, corsHeaders);
    res.end();
    return;
  }

  // ── Health check endpoint ──
  if (req.url === '/health') {
    let peerCount = 0;
    let totalOps = 0;
    for (const room of rooms.values()) {
      peerCount += room.peers.size;
      totalOps += room.opLog.length;
    }
    const memUsage = process.memoryUsage();
    res.writeHead(200, { 'Content-Type': 'application/json', ...corsHeaders });
    res.end(JSON.stringify({
      status: 'ok',
      uptime: Math.floor((Date.now() - startTime) / 1000),
      rooms: rooms.size,
      peers: peerCount,
      totalOps,
      memoryMB: Math.round(memUsage.heapUsed / 1024 / 1024 * 10) / 10,
    }));
    return;
  }

  // ── Prometheus metrics endpoint ──
  if (req.url === '/metrics') {
    let peerCount = 0;
    let totalOpsInMemory = 0;
    for (const room of rooms.values()) {
      peerCount += room.peers.size;
      totalOpsInMemory += room.opLog.length;
    }
    const memUsage = process.memoryUsage();
    const lines = [
      '# HELP relay_rooms_active Number of active rooms',
      '# TYPE relay_rooms_active gauge',
      `relay_rooms_active ${rooms.size}`,
      '# HELP relay_peers_connected Number of connected peers',
      '# TYPE relay_peers_connected gauge',
      `relay_peers_connected ${peerCount}`,
      '# HELP relay_ops_total Total operations processed since startup',
      '# TYPE relay_ops_total counter',
      `relay_ops_total ${totalOpsProcessed}`,
      '# HELP relay_ops_per_second Operations per second (last window)',
      '# TYPE relay_ops_per_second gauge',
      `relay_ops_per_second ${opsPerSecond}`,
      '# HELP relay_memory_bytes Heap memory usage in bytes',
      '# TYPE relay_memory_bytes gauge',
      `relay_memory_bytes ${memUsage.heapUsed}`,
      '# HELP relay_uptime_seconds Server uptime in seconds',
      '# TYPE relay_uptime_seconds gauge',
      `relay_uptime_seconds ${Math.floor((Date.now() - startTime) / 1000)}`,
    ];
    res.writeHead(200, { 'Content-Type': 'text/plain; version=0.0.4', ...corsHeaders });
    res.end(lines.join('\n') + '\n');
    return;
  }

  // ── Admin rooms endpoint ──
  if (req.url === '/admin/rooms') {
    if (!ADMIN_TOKEN) {
      res.writeHead(403, { 'Content-Type': 'application/json', ...corsHeaders });
      res.end(JSON.stringify({ error: 'Admin API not configured (set ADMIN_TOKEN env var)' }));
      return;
    }

    const authHeader = req.headers['authorization'] || '';
    const token = authHeader.startsWith('Bearer ') ? authHeader.slice(7) : authHeader;
    if (token !== ADMIN_TOKEN) {
      res.writeHead(401, { 'Content-Type': 'application/json', ...corsHeaders });
      res.end(JSON.stringify({ error: 'Unauthorized' }));
      return;
    }

    const info = [];
    for (const [id, room] of rooms) {
      const peers = [];
      for (const [pid, peer] of room.peers) {
        peers.push({
          peerId: pid,
          userName: peer.userName,
          userColor: peer.userColor,
          access: peer.access,
        });
      }
      info.push({
        room: id,
        peers,
        opLogSize: room.opLog.length,
        createdAt: room.createdAt,
      });
    }
    res.writeHead(200, { 'Content-Type': 'application/json', ...corsHeaders });
    res.end(JSON.stringify(info));
    return;
  }

  // ── Room info endpoint (backwards compatible) ──
  if (req.url === '/rooms') {
    const info = [];
    for (const [id, room] of rooms) {
      const peers = [];
      for (const [pid, peer] of room.peers) {
        peers.push({ peerId: pid, userName: peer.userName });
      }
      info.push({ room: id, peers, opLogSize: room.opLog.length });
    }
    res.writeHead(200, { 'Content-Type': 'application/json', ...corsHeaders });
    res.end(JSON.stringify(info));
    return;
  }

  // ── Static file serving (when PUBLIC_DIR is set) ──
  if (PUBLIC_DIR && req.method === 'GET') {
    if (serveStatic(req, res)) return;
  }

  // ── Default landing page ──
  res.writeHead(200, { 'Content-Type': 'text/plain' });
  res.end(
    's1engine collaboration relay server\n\n' +
    `WebSocket endpoint: ws://localhost:${PORT}/\n` +
    `Health check:       http://localhost:${PORT}/health\n` +
    `Metrics:            http://localhost:${PORT}/metrics\n` +
    `Room info:          http://localhost:${PORT}/rooms\n`
  );
});

server.on('upgrade', (req, socket, head) => {
  acceptWebSocket(req, socket, head);
});

// ─── Startup ─────────────────────────────────────────

ensureDataDir();
cleanupOldRooms();

server.listen(PORT, () => {
  log(`s1engine relay server listening on ws://localhost:${PORT}`);
  log(`Health check: http://localhost:${PORT}/health`);
  log(`Metrics:      http://localhost:${PORT}/metrics`);
  log(`Room info:    http://localhost:${PORT}/rooms`);
  if (ADMIN_TOKEN) {
    log(`Admin API:    http://localhost:${PORT}/admin/rooms (token required)`);
  }
  if (PUBLIC_DIR) {
    log(`Static files: ${PUBLIC_DIR}`);
    log(`Editor:       http://localhost:${PORT}/`);
  }
  if (JWT_SECRET) {
    log('JWT authentication enabled');
  } else {
    log('JWT authentication disabled (set JWT_SECRET to enable)');
  }
  log(`Rate limit: ${RATE_LIMIT} msgs/sec/peer`);
  log(`Max peers per room: ${MAX_PEERS_PER_ROOM}`);
  log(`Data directory: ${DATA_DIR}`);
  log(`Room TTL: ${ROOM_TTL_DAYS} days`);
});

// ─── Graceful Shutdown ───────────────────────────────

function shutdown(signal) {
  log(`${signal} received, saving room state...`);
  for (const [roomId, room] of rooms) {
    if (room.opLog.length > 0) {
      saveRoom(roomId, room);
    }
  }
  log('All rooms persisted. Shutting down.');
  process.exit(0);
}

process.on('SIGINT', () => shutdown('SIGINT'));
process.on('SIGTERM', () => shutdown('SIGTERM'));
