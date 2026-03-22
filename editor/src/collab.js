// Real-time collaboration — WebSocket transport layer.
//
// Architecture: state.doc (WasmDocument) remains the source of truth.
// This module broadcasts local edits to peers via the relay server and
// replays remote edits into state.doc. No separate WasmCollabDocument.
//
// Hook points:
//   - syncParagraphText → broadcastTextSync (full paragraph text set)
//   - split_paragraph   → broadcastOp
//   - merge_paragraphs  → broadcastOp
//   - format_selection  → broadcastOp
//   - delete_selection  → broadcastOp
import { state, $ } from './state.js';
import { renderDocument, renderNodeById } from './render.js';

// ─── Configuration ────────────────────────────────────

// Default WebSocket URL — points to s1-server (not the static file server).
// Override via URL param ?relay=ws://... or via share dialog.
const DEFAULT_RELAY_URL = window.S1_CONFIG?.relayUrl
  || (window.location.protocol === 'https:' ? 'wss://' : 'ws://') + window.location.host + '/ws/collab';
const RECONNECT_DELAYS = [2000, 4000, 8000, 16000, 30000];
const MAX_RECONNECT_ATTEMPTS = 5;
const CURSOR_BROADCAST_INTERVAL = 500;

// Fixed palette of distinct, accessible peer colors (C21).
// Cycles through 25 colors — avoids the golden-angle HSL approach
// which repeated after ~360 peers and produced near-duplicate hues.
const PEER_COLORS = [
  '#1a73e8', '#e8710a', '#0d652d', '#9334e6', '#c5221f',
  '#137333', '#185abc', '#e37400', '#7627bb', '#a50e0e',
  '#0277bd', '#558b2f', '#6a1b9a', '#d84315', '#00838f',
  '#4e342e', '#283593', '#bf360c', '#00695c', '#ad1457',
  '#1565c0', '#2e7d32', '#4a148c', '#e65100', '#006064',
];

function getPeerColor(index) {
  return PEER_COLORS[index % PEER_COLORS.length];
}

/**
 * Generate a cryptographically secure room ID using Web Crypto API.
 * Falls back to Math.random if crypto is unavailable.
 */
function _generateSecureRoomId() {
  try {
    return 'room-' + crypto.getRandomValues(new Uint8Array(12))
      .reduce((s, b) => s + b.toString(16).padStart(2, '0'), '');
  } catch (_) {
    // Fallback for environments without crypto
    return 'room-' + Math.random().toString(36).substring(2, 10) + Date.now().toString(36);
  }
}

// ─── State ────────────────────────────────────────────

let ws = null;
let roomId = null;
let peerId = null;
let userName = 'Anonymous';
let userColor = '#4285f4';
let reconnectAttempt = 0;
let reconnectTimer = null;
let cursorTimer = null;
let offlineBuffer = [];
let connected = false;
let applyingRemote = false; // flag to prevent echo
let lastRelayUrl = null; // stored for reconnection
let accessLevel = 'edit'; // 'edit', 'comment', or 'view'
let localVersion = 0; // Increments on every local edit (for fullSync ordering)
let lastSyncedVersion = 0; // Version of last applied fullSync
let serverVersion = 0; // Tracks the latest version seen from the server/relay
let _pendingImmediateSync = false; // Bug C8: throttle immediate fullSync on node ID mismatch
let _fullSyncTimeout = null; // Timeout for fullSync response (Bug C3)
let missedPongs = 0; // Bug C6: count missed heartbeat responses
let _roomFullRetries = 0; // C11: exponential backoff retry when room is full
// X18: Deferred remote ops — queued during undo/redo execution to prevent race conditions
let _deferredRemoteOps = [];
let _deltaSyncReceived = false; // Tracks whether delta sync delivered ops (C15 fallback guard)

// ─── Public API ───────────────────────────────────────

/**
 * Start a collaboration session.
 */
export function startCollab(room, name, relayUrl) {
  if (!state.doc) return;

  roomId = room;
  userName = name || 'Anonymous';
  peerId = peerId || ('u-' + Math.random().toString(36).slice(2, 10));
  userColor = getPeerColor(Math.floor(Math.random() * 360));

  // Reset version state for new session
  localVersion = 0;
  lastSyncedVersion = 0;
  serverVersion = 0;

  // Create CRDT collab document if not already set (sharer path)
  if (!state.collabDoc && state.engine && typeof state.engine.open_collab === 'function') {
    try {
      const bytes = state.doc.export('docx');
      const replicaId = Math.floor(Math.random() * 2147483647) + 1;
      state.collabDoc = state.engine.open_collab(bytes, replicaId);
      tracing('Created CRDT collab doc for sharer, replicaId:', replicaId);
    } catch (e) {
      console.warn('CRDT collab not available:', e);
    }
  }
  // Warn users when CRDT is not available — collaboration will use reduced-
  // fidelity best-effort mode with higher latency and more snapshot-based
  // convergence. This typically means the WASM build lacks CRDT support.
  if (!state.collabDoc) {
    console.warn('[collab] CRDT not available — using reduced-fidelity sync mode. ' +
      'Text edits may have higher latency and structural operations rely on periodic snapshots.');
    showCollabToast('Real-time sync: reduced fidelity (CRDT unavailable)');
  }

  connect(relayUrl || DEFAULT_RELAY_URL);
  startCursorBroadcast();
  updateCollabUI();
}

/**
 * Stop the collaboration session.
 */
export function stopCollab() {
  if (ws) {
    try {
      ws.send(JSON.stringify({ type: 'leave', room: roomId }));
      ws.close();
    } catch (_) {}
  }
  ws = null;
  connected = false;
  roomId = null;
  peerId = null;
  offlineBuffer = [];
  clearInterval(cursorTimer);
  clearTimeout(reconnectTimer);
  clearTimeout(_fullSyncTimer);
  clearTimeout(_fullSyncTimeout); // Bug C3
  _fullSyncTimeout = null;
  clearPeerCursors();
  // Reset version state
  localVersion = 0;
  lastSyncedVersion = 0;
  serverVersion = 0;
  // Free CRDT collab document
  if (state.collabDoc) {
    try { state.collabDoc.free_doc(); } catch (_) {}
    state.collabDoc = null;
  }
  updateCollabUI();
}

/**
 * Broadcast a paragraph text change to peers.
 * Called from syncParagraphText in render.js.
 *
 * If CRDT is active (state.collabDoc), this is a no-op because CRDT
 * text ops are broadcast natively via apply_local_insert_text/delete_text.
 * If non-CRDT mode, sends setText + debounced fullSync.
 */
export function broadcastTextSync(nodeId, text) {
  if (!roomId || applyingRemote) return;
  // In CRDT mode, text changes are already broadcast at the character level.
  // Only send setText as fallback for non-CRDT mode.
  if (!state.collabDoc) {
    sendOp({ action: 'setText', nodeId, text });
    scheduleDebouncedFullSync();
  }
}

/**
 * Broadcast a structural operation (split, merge, format, delete).
 * Called from input.js / toolbar.js after applying the operation locally.
 *
 * When CRDT is active, also applies the structural op to collabDoc to keep
 * it in sync, and uses a longer fullSync debounce since CRDT handles text
 * convergence natively. Structural ops still need fullSync as a safety net
 * because the CRDT layer does not yet model structural operations natively.
 */
export function broadcastOp(opData) {
  if (!roomId || applyingRemote) return;

  // Keep collabDoc in sync with structural ops when CRDT is active.
  // This prevents the collabDoc from drifting from state.doc between
  // fullSync cycles, reducing the need for coarse snapshot replacement.
  if (state.collabDoc) {
    try {
      _applyCrdtStructuralOp(opData);
    } catch (e) {
      tracing('collabDoc structural sync failed (non-fatal):', e.message);
    }
  }

  // D17: Only increment version after successful send to keep version monotonic
  const pendingVersion = localVersion + 1;
  try {
    sendOp(opData);
    localVersion = pendingVersion;
  } catch (e) {
    // Don't increment — operation wasn't sent
    console.warn('broadcastOp send failed:', e);
  }
  // In CRDT mode, use longer debounce since text convergence is already
  // handled natively — fullSync is only a safety net for structural ops.
  scheduleDebouncedFullSync(state.collabDoc ? true : false);
}

/**
 * Apply a structural operation to the CRDT collabDoc to keep it in sync.
 * This is best-effort — failures are non-fatal since fullSync will correct.
 */
function _applyCrdtStructuralOp(opData) {
  const cd = state.collabDoc;
  if (!cd) return;
  switch (opData.action) {
    case 'splitParagraph': cd.split_paragraph(opData.nodeId, opData.offset); break;
    case 'mergeParagraphs': cd.merge_paragraphs(opData.nodeId1, opData.nodeId2); break;
    case 'setHeading': cd.set_heading_level(opData.nodeId, opData.level); break;
    case 'setAlignment': cd.set_alignment(opData.nodeId, opData.alignment); break;
    case 'setListFormat': cd.set_list_format(opData.nodeId, opData.format, opData.level || 0); break;
    case 'setIndent': cd.set_indent(opData.nodeId, opData.side, opData.value); break;
    case 'setLineSpacing': cd.set_line_spacing(opData.nodeId, opData.value); break;
    case 'insertParagraph': cd.insert_paragraph_after(opData.afterNodeId, opData.text || ''); break;
    case 'deleteNode': cd.delete_node(opData.nodeId); break;
    case 'insertTable': cd.insert_table(opData.afterNodeId, opData.rows, opData.cols); break;
    case 'insertHR': cd.insert_horizontal_rule(opData.afterNodeId); break;
    case 'insertPageBreak': cd.insert_page_break(opData.afterNodeId); break;
    case 'formatSelection': cd.format_selection(opData.startNode, opData.startOffset, opData.endNode, opData.endOffset, opData.key, opData.value); break;
    case 'deleteSelection': cd.delete_selection(opData.startNode, opData.startOffset, opData.endNode, opData.endOffset); break;
    // Ops without collabDoc equivalents are silently skipped —
    // fullSync will handle convergence for these.
  }
}

/**
 * Send a CRDT operation to peers (produced by WasmCollabDocument).
 * @param {string} crdtOpJson — JSON string from apply_local_insert_text etc.
 */
export function broadcastCrdtOp(crdtOpJson) {
  if (!roomId || applyingRemote || !crdtOpJson || crdtOpJson === '[]' || crdtOpJson === 'null') return;
  sendOp({ action: 'crdtOp', ops: crdtOpJson });
}

// ─── Debounced Full Sync ─────────────────────────────
// After any local edit, schedule a full document sync.
// This ensures the receiver always converges to the correct state
// even if individual ops fail due to node ID mismatch.
// When CRDT is active, fullSync is a safety net (longer debounce)
// since CRDT handles text convergence natively.
let _fullSyncTimer = null;
const FULL_SYNC_DEBOUNCE_MS = 1500; // Non-CRDT: 1.5s for faster convergence
const FULL_SYNC_DEBOUNCE_CRDT_MS = 5000; // CRDT active: 5s (safety net only)
let _consecutiveOpFailures = 0; // Track consecutive failures to avoid fullSync storms

function scheduleDebouncedFullSync(crdtActive = false) {
  if (_fullSyncTimer) clearTimeout(_fullSyncTimer);
  const delay = crdtActive ? FULL_SYNC_DEBOUNCE_CRDT_MS : FULL_SYNC_DEBOUNCE_MS;
  _fullSyncTimer = setTimeout(() => {
    _fullSyncTimer = null;
    sendFullSync();
  }, delay);
}

// Bug C8: Request immediate fullSync on node ID mismatch instead of waiting for debounce
function requestImmediateFullSync() {
  if (_pendingImmediateSync) return;
  _pendingImmediateSync = true;
  setTimeout(() => {
    _pendingImmediateSync = false;
    if (ws && ws.readyState === WebSocket.OPEN) {
      sendOp({ action: 'requestFullSync', myVersion: serverVersion });
    }
  }, 100); // Small delay to batch multiple failures
}

/**
 * Check if collaboration is active.
 */
export function isCollabActive() {
  return roomId !== null && connected;
}

/**
 * Check if remote operations are currently being applied.
 * Used to guard against local undo/redo during remote op application.
 */
export function isApplyingRemote() {
  return applyingRemote;
}

/**
 * Get current room ID.
 */
export function getCollabRoom() {
  return roomId;
}

/**
 * X18: Flush deferred remote ops that were queued during undo/redo execution.
 * Called by input.js after undo/redo completes.
 */
export function flushDeferredRemoteOps() {
  if (_deferredRemoteOps.length === 0) return;
  const ops = _deferredRemoteOps.splice(0);
  for (const { dataStr, fromPeerId } of ops) {
    applyRemoteOp(dataStr, fromPeerId);
  }
}

// ─── WebSocket Connection ─────────────────────────────

function connect(url) {
  if (ws) { try { ws.close(); } catch (_) {} }
  lastRelayUrl = url;

  // Validate WebSocket URL scheme
  try {
    const parsed = new URL(url);
    if (parsed.protocol !== 'ws:' && parsed.protocol !== 'wss:') {
      console.error('[collab] Invalid WebSocket protocol:', parsed.protocol);
      return;
    }
  } catch (e) {
    console.error('[collab] Invalid relay URL:', url);
    return;
  }

  // Append room ID to URL path for s1-server WebSocket endpoints.
  // Server expects: /ws/collab/{file_id}?user=Name&uid=id&mode=edit
  let wsUrl = url;
  if (roomId && (url.includes('/ws/collab') || url.includes('/ws/edit'))) {
    const pathPattern = /\/ws\/(collab|edit)\/?$/;
    if (pathPattern.test(url)) {
      wsUrl = url.replace(/\/?$/, '/') + encodeURIComponent(roomId);
    } else if (!url.includes('/ws/collab/') && !url.includes('/ws/edit/')) {
      wsUrl = url.replace(/\/?$/, '/') + encodeURIComponent(roomId);
    }
    // Add user info as query params for the server
    const sep = wsUrl.includes('?') ? '&' : '?';
    wsUrl += `${sep}user=${encodeURIComponent(userName)}&uid=${encodeURIComponent(peerId || 'u-' + Math.random().toString(36).slice(2,8))}&access=${encodeURIComponent(accessLevel)}`;
  }

  try {
    ws = new WebSocket(wsUrl);
  } catch (_) {
    scheduleReconnect(url);
    return;
  }

  // Connection timeout: if the socket hasn't opened within 8 seconds, close and retry
  const connectTimeout = setTimeout(() => {
    if (ws && ws.readyState !== WebSocket.OPEN) {
      console.warn('[collab] Connection timeout after 8s');
      try { ws.close(); } catch (_) {}
      ws = null;
      scheduleReconnect(url);
    }
  }, 8000);

  ws.onopen = () => {
    clearTimeout(connectTimeout);
    connected = true;
    reconnectAttempt = 0;
    missedPongs = 0; // Bug C6: Reset on connect
    updateConnectionStatus('connected');

    // I7: Clear stale peer entries from previous connection to prevent unbounded growth
    clearPeerCursors();
    state.collabPeers = peers; // C22: use same reference — clearPeerCursors() already clears peers

    ws.send(JSON.stringify({
      type: 'join',
      room: roomId,
      userName,
      userColor,
      clientVersion: serverVersion,
    }));

    // Restore any crashed offline buffer from sessionStorage (Bug C1)
    try {
      const saved = sessionStorage.getItem('rudra_collab_offline_buffer');
      if (saved) {
        const restored = JSON.parse(saved);
        if (Array.isArray(restored) && restored.length > 0) {
          offlineBuffer.push(...restored);
          console.log(`[collab] Restored ${restored.length} offline operations from session`);
        }
        sessionStorage.removeItem('rudra_collab_offline_buffer');
      }
    } catch (_) {}

    // Bug C7: Request server catchup before flushing local buffer
    // This ensures we apply any ops we missed while disconnected before replaying ours
    try {
      ws.send(JSON.stringify({ type: 'sync-req', room: roomId, stateVector: null }));
      tracing('Requested server catchup on reconnect (fromVersion:', serverVersion, ')');
    } catch (_) {}

    // Flush offline buffer with send verification (Bug C7)
    if (offlineBuffer.length > 0) {
      updateSyncStatus('syncing');
      let flushFailed = false;
      while (offlineBuffer.length > 0) {
        const op = offlineBuffer[0];
        try {
          if (ws.readyState !== WebSocket.OPEN) {
            flushFailed = true;
            break;
          }
          ws.send(JSON.stringify({ type: 'op', room: roomId, data: JSON.stringify(op) }));
          offlineBuffer.shift(); // Only remove after successful send
        } catch (e) {
          console.warn('[collab] Buffer flush failed:', e);
          flushFailed = true;
          break;
        }
      }
      if (flushFailed && offlineBuffer.length > 0) {
        console.warn(`[collab] ${offlineBuffer.length} ops remain in buffer after flush`);
      }
      // Clear persisted buffer after successful flush (Bug C1)
      if (offlineBuffer.length === 0) {
        try { sessionStorage.removeItem('rudra_collab_offline_buffer'); } catch (_) {}
      }
    }
    // Reset buffer warning flags after successful reconnect
    _bufferWarningShown = false;
    _removeBufferWarningBanner();
    // D5 fix: Re-enable editing if it was disabled due to buffer overflow
    if (_bufferFullWarningShown) {
      state.readOnlyMode = false;
      _removeBufferFullBanner();
    }
    _bufferFullWarningShown = false;
    updateSyncStatus('synced');
  };

  ws.onmessage = (event) => {
    // Bug C6: Any message from server resets the missed pong counter
    missedPongs = 0;
    try {
      const msg = JSON.parse(event.data);
      // Track server version from every message
      if (msg.serverVersion && msg.serverVersion > serverVersion) {
        // Gap detection: if we skipped versions, we missed messages
        if (msg.serverVersion > serverVersion + 1) {
          console.warn('[collab] Version gap detected:', serverVersion, '\u2192', msg.serverVersion);
          // Request a fullSync from any peer that is up to date
          sendOp({ action: 'requestFullSync', myVersion: serverVersion });
          // Timeout for fullSync response (Bug C3)
          clearTimeout(_fullSyncTimeout);
          _fullSyncTimeout = setTimeout(() => {
            console.warn('[collab] fullSync timeout — retrying...');
            _fullSyncTimeout = null;
            // Retry once before forcing reconnect
            if (ws && ws.readyState === WebSocket.OPEN) {
              ws.send(JSON.stringify({ type: 'requestFullSync', room: roomId }));
              _fullSyncTimeout = setTimeout(() => {
                console.warn('[collab] fullSync retry timeout — forcing reconnect');
                _fullSyncTimeout = null;
                if (ws) ws.close();
              }, 5000);
            } else if (ws) {
              ws.close();
            }
          }, 5000);
        }
        serverVersion = msg.serverVersion;
      }
      handleMessage(msg);
    } catch (e) {
      console.error('[collab] Message handling error:', e, 'raw:', event.data?.substring?.(0, 200));
    }
  };

  ws.onclose = () => {
    clearTimeout(connectTimeout);
    clearTimeout(_fullSyncTimeout); // Clear fullSync timeout on disconnect (Bug C3)
    _fullSyncTimeout = null;
    connected = false;
    updateConnectionStatus('disconnected');
    updateSyncStatus('offline');
    if (roomId) scheduleReconnect(url);
  };

  ws.onerror = () => {
    clearTimeout(connectTimeout);
  };
}

function scheduleReconnect(url) {
  if (!roomId) return;
  if (reconnectAttempt >= MAX_RECONNECT_ATTEMPTS) {
    updateConnectionStatus('disconnected');
    return; // Silently give up
  }
  const delay = RECONNECT_DELAYS[Math.min(reconnectAttempt, RECONNECT_DELAYS.length - 1)];
  reconnectAttempt++;
  updateConnectionStatus('reconnecting');
  reconnectTimer = setTimeout(() => connect(url), delay);
}

const MAX_OFFLINE_BUFFER = 10000;
const OFFLINE_BUFFER_WARNING_THRESHOLD = 8000;
let _bufferWarningShown = false;
let _bufferFullWarningShown = false;

// Bug C17: Client-side rate limiting — max 60 ops/second (~16ms between sends)
let _sendCount = 0;
let _sendResetTimer = null;
let _rateLimitBuffer = []; // Ops deferred due to rate limiting (flushed on window reset)

function _flushRateLimitBuffer() {
  // Flush rate-limited ops on the next window
  while (_rateLimitBuffer.length > 0 && _sendCount < 60) {
    const deferred = _rateLimitBuffer.shift();
    _sendCount++;
    const payload = JSON.stringify(deferred);
    if (connected && ws && ws.readyState === 1) {
      try { ws.send(JSON.stringify({ type: 'op', room: roomId, data: payload })); } catch (_) {}
    }
  }
}

function sendOp(opData) {
  // Bug C14: Attach idempotency token for future server-side deduplication
  opData._opId = opData._opId || (Date.now().toString(36) + Math.random().toString(36).slice(2, 8));

  // Bug C17: Rate limit outgoing ops (max 60/second)
  _sendCount++;
  if (!_sendResetTimer) {
    _sendResetTimer = setTimeout(() => {
      _sendCount = 0;
      _sendResetTimer = null;
      _flushRateLimitBuffer();
    }, 1000);
  }
  if (_sendCount > 60) {
    // Queue rate-limited ops for next window instead of dropping to offline buffer
    if (_rateLimitBuffer.length < 200) {
      _rateLimitBuffer.push(opData);
    } else if (offlineBuffer.length < MAX_OFFLINE_BUFFER) {
      opData._localVersion = localVersion;
      offlineBuffer.push(opData);
    }
    return;
  }

  const payload = JSON.stringify(opData);
  if (connected && ws && ws.readyState === 1) {
    ws.send(JSON.stringify({ type: 'op', room: roomId, data: payload }));
  } else {
    if (offlineBuffer.length < MAX_OFFLINE_BUFFER) {
      // Tag with local version for fullSync replay ordering (Bug C4)
      opData._localVersion = localVersion;
      offlineBuffer.push(opData);
      // Persist to sessionStorage for crash recovery (Bug C1)
      try {
        const serialized = JSON.stringify(offlineBuffer);
        sessionStorage.setItem('rudra_collab_offline_buffer', serialized);
        // C13: Warn if buffer is getting large (risk of sessionStorage overflow at ~5MB)
        if (serialized.length > 2 * 1024 * 1024) {
          console.warn('[collab] Offline buffer exceeds 2MB — risk of sessionStorage overflow');
        }
      } catch (e) {
        console.warn('[collab] Cannot persist offline buffer:', e.message);
      }
      // Warn when buffer is near capacity — show persistent warning banner
      if (offlineBuffer.length >= OFFLINE_BUFFER_WARNING_THRESHOLD && !_bufferWarningShown) {
        _bufferWarningShown = true;
        _showBufferWarningBanner();
      }
    } else {
      if (!_bufferFullWarningShown) {
        _bufferFullWarningShown = true;
        console.warn('Offline buffer limit reached. Blocking further edits to prevent silent data loss.');
        // D5 fix: Show persistent warning and block editing to prevent silent data loss
        _showBufferFullBanner();
        state.readOnlyMode = true;
        // Update sync status to show critical state
        const syncEl = $('collabSyncStatus');
        if (syncEl) {
          syncEl.textContent = 'Offline (editing disabled)';
          syncEl.className = 'collab-sync-status offline';
        }
      }
      return; // Don't silently drop operations
    }
    // Update offline pending count in sync status
    updateSyncStatus('offline');
  }
}

/**
 * Update the sync status indicator in the status bar.
 * @param {'synced'|'syncing'|'offline'} status
 */
function updateSyncStatus(status) {
  const syncEl = $('collabSyncStatus');
  if (!syncEl || !roomId) return;

  if (status === 'synced') {
    syncEl.textContent = 'Synced';
    syncEl.className = 'collab-sync-status synced';
  } else if (status === 'syncing') {
    syncEl.textContent = 'Syncing...';
    syncEl.className = 'collab-sync-status syncing';
  } else if (status === 'offline') {
    const pending = offlineBuffer.length;
    syncEl.textContent = pending > 0 ? `Offline (${pending} pending)` : 'Offline';
    syncEl.className = 'collab-sync-status offline';
  }
  syncEl.style.display = 'inline-flex';
}

// ─── Message Handlers ─────────────────────────────────

function handleMessage(msg) {
  switch (msg.type) {
    case 'snapshot': {
      // Server sends latest document snapshot (base64) to new joiners.
      // Only apply if we don't already have a document loaded
      // (the sharer already has the doc; joiners get it from checkAutoJoin).
      // Skip if doc is already open to avoid overwriting local edits.
      if (msg.data && state.engine && !state.doc) {
        try {
          const binary = atob(msg.data);
          const bytes = new Uint8Array(binary.length);
          for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
          if (bytes.length > 0) {
            state.doc = state.engine.open(bytes);
            renderDocument();
            tracing('Applied server snapshot:', msg.size, 'bytes');
          }
        } catch (e) {
          console.warn('Failed to apply server snapshot:', e);
        }
      }
      break;
    }

    case 'catchUp': {
      // Server sends catch-up ops that were recorded while this peer was absent.
      // These ops were sent by other peers and recorded in the room's ops log.
      if (msg.op) {
        try {
          const opStr = typeof msg.op === 'string' ? msg.op : JSON.stringify(msg.op);
          // Parse the relay message envelope to get the inner op data
          const envelope = JSON.parse(opStr);
          if (envelope.data) {
            applyRemoteOp(envelope.data, null);
          } else if (envelope.action) {
            // Direct op format
            applyRemoteOp(opStr, null);
          }
        } catch (e) {
          console.warn('Failed to apply catch-up op:', e);
        }
      }
      break;
    }

    case 'joined':
      peerId = msg.peerId;
      _roomFullRetries = 0; // C11: Reset retry counter on successful join
      updatePeerList(msg.peers || []);
      updateCollabUI();
      // Version check on reconnect
      if (msg.serverVersion && msg.serverVersion > serverVersion) {
        tracing('Behind server version:', serverVersion, '\u2192', msg.serverVersion);
      }
      serverVersion = msg.serverVersion || serverVersion;
      tracing('Joined room, peerId:', peerId, 'peers:', (msg.peers || []).length);
      // Enforce access level from server (view/comment/edit)
      if (msg.access === 'view' || msg.access === 'comment') {
        state.readOnlyMode = true;
        // Disable contenteditable on all pages
        document.querySelectorAll('.page-content').forEach(p => { p.contentEditable = 'false'; });
        // Disable toolbar
        const toolbar = $('toolbar');
        if (toolbar) toolbar.style.pointerEvents = 'none';
        tracing('Access level:', msg.access, '— editor in read-only mode');
      }
      // If CRDT available, send state vector for delta sync
      if (state.collabDoc && (msg.peers || []).length > 0) {
        try {
          const sv = state.collabDoc.get_state_vector();
          sendOp({ action: 'stateVector', sv });
          _deltaSyncReceived = false;
        } catch (_) {}
        // C15: Fallback — if delta sync doesn't deliver within 10s, request fullSync.
        // Extended from 5s to 10s to give delta exchange more time with 3+ peers.
        // Only triggers if no delta ops were received in the meantime.
        setTimeout(() => {
          if (roomId && connected && !_deltaSyncReceived) {
            tracing('Delta sync timeout — requesting fullSync fallback');
            requestImmediateFullSync();
          }
        }, 10000);
      }
      break;

    case 'peer-join':
      // Only handle if it's a different peer (not self)
      if (msg.peerId === peerId) break;
      addPeer(msg.peerId, msg.userName, msg.userColor);
      // Send CRDT delta if available — prefer delta over fullSync to
      // reduce bandwidth and avoid disrupting other peers' editing context.
      if (state.collabDoc) {
        try {
          const sv = state.collabDoc.get_state_vector();
          sendOp({ action: 'stateVector', sv });
        } catch (_) {}
      }
      // C15: Send fullSync as fallback for the new peer, but use a short
      // delay to give delta sync a chance to work first.
      setTimeout(() => {
        if (roomId && connected) {
          sendFullSync();
        }
      }, 2000);
      break;

    case 'stateVector': {
      // Peer sent their state vector — respond with CRDT ops they're missing
      if (state.collabDoc && msg.sv) {
        try {
          const changes = state.collabDoc.get_changes_since(typeof msg.sv === 'string' ? msg.sv : JSON.stringify(msg.sv));
          if (changes && changes !== '[]') {
            // Send each change as a CRDT op
            const ops = JSON.parse(changes);
            for (const op of ops) {
              sendOp({ action: 'crdtOp', ops: JSON.stringify(op) });
            }
            tracing('Sent', ops.length, 'CRDT delta ops to new peer');
          }
        } catch (e) { console.warn('State vector delta sync error:', e); }
      }
      break;
    }

    case 'peer-leave':
      removePeer(msg.peerId);
      break;

    case 'op':
      applyRemoteOp(msg.data, msg.peerId);
      break;

    case 'awareness':
      applyRemoteAwareness(msg.data, msg.peerId);
      break;

    case 'sync-resp':
      applySyncResponse(msg.ops || []);
      break;

    case 'error':
      console.error('Relay error:', msg.message);
      // C11: Room full — exponential backoff retry via full reconnect
      // (Server closes the WebSocket after room_full, so we must reconnect)
      if (msg.message && msg.message.includes('full')) {
        const retryDelay = Math.min(30000, 2000 * Math.pow(2, _roomFullRetries));
        _roomFullRetries++;
        if (_roomFullRetries <= 5) {
          showCollabToast(`Room is full. Retrying in ${Math.round(retryDelay / 1000)}s...`);
          // Suppress normal reconnect — we'll handle it with our own timer
          reconnectAttempt = MAX_RECONNECT_ATTEMPTS;
          setTimeout(() => {
            if (roomId) {
              reconnectAttempt = 0; // Allow reconnect to proceed
              connect(lastRelayUrl || DEFAULT_RELAY_URL);
            }
          }, retryDelay);
        } else {
          showCollabToast('Room is full. Please try again later.');
          _roomFullRetries = 0;
        }
        return;
      }
      break;
  }
}

function tracing(...args) {
  if (console.debug) console.debug('[collab]', ...args);
}

/**
 * Calculate character offset from the start of a paragraph element.
 * sel.anchorOffset is relative to sel.anchorNode (a text node inside a span),
 * so we need to walk text nodes to find the paragraph-level offset.
 */
function getParaOffset(paraEl, targetNode, nodeOffset) {
  let offset = 0;
  const walker = document.createTreeWalker(paraEl, NodeFilter.SHOW_TEXT, null);
  let textNode;
  while ((textNode = walker.nextNode())) {
    if (textNode === targetNode) {
      return offset + nodeOffset;
    }
    offset += textNode.textContent.length;
  }
  // Fallback: if targetNode is an element, estimate from child index
  return offset + nodeOffset;
}

// ─── Editing Context Save/Restore ─────────────────────
// Preserves cursor, selection, scroll position, and active formatting
// across fullSync document replacements to maintain UX continuity.

/** Save full editing context before a document replacement. */
function _saveEditingContext() {
  const ctx = { nodeId: null, offset: 0, scrollTop: 0, scrollLeft: 0 };
  try {
    const sel = window.getSelection();
    if (sel && sel.rangeCount > 0) {
      let el = sel.anchorNode;
      while (el && el !== document && !el.dataset?.nodeId) el = el.parentElement;
      ctx.nodeId = el?.dataset?.nodeId || null;
      if (ctx.nodeId && el) {
        ctx.offset = getParaOffset(el, sel.anchorNode, sel.anchorOffset);
      }
    }
  } catch (_) {}
  try {
    const container = $('pageContainer')?.parentElement;
    if (container) {
      ctx.scrollTop = container.scrollTop;
      ctx.scrollLeft = container.scrollLeft;
    }
  } catch (_) {}
  return ctx;
}

/** Restore editing context after a document replacement + re-render. */
function _restoreEditingContext(ctx) {
  if (!ctx) return;
  // Restore scroll position
  try {
    const container = $('pageContainer')?.parentElement;
    if (container) {
      container.scrollTop = ctx.scrollTop;
      container.scrollLeft = ctx.scrollLeft;
    }
  } catch (_) {}
  // Restore cursor position
  if (ctx.nodeId) {
    _restoreSelectionToNode(ctx.nodeId, ctx.offset);
  }
}

// Bug C19: Legacy helpers (kept for inline fullDocSync handler compatibility)
function _getSelectionNodeId() {
  try {
    const sel = window.getSelection();
    if (!sel || sel.rangeCount === 0) return null;
    let el = sel.anchorNode;
    while (el && el !== document && !el.dataset?.nodeId) el = el.parentElement;
    return el?.dataset?.nodeId || null;
  } catch (_) { return null; }
}

function _getSelectionOffset() {
  try {
    const sel = window.getSelection();
    if (!sel || sel.rangeCount === 0) return 0;
    let paraEl = sel.anchorNode;
    while (paraEl && paraEl !== document && !paraEl.dataset?.nodeId) paraEl = paraEl.parentElement;
    if (!paraEl) return sel.anchorOffset;
    return getParaOffset(paraEl, sel.anchorNode, sel.anchorOffset);
  } catch (_) { return 0; }
}

function _restoreSelectionToNode(nodeId, offset) {
  try {
    const page = $('pageContainer');
    if (!page) return;
    const paraEl = page.querySelector(`[data-node-id="${nodeId}"]`);
    if (!paraEl) return;
    // Walk text nodes to find the right position
    let remaining = offset || 0;
    const walker = document.createTreeWalker(paraEl, NodeFilter.SHOW_TEXT, null);
    let textNode;
    while ((textNode = walker.nextNode())) {
      if (textNode.parentElement?.closest('.peer-cursor')) continue;
      const len = textNode.textContent.length;
      if (remaining <= len) {
        const range = document.createRange();
        range.setStart(textNode, Math.min(remaining, len));
        range.collapse(true);
        const sel = window.getSelection();
        sel.removeAllRanges();
        sel.addRange(range);
        return;
      }
      remaining -= len;
    }
  } catch (_) {}
}

/** Quick hash of doc plain text for skip-if-identical optimization. */
function _quickDocHash() {
  try {
    const text = state.doc?.to_plain_text?.() || '';
    // Simple FNV-1a style hash for fast comparison
    let hash = 0x811c9dc5;
    for (let i = 0; i < text.length; i++) {
      hash ^= text.charCodeAt(i);
      hash = (hash * 0x01000193) >>> 0;
    }
    return hash;
  } catch (_) { return 0; }
}
let _lastAppliedDocHash = 0;

function sendFullSync() {
  if (!state.doc) return;
  try {
    const bytes = state.doc.export('docx');
    const base64 = btoa(String.fromCharCode(...new Uint8Array(bytes)));
    // D17: Only increment version after successful send
    const pendingVersion = localVersion + 1;
    sendOp({ action: 'fullSync', docBase64: base64, version: pendingVersion, replicaId: peerId });
    localVersion = pendingVersion;
  } catch (_) {}
}

// Bug C14: Track recently seen op IDs to prevent duplicate application on retry
const _seenOpIds = new Set();
const MAX_SEEN_OP_IDS = 2000;
let _seenOpIdCount = 0;

function _trackOpId(opId) {
  if (!opId) return false; // no ID — can't dedup
  if (_seenOpIds.has(opId)) return true; // duplicate
  _seenOpIds.add(opId);
  _seenOpIdCount++;
  // Evict oldest entries when set grows too large
  if (_seenOpIdCount > MAX_SEEN_OP_IDS) {
    const iter = _seenOpIds.values();
    for (let i = 0; i < MAX_SEEN_OP_IDS / 2; i++) {
      _seenOpIds.delete(iter.next().value);
    }
    _seenOpIdCount = _seenOpIds.size;
  }
  return false;
}

function applyRemoteOp(dataStr, fromPeerId) {
  if (!state.doc || !dataStr) return;

  // X18: If undo/redo is in progress, defer remote ops to avoid race conditions
  if (state._applyingUndo) {
    _deferredRemoteOps.push({ dataStr, fromPeerId });
    return;
  }

  try {
    const op = JSON.parse(dataStr);

    // Bug C14: Deduplicate ops using _opId to prevent double application on retry
    if (op._opId && _trackOpId(op._opId)) {
      return; // Already applied this op
    }

    applyingRemote = true;

    // Track peer typing activity for presence indicator
    if (fromPeerId && fromPeerId !== peerId) {
      trackPeerTyping(fromPeerId);
    }

    // Flash the affected paragraph for visual feedback (no toast — too noisy)
    if (fromPeerId && fromPeerId !== peerId) {
      const affectedNodeId = op.nodeId || op.startNode;
      if (affectedNodeId) {
        flashParagraph(affectedNodeId);
      }
    }

    switch (op.action) {
      case 'crdtOp': {
        // CRDT operation from a peer — apply via WasmCollabDocument
        _deltaSyncReceived = true; // C15: mark that delta sync is working
        if (state.collabDoc && op.ops) {
          try {
            const opsStr = typeof op.ops === 'string' ? op.ops : JSON.stringify(op.ops);
            const parsed = JSON.parse(opsStr);
            const opsList = Array.isArray(parsed) ? parsed : [parsed];

            // Track affected node IDs for targeted re-render
            const affectedNodes = new Set();
            for (const singleOp of opsList) {
              state.collabDoc.apply_remote_ops(JSON.stringify(singleOp));
              // Extract nodeId from the CRDT operation for targeted render
              const innerOp = singleOp.operation || singleOp;
              const nid = innerOp.nodeId || innerOp.target_id || innerOp.targetId;
              if (nid) affectedNodes.add(typeof nid === 'string' ? nid : `${nid.replica || 0}:${nid.counter || 0}`);
            }

            // Also sync the non-collab doc model for rendering
            if (state.doc && state.collabDoc) {
              try {
                const bytes = state.collabDoc.export('docx');
                state.doc = state.engine.open(new Uint8Array(bytes));
              } catch (_) {
                // If export fails, do a full render from collab HTML
              }
            }

            // Try incremental render if only one node affected
            if (affectedNodes.size === 1) {
              const nodeId = affectedNodes.values().next().value;
              if (!renderNodeById(nodeId)) renderDocument();
            } else {
              renderDocument();
            }
          } catch (e) { console.error('CRDT remote op:', e); }
        }
        break;
      }

      case 'setText': {
        // Set paragraph text from remote
        try {
          state.doc.set_paragraph_text(op.nodeId, op.text);
          const pageContainer = $('pageContainer');
          if (pageContainer) {
            const el = pageContainer.querySelector(`[data-node-id="${op.nodeId}"]`);
            if (el) {
              renderNodeById(op.nodeId);
              // Don't move local cursor
            }
          }
        } catch (e) { requestImmediateFullSync(); console.debug('[collab] remote op skipped (node ID mismatch, fullSync will correct):setText:', e); }
        break;
      }

      case 'splitParagraph': {
        try {
          const newId = state.doc.split_paragraph(op.nodeId, op.offset);
          renderDocument();
        } catch (e) { requestImmediateFullSync(); console.debug('[collab] remote op skipped (node ID mismatch, fullSync will correct):split:', e); }
        break;
      }

      case 'mergeParagraphs': {
        try {
          state.doc.merge_paragraphs(op.nodeId1, op.nodeId2);
          renderDocument();
        } catch (e) { requestImmediateFullSync(); console.debug('[collab] remote op skipped (node ID mismatch, fullSync will correct):merge:', e); }
        break;
      }

      case 'formatSelection': {
        try {
          state.doc.format_selection(
            op.startNode, op.startOffset,
            op.endNode, op.endOffset,
            op.key, op.value
          );
          renderNodeById(op.startNode);
          if (op.endNode !== op.startNode) renderNodeById(op.endNode);
        } catch (e) { requestImmediateFullSync(); console.debug('[collab] remote op skipped (node ID mismatch, fullSync will correct):format:', e); }
        break;
      }

      case 'deleteSelection': {
        try {
          state.doc.delete_selection(op.startNode, op.startOffset, op.endNode, op.endOffset);
          renderDocument();
        } catch (e) { requestImmediateFullSync(); console.debug('[collab] remote op skipped (node ID mismatch, fullSync will correct):delete:', e); }
        break;
      }

      case 'setHeading': {
        try {
          state.doc.set_heading_level(op.nodeId, op.level);
          renderNodeById(op.nodeId);
        } catch (e) { requestImmediateFullSync(); console.debug('[collab] remote op skipped (node ID mismatch, fullSync will correct):heading:', e); }
        break;
      }

      case 'setAlignment': {
        try {
          state.doc.set_alignment(op.nodeId, op.alignment);
          renderNodeById(op.nodeId);
        } catch (e) { requestImmediateFullSync(); console.debug('[collab] remote op skipped (node ID mismatch, fullSync will correct):align:', e); }
        break;
      }

      case 'insertParagraph': {
        try {
          state.doc.insert_paragraph_after(op.afterNodeId, op.text || '');
          renderDocument();
        } catch (e) { requestImmediateFullSync(); console.debug('[collab] remote op skipped (node ID mismatch, fullSync will correct):insertPara:', e); }
        break;
      }

      case 'deleteNode': {
        try {
          state.doc.delete_node(op.nodeId);
          renderDocument();
        } catch (e) { requestImmediateFullSync(); console.debug('[collab] remote op skipped (node ID mismatch, fullSync will correct):deleteNode:', e); }
        break;
      }

      case 'setListFormat': {
        try {
          state.doc.set_list_format(op.nodeId, op.format, op.level || 0);
          renderDocument();
        } catch (e) { requestImmediateFullSync(); console.debug('[collab] remote op skipped (node ID mismatch, fullSync will correct):list:', e); }
        break;
      }

      case 'setIndent': {
        try {
          state.doc.set_indent(op.nodeId, op.side, op.value);
          renderNodeById(op.nodeId);
        } catch (e) { requestImmediateFullSync(); console.debug('[collab] remote op skipped (node ID mismatch, fullSync will correct):indent:', e); }
        break;
      }

      case 'setLineSpacing': {
        try {
          state.doc.set_line_spacing(op.nodeId, op.value);
          renderNodeById(op.nodeId);
        } catch (e) { requestImmediateFullSync(); console.debug('[collab] remote op skipped (node ID mismatch, fullSync will correct):lineSpacing:', e); }
        break;
      }

      case 'insertTable': {
        try {
          state.doc.insert_table(op.afterNodeId, op.rows, op.cols);
          renderDocument();
        } catch (e) { requestImmediateFullSync(); console.debug('[collab] remote op skipped (node ID mismatch, fullSync will correct):insertTable:', e); }
        break;
      }

      case 'insertTableRow': {
        try {
          state.doc.insert_table_row(op.tableId, op.index);
          renderDocument();
        } catch (e) { requestImmediateFullSync(); console.debug('[collab] remote op skipped (node ID mismatch, fullSync will correct):insertRow:', e); }
        break;
      }

      case 'deleteTableRow': {
        try {
          state.doc.delete_table_row(op.tableId, op.index);
          renderDocument();
        } catch (e) { requestImmediateFullSync(); console.debug('[collab] remote op skipped (node ID mismatch, fullSync will correct):deleteRow:', e); }
        break;
      }

      case 'insertTableColumn': {
        try {
          state.doc.insert_table_column(op.tableId, op.index);
          renderDocument();
        } catch (e) { requestImmediateFullSync(); console.debug('[collab] remote op skipped (node ID mismatch, fullSync will correct):insertCol:', e); }
        break;
      }

      case 'deleteTableColumn': {
        try {
          state.doc.delete_table_column(op.tableId, op.index);
          renderDocument();
        } catch (e) { requestImmediateFullSync(); console.debug('[collab] remote op skipped (node ID mismatch, fullSync will correct):deleteCol:', e); }
        break;
      }

      case 'insertHR': {
        try {
          state.doc.insert_horizontal_rule(op.afterNodeId);
          renderDocument();
        } catch (e) { requestImmediateFullSync(); console.debug('[collab] remote op skipped (node ID mismatch, fullSync will correct):insertHR:', e); }
        break;
      }

      case 'insertPageBreak': {
        try {
          state.doc.insert_page_break(op.afterNodeId);
          renderDocument();
        } catch (e) { requestImmediateFullSync(); console.debug('[collab] remote op skipped (node ID mismatch, fullSync will correct):insertPageBreak:', e); }
        break;
      }

      case 'insertSectionBreak': {
        try {
          state.doc.insert_section_break(op.afterNodeId, op.breakType || 'nextPage');
          renderDocument();
        } catch (e) { requestImmediateFullSync(); console.debug('[collab] remote op skipped (node ID mismatch, fullSync will correct):insertSectionBreak:', e); }
        break;
      }

      case 'insertImage': {
        // Image binary data can't be easily synced via WebSocket;
        // full sync handles this instead
        renderDocument();
        break;
      }

      case 'moveNodeBefore': {
        try {
          state.doc.move_node_before(op.nodeId, op.beforeId);
          renderDocument();
        } catch (e) { requestImmediateFullSync(); console.debug('[collab] remote op skipped (node ID mismatch, fullSync will correct):moveBefore:', e); }
        break;
      }

      case 'moveNodeAfter': {
        try {
          state.doc.move_node_after(op.nodeId, op.afterId);
          renderDocument();
        } catch (e) { requestImmediateFullSync(); console.debug('[collab] remote op skipped (node ID mismatch, fullSync will correct):moveAfter:', e); }
        break;
      }

      case 'resizeImage': {
        try {
          state.doc.resize_image(op.nodeId, op.width, op.height);
          renderDocument();
        } catch (e) { requestImmediateFullSync(); console.debug('[collab] remote op skipped (node ID mismatch, fullSync will correct):resizeImage:', e); }
        break;
      }

      case 'setCellBackground': {
        try {
          state.doc.set_cell_background(op.cellId, op.color);
          renderDocument();
        } catch (e) { requestImmediateFullSync(); console.debug('[collab] remote op skipped (node ID mismatch, fullSync will correct):setCellBg:', e); }
        break;
      }

      case 'setImageAltText': {
        try {
          state.doc.set_image_alt_text(op.nodeId, op.alt);
          renderDocument();
        } catch (e) { requestImmediateFullSync(); console.debug('[collab] remote op skipped (node ID mismatch, fullSync will correct):altText:', e); }
        break;
      }

      case 'insertLineBreak': {
        try {
          state.doc.insert_line_break(op.nodeId, op.offset);
          renderNodeById(op.nodeId);
        } catch (e) { requestImmediateFullSync(); console.debug('[collab] remote op skipped (node ID mismatch, fullSync will correct):insertLineBreak:', e); }
        break;
      }

      case 'pasteText': {
        try {
          state.doc.paste_plain_text(op.nodeId, op.offset, op.text);
          renderDocument();
        } catch (e) { requestImmediateFullSync(); console.debug('[collab] remote op skipped (node ID mismatch, fullSync will correct):pasteText:', e); }
        break;
      }

      case 'pasteFormattedRuns': {
        try {
          state.doc.paste_formatted_runs_json(op.nodeId, op.offset, op.runsJson);
          renderDocument();
        } catch (e) { requestImmediateFullSync(); console.debug('[collab] remote op skipped (node ID mismatch, fullSync will correct):pasteFormattedRuns:', e); }
        break;
      }

      case 'insertText': {
        try {
          state.doc.insert_text_in_paragraph(op.nodeId, op.offset, op.text);
          renderNodeById(op.nodeId);
        } catch (e) { requestImmediateFullSync(); console.debug('[collab] remote op skipped (node ID mismatch, fullSync will correct):insertText:', e); }
        break;
      }

      case 'replaceText': {
        try {
          state.doc.replace_text(op.nodeId, op.offset, op.length, op.replacement);
          renderDocument();
        } catch (e) { requestImmediateFullSync(); console.debug('[collab] remote op skipped (node ID mismatch, fullSync will correct):replaceText:', e); }
        break;
      }

      case 'replaceAll': {
        try {
          state.doc.replace_all(op.query, op.replacement, op.caseInsensitive);
          renderDocument();
        } catch (e) { requestImmediateFullSync(); console.debug('[collab] remote op skipped (node ID mismatch, fullSync will correct):replaceAll:', e); }
        break;
      }

      case 'insertComment': {
        try {
          state.doc.insert_comment(op.startNodeId, op.endNodeId, op.author, op.text);
          renderDocument();
        } catch (e) { requestImmediateFullSync(); console.debug('[collab] remote op skipped (node ID mismatch, fullSync will correct):insertComment:', e); }
        break;
      }

      case 'deleteComment': {
        try {
          state.doc.delete_comment(op.commentId);
          renderDocument();
        } catch (e) { requestImmediateFullSync(); console.debug('[collab] remote op skipped (node ID mismatch, fullSync will correct):deleteComment:', e); }
        break;
      }

      case 'acceptChange': {
        try {
          state.doc.accept_change(op.nodeId);
          renderDocument();
        } catch (e) { requestImmediateFullSync(); console.debug('[collab] remote op skipped (node ID mismatch, fullSync will correct):acceptChange:', e); }
        break;
      }

      case 'rejectChange': {
        try {
          state.doc.reject_change(op.nodeId);
          renderDocument();
        } catch (e) { requestImmediateFullSync(); console.debug('[collab] remote op skipped (node ID mismatch, fullSync will correct):rejectChange:', e); }
        break;
      }

      case 'acceptAllChanges': {
        try {
          state.doc.accept_all_changes();
          renderDocument();
        } catch (e) { requestImmediateFullSync(); console.debug('[collab] remote op skipped (node ID mismatch, fullSync will correct):acceptAllChanges:', e); }
        break;
      }

      case 'rejectAllChanges': {
        try {
          state.doc.reject_all_changes();
          renderDocument();
        } catch (e) { requestImmediateFullSync(); console.debug('[collab] remote op skipped (node ID mismatch, fullSync will correct):rejectAllChanges:', e); }
        break;
      }

      case 'requestFullSync': {
        // Another peer is behind — send our full state if we're ahead
        if (op.myVersion < serverVersion) {
          sendFullSync();
        }
        break;
      }

      case 'fullDocSync': {
        // C16: Peer performed undo/redo. If they included their doc state inline,
        // apply it directly without a round-trip requestFullSync. This preserves
        // cursor continuity and reduces convergence latency.
        if (op.docBase64 && state.engine) {
          try {
            const binary = atob(op.docBase64);
            const bytes = new Uint8Array(binary.length);
            for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
            if (bytes.length > 0) {
              const savedNodeId = _getSelectionNodeId();
              const savedOffset = _getSelectionOffset();
              state.doc = state.engine.open(bytes);
              renderDocument();
              if (savedNodeId) _restoreSelectionToNode(savedNodeId, savedOffset);
              tracing('Applied inline undo/redo sync', bytes.length, 'bytes');
            }
          } catch (e) {
            console.debug('[collab] inline fullDocSync failed, requesting full sync:', e);
            requestImmediateFullSync();
          }
        } else {
          // Legacy fallback: peer didn't include doc state, request it
          requestImmediateFullSync();
        }
        break;
      }

      case 'fullSync': {
        // Clear fullSync timeout since we received a response (Bug C3)
        clearTimeout(_fullSyncTimeout);
        _fullSyncTimeout = null;
        // Full document sync with version tracking.
        // Only apply if the incoming version is newer than what we last synced.
        try {
          const incomingVersion = op.version || 0;
          // Reject stale fullSync (Bug C4) — check both lastSyncedVersion and serverVersion
          if (incomingVersion > 0 && incomingVersion <= serverVersion) {
            console.warn(`[collab] Ignoring stale fullSync (v${incomingVersion} <= current server v${serverVersion})`);
            break;
          }
          if (incomingVersion > 0 && incomingVersion <= lastSyncedVersion) {
            tracing('Ignoring stale fullSync v' + incomingVersion + ' (have v' + lastSyncedVersion + ')');
            break;
          }
          if (op.docBase64 && state.engine) {
            // Snapshot any pending local ops before overwriting (Bug C4)
            const pendingLocalOps = offlineBuffer.filter(o => o._localVersion > incomingVersion);
            const binary = atob(op.docBase64);
            const bytes = new Uint8Array(binary.length);
            for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
            if (bytes.length > 0) {
              // Skip replacement if incoming doc matches current state (no-op optimization).
              // This avoids unnecessary re-renders, cursor resets, and scroll jumps
              // when the fullSync contains the same document we already have.
              const incomingDoc = state.engine.open(bytes);
              const incomingText = incomingDoc.to_plain_text();
              const localText = state.doc?.to_plain_text?.() || '';
              if (incomingText === localText && pendingLocalOps.length === 0) {
                tracing('Skipping no-op fullSync v' + incomingVersion, '(content identical)');
                if (incomingVersion > 0) lastSyncedVersion = incomingVersion;
                break;
              }

              // Save full editing context (cursor, scroll) before replacement
              const editCtx = _saveEditingContext();
              state.doc = incomingDoc;
              if (incomingVersion > 0) lastSyncedVersion = incomingVersion;
              _lastAppliedDocHash = _quickDocHash();
              renderDocument();
              // Restore full editing context after re-render
              _restoreEditingContext(editCtx);
              tracing('Applied fullSync v' + incomingVersion, bytes.length, 'bytes');
              // Re-apply any locally-sent ops that were sent after the sync version (Bug C4)
              if (pendingLocalOps.length > 0) {
                tracing('Replaying', pendingLocalOps.length, 'local ops after fullSync');
                for (const pendingOp of pendingLocalOps) {
                  try {
                    applyRemoteOp(JSON.stringify(pendingOp), peerId);
                  } catch (_) {}
                }
              }
            }
          }
        } catch (e) { console.debug('[collab] fullSync error:', e); }
        break;
      }

      // S4.1: Spreadsheet collaboration ops — forwarded to the active SpreadsheetView
      case 'ssSetCell':
      case 'ssFormat':
      case 'ssSync':
      case 'ssCursor': {
        try {
          if (state.spreadsheetView && typeof state.spreadsheetView._applyRemoteSSop === 'function') {
            state.spreadsheetView._applyRemoteSSop(op, fromPeerId);
          }
        } catch (e) { console.debug('[collab] spreadsheet op error:', e); }
        break;
      }

      default:
        console.warn('Unknown remote op:', op.action);
    }

    applyingRemote = false;
  } catch (e) {
    applyingRemote = false;
    console.warn('[collab] Remote op failed, requesting full sync:', e.message);
    // Request full document sync to converge instead of silently diverging
    if (ws && ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({ type: 'requestFullSync', room: state.collabRoom }));
    }
  }
}

function applySyncResponse(ops) {
  for (const opStr of ops) {
    applyRemoteOp(opStr, null);
  }
}

// ─── Cursor Awareness ─────────────────────────────────

function startCursorBroadcast() {
  clearInterval(cursorTimer);
  cursorTimer = setInterval(broadcastCursor, CURSOR_BROADCAST_INTERVAL);
}

function broadcastCursor() {
  if (!connected || !ws || !roomId) return;

  // Use cursor broadcast as a heartbeat — if WebSocket is dead, detect and reconnect
  if (ws.readyState !== WebSocket.OPEN) {
    connected = false;
    updateConnectionStatus('disconnected');
    scheduleReconnect(lastRelayUrl || DEFAULT_RELAY_URL);
    return;
  }

  // Bug C6: If no message received in 3 heartbeat intervals, force reconnect
  if (missedPongs >= 3) {
    console.warn('[collab] No server response for 3 heartbeats, reconnecting...');
    missedPongs = 0;
    try { ws.close(); } catch (_) {}
    return;
  }
  missedPongs++;

  const sel = window.getSelection();

  // For view-only users or when no selection exists, send a heartbeat
  // so the server knows we're still connected
  if (!sel || sel.rangeCount === 0) {
    try {
      ws.send(JSON.stringify({ type: 'awareness', room: roomId,
        data: JSON.stringify({ peerId, userName, userColor, heartbeat: true, clientVersion: serverVersion }) }));
    } catch (_) {}
    return;
  }

  // Find the paragraph element containing the cursor
  let paraEl = sel.anchorNode;
  while (paraEl && paraEl !== document && !paraEl.dataset?.nodeId) {
    paraEl = paraEl.parentElement;
  }
  if (!paraEl || !paraEl.dataset?.nodeId) {
    // No valid paragraph — still send heartbeat
    try {
      ws.send(JSON.stringify({ type: 'awareness', room: roomId,
        data: JSON.stringify({ peerId, userName, userColor, heartbeat: true, clientVersion: serverVersion }) }));
    } catch (_) {}
    return;
  }

  // Calculate paragraph-relative offset by walking text nodes
  // (sel.anchorOffset is relative to sel.anchorNode which may be a span's text node)
  const offset = getParaOffset(paraEl, sel.anchorNode, sel.anchorOffset);

  const payload = {
    peerId,
    nodeId: paraEl.dataset.nodeId,
    offset,
    userName,
    userColor,
    clientVersion: serverVersion,
  };

  // Include selection range if non-collapsed
  if (!sel.isCollapsed && sel.rangeCount > 0) {
    const range = sel.getRangeAt(0);
    let startPara = range.startContainer;
    while (startPara && startPara !== document && !startPara.dataset?.nodeId) startPara = startPara.parentElement;
    let endPara = range.endContainer;
    while (endPara && endPara !== document && !endPara.dataset?.nodeId) endPara = endPara.parentElement;
    if (startPara?.dataset?.nodeId && endPara?.dataset?.nodeId) {
      payload.selStartNodeId = startPara.dataset.nodeId;
      payload.selStartOffset = getParaOffset(startPara, range.startContainer, range.startOffset);
      payload.selEndNodeId = endPara.dataset.nodeId;
      payload.selEndOffset = getParaOffset(endPara, range.endContainer, range.endOffset);
    }
  }

  // Update CRDT awareness state if available
  if (state.collabDoc) {
    try {
      state.collabDoc.set_cursor(paraEl.dataset.nodeId, offset, userName, userColor);
    } catch (_) {}
  }

  try {
    ws.send(JSON.stringify({
      type: 'awareness',
      room: roomId,
      data: JSON.stringify(payload),
    }));
  } catch (_) {}

  // Bug C12: Remove ghost cursors from crashed peers (no update in 30 seconds)
  const now = Date.now();
  for (const [pid, peer] of peers) {
    if (peer.lastSeen && now - peer.lastSeen > 30000) {
      removePeerCursor(pid);
      peers.delete(pid);
      peerOpTimestamps.delete(pid);
      if (peerTypingTimers.has(pid)) {
        clearTimeout(peerTypingTimers.get(pid));
        peerTypingTimers.delete(pid);
      }
      updatePeerCount();
      tracing('Removed ghost cursor for peer', pid, '(no activity for 30s)');
    }
  }
}

function applyRemoteAwareness(dataStr, fromPeerId) {
  if (!dataStr) return;
  try {
    const cursor = typeof dataStr === 'string' ? JSON.parse(dataStr) : dataStr;
    // Never render our own cursor as a peer cursor
    if (cursor.peerId === peerId) return;
    // Bug C12: Track last seen time for ghost cursor cleanup
    const peer = peers.get(cursor.peerId);
    if (peer) peer.lastSeen = Date.now();
    renderPeerCursor(cursor);
  } catch (_) {}
}

// ─── Peer Cursor Rendering ────────────────────────────

const peers = new Map();

function updatePeerList(peerList) {
  peers.clear();
  for (const p of peerList) {
    // Never add self to the peer list
    if (p.peerId === peerId) continue;
    peers.set(p.peerId, { userName: p.userName, userColor: p.userColor, lastSeen: Date.now() });
  }
  updatePeerCount();
}

function addPeer(pid, name, color) {
  // Never add self to the peer list
  if (pid === peerId) return;
  peers.set(pid, { userName: name, userColor: color, lastSeen: Date.now() });
  updatePeerCount();
}

function removePeer(pid) {
  peers.delete(pid);
  removePeerCursor(pid);
  // Clean up typing state for this peer
  peerOpTimestamps.delete(pid);
  if (peerTypingTimers.has(pid)) {
    clearTimeout(peerTypingTimers.get(pid));
    peerTypingTimers.delete(pid);
  }
  updatePeerCount();
}

// Track last known cursor state per peer to avoid unnecessary DOM updates
const peerCursorState = new Map();

// ─── Typing Activity Tracker ─────────────────────────
// Track op timestamps per peer to detect rapid typing (>2 ops in 3 seconds)
const peerOpTimestamps = new Map(); // peerId -> [timestamps]
const peerTypingTimers = new Map(); // peerId -> clearTimeout handle
const TYPING_OP_THRESHOLD = 2;     // ops needed to trigger "typing..."
const TYPING_WINDOW_MS = 3000;     // time window to measure ops
const TYPING_DISPLAY_MS = 3000;    // how long to show "typing..." after last op

function trackPeerTyping(pid) {
  if (pid === peerId) return;
  const now = Date.now();
  let timestamps = peerOpTimestamps.get(pid);
  if (!timestamps) {
    timestamps = [];
    peerOpTimestamps.set(pid, timestamps);
  }
  timestamps.push(now);
  // Remove timestamps outside the window
  const cutoff = now - TYPING_WINDOW_MS;
  while (timestamps.length > 0 && timestamps[0] < cutoff) timestamps.shift();

  if (timestamps.length > TYPING_OP_THRESHOLD) {
    showPeerTyping(pid, true);
    // Clear any existing timer and set a new one to hide after inactivity
    if (peerTypingTimers.has(pid)) clearTimeout(peerTypingTimers.get(pid));
    peerTypingTimers.set(pid, setTimeout(() => {
      showPeerTyping(pid, false);
      peerTypingTimers.delete(pid);
    }, TYPING_DISPLAY_MS));
  }
}

function showPeerTyping(pid, isTyping) {
  const cursorEl = document.getElementById(`peer-cursor-${pid}`);
  if (!cursorEl) return;
  let typingEl = cursorEl.querySelector('.peer-cursor-typing');
  if (isTyping) {
    if (!typingEl) {
      typingEl = document.createElement('span');
      typingEl.className = 'peer-cursor-typing';
      typingEl.textContent = 'typing...';
      const label = cursorEl.querySelector('.peer-cursor-label');
      if (label) {
        label.appendChild(typingEl);
      }
    }
  } else {
    if (typingEl) typingEl.remove();
  }
}

function renderPeerCursor(cursor) {
  if (!cursor || !cursor.nodeId) return;
  if (cursor.peerId === peerId) return;

  const page = $('pageContainer');
  if (!page) return;

  const paraEl = page.querySelector(`[data-node-id="${cursor.nodeId}"]`);
  if (!paraEl) return;

  // Check if cursor position actually changed
  const prev = peerCursorState.get(cursor.peerId);
  const posKey = `${cursor.nodeId}:${cursor.offset}`;
  if (prev && prev.posKey === posKey) return;

  // Compute position relative to pageContainer (not paragraph)
  // This avoids putting elements inside contenteditable paragraphs
  let leftPx = 0;
  let topPx = 0;
  let height = 18;
  try {
    const targetOffset = cursor.offset || 0;
    let remaining = targetOffset;
    const walker = document.createTreeWalker(paraEl, NodeFilter.SHOW_TEXT, null);
    let textNode;
    while ((textNode = walker.nextNode())) {
      // Skip text inside peer-cursor labels
      if (textNode.parentElement?.closest('.peer-cursor')) continue;
      const len = textNode.textContent.length;
      if (remaining <= len) {
        const range = document.createRange();
        range.setStart(textNode, Math.min(remaining, len));
        range.collapse(true);
        const rect = range.getBoundingClientRect();
        const pageRect = page.getBoundingClientRect();
        leftPx = rect.left - pageRect.left + page.scrollLeft;
        topPx = rect.top - pageRect.top + page.scrollTop;
        height = rect.height || 18;
        break;
      }
      remaining -= len;
    }
  } catch (_) {
    // Fallback: position at paragraph start
    const paraRect = paraEl.getBoundingClientRect();
    const pageRect = page.getBoundingClientRect();
    leftPx = paraRect.left - pageRect.left + page.scrollLeft;
    topPx = paraRect.top - pageRect.top + page.scrollTop;
  }

  // Reuse existing cursor element or create new
  let cursorEl = document.getElementById(`peer-cursor-${cursor.peerId}`);
  if (!cursorEl) {
    cursorEl = document.createElement('div');
    cursorEl.className = 'peer-cursor';
    cursorEl.id = `peer-cursor-${cursor.peerId}`;
    cursorEl.style.borderLeftColor = cursor.userColor || '#999';

    const label = document.createElement('span');
    label.className = 'peer-cursor-label';
    label.textContent = cursor.userName || 'Peer';
    label.style.backgroundColor = cursor.userColor || '#999';
    cursorEl.appendChild(label);

    // Append to pageContainer (NOT inside contenteditable paragraph)
    page.style.position = 'relative';
    page.appendChild(cursorEl);
  }

  cursorEl.style.left = leftPx + 'px';
  cursorEl.style.top = topPx + 'px';
  cursorEl.style.height = height + 'px';

  // Update selection highlights
  const selKey = cursor.selStartNodeId
    ? `${cursor.selStartNodeId}:${cursor.selStartOffset}-${cursor.selEndNodeId}:${cursor.selEndOffset}`
    : '';
  if (!prev || prev.selKey !== selKey) {
    clearPeerSelection(cursor.peerId);
    if (cursor.selStartNodeId && cursor.selEndNodeId) {
      renderPeerSelection(cursor);
    }
  }

  peerCursorState.set(cursor.peerId, { posKey, selKey });
}

/**
 * Remove a peer's cursor element and tracked state.
 * Called only on peer-leave — cursors persist as long as the peer is connected.
 */
function removePeerCursor(pid) {
  const el = document.getElementById(`peer-cursor-${pid}`);
  if (el) el.remove();
  clearPeerSelection(pid);
  peerCursorState.delete(pid);
}

/**
 * Render semi-transparent selection highlights for a peer's selected text range.
 * Highlights use the peer's color at 20% opacity, positioned as overlays.
 */
function renderPeerSelection(cursor) {
  const page = $('pageContainer');
  if (!page) return;

  const color = cursor.userColor || '#999';
  // Parse hex color to rgba at 20% opacity
  const rgba = hexToRgba(color, 0.2);

  // Collect all paragraph elements between start and end node (inclusive)
  const allParas = Array.from(page.querySelectorAll('[data-node-id]'));
  const startIdx = allParas.findIndex(el => el.dataset.nodeId === cursor.selStartNodeId);
  const endIdx = allParas.findIndex(el => el.dataset.nodeId === cursor.selEndNodeId);
  if (startIdx < 0 || endIdx < 0) return;

  const lo = Math.min(startIdx, endIdx);
  const hi = Math.max(startIdx, endIdx);

  for (let i = lo; i <= hi; i++) {
    const paraEl = allParas[i];
    paraEl.style.position = 'relative';

    const overlay = document.createElement('div');
    overlay.className = 'peer-selection-highlight';
    overlay.dataset.peerId = cursor.peerId;
    overlay.style.cssText = `
      position: absolute; top: 0; left: 0; right: 0; bottom: 0;
      background: ${rgba}; pointer-events: none; z-index: 1;
      border-radius: 2px;
    `;
    paraEl.appendChild(overlay);
  }
}

/**
 * Remove all selection highlight overlays for a specific peer.
 */
function clearPeerSelection(peerId) {
  document.querySelectorAll(`.peer-selection-highlight[data-peer-id="${peerId}"]`).forEach(el => el.remove());
}

/**
 * Convert a hex color string to rgba() with given alpha.
 */
function hexToRgba(hex, alpha) {
  const h = hex.replace('#', '');
  const r = parseInt(h.substring(0, 2), 16) || 0;
  const g = parseInt(h.substring(2, 4), 16) || 0;
  const b = parseInt(h.substring(4, 6), 16) || 0;
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

function clearPeerCursors() {
  document.querySelectorAll('.peer-cursor').forEach(el => el.remove());
  document.querySelectorAll('.peer-selection-highlight').forEach(el => el.remove());
  peers.clear();
  peerCursorState.clear();
  // Clear typing state
  for (const timer of peerTypingTimers.values()) clearTimeout(timer);
  peerTypingTimers.clear();
  peerOpTimestamps.clear();
  updatePeerCount();
}

// ─── UI Updates ───────────────────────────────────────

function updateCollabUI() {
  const shareBtn = $('btnShare');
  const collabStatus = $('collabStatus');

  if (shareBtn) {
    if (roomId) {
      shareBtn.innerHTML = '<span class="msi">link_off</span> Disconnect';
    } else {
      shareBtn.innerHTML = '<span class="msi">share</span> Share';
    }
    shareBtn.title = roomId ? 'Leave collaboration session' : 'Start a collaboration session';
  }

  if (collabStatus) {
    collabStatus.style.display = roomId ? 'inline-flex' : 'none';
  }

  // Update sync status in status bar
  const syncEl = $('collabSyncStatus');
  if (syncEl) {
    syncEl.style.display = roomId ? 'inline-flex' : 'none';
    if (!roomId) {
      state.collabStatus = 'disconnected';
    }
  }

  // Update status bar peers
  renderStatusBarPeers();
}

function updateConnectionStatus(status) {
  const indicator = $('collabIndicator');
  if (!indicator) return;

  indicator.className = 'collab-indicator collab-' + status;
  const indicatorTitle =
    status === 'connected' ? 'Connected to relay server' :
    status === 'reconnecting' ? 'Reconnecting...' :
    'Disconnected';
  indicator.title = indicatorTitle;
  indicator.setAttribute('aria-label', indicatorTitle);

  // Update state for status bar sync indicator
  state.collabStatus = status === 'connected' ? 'connected' :
    status === 'reconnecting' ? 'connecting' : 'disconnected';

  // Update status bar sync text
  const syncEl = $('collabSyncStatus');
  if (syncEl) {
    if (status === 'connected') {
      syncEl.textContent = 'Synced';
      syncEl.className = 'collab-sync-status synced';
    } else if (status === 'reconnecting') {
      syncEl.textContent = 'Syncing...';
      syncEl.className = 'collab-sync-status syncing';
    } else {
      syncEl.textContent = 'Offline';
      syncEl.className = 'collab-sync-status offline';
    }
    syncEl.style.display = roomId ? 'inline-flex' : 'none';
  }
}

function updatePeerCount() {
  const el = $('collabPeerCount');
  if (el) {
    const count = peers.size; // peers map does NOT include self
    el.textContent = count > 0 ? `${count + 1} users` : '1 user';
  }

  // Bug C22: Sync to state — use same reference to prevent divergence between peers and state.collabPeers
  state.collabPeers = peers;

  // Update status bar peer dots
  renderStatusBarPeers();

  // Update title bar avatar circles
  renderTitleBarAvatars();

  // Update share modal peer list if open
  updateSharePeerList();
}

/**
 * Render small colored dots in the status bar for each connected peer.
 */
function renderStatusBarPeers() {
  const container = $('statusBarPeers');
  if (!container) return;

  container.innerHTML = '';
  if (!roomId || peers.size === 0) {
    container.style.display = 'none';
    return;
  }

  container.style.display = 'inline-flex';

  for (const [pid, p] of peers) {
    const dot = document.createElement('span');
    dot.className = 'status-peer-dot';
    dot.style.background = p.userColor || '#999';
    const peerTitle = p.userName || 'Peer';
    dot.title = peerTitle;
    dot.setAttribute('aria-label', peerTitle);
    container.appendChild(dot);
  }
}

/**
 * Render colored avatar circles in the title bar for each connected peer.
 * Shows up to 5 peers with initial letters, then "+N more" overflow.
 */
const MAX_VISIBLE_AVATARS = 5;

function renderTitleBarAvatars() {
  const container = $('collabAvatars');
  if (!container) return;

  container.innerHTML = '';
  if (!roomId || peers.size === 0) {
    container.classList.remove('visible');
    return;
  }

  container.classList.add('visible');
  const peerEntries = Array.from(peers.entries());
  const visible = peerEntries.slice(0, MAX_VISIBLE_AVATARS);
  const overflow = peerEntries.length - MAX_VISIBLE_AVATARS;

  for (const [pid, p] of visible) {
    const avatar = document.createElement('span');
    avatar.className = 'collab-avatar';
    avatar.style.backgroundColor = p.userColor || '#999';
    const name = p.userName || 'Peer';
    avatar.textContent = name.charAt(0).toUpperCase();
    avatar.title = name;
    avatar.setAttribute('aria-label', name);
    container.appendChild(avatar);
  }

  if (overflow > 0) {
    const more = document.createElement('span');
    more.className = 'collab-avatar-overflow';
    more.textContent = '+' + overflow;
    more.title = peerEntries.slice(MAX_VISIBLE_AVATARS).map(([, p]) => p.userName || 'Peer').join(', ');
    more.setAttribute('aria-label', overflow + ' more collaborators');
    container.appendChild(more);
  }
}

// ─── Share Dialog ─────────────────────────────────────

export async function showShareDialog() {
  if (roomId) {
    // Already in a session — confirm disconnect
    const modal = $('shareModal');
    if (modal) modal.classList.remove('show');
    stopCollab();
    return;
  }

  if (!state.doc) {
    // Show non-blocking toast instead of alert
    const tc = document.getElementById('toastContainer');
    if (tc) {
      const t = document.createElement('div');
      t.className = 'toast toast-error';
      t.textContent = 'Open or create a document first.';
      tc.appendChild(t);
      setTimeout(() => { t.style.transition = 'opacity 0.2s'; t.style.opacity = '0'; setTimeout(() => t.remove(), 220); }, 4000);
    }
    return;
  }

  const permSelect = $('sharePermission');
  const access = permSelect ? permSelect.value : 'edit';
  const docName = $('docName')?.value?.trim() || 'Untitled';

  // Upload the current document to the server to create a file session.
  // The server returns a fileId that becomes the shareable link.
  const urlInput = $('shareUrlInput');
  if (urlInput) urlInput.value = 'Creating session...';

  const modal = $('shareModal');
  if (modal) modal.classList.add('show');

  const nameInput = $('shareNameInput');
  if (nameInput) nameInput.value = 'User ' + Math.floor(Math.random() * 100);

  const relayInput = $('shareRelayInput');
  if (relayInput) relayInput.value = DEFAULT_RELAY_URL;

  // Upload document to server
  try {
    const bytes = state.doc.export('docx');
    const blob = new Blob([bytes], { type: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document' });
    const formData = new FormData();
    formData.append('file', blob, docName + '.docx');

    const apiBase = window.S1_CONFIG?.apiUrl || '/api/v1';
    const resp = await fetch(`${apiBase}/files`, { method: 'POST', body: formData });

    if (resp.ok) {
      const result = await resp.json();
      const fileId = result.fileId;
      const shareUrl = `${window.location.origin}/?file=${fileId}&access=${access}`;

      if (urlInput) urlInput.value = shareUrl;
      if (modal) modal.dataset.room = fileId;

      // Also start collab session with the fileId as the room
      state._pendingShareFileId = fileId;
    } else {
      // Fallback to client-side room (no server)
      const generatedRoom = _generateSecureRoomId();
      const shareUrl = `${window.location.origin}/?room=${generatedRoom}&relay=${encodeURIComponent(DEFAULT_RELAY_URL)}&access=${access}`;
      if (urlInput) urlInput.value = shareUrl;
      if (modal) modal.dataset.room = generatedRoom;
    }
  } catch (e) {
    // Server unavailable — fallback to client-side room
    console.warn('File upload for sharing failed:', e);
    const generatedRoom = _generateSecureRoomId();
    const shareUrl = `${window.location.origin}/?room=${generatedRoom}&relay=${encodeURIComponent(DEFAULT_RELAY_URL)}&access=${access}`;
    if (urlInput) urlInput.value = shareUrl;
    if (modal) modal.dataset.room = generatedRoom;
  }

  updateSharePeerList();
}

/**
 * Called when the user clicks "Start Session" in the share modal.
 */
export function startShareSession() {
  const modal = $('shareModal');
  if (!modal) return;

  const room = modal.dataset.room;
  const nameInput = $('shareNameInput');
  const relayInput = $('shareRelayInput');

  const name = nameInput ? nameInput.value.trim() || 'Anonymous' : 'Anonymous';
  const relay = relayInput ? relayInput.value.trim() || DEFAULT_RELAY_URL : DEFAULT_RELAY_URL;

  modal.classList.remove('show');
  startCollab(room, name, relay);
}

/**
 * Copy share URL to clipboard.
 */
export function copyShareUrl() {
  const urlInput = $('shareUrlInput');
  if (!urlInput) return;

  const showCopied = () => {
    const btn = $('shareCopyBtn');
    if (btn) {
      const origHTML = btn.innerHTML;
      btn.innerHTML = '<span class="msi" style="font-size:16px;vertical-align:middle">check</span> Copied!';
      btn.style.color = '#1e8e3e';
      btn.style.borderColor = '#1e8e3e';
      setTimeout(() => { btn.innerHTML = origHTML; btn.style.color = ''; btn.style.borderColor = ''; }, 1500);
    }
  };

  // Try modern clipboard API (requires HTTPS or localhost)
  if (navigator.clipboard && navigator.clipboard.writeText) {
    navigator.clipboard.writeText(urlInput.value).then(showCopied).catch(() => {
      // Fallback: execCommand
      urlInput.select();
      document.execCommand('copy');
      showCopied();
    });
  } else {
    // Fallback for HTTP: select + execCommand
    urlInput.select();
    document.execCommand('copy');
    showCopied();
  }
}

/**
 * Update the peer list shown in the share modal.
 */
function updateSharePeerList() {
  const list = $('sharePeerList');
  if (!list) return;
  list.innerHTML = '';

  if (peers.size === 0 && !roomId) {
    list.innerHTML = '<div class="share-no-peers">No peers connected</div>';
    return;
  }

  // Show self at top with "(you)" label and editing status
  if (roomId) {
    const self = document.createElement('div');
    self.className = 'share-peer-item';
    self.innerHTML = `<span class="share-peer-dot" style="background:${userColor}"></span>
      <span class="share-peer-name">${userName}</span>
      <span class="share-peer-you">(you)</span>
      <span class="share-peer-status">editing</span>`;
    list.appendChild(self);
  }

  for (const [pid, p] of peers) {
    const el = document.createElement('div');
    el.className = 'share-peer-item';
    el.innerHTML = `<span class="share-peer-dot" style="background:${p.userColor || '#999'}"></span>
      <span class="share-peer-name">${p.userName || 'Anonymous'}</span>
      <span class="share-peer-status">editing</span>`;
    list.appendChild(el);
  }

  if (list.children.length === 0) {
    list.innerHTML = '<div class="share-no-peers">No peers connected</div>';
  }
}

// ─── Buffer Full Banner ──────────────────────────────

/**
 * D5 fix: Show a persistent (non-dismissing) banner when offline buffer is full.
 * Removed automatically when connection is re-established.
 */
function _showBufferFullBanner() {
  // Remove any existing banner first
  _removeBufferFullBanner();
  const container = $('toastContainer');
  if (!container) return;
  const banner = document.createElement('div');
  banner.id = 'bufferFullBanner';
  banner.className = 'collab-toast show';
  banner.style.background = '#d32f2f';
  banner.style.color = '#fff';
  banner.style.fontWeight = '600';
  banner.textContent = 'Connection lost and change buffer is full. New changes will not be saved until reconnected.';
  container.appendChild(banner);
}

function _removeBufferFullBanner() {
  const banner = document.getElementById('bufferFullBanner');
  if (banner) banner.remove();
}

/**
 * Show a persistent amber warning banner when the offline buffer reaches 80% capacity.
 * Removed automatically when connection is re-established.
 */
function _showBufferWarningBanner() {
  _removeBufferWarningBanner();
  const container = $('toastContainer');
  if (!container) return;
  const banner = document.createElement('div');
  banner.id = 'bufferWarningBanner';
  banner.className = 'collab-toast show';
  banner.style.background = '#f9a825';
  banner.style.color = '#333';
  banner.style.fontWeight = '600';
  banner.textContent = 'Connection lost. Changes are being buffered locally \u2014 reconnect soon to avoid data loss.';
  container.appendChild(banner);
}

function _removeBufferWarningBanner() {
  const banner = document.getElementById('bufferWarningBanner');
  if (banner) banner.remove();
}

// ─── Toast Notifications ─────────────────────────────

let toastThrottleTimer = null;

/**
 * Show a toast notification that auto-dismisses after 3 seconds.
 * Throttled to at most one toast per second.
 */
function showCollabToast(message) {
  if (toastThrottleTimer) return;
  toastThrottleTimer = setTimeout(() => { toastThrottleTimer = null; }, 1000);

  const container = $('toastContainer');
  if (!container) return;

  const toast = document.createElement('div');
  toast.className = 'collab-toast';
  toast.textContent = message;
  container.appendChild(toast);

  // Trigger enter animation
  requestAnimationFrame(() => toast.classList.add('show'));

  // Auto-dismiss after 3 seconds
  setTimeout(() => {
    toast.classList.remove('show');
    toast.classList.add('hide');
    setTimeout(() => toast.remove(), 300);
  }, 3000);
}

/**
 * Briefly flash a paragraph element to indicate a remote edit.
 */
function flashParagraph(nodeId) {
  const page = $('pageContainer');
  if (!page) return;
  const el = page.querySelector(`[data-node-id="${nodeId}"]`);
  if (!el) return;

  el.classList.add('collab-flash');
  setTimeout(() => el.classList.remove('collab-flash'), 600);
}

// ─── Init: wire up share modal buttons ───────────────

export function initCollabUI() {
  const closeBtn = $('shareModalClose');
  if (closeBtn) closeBtn.addEventListener('click', () => {
    const modal = $('shareModal');
    if (modal) modal.classList.remove('show');
  });

  const cancelBtn = $('shareCancelBtn');
  if (cancelBtn) cancelBtn.addEventListener('click', () => {
    const modal = $('shareModal');
    if (modal) modal.classList.remove('show');
  });

  const startBtn = $('shareStartBtn');
  if (startBtn) startBtn.addEventListener('click', startShareSession);

  const copyBtn = $('shareCopyBtn');
  if (copyBtn) copyBtn.addEventListener('click', copyShareUrl);

  // Close on overlay click
  const modal = $('shareModal');
  if (modal) modal.addEventListener('click', (e) => {
    if (e.target === modal) modal.classList.remove('show');
  });
}

/**
 * Auto-join if URL has ?file= or ?room= parameter.
 *
 * ?file={fileId} — download document from server, open it, connect to WebSocket.
 * ?room={roomId} — join existing relay room (legacy/fallback).
 */
export async function checkAutoJoin() {
  const params = new URLSearchParams(window.location.search);
  const fileId = params.get('file');
  const room = params.get('room');
  const docType = params.get('type'); // S4.2: 'sheet' for spreadsheet

  // S4.2: If type=sheet and we have a spreadsheet view, use spreadsheet collab
  if (fileId && docType === 'sheet') {
    try {
      const apiBase = window.S1_CONFIG?.apiUrl || '/api/v1';
      const resp = await fetch(`${apiBase}/files/${encodeURIComponent(fileId)}/download`);
      if (!resp.ok) {
        showCollabToast('Shared spreadsheet not found or session expired');
        return false;
      }
      const bytes = new Uint8Array(await resp.arrayBuffer());
      // Open in spreadsheet view if available
      if (state.spreadsheetView) {
        let filename = 'Shared Spreadsheet.xlsx';
        try {
          const infoResp = await fetch(`${apiBase}/files/${encodeURIComponent(fileId)}`);
          if (infoResp.ok) { const info = await infoResp.json(); filename = info.filename || filename; }
        } catch (_) {}
        state.spreadsheetView.loadWorkbook(bytes, filename);
        const name = 'User ' + Math.floor(Math.random() * 100);
        const relay = params.get('relay') || DEFAULT_RELAY_URL;
        state.spreadsheetView.startCollab(fileId, name, relay);
        return true;
      }
    } catch (e) {
      console.error('Failed to open shared spreadsheet:', e);
      showCollabToast('Failed to open shared spreadsheet');
      return false;
    }
  }

  if (fileId) {
    // Server-based share: fetch document from server, open it, then join collab
    try {
      const apiBase = window.S1_CONFIG?.apiUrl || '/api/v1';
      const resp = await fetch(`${apiBase}/files/${encodeURIComponent(fileId)}/download`);
      if (!resp.ok) {
        console.error(`Failed to fetch shared file ${fileId}: HTTP ${resp.status}`);
        showCollabToast('Shared document not found or session expired');
        return false;
      }
      const bytes = new Uint8Array(await resp.arrayBuffer());

      // Get session info for filename
      let filename = 'Shared Document.docx';
      try {
        const infoResp = await fetch(`${apiBase}/files/${encodeURIComponent(fileId)}`);
        if (infoResp.ok) {
          const info = await infoResp.json();
          filename = info.filename || filename;
        }
      } catch (_) {}

      // Open the document in the editor (dynamic import to avoid circular dep)
      const { openFile } = await import('./file.js');
      await openFile(bytes, filename);

      // Try to open as CRDT collaborative document for real-time sync
      try {
        if (state.engine && typeof state.engine.open_collab === 'function') {
          const replicaId = Math.floor(Math.random() * 2147483647) + 1;
          state.collabDoc = state.engine.open_collab(bytes, replicaId);
          tracing('Opened CRDT collab doc with replicaId:', replicaId);
        }
      } catch (e) {
        console.warn('CRDT collab not available, using fullSync mode:', e);
        state.collabDoc = null;
      }

      // Connect to WebSocket for co-editing using fileId as room
      const name = 'User ' + Math.floor(Math.random() * 100);
      const relay = params.get('relay') || DEFAULT_RELAY_URL;
      accessLevel = params.get('access') || 'edit';
      startCollab(fileId, name, relay);

      return true;
    } catch (e) {
      console.error('Failed to open shared document:', e);
      showCollabToast('Failed to open shared document');
      return false;
    }
  }

  if (room) {
    // Legacy relay-only mode
    const relay = params.get('relay') || DEFAULT_RELAY_URL;
    const name = 'User ' + Math.floor(Math.random() * 100);
    setTimeout(() => {
      if (state.engine && state.doc) {
        startCollab(room, name, relay);
      }
    }, 1000);
    return true;
  }

  return false;
}
