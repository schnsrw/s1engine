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

const DEFAULT_RELAY_URL = 'ws://localhost:8787';
const RECONNECT_DELAYS = [2000, 4000, 8000, 16000, 30000];
const MAX_RECONNECT_ATTEMPTS = 5;
const CURSOR_BROADCAST_INTERVAL = 2000;
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

// ─── Public API ───────────────────────────────────────

/**
 * Start a collaboration session.
 */
export function startCollab(room, name, relayUrl) {
  if (!state.doc) return;

  roomId = room;
  userName = name || 'Anonymous';
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

  try {
    ws = new WebSocket(url);
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
    for (const op of offlineBuffer) {
      ws.send(JSON.stringify({ type: 'op', room: roomId, data: JSON.stringify(op) }));
    }
    offlineBuffer = [];
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

function sendOp(opData) {
  const payload = JSON.stringify(opData);
  if (connected && ws && ws.readyState === 1) {
    ws.send(JSON.stringify({ type: 'op', room: roomId, data: payload }));
  } else {
    offlineBuffer.push(opData);
  }
}

// ─── Message Handlers ─────────────────────────────────

function handleMessage(msg) {
  switch (msg.type) {
    case 'joined':
      peerId = msg.peerId;
      updatePeerList(msg.peers || []);
      // Send full document state for sync
      sendFullSync();
      break;

    case 'peer-join':
      addPeer(msg.peerId, msg.userName, msg.userColor);
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
          const el = $('pageContainer').querySelector(`[data-node-id="${op.nodeId}"]`);
          if (el) {
            const updated = renderNodeById(op.nodeId);
            // Don't move local cursor
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
        // Full document sync — only apply if we have no content yet
        // (first joiner gets the doc from the host)
        try {
          if (op.docBase64) {
            const binary = atob(op.docBase64);
            const bytes = new Uint8Array(binary.length);
            for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
            state.doc = state.engine.open(bytes);
            renderDocument();
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
  const sel = window.getSelection();
  if (!sel || sel.rangeCount === 0) return;

  let node = sel.anchorNode;
  while (node && node !== document && !node.dataset?.nodeId) {
    node = node.parentElement;
  }
  if (!node || !node.dataset?.nodeId) return;

  try {
    ws.send(JSON.stringify({
      type: 'awareness',
      room: roomId,
      data: JSON.stringify({
        peerId,
        nodeId: node.dataset.nodeId,
        offset: sel.anchorOffset,
        userName,
        userColor,
      }),
    }));
  } catch (_) {}
}

function applyRemoteAwareness(dataStr, fromPeerId) {
  if (!dataStr) return;
  try {
    const cursor = JSON.parse(dataStr);
    renderPeerCursor(cursor);
  } catch (_) {}
}

// ─── Peer Cursor Rendering ────────────────────────────

const peers = new Map();

function updatePeerList(peerList) {
  peers.clear();
  for (const p of peerList) {
    peers.set(p.peerId, { userName: p.userName, userColor: p.userColor });
  }
  updatePeerCount();
}

function addPeer(pid, name, color) {
  peers.set(pid, { userName: name, userColor: color });
  updatePeerCount();
}

function removePeer(pid) {
  peers.delete(pid);
  const el = document.getElementById(`peer-cursor-${pid}`);
  if (el) el.remove();
  updatePeerCount();
}

function renderPeerCursor(cursor) {
  if (!cursor || !cursor.nodeId) return;

  // Remove old cursor for this peer
  const oldEl = document.getElementById(`peer-cursor-${cursor.peerId}`);
  if (oldEl) oldEl.remove();

  const page = $('pageContainer');
  if (!page) return;

  const paraEl = page.querySelector(`[data-node-id="${cursor.nodeId}"]`);
  if (!paraEl) return;

  // Create cursor line
  const cursorEl = document.createElement('div');
  cursorEl.className = 'peer-cursor';
  cursorEl.id = `peer-cursor-${cursor.peerId}`;
  cursorEl.style.borderLeftColor = cursor.userColor || '#999';

  // Name label
  const label = document.createElement('span');
  label.className = 'peer-cursor-label';
  label.textContent = cursor.userName || 'Peer';
  label.style.backgroundColor = cursor.userColor || '#999';
  cursorEl.appendChild(label);

  // Try to position at the correct character offset
  // cursor.offset is in codepoints; DOM TextNode offsets are UTF-16 code units.
  // Convert codepoint offset to UTF-16 offset for range positioning.
  try {
    const range = document.createRange();
    const textNode = paraEl.firstChild;
    if (textNode && textNode.nodeType === 3) {
      const cpOffset = cursor.offset || 0;
      const chars = [...textNode.textContent];
      const clampedCp = Math.min(cpOffset, chars.length);
      // Convert codepoint offset to UTF-16 string offset
      let utf16Off = 0;
      for (let i = 0; i < clampedCp; i++) utf16Off += chars[i].length;
      range.setStart(textNode, Math.min(utf16Off, textNode.length));
      range.collapse(true);
      const rect = range.getBoundingClientRect();
      const paraRect = paraEl.getBoundingClientRect();
      cursorEl.style.left = (rect.left - paraRect.left) + 'px';
    }
  } catch (_) {
    cursorEl.style.left = '0px';
  }

  paraEl.style.position = 'relative';
  paraEl.appendChild(cursorEl);

  // Auto-remove after 5s if no update
  setTimeout(() => {
    const el = document.getElementById(`peer-cursor-${cursor.peerId}`);
    if (el) el.remove();
  }, 5000);
}

function clearPeerCursors() {
  document.querySelectorAll('.peer-cursor').forEach(el => el.remove());
  peers.clear();
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
  indicator.title =
    status === 'connected' ? 'Connected to relay server' :
    status === 'reconnecting' ? 'Reconnecting...' :
    'Disconnected';

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
    const count = peers.size;
    el.textContent = count > 0 ? `${count + 1} users` : '';
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
    dot.title = p.userName || 'Peer';
    container.appendChild(dot);
  }
}

// ─── Share Dialog ─────────────────────────────────────

export function showShareDialog() {
  if (roomId) {
    // Already in a session — confirm disconnect
    const modal = $('shareModal');
    if (modal) modal.classList.remove('show');
    stopCollab();
    return;
  }

  if (!state.doc) {
    alert('Open or create a document first.');
    return;
  }

  // Generate a room ID and shareable URL
  const generatedRoom = Math.random().toString(36).substring(2, 10);
  const shareUrl = `${window.location.origin}${window.location.pathname}?room=${generatedRoom}&relay=${encodeURIComponent(DEFAULT_RELAY_URL)}`;

  // Populate modal fields
  const urlInput = $('shareUrlInput');
  if (urlInput) urlInput.value = shareUrl;

  const nameInput = $('shareNameInput');
  if (nameInput) nameInput.value = 'User ' + Math.floor(Math.random() * 100);

  const relayInput = $('shareRelayInput');
  if (relayInput) relayInput.value = DEFAULT_RELAY_URL;

  // Update peer list display
  updateSharePeerList();

  // Show modal
  const modal = $('shareModal');
  if (modal) modal.classList.add('show');

  // Store generated room for the Start button
  if (modal) modal.dataset.room = generatedRoom;
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
      const orig = btn.textContent;
      btn.textContent = 'Copied!';
      setTimeout(() => { btn.textContent = orig; }, 1500);
    }
  }).catch(() => {
    urlInput.select();
    document.execCommand('copy');
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

  // Show self
  if (roomId) {
    const self = document.createElement('div');
    self.className = 'share-peer-item';
    self.innerHTML = `<span class="share-peer-dot" style="background:${userColor}"></span>
      <span class="share-peer-name">${userName} (you)</span>`;
    list.appendChild(self);
  }

  for (const [pid, p] of peers) {
    const el = document.createElement('div');
    el.className = 'share-peer-item';
    el.innerHTML = `<span class="share-peer-dot" style="background:${p.userColor || '#999'}"></span>
      <span class="share-peer-name">${p.userName || 'Anonymous'}</span>`;
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
 * Auto-join if URL has ?room= parameter.
 */
export function checkAutoJoin() {
  const params = new URLSearchParams(window.location.search);
  const room = params.get('room');
  if (!room) return;

  const relay = params.get('relay') || DEFAULT_RELAY_URL;
  const name = 'User ' + Math.floor(Math.random() * 100);

  // Wait for document to be ready
  setTimeout(() => {
    if (state.engine && state.doc) {
      startCollab(room, name, relay);
    }
  }, 1000);
}
