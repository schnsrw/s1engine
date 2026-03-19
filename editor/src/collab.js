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
  || (window.location.protocol === 'https:' ? 'wss://' : 'ws://') + window.location.hostname + ':8080/ws/collab';
const RECONNECT_DELAYS = [2000, 4000, 8000, 16000, 30000];
const MAX_RECONNECT_ATTEMPTS = 5;
const CURSOR_BROADCAST_INTERVAL = 500;
const PEER_COLORS = [
  '#4285f4', '#ea4335', '#34a853', '#fbbc04', '#9c27b0',
  '#00bcd4', '#ff5722', '#607d8b', '#e91e63', '#3f51b5',
];

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

// ─── Public API ───────────────────────────────────────

/**
 * Start a collaboration session.
 */
export function startCollab(room, name, relayUrl) {
  if (!state.doc) return;

  roomId = room;
  userName = name || 'Anonymous';
  peerId = peerId || ('u-' + Math.random().toString(36).slice(2, 10));
  userColor = PEER_COLORS[Math.floor(Math.random() * PEER_COLORS.length)];

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
  clearPeerCursors();
  updateCollabUI();
}

/**
 * Broadcast a paragraph text change to peers.
 * Called from syncParagraphText in render.js.
 */
export function broadcastTextSync(nodeId, text) {
  if (!roomId || applyingRemote) return;
  sendOp({ action: 'setText', nodeId, text });
}

/**
 * Broadcast a structural operation (split, merge, format, delete).
 * Called from input.js / toolbar.js after applying the operation locally.
 */
export function broadcastOp(opData) {
  if (!roomId || applyingRemote) return;
  sendOp(opData);
}

/**
 * Check if collaboration is active.
 */
export function isCollabActive() {
  return roomId !== null && connected;
}

/**
 * Get current room ID.
 */
export function getCollabRoom() {
  return roomId;
}

// ─── WebSocket Connection ─────────────────────────────

function connect(url) {
  if (ws) { try { ws.close(); } catch (_) {} }
  lastRelayUrl = url;

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
    wsUrl += `${sep}user=${encodeURIComponent(userName)}&uid=${encodeURIComponent(peerId || 'u-' + Math.random().toString(36).slice(2,8))}`;
  }

  try {
    ws = new WebSocket(wsUrl);
  } catch (_) {
    scheduleReconnect(url);
    return;
  }

  ws.onopen = () => {
    connected = true;
    reconnectAttempt = 0;
    updateConnectionStatus('connected');

    ws.send(JSON.stringify({
      type: 'join',
      room: roomId,
      userName,
      userColor,
    }));

    // Flush offline buffer
    if (offlineBuffer.length > 0) {
      updateSyncStatus('syncing');
      for (const op of offlineBuffer) {
        ws.send(JSON.stringify({ type: 'op', room: roomId, data: JSON.stringify(op) }));
      }
      offlineBuffer = [];
    }
    // Reset buffer warning flags after successful reconnect
    _bufferWarningShown = false;
    _bufferFullWarningShown = false;
    updateSyncStatus('synced');
  };

  ws.onmessage = (event) => {
    try {
      const msg = JSON.parse(event.data);
      handleMessage(msg);
    } catch (_) {}
  };

  ws.onclose = () => {
    connected = false;
    updateConnectionStatus('disconnected');
    updateSyncStatus('offline');
    if (roomId) scheduleReconnect(url);
  };

  ws.onerror = () => {};
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

function sendOp(opData) {
  const payload = JSON.stringify(opData);
  if (connected && ws && ws.readyState === 1) {
    ws.send(JSON.stringify({ type: 'op', room: roomId, data: payload }));
  } else {
    if (offlineBuffer.length < MAX_OFFLINE_BUFFER) {
      offlineBuffer.push(opData);
      // Warn when buffer is near capacity
      if (offlineBuffer.length >= OFFLINE_BUFFER_WARNING_THRESHOLD && !_bufferWarningShown) {
        _bufferWarningShown = true;
        showCollabToast('Offline buffer nearly full \u2014 reconnect soon to avoid data loss');
      }
    } else {
      if (!_bufferFullWarningShown) {
        _bufferFullWarningShown = true;
        console.warn('Offline buffer limit reached. Some changes may not sync when reconnected.');
        showCollabToast('Connection lost \u2014 changes may not sync');
        // Update sync status to show critical state
        const syncEl = $('collabSyncStatus');
        if (syncEl) {
          syncEl.textContent = 'Offline (changes may be lost)';
          syncEl.className = 'collab-sync-status offline';
        }
      }
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
    case 'welcome':
      // Server welcome message with session info.
      // Use this as the 'joined' equivalent when connecting to s1-server.
      peerId = msg.user || peerId || ('peer-' + Math.random().toString(36).slice(2, 8));
      tracing('Welcome to room', msg.fileId, '— ops:', msg.opsCount);
      updateCollabUI();
      break;

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
      updatePeerList(msg.peers || []);
      // Don't send fullSync on join — existing peers will send it via peer-join.
      break;

    case 'peer-join':
      // Only handle if it's a different peer (not self)
      if (msg.peerId === peerId) break;
      addPeer(msg.peerId, msg.userName, msg.userColor);
      // Existing peer sends full document state so the new joiner gets latest version
      sendFullSync();
      break;

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

function sendFullSync() {
  if (!state.doc) return;
  try {
    // Send current document as full sync for new peers
    const bytes = state.doc.export('docx');
    // Convert to base64 for transport
    const base64 = btoa(String.fromCharCode(...new Uint8Array(bytes)));
    sendOp({ action: 'fullSync', docBase64: base64 });
  } catch (_) {}
}

function applyRemoteOp(dataStr, fromPeerId) {
  if (!state.doc || !dataStr) return;
  try {
    const op = JSON.parse(dataStr);
    applyingRemote = true;

    // Show toast for remote changes (throttled)
    if (fromPeerId && fromPeerId !== peerId) {
      const peer = peers.get(fromPeerId);
      const peerName = peer ? peer.userName : 'A peer';
      showCollabToast(`Changes from ${peerName} applied`);

      // Flash the affected paragraph if possible
      const affectedNodeId = op.nodeId || op.startNode;
      if (affectedNodeId) {
        flashParagraph(affectedNodeId);
      }
    }

    switch (op.action) {
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
        } catch (e) { console.error('remote setText:', e); }
        break;
      }

      case 'splitParagraph': {
        try {
          const newId = state.doc.split_paragraph(op.nodeId, op.offset);
          renderDocument();
        } catch (e) { console.error('remote split:', e); }
        break;
      }

      case 'mergeParagraphs': {
        try {
          state.doc.merge_paragraphs(op.nodeId1, op.nodeId2);
          renderDocument();
        } catch (e) { console.error('remote merge:', e); }
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
        } catch (e) { console.error('remote format:', e); }
        break;
      }

      case 'deleteSelection': {
        try {
          state.doc.delete_selection(op.startNode, op.startOffset, op.endNode, op.endOffset);
          renderDocument();
        } catch (e) { console.error('remote delete:', e); }
        break;
      }

      case 'setHeading': {
        try {
          state.doc.set_heading_level(op.nodeId, op.level);
          renderNodeById(op.nodeId);
        } catch (e) { console.error('remote heading:', e); }
        break;
      }

      case 'setAlignment': {
        try {
          state.doc.set_alignment(op.nodeId, op.alignment);
          renderNodeById(op.nodeId);
        } catch (e) { console.error('remote align:', e); }
        break;
      }

      case 'insertParagraph': {
        try {
          state.doc.insert_paragraph_after(op.afterNodeId, op.text || '');
          renderDocument();
        } catch (e) { console.error('remote insertPara:', e); }
        break;
      }

      case 'deleteNode': {
        try {
          state.doc.delete_node(op.nodeId);
          renderDocument();
        } catch (e) { console.error('remote deleteNode:', e); }
        break;
      }

      case 'setListFormat': {
        try {
          state.doc.set_list_format(op.nodeId, op.format, op.level || 0);
          renderDocument();
        } catch (e) { console.error('remote list:', e); }
        break;
      }

      case 'setIndent': {
        try {
          state.doc.set_indent(op.nodeId, op.side, op.value);
          renderNodeById(op.nodeId);
        } catch (e) { console.error('remote indent:', e); }
        break;
      }

      case 'setLineSpacing': {
        try {
          state.doc.set_line_spacing(op.nodeId, op.value);
          renderNodeById(op.nodeId);
        } catch (e) { console.error('remote lineSpacing:', e); }
        break;
      }

      case 'insertTable': {
        try {
          state.doc.insert_table(op.afterNodeId, op.rows, op.cols);
          renderDocument();
        } catch (e) { console.error('remote insertTable:', e); }
        break;
      }

      case 'insertTableRow': {
        try {
          state.doc.insert_table_row(op.tableId, op.index);
          renderDocument();
        } catch (e) { console.error('remote insertRow:', e); }
        break;
      }

      case 'deleteTableRow': {
        try {
          state.doc.delete_table_row(op.tableId, op.index);
          renderDocument();
        } catch (e) { console.error('remote deleteRow:', e); }
        break;
      }

      case 'insertTableColumn': {
        try {
          state.doc.insert_table_column(op.tableId, op.index);
          renderDocument();
        } catch (e) { console.error('remote insertCol:', e); }
        break;
      }

      case 'deleteTableColumn': {
        try {
          state.doc.delete_table_column(op.tableId, op.index);
          renderDocument();
        } catch (e) { console.error('remote deleteCol:', e); }
        break;
      }

      case 'insertHR': {
        try {
          state.doc.insert_horizontal_rule(op.afterNodeId);
          renderDocument();
        } catch (e) { console.error('remote insertHR:', e); }
        break;
      }

      case 'insertPageBreak': {
        try {
          state.doc.insert_page_break(op.afterNodeId);
          renderDocument();
        } catch (e) { console.error('remote insertPageBreak:', e); }
        break;
      }

      case 'insertSectionBreak': {
        try {
          state.doc.insert_section_break(op.afterNodeId, op.breakType || 'nextPage');
          renderDocument();
        } catch (e) { console.error('remote insertSectionBreak:', e); }
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
        } catch (e) { console.error('remote moveBefore:', e); }
        break;
      }

      case 'moveNodeAfter': {
        try {
          state.doc.move_node_after(op.nodeId, op.afterId);
          renderDocument();
        } catch (e) { console.error('remote moveAfter:', e); }
        break;
      }

      case 'resizeImage': {
        try {
          state.doc.resize_image(op.nodeId, op.width, op.height);
          renderDocument();
        } catch (e) { console.error('remote resizeImage:', e); }
        break;
      }

      case 'setCellBackground': {
        try {
          state.doc.set_cell_background(op.cellId, op.color);
          renderDocument();
        } catch (e) { console.error('remote setCellBg:', e); }
        break;
      }

      case 'setImageAltText': {
        try {
          state.doc.set_image_alt_text(op.nodeId, op.alt);
          renderDocument();
        } catch (e) { console.error('remote altText:', e); }
        break;
      }

      case 'insertLineBreak': {
        try {
          state.doc.insert_line_break(op.nodeId, op.offset);
          renderNodeById(op.nodeId);
        } catch (e) { console.error('remote insertLineBreak:', e); }
        break;
      }

      case 'pasteText': {
        try {
          state.doc.paste_plain_text(op.nodeId, op.offset, op.text);
          renderDocument();
        } catch (e) { console.error('remote pasteText:', e); }
        break;
      }

      case 'pasteFormattedRuns': {
        try {
          state.doc.paste_formatted_runs_json(op.nodeId, op.offset, op.runsJson);
          renderDocument();
        } catch (e) { console.error('remote pasteFormattedRuns:', e); }
        break;
      }

      case 'insertText': {
        try {
          state.doc.insert_text_in_paragraph(op.nodeId, op.offset, op.text);
          renderNodeById(op.nodeId);
        } catch (e) { console.error('remote insertText:', e); }
        break;
      }

      case 'replaceText': {
        try {
          state.doc.replace_text(op.nodeId, op.offset, op.length, op.replacement);
          renderDocument();
        } catch (e) { console.error('remote replaceText:', e); }
        break;
      }

      case 'replaceAll': {
        try {
          state.doc.replace_all(op.query, op.replacement, op.caseInsensitive);
          renderDocument();
        } catch (e) { console.error('remote replaceAll:', e); }
        break;
      }

      case 'insertComment': {
        try {
          state.doc.insert_comment(op.startNodeId, op.endNodeId, op.author, op.text);
          renderDocument();
        } catch (e) { console.error('remote insertComment:', e); }
        break;
      }

      case 'deleteComment': {
        try {
          state.doc.delete_comment(op.commentId);
          renderDocument();
        } catch (e) { console.error('remote deleteComment:', e); }
        break;
      }

      case 'acceptChange': {
        try {
          state.doc.accept_change(op.nodeId);
          renderDocument();
        } catch (e) { console.error('remote acceptChange:', e); }
        break;
      }

      case 'rejectChange': {
        try {
          state.doc.reject_change(op.nodeId);
          renderDocument();
        } catch (e) { console.error('remote rejectChange:', e); }
        break;
      }

      case 'acceptAllChanges': {
        try {
          state.doc.accept_all_changes();
          renderDocument();
        } catch (e) { console.error('remote acceptAllChanges:', e); }
        break;
      }

      case 'rejectAllChanges': {
        try {
          state.doc.reject_all_changes();
          renderDocument();
        } catch (e) { console.error('remote rejectAllChanges:', e); }
        break;
      }

      case 'fullDocSync': {
        // Peer performed undo/redo — request full sync
        // For now, trigger a full document re-render from current state
        // (The actual undo happened on the sender's side, and the text changes
        // are broadcast separately via setText operations)
        renderDocument();
        break;
      }

      case 'fullSync': {
        // Full document sync from an existing peer.
        // This is sent when we join — the existing peer has the latest state.
        try {
          if (op.docBase64 && state.engine) {
            const binary = atob(op.docBase64);
            const bytes = new Uint8Array(binary.length);
            for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
            if (bytes.length > 0) {
              state.doc = state.engine.open(bytes);
              renderDocument();
              tracing('Applied fullSync from peer:', bytes.length, 'bytes');
            }
          }
        } catch (e) { console.error('remote fullSync:', e); }
        break;
      }

      default:
        console.warn('Unknown remote op:', op.action);
    }

    applyingRemote = false;
  } catch (e) {
    applyingRemote = false;
    console.error('apply remote op:', e);
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

  const sel = window.getSelection();
  if (!sel || sel.rangeCount === 0) return;

  // Find the paragraph element containing the cursor
  let paraEl = sel.anchorNode;
  while (paraEl && paraEl !== document && !paraEl.dataset?.nodeId) {
    paraEl = paraEl.parentElement;
  }
  if (!paraEl || !paraEl.dataset?.nodeId) return;

  // Calculate paragraph-relative offset by walking text nodes
  // (sel.anchorOffset is relative to sel.anchorNode which may be a span's text node)
  const offset = getParaOffset(paraEl, sel.anchorNode, sel.anchorOffset);

  const payload = {
    peerId,
    nodeId: paraEl.dataset.nodeId,
    offset,
    userName,
    userColor,
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

  try {
    ws.send(JSON.stringify({
      type: 'awareness',
      room: roomId,
      data: JSON.stringify(payload),
    }));
  } catch (_) {}
}

function applyRemoteAwareness(dataStr, fromPeerId) {
  if (!dataStr) return;
  try {
    const cursor = typeof dataStr === 'string' ? JSON.parse(dataStr) : dataStr;
    // Never render our own cursor as a peer cursor
    if (cursor.peerId === peerId) return;
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
    peers.set(p.peerId, { userName: p.userName, userColor: p.userColor });
  }
  updatePeerCount();
}

function addPeer(pid, name, color) {
  // Never add self to the peer list
  if (pid === peerId) return;
  peers.set(pid, { userName: name, userColor: color });
  updatePeerCount();
}

function removePeer(pid) {
  peers.delete(pid);
  removePeerCursor(pid);
  updatePeerCount();
}

// Track last known cursor state per peer to avoid unnecessary DOM updates
const peerCursorState = new Map();

function renderPeerCursor(cursor) {
  if (!cursor || !cursor.nodeId) return;
  // Never render own cursor
  if (cursor.peerId === peerId) return;

  const page = $('pageContainer');
  if (!page) return;

  const paraEl = page.querySelector(`[data-node-id="${cursor.nodeId}"]`);
  if (!paraEl) return;

  // Check if cursor position actually changed — skip DOM update if identical
  const prev = peerCursorState.get(cursor.peerId);
  const posKey = `${cursor.nodeId}:${cursor.offset}`;
  if (prev && prev.posKey === posKey) {
    // Position unchanged — just keep the existing element alive
    return;
  }

  // Compute new position before touching DOM
  let leftPx = 0;
  let topPx = 0;
  try {
    const targetOffset = cursor.offset || 0;
    let remaining = targetOffset;
    const walker = document.createTreeWalker(paraEl, NodeFilter.SHOW_TEXT, null);
    let textNode;
    while ((textNode = walker.nextNode())) {
      const len = textNode.textContent.length;
      if (remaining <= len) {
        const range = document.createRange();
        range.setStart(textNode, Math.min(remaining, len));
        range.collapse(true);
        const rect = range.getBoundingClientRect();
        const paraRect = paraEl.getBoundingClientRect();
        leftPx = rect.left - paraRect.left;
        topPx = rect.top - paraRect.top;
        break;
      }
      remaining -= len;
    }
  } catch (_) {}

  // Reuse existing cursor element if it exists, otherwise create
  let cursorEl = document.getElementById(`peer-cursor-${cursor.peerId}`);
  if (cursorEl) {
    // Move to new paragraph if needed
    if (cursorEl.parentElement !== paraEl) {
      cursorEl.remove();
      paraEl.style.position = 'relative';
      paraEl.appendChild(cursorEl);
    }
    // Update position
    cursorEl.style.left = leftPx + 'px';
    cursorEl.style.top = topPx + 'px';
  } else {
    // Create new cursor element
    cursorEl = document.createElement('div');
    cursorEl.className = 'peer-cursor';
    cursorEl.id = `peer-cursor-${cursor.peerId}`;
    cursorEl.style.borderLeftColor = cursor.userColor || '#999';
    cursorEl.style.left = leftPx + 'px';
    cursorEl.style.top = topPx + 'px';

    // Name label
    const label = document.createElement('span');
    label.className = 'peer-cursor-label';
    label.textContent = cursor.userName || 'Peer';
    label.style.backgroundColor = cursor.userColor || '#999';
    cursorEl.appendChild(label);

    paraEl.style.position = 'relative';
    paraEl.appendChild(cursorEl);
  }

  // Update selection highlights only if selection data changed
  const selKey = cursor.selStartNodeId
    ? `${cursor.selStartNodeId}:${cursor.selStartOffset}-${cursor.selEndNodeId}:${cursor.selEndOffset}`
    : '';
  if (!prev || prev.selKey !== selKey) {
    clearPeerSelection(cursor.peerId);
    if (cursor.selStartNodeId && cursor.selEndNodeId) {
      renderPeerSelection(cursor);
    }
  }

  // Save state for next comparison
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

  // Sync to state
  state.collabPeers = new Map(peers);

  // Update status bar peer dots
  renderStatusBarPeers();

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
      const generatedRoom = `room-${Math.random().toString(36).substring(2, 10)}`;
      const shareUrl = `${window.location.origin}/?room=${generatedRoom}&relay=${encodeURIComponent(DEFAULT_RELAY_URL)}&access=${access}`;
      if (urlInput) urlInput.value = shareUrl;
      if (modal) modal.dataset.room = generatedRoom;
    }
  } catch (e) {
    // Server unavailable — fallback to client-side room
    console.warn('File upload for sharing failed:', e);
    const generatedRoom = `room-${Math.random().toString(36).substring(2, 10)}`;
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
  navigator.clipboard.writeText(urlInput.value).then(() => {
    const btn = $('shareCopyBtn');
    if (btn) {
      const origHTML = btn.innerHTML;
      // ED2-30: Show green "Copied!" feedback with checkmark, then revert
      btn.innerHTML = '<span class="msi" style="font-size:16px;vertical-align:middle">check</span> Copied!';
      btn.style.color = '#1e8e3e';
      btn.style.borderColor = '#1e8e3e';
      setTimeout(() => {
        btn.innerHTML = origHTML;
        btn.style.color = '';
        btn.style.borderColor = '';
      }, 1500);
    }
  }).catch(() => {
    // Fallback: select the text so the user can manually Ctrl+C
    urlInput.select();
    urlInput.focus();
  });
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

      // Connect to WebSocket for co-editing using fileId as room
      const name = 'User ' + Math.floor(Math.random() * 100);
      const relay = params.get('relay') || DEFAULT_RELAY_URL;
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
