/**
 * collab.js — Real-time collaboration via s1engine CRDT + WebSocket relay
 *
 * Architecture (ADR-001: s1engine-first):
 * - s1engine CRDT (Fugue algorithm) is the collaboration backend
 * - OnlyOffice is the rendering facade
 * - WebSocket relay (scripts/relay.js) broadcasts ops between clients
 *
 * Flow:
 * 1. User edits in OnlyOffice → adapter captures change
 * 2. Change applied to s1engine CRDT → returns serialized ops
 * 3. Ops sent to relay via WebSocket
 * 4. Remote ops received → applied to s1engine CRDT
 * 5. s1engine model updated → adapter refreshes OnlyOffice display
 */

let ws = null;
let collabDoc = null;
let replicaId = null;
let userName = 'User';
let userColor = '#1a73e8';

/**
 * Connect to the collaboration relay server.
 * @param {string} docId - Document identifier
 * @param {string} relayUrl - WebSocket relay URL (e.g., ws://localhost:9090)
 * @param {WasmCollabDocument} wasmCollabDoc - The CRDT document from s1engine
 * @param {object} api - sdkjs API instance
 * @param {object} opts - { userName, userColor }
 */
export function connect(docId, relayUrl, wasmCollabDoc, api, opts) {
  collabDoc = wasmCollabDoc;
  replicaId = collabDoc.replica_id();
  if (opts) {
    userName = opts.userName || userName;
    userColor = opts.userColor || userColor;
  }

  ws = new WebSocket(relayUrl);

  ws.onopen = function() {
    console.log('[collab] Connected to relay');
    // Join the document room
    ws.send(JSON.stringify({
      type: 'join',
      docId: docId,
      replicaId: replicaId,
      userName: userName
    }));

    // Send state vector for delta sync
    var sv = collabDoc.get_state_vector();
    ws.send(JSON.stringify({
      type: 'sync_request',
      stateVector: sv
    }));
  };

  ws.onmessage = function(event) {
    try {
      var msg = JSON.parse(event.data);
      handleMessage(msg, api);
    } catch (e) {
      console.error('[collab] Failed to handle message:', e);
    }
  };

  ws.onclose = function() {
    console.log('[collab] Disconnected from relay');
  };

  ws.onerror = function(err) {
    console.error('[collab] WebSocket error:', err);
  };
}

/**
 * Handle incoming messages from the relay.
 */
function handleMessage(msg, api) {
  switch (msg.type) {
    case 'ops':
      // Remote operations from another replica
      try {
        collabDoc.apply_remote_ops(JSON.stringify(msg.ops));
        // Refresh OnlyOffice display from updated s1engine model
        refreshEditorFromModel(api);
      } catch (e) {
        console.error('[collab] Failed to apply remote ops:', e);
      }
      break;

    case 'sync_response':
      // Delta sync — apply missing operations
      if (msg.changes) {
        var changes = JSON.parse(msg.changes);
        for (var i = 0; i < changes.length; i++) {
          try {
            collabDoc.apply_remote_ops(JSON.stringify(changes[i]));
          } catch (e) {}
        }
        refreshEditorFromModel(api);
      }
      break;

    case 'cursor':
      // Remote cursor position update
      // TODO: render remote cursor indicators in OnlyOffice
      break;

    case 'joined':
      console.log('[collab] User joined: ' + msg.userName);
      break;

    case 'left':
      console.log('[collab] User left: ' + msg.userName);
      break;
  }
}

/**
 * Send a local operation to the relay for broadcast.
 */
export function broadcastOps(opsJson) {
  if (ws && ws.readyState === WebSocket.OPEN) {
    ws.send(JSON.stringify({
      type: 'ops',
      replicaId: replicaId,
      ops: JSON.parse(opsJson)
    }));
  }
}

/**
 * Send cursor position update to relay.
 */
export function broadcastCursor(nodeId, offset) {
  if (ws && ws.readyState === WebSocket.OPEN && collabDoc) {
    try {
      var cursorJson = collabDoc.set_cursor(nodeId, offset, userName, userColor);
      ws.send(JSON.stringify({
        type: 'cursor',
        replicaId: replicaId,
        cursor: JSON.parse(cursorJson)
      }));
    } catch (e) {}
  }
}

/**
 * Disconnect from relay.
 */
export function disconnect() {
  if (ws) {
    ws.close();
    ws = null;
  }
  collabDoc = null;
}

/**
 * Refresh OnlyOffice editor from s1engine CRDT model.
 * Called after applying remote operations.
 */
function refreshEditorFromModel(api) {
  // TODO: Incremental update instead of full rebuild.
  // For now, get the updated text and update the editor.
  // This is a placeholder — full implementation needs M3-level
  // structural mapping from the updated s1engine model to sdkjs.
  var logicDoc = api.WordControl.m_oLogicDocument;
  if (logicDoc) {
    logicDoc.Recalculate();
  }
}

/**
 * Check if currently connected.
 */
export function isConnected() {
  return ws !== null && ws.readyState === WebSocket.OPEN;
}
