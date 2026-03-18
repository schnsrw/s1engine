/* @ts-self-types="./s1engine_wasm.d.ts" */

//#region exports

/**
 * A collaborative document that supports CRDT-based real-time editing.
 *
 * Each instance represents one replica. Local edits produce operations that
 * must be broadcast to other replicas. Remote operations are applied via
 * `apply_remote_ops`.
 */
export class WasmCollabDocument {
    constructor() {
        throw new Error('cannot invoke `new` directly');
    }
    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(WasmCollabDocument.prototype);
        obj.__wbg_ptr = ptr;
        WasmCollabDocumentFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmCollabDocumentFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmcollabdocument_free(ptr, 0);
    }
    /**
     * Apply a remote awareness (cursor) update from another replica.
     * @param {string} update_json
     */
    apply_awareness_update(update_json) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(update_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmcollabdocument_apply_awareness_update(this.__wbg_ptr, ptr0, len0);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Apply a local text deletion and return serialized ops for broadcast.
     * @param {string} target_id
     * @param {number} offset
     * @param {number} length
     * @returns {string}
     */
    apply_local_delete_text(target_id, offset, length) {
        let deferred3_0;
        let deferred3_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(target_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            _assertNum(offset);
            _assertNum(length);
            const ret = wasm.wasmcollabdocument_apply_local_delete_text(this.__wbg_ptr, ptr0, len0, offset, length);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * Apply a local formatting change and return serialized ops for broadcast.
     * @param {string} target_id
     * @param {string} key
     * @param {string} value
     * @returns {string}
     */
    apply_local_format(target_id, key, value) {
        let deferred5_0;
        let deferred5_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(target_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ptr1 = passStringToWasm0(key, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            const ptr2 = passStringToWasm0(value, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len2 = WASM_VECTOR_LEN;
            const ret = wasm.wasmcollabdocument_apply_local_format(this.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2);
            var ptr4 = ret[0];
            var len4 = ret[1];
            if (ret[3]) {
                ptr4 = 0; len4 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred5_0 = ptr4;
            deferred5_1 = len4;
            return getStringFromWasm0(ptr4, len4);
        } finally {
            wasm.__wbindgen_free(deferred5_0, deferred5_1, 1);
        }
    }
    /**
     * Apply a local text insertion and return serialized ops for broadcast.
     *
     * Returns a JSON string of the operations that must be sent to other replicas.
     * @param {string} target_id
     * @param {number} offset
     * @param {string} text
     * @returns {string}
     */
    apply_local_insert_text(target_id, offset, text) {
        let deferred4_0;
        let deferred4_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(target_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            _assertNum(offset);
            const ptr1 = passStringToWasm0(text, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            const ret = wasm.wasmcollabdocument_apply_local_insert_text(this.__wbg_ptr, ptr0, len0, offset, ptr1, len1);
            var ptr3 = ret[0];
            var len3 = ret[1];
            if (ret[3]) {
                ptr3 = 0; len3 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred4_0 = ptr3;
            deferred4_1 = len3;
            return getStringFromWasm0(ptr3, len3);
        } finally {
            wasm.__wbindgen_free(deferred4_0, deferred4_1, 1);
        }
    }
    /**
     * Apply remote operations received from another replica.
     *
     * Accepts a JSON string of a CRDT operation (as produced by apply_local_* methods).
     * @param {string} ops_json
     */
    apply_remote_ops(ops_json) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(ops_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmcollabdocument_apply_remote_ops(this.__wbg_ptr, ptr0, len0);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Check if redo is available.
     * @returns {boolean}
     */
    can_redo() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wasmcollabdocument_can_redo(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Check if undo is available.
     * @returns {boolean}
     */
    can_undo() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wasmcollabdocument_can_undo(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Compact the operation log (merge consecutive single-char inserts).
     */
    compact_op_log() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wasmcollabdocument_compact_op_log(this.__wbg_ptr);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Export the collaborative document to a format (docx, odt, txt, md).
     * @param {string} format
     * @returns {Uint8Array}
     */
    export(format) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(format, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmcollabdocument_export(this.__wbg_ptr, ptr0, len0);
        if (ret[3]) {
            throw takeFromExternrefTable0(ret[2]);
        }
        var v2 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        return v2;
    }
    /**
     * Free the document (for manual memory management from JS).
     */
    free_doc() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        wasm.wasmcollabdocument_free_doc(this.__wbg_ptr);
    }
    /**
     * Get operations that have happened since a given state vector.
     *
     * Used for delta sync: peer sends their state vector, you return
     * the operations they're missing.
     * @param {string} state_vector_json
     * @returns {string}
     */
    get_changes_since(state_vector_json) {
        let deferred3_0;
        let deferred3_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(state_vector_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmcollabdocument_get_changes_since(this.__wbg_ptr, ptr0, len0);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * Get all peer cursors as JSON.
     *
     * Returns a JSON array of cursor states:
     * `[{"replicaId":2,"nodeId":"1:5","offset":3,"userName":"Alice","userColor":"#ff0000"},...]`
     * @returns {string}
     */
    get_peers_json() {
        let deferred2_0;
        let deferred2_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ret = wasm.wasmcollabdocument_get_peers_json(this.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Get the current state vector as JSON.
     *
     * Used for delta synchronization — send your state vector to a peer
     * to find out what operations you're missing.
     * @returns {string}
     */
    get_state_vector() {
        let deferred2_0;
        let deferred2_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ret = wasm.wasmcollabdocument_get_state_vector(this.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Get the size of the operation log.
     * @returns {number}
     */
    op_log_size() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wasmcollabdocument_op_log_size(this.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ret[0] >>> 0;
    }
    /**
     * Redo the last undone operation.
     * @returns {string}
     */
    redo() {
        let deferred2_0;
        let deferred2_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ret = wasm.wasmcollabdocument_redo(this.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Get the replica ID of this collaborative document.
     * @returns {bigint}
     */
    replica_id() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wasmcollabdocument_replica_id(this.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return BigInt.asUintN(64, ret[0]);
    }
    /**
     * Set the local cursor position and return an awareness update for broadcast.
     * @param {string} node_id
     * @param {number} offset
     * @param {string} user_name
     * @param {string} user_color
     * @returns {string}
     */
    set_cursor(node_id, offset, user_name, user_color) {
        let deferred5_0;
        let deferred5_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(node_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            _assertNum(offset);
            const ptr1 = passStringToWasm0(user_name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            const ptr2 = passStringToWasm0(user_color, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len2 = WASM_VECTOR_LEN;
            const ret = wasm.wasmcollabdocument_set_cursor(this.__wbg_ptr, ptr0, len0, offset, ptr1, len1, ptr2, len2);
            var ptr4 = ret[0];
            var len4 = ret[1];
            if (ret[3]) {
                ptr4 = 0; len4 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred5_0 = ptr4;
            deferred5_1 = len4;
            return getStringFromWasm0(ptr4, len4);
        } finally {
            wasm.__wbindgen_free(deferred5_0, deferred5_1, 1);
        }
    }
    /**
     * Get the document content as HTML.
     * @returns {string}
     */
    to_html() {
        let deferred2_0;
        let deferred2_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ret = wasm.wasmcollabdocument_to_html(this.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Get the document content as plain text.
     * @returns {string}
     */
    to_plain_text() {
        let deferred2_0;
        let deferred2_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ret = wasm.wasmcollabdocument_to_plain_text(this.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Get the number of tombstones.
     * @returns {number}
     */
    tombstone_count() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wasmcollabdocument_tombstone_count(this.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ret[0] >>> 0;
    }
    /**
     * Undo the last local operation.
     *
     * Returns JSON of the undo operation for broadcast, or null if nothing to undo.
     * @returns {string}
     */
    undo() {
        let deferred2_0;
        let deferred2_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ret = wasm.wasmcollabdocument_undo(this.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
}
if (Symbol.dispose) WasmCollabDocument.prototype[Symbol.dispose] = WasmCollabDocument.prototype.free;

/**
 * A document handle for reading, editing, and exporting.
 */
export class WasmDocument {
    constructor() {
        throw new Error('cannot invoke `new` directly');
    }
    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(WasmDocument.prototype);
        obj.__wbg_ptr = ptr;
        WasmDocumentFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmDocumentFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmdocument_free(ptr, 0);
    }
    /**
     * Accept all tracked changes in the document.
     *
     * Insertions keep their content; deletions are removed; format changes
     * keep the new formatting. All revision attributes are stripped.
     */
    accept_all_changes() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wasmdocument_accept_all_changes(this.__wbg_ptr);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Accept a single tracked change by node ID string ("replica:counter").
     * @param {string} node_id_str
     */
    accept_change(node_id_str) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(node_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmdocument_accept_change(this.__wbg_ptr, ptr0, len0);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Append a heading at the given level (1-6).
     *
     * Returns the heading paragraph's node ID.
     * @param {number} level
     * @param {string} text
     * @returns {string}
     */
    append_heading(level, text) {
        let deferred3_0;
        let deferred3_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            _assertNum(level);
            const ptr0 = passStringToWasm0(text, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_append_heading(this.__wbg_ptr, level, ptr0, len0);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * Append a new paragraph with plain text at the end of the document body.
     *
     * Returns the new paragraph's node ID as "replica:counter".
     * @param {string} text
     * @returns {string}
     */
    append_paragraph(text) {
        let deferred3_0;
        let deferred3_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(text, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_append_paragraph(this.__wbg_ptr, ptr0, len0);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * Begin a batch of operations that form a single undo step.
     *
     * All operations between `begin_batch()` and `end_batch()` are applied
     * individually. On `end_batch()`, they are merged into a single undo
     * unit by collapsing the undo history.
     * @param {string} label
     */
    begin_batch(label) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(label, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmdocument_begin_batch(this.__wbg_ptr, ptr0, len0);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Get all body-level node IDs with their types as JSON.
     *
     * Returns `[{"id":"0:5","type":"Paragraph"},{"id":"0:12","type":"Table"},...]`
     * @returns {string}
     */
    body_children_json() {
        let deferred2_0;
        let deferred2_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ret = wasm.wasmdocument_body_children_json(this.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Get the body node ID as "replica:counter" string.
     * @returns {string | undefined}
     */
    body_id() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wasmdocument_body_id(this.__wbg_ptr);
        if (ret[3]) {
            throw takeFromExternrefTable0(ret[2]);
        }
        let v1;
        if (ret[0] !== 0) {
            v1 = getStringFromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        }
        return v1;
    }
    /**
     * Check if redo is available.
     * @returns {boolean}
     */
    can_redo() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wasmdocument_can_redo(this.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ret[0] !== 0;
    }
    /**
     * Check if undo is available.
     * @returns {boolean}
     */
    can_undo() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wasmdocument_can_undo(this.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ret[0] !== 0;
    }
    /**
     * Clear all undo/redo history.
     */
    clear_history() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wasmdocument_clear_history(this.__wbg_ptr);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Explicitly release document memory. The document cannot be used after this.
     */
    close() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        wasm.wasmdocument_close(this.__wbg_ptr);
    }
    /**
     * Delete a comment and its range markers.
     * @param {string} comment_id
     */
    delete_comment(comment_id) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(comment_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmdocument_delete_comment(this.__wbg_ptr, ptr0, len0);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Delete an image node (and its containing paragraph if empty).
     * @param {string} image_id_str
     */
    delete_image(image_id_str) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(image_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmdocument_delete_image(this.__wbg_ptr, ptr0, len0);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Delete a body-level node (paragraph, table, heading, etc.).
     * @param {string} node_id_str
     */
    delete_node(node_id_str) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(node_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmdocument_delete_node(this.__wbg_ptr, ptr0, len0);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Delete a selection range spanning one or more paragraphs.
     *
     * If start and end are in the same paragraph, deletes the text range.
     * If they span multiple paragraphs, deletes the tail of the first,
     * all intermediate paragraphs, the head of the last, then merges
     * the first and last paragraphs.
     * @param {string} start_node_str
     * @param {number} start_offset
     * @param {string} end_node_str
     * @param {number} end_offset
     */
    delete_selection(start_node_str, start_offset, end_node_str, end_offset) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(start_node_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        _assertNum(start_offset);
        const ptr1 = passStringToWasm0(end_node_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        _assertNum(end_offset);
        const ret = wasm.wasmdocument_delete_selection(this.__wbg_ptr, ptr0, len0, start_offset, ptr1, len1, end_offset);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Delete a column at the given index across all rows.
     * @param {string} table_id_str
     * @param {number} col_index
     */
    delete_table_column(table_id_str, col_index) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(table_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        _assertNum(col_index);
        const ret = wasm.wasmdocument_delete_table_column(this.__wbg_ptr, ptr0, len0, col_index);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Delete a row at the given index in a table.
     * @param {string} table_id_str
     * @param {number} row_index
     */
    delete_table_row(table_id_str, row_index) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(table_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        _assertNum(row_index);
        const ret = wasm.wasmdocument_delete_table_row(this.__wbg_ptr, ptr0, len0, row_index);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Delete text in a paragraph at a given character offset.
     *
     * Correctly handles multi-run paragraphs by finding the right text node(s).
     * @param {string} node_id_str
     * @param {number} offset
     * @param {number} length
     */
    delete_text_in_paragraph(node_id_str, offset, length) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(node_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        _assertNum(offset);
        _assertNum(length);
        const ret = wasm.wasmdocument_delete_text_in_paragraph(this.__wbg_ptr, ptr0, len0, offset, length);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * End a batch and merge all operations since `begin_batch()` into
     * a single undo step.
     */
    end_batch() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wasmdocument_end_batch(this.__wbg_ptr);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Export the document to the specified format.
     *
     * Format should be one of: "docx", "odt", "txt", "pdf".
     * Returns the exported bytes.
     * @param {string} format
     * @returns {Uint8Array}
     */
    export(format) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(format, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmdocument_export(this.__wbg_ptr, ptr0, len0);
        if (ret[3]) {
            throw takeFromExternrefTable0(ret[2]);
        }
        var v2 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        return v2;
    }
    /**
     * Export a selection range as clean, portable semantic HTML.
     *
     * The output contains no `data-node-id` attributes, no editor-specific
     * classes, and no track-changes markup. Suitable for clipboard
     * rich-text copy/paste.
     *
     * `start_node_str` / `end_node_str` are paragraph node IDs (e.g.
     * `"0:5"`). `start_offset` / `end_offset` are character offsets within
     * those paragraphs.
     * @param {string} start_node_str
     * @param {number} start_offset
     * @param {string} end_node_str
     * @param {number} end_offset
     * @returns {string}
     */
    export_selection_html(start_node_str, start_offset, end_node_str, end_offset) {
        let deferred4_0;
        let deferred4_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(start_node_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            _assertNum(start_offset);
            const ptr1 = passStringToWasm0(end_node_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            _assertNum(end_offset);
            const ret = wasm.wasmdocument_export_selection_html(this.__wbg_ptr, ptr0, len0, start_offset, ptr1, len1, end_offset);
            var ptr3 = ret[0];
            var len3 = ret[1];
            if (ret[3]) {
                ptr3 = 0; len3 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred4_0 = ptr3;
            deferred4_1 = len3;
            return getStringFromWasm0(ptr3, len3);
        } finally {
            wasm.__wbindgen_free(deferred4_0, deferred4_1, 1);
        }
    }
    /**
     * Find all occurrences of text in the document.
     *
     * Returns JSON array of `{"nodeId":"0:5","offset":3,"length":5}`.
     * @param {string} query
     * @param {boolean} case_sensitive
     * @returns {string}
     */
    find_text(query, case_sensitive) {
        let deferred3_0;
        let deferred3_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(query, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            _assertBoolean(case_sensitive);
            const ret = wasm.wasmdocument_find_text(this.__wbg_ptr, ptr0, len0, case_sensitive);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * Set a formatting attribute on a specific Run node.
     *
     * key/value are string representations parsed to AttributeKey/AttributeValue.
     * Supported keys: "bold", "italic", "underline", "strikethrough",
     * "fontSize", "fontFamily", "color", "highlightColor", "superscript", "subscript".
     * @param {string} run_id_str
     * @param {string} key
     * @param {string} value
     */
    format_run(run_id_str, key, value) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(run_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(key, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(value, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        const ret = wasm.wasmdocument_format_run(this.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Format a text range spanning one or more runs/paragraphs.
     *
     * Internally splits start/end runs as needed and applies the attribute
     * to all runs in the selection range. Single transaction (atomic undo).
     *
     * start_node/end_node are paragraph IDs, offsets are character positions.
     * @param {string} start_node_str
     * @param {number} start_offset
     * @param {string} end_node_str
     * @param {number} end_offset
     * @param {string} key
     * @param {string} value
     */
    format_selection(start_node_str, start_offset, end_node_str, end_offset, key, value) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(start_node_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        _assertNum(start_offset);
        const ptr1 = passStringToWasm0(end_node_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        _assertNum(end_offset);
        const ptr2 = passStringToWasm0(key, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        const ptr3 = passStringToWasm0(value, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len3 = WASM_VECTOR_LEN;
        const ret = wasm.wasmdocument_format_selection(this.__wbg_ptr, ptr0, len0, start_offset, ptr1, len1, end_offset, ptr2, len2, ptr3, len3);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Free the document, releasing memory.
     *
     * After calling this, all other methods will return an error.
     */
    free() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        wasm.wasmdocument_free(this.__wbg_ptr);
    }
    /**
     * Get the node ID of a cell at a given row/column index.
     * @param {string} table_id_str
     * @param {number} row
     * @param {number} col
     * @returns {string}
     */
    get_cell_id(table_id_str, row, col) {
        let deferred3_0;
        let deferred3_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(table_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            _assertNum(row);
            _assertNum(col);
            const ret = wasm.wasmdocument_get_cell_id(this.__wbg_ptr, ptr0, len0, row, col);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * Get the text content of a table cell.
     * @param {string} cell_id_str
     * @returns {string}
     */
    get_cell_text(cell_id_str) {
        let deferred3_0;
        let deferred3_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(cell_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_get_cell_text(this.__wbg_ptr, ptr0, len0);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * Get all comments as a JSON array.
     * @returns {string}
     */
    get_comments_json() {
        let deferred2_0;
        let deferred2_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ret = wasm.wasmdocument_get_comments_json(this.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Get all text in the document as a single string.
     * @returns {string}
     */
    get_document_text() {
        let deferred2_0;
        let deferred2_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ret = wasm.wasmdocument_get_document_text(this.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Get all endnotes as JSON array.
     *
     * Returns `[{"number":1,"text":"Endnote text"},...]`.
     * @returns {string}
     */
    get_endnotes_json() {
        let deferred2_0;
        let deferred2_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ret = wasm.wasmdocument_get_endnotes_json(this.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Get all footnotes as JSON array.
     *
     * Returns `[{"number":1,"text":"Footnote text"},...]`.
     * @returns {string}
     */
    get_footnotes_json() {
        let deferred2_0;
        let deferred2_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ret = wasm.wasmdocument_get_footnotes_json(this.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Get the formatting state of a paragraph as JSON.
     *
     * Returns JSON with keys: bold, italic, underline, strikethrough,
     * fontSize, fontFamily, color, alignment, headingLevel.
     * Values come from the paragraph's attributes and first run's attributes.
     * @param {string} node_id_str
     * @returns {string}
     */
    get_formatting_json(node_id_str) {
        let deferred3_0;
        let deferred3_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(node_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_get_formatting_json(this.__wbg_ptr, ptr0, len0);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * Get header/footer info for a section as JSON.
     *
     * Returns JSON: `{"hasDefaultHeader":true,"hasFirstHeader":false,
     * "defaultHeaderText":"My Header","firstHeaderText":"",
     * "hasDefaultFooter":true,"hasFirstFooter":false,
     * "defaultFooterText":"Page 1","firstFooterText":"",
     * "titlePage":false}`
     * @param {number} section_index
     * @returns {string}
     */
    get_header_footer_info(section_index) {
        let deferred2_0;
        let deferred2_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            _assertNum(section_index);
            const ret = wasm.wasmdocument_get_header_footer_info(this.__wbg_ptr, section_index);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Get the document heading hierarchy as JSON.
     *
     * Returns a JSON array of objects: `[{"nodeId":"r:c","level":1,"text":"..."},...]`
     * Useful for building outline panels and TOC navigation.
     * @returns {string}
     */
    get_headings_json() {
        let deferred2_0;
        let deferred2_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ret = wasm.wasmdocument_get_headings_json(this.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Get image as a data URL for display.
     * @param {string} image_id_str
     * @returns {string}
     */
    get_image_data_url(image_id_str) {
        let deferred3_0;
        let deferred3_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(image_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_get_image_data_url(this.__wbg_ptr, ptr0, len0);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * Get the wrap mode for an image node.
     *
     * Returns one of: "inline", "wrapLeft", "wrapRight", "wrapBoth",
     * "topAndBottom", "behind", "inFront". Defaults to "inline".
     * @param {string} image_id_str
     * @returns {string}
     */
    get_image_wrap_mode(image_id_str) {
        let deferred3_0;
        let deferred3_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(image_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_get_image_wrap_mode(this.__wbg_ptr, ptr0, len0);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * Get layout cache statistics as JSON.
     *
     * Returns `{"hits":N,"misses":N,"entries":N}`.
     * @returns {string}
     */
    get_layout_cache_stats() {
        let deferred2_0;
        let deferred2_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ret = wasm.wasmdocument_get_layout_cache_stats(this.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Get page break information from the layout engine as JSON.
     *
     * Returns `{"pages": [{"pageNum":1, "nodeIds":["0:5","0:12"], "footer":"Page 1", "header":"..."}, ...]}`.
     * This tells the editor which node IDs are on which page, so the editor
     * can show visual page breaks matching the actual layout engine output.
     * @returns {string}
     */
    get_page_map_json() {
        let deferred2_0;
        let deferred2_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ret = wasm.wasmdocument_get_page_map_json(this.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Get page setup properties for the first section as JSON.
     *
     * Returns JSON: `{"pageWidth":612,"pageHeight":792,"marginTop":72,
     * "marginBottom":72,"marginLeft":72,"marginRight":72,"orientation":"portrait"}`
     *
     * All dimensions are in points (1 inch = 72 points).
     * @returns {string}
     */
    get_page_setup_json() {
        let deferred2_0;
        let deferred2_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ret = wasm.wasmdocument_get_page_setup_json(this.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Get the text content of a paragraph (concatenates all runs).
     * @param {string} node_id_str
     * @returns {string}
     */
    get_paragraph_text(node_id_str) {
        let deferred3_0;
        let deferred3_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(node_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_get_paragraph_text(this.__wbg_ptr, ptr0, len0);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * Get formatting of a specific run as JSON.
     *
     * Returns `{"bold":true,"italic":false,...}`.
     * @param {string} run_id_str
     * @returns {string}
     */
    get_run_formatting_json(run_id_str) {
        let deferred3_0;
        let deferred3_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(run_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_get_run_formatting_json(this.__wbg_ptr, ptr0, len0);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * Get run IDs within a paragraph as a JSON array.
     *
     * Returns `["0:5","0:8",...]` — the IDs of all Run nodes in the paragraph.
     * @param {string} paragraph_id_str
     * @returns {string}
     */
    get_run_ids(paragraph_id_str) {
        let deferred3_0;
        let deferred3_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(paragraph_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_get_run_ids(this.__wbg_ptr, ptr0, len0);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * Get the text content of a specific run.
     * @param {string} run_id_str
     * @returns {string}
     */
    get_run_text(run_id_str) {
        let deferred3_0;
        let deferred3_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(run_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_get_run_text(this.__wbg_ptr, ptr0, len0);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * Get section break information for all sections as JSON.
     *
     * Returns a JSON array of objects with section index, break type, and
     * page dimensions for each section.
     * @returns {string}
     */
    get_section_breaks_json() {
        let deferred2_0;
        let deferred2_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ret = wasm.wasmdocument_get_section_breaks_json(this.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Get the column configuration for a section as JSON.
     *
     * Returns JSON: `{"columns":2,"spacing":36.0}`
     * @param {number} section_index
     * @returns {string}
     */
    get_section_columns(section_index) {
        let deferred2_0;
        let deferred2_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            _assertNum(section_index);
            const ret = wasm.wasmdocument_get_section_columns(this.__wbg_ptr, section_index);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Get section properties as JSON.
     * @returns {string}
     */
    get_sections_json() {
        let deferred2_0;
        let deferred2_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ret = wasm.wasmdocument_get_sections_json(this.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Get common formatting across a selection range as JSON.
     *
     * Returns JSON with `true`/`false`/`"mixed"` per property.
     * E.g., `{"bold":true,"italic":"mixed","underline":false}`.
     * @param {string} start_node_str
     * @param {number} start_offset
     * @param {string} end_node_str
     * @param {number} end_offset
     * @returns {string}
     */
    get_selection_formatting_json(start_node_str, start_offset, end_node_str, end_offset) {
        let deferred4_0;
        let deferred4_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(start_node_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            _assertNum(start_offset);
            const ptr1 = passStringToWasm0(end_node_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            _assertNum(end_offset);
            const ret = wasm.wasmdocument_get_selection_formatting_json(this.__wbg_ptr, ptr0, len0, start_offset, ptr1, len1, end_offset);
            var ptr3 = ret[0];
            var len3 = ret[1];
            if (ret[3]) {
                ptr3 = 0; len3 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred4_0 = ptr3;
            deferred4_1 = len3;
            return getStringFromWasm0(ptr3, len3);
        } finally {
            wasm.__wbindgen_free(deferred4_0, deferred4_1, 1);
        }
    }
    /**
     * Get table dimensions as JSON: `{"rows":N,"cols":M}`.
     * @param {string} table_id_str
     * @returns {string}
     */
    get_table_dimensions(table_id_str) {
        let deferred3_0;
        let deferred3_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(table_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_get_table_dimensions(this.__wbg_ptr, ptr0, len0);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * Get all unique font families used in the document.
     *
     * Returns a JSON array of font family names, e.g. `["Arial","Calibri","Georgia"]`.
     * Useful for determining which fonts need to be loaded before layout.
     * @returns {string}
     */
    get_used_fonts() {
        let deferred2_0;
        let deferred2_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ret = wasm.wasmdocument_get_used_fonts(this.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Insert bookmark start/end around a paragraph.
     *
     * Returns the bookmark start node ID.
     * @param {string} para_id_str
     * @param {string} name
     * @returns {string}
     */
    insert_bookmark(para_id_str, name) {
        let deferred4_0;
        let deferred4_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(para_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ptr1 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_insert_bookmark(this.__wbg_ptr, ptr0, len0, ptr1, len1);
            var ptr3 = ret[0];
            var len3 = ret[1];
            if (ret[3]) {
                ptr3 = 0; len3 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred4_0 = ptr3;
            deferred4_1 = len3;
            return getStringFromWasm0(ptr3, len3);
        } finally {
            wasm.__wbindgen_free(deferred4_0, deferred4_1, 1);
        }
    }
    /**
     * Insert a column break inside the specified paragraph.
     *
     * Inserts a ColumnBreak node at the end of the paragraph's children.
     * Returns the column break node ID.
     * @param {string} para_id_str
     * @returns {string}
     */
    insert_column_break(para_id_str) {
        let deferred3_0;
        let deferred3_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(para_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_insert_column_break(this.__wbg_ptr, ptr0, len0);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * Insert a comment with range markers and body.
     *
     * Returns the comment ID string.
     * @param {string} start_node_str
     * @param {string} end_node_str
     * @param {string} author
     * @param {string} text
     * @returns {string}
     */
    insert_comment(start_node_str, end_node_str, author, text) {
        let deferred6_0;
        let deferred6_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(start_node_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ptr1 = passStringToWasm0(end_node_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            const ptr2 = passStringToWasm0(author, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len2 = WASM_VECTOR_LEN;
            const ptr3 = passStringToWasm0(text, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len3 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_insert_comment(this.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3);
            var ptr5 = ret[0];
            var len5 = ret[1];
            if (ret[3]) {
                ptr5 = 0; len5 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred6_0 = ptr5;
            deferred6_1 = len5;
            return getStringFromWasm0(ptr5, len5);
        } finally {
            wasm.__wbindgen_free(deferred6_0, deferred6_1, 1);
        }
    }
    /**
     * Insert a comment with markers positioned at the selected text range.
     *
     * Unlike `insert_comment` which places markers at paragraph boundaries,
     * this positions CommentStart/CommentEnd at the correct run indices
     * based on character offsets within the paragraphs.
     * @param {string} start_node_str
     * @param {number} start_offset
     * @param {string} end_node_str
     * @param {number} end_offset
     * @param {string} author
     * @param {string} text
     * @returns {string}
     */
    insert_comment_at_range(start_node_str, start_offset, end_node_str, end_offset, author, text) {
        let deferred6_0;
        let deferred6_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(start_node_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            _assertNum(start_offset);
            const ptr1 = passStringToWasm0(end_node_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            _assertNum(end_offset);
            const ptr2 = passStringToWasm0(author, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len2 = WASM_VECTOR_LEN;
            const ptr3 = passStringToWasm0(text, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len3 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_insert_comment_at_range(this.__wbg_ptr, ptr0, len0, start_offset, ptr1, len1, end_offset, ptr2, len2, ptr3, len3);
            var ptr5 = ret[0];
            var len5 = ret[1];
            if (ret[3]) {
                ptr5 = 0; len5 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred6_0 = ptr5;
            deferred6_1 = len5;
            return getStringFromWasm0(ptr5, len5);
        } finally {
            wasm.__wbindgen_free(deferred6_0, deferred6_1, 1);
        }
    }
    /**
     * Insert a reply to an existing comment.
     *
     * Returns the reply comment ID string.
     * @param {string} parent_comment_id
     * @param {string} author
     * @param {string} text
     * @returns {string}
     */
    insert_comment_reply(parent_comment_id, author, text) {
        let deferred5_0;
        let deferred5_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(parent_comment_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ptr1 = passStringToWasm0(author, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            const ptr2 = passStringToWasm0(text, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len2 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_insert_comment_reply(this.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2);
            var ptr4 = ret[0];
            var len4 = ret[1];
            if (ret[3]) {
                ptr4 = 0; len4 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred5_0 = ptr4;
            deferred5_1 = len4;
            return getStringFromWasm0(ptr4, len4);
        } finally {
            wasm.__wbindgen_free(deferred5_0, deferred5_1, 1);
        }
    }
    /**
     * Insert an endnote at the current position in a paragraph.
     *
     * Creates an endnote reference in the paragraph and an endnote body
     * at the document root. Returns the endnote body node ID.
     * @param {string} node_id_str
     * @param {string} text
     * @returns {string}
     */
    insert_endnote(node_id_str, text) {
        let deferred4_0;
        let deferred4_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(node_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ptr1 = passStringToWasm0(text, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_insert_endnote(this.__wbg_ptr, ptr0, len0, ptr1, len1);
            var ptr3 = ret[0];
            var len3 = ret[1];
            if (ret[3]) {
                ptr3 = 0; len3 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred4_0 = ptr3;
            deferred4_1 = len3;
            return getStringFromWasm0(ptr3, len3);
        } finally {
            wasm.__wbindgen_free(deferred4_0, deferred4_1, 1);
        }
    }
    /**
     * Insert an equation (inline math) into a paragraph.
     *
     * `node_id_str` is the paragraph to insert into.
     * `latex_source` is the equation source (LaTeX or raw XML).
     * Returns the equation node ID string.
     * @param {string} node_id_str
     * @param {string} latex_source
     * @returns {string}
     */
    insert_equation(node_id_str, latex_source) {
        let deferred4_0;
        let deferred4_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(node_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ptr1 = passStringToWasm0(latex_source, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_insert_equation(this.__wbg_ptr, ptr0, len0, ptr1, len1);
            var ptr3 = ret[0];
            var len3 = ret[1];
            if (ret[3]) {
                ptr3 = 0; len3 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred4_0 = ptr3;
            deferred4_1 = len3;
            return getStringFromWasm0(ptr3, len3);
        } finally {
            wasm.__wbindgen_free(deferred4_0, deferred4_1, 1);
        }
    }
    /**
     * Insert a footnote at the current position in a paragraph.
     *
     * Creates a footnote reference in the paragraph and a footnote body
     * at the document root. Returns the footnote body node ID.
     * @param {string} node_id_str
     * @param {string} text
     * @returns {string}
     */
    insert_footnote(node_id_str, text) {
        let deferred4_0;
        let deferred4_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(node_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ptr1 = passStringToWasm0(text, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_insert_footnote(this.__wbg_ptr, ptr0, len0, ptr1, len1);
            var ptr3 = ret[0];
            var len3 = ret[1];
            if (ret[3]) {
                ptr3 = 0; len3 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred4_0 = ptr3;
            deferred4_1 = len3;
            return getStringFromWasm0(ptr3, len3);
        } finally {
            wasm.__wbindgen_free(deferred4_0, deferred4_1, 1);
        }
    }
    /**
     * Insert a horizontal rule (thematic break) after the given node.
     *
     * Returns the new node ID.
     * @param {string} after_node_str
     * @returns {string}
     */
    insert_horizontal_rule(after_node_str) {
        let deferred3_0;
        let deferred3_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(after_node_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_insert_horizontal_rule(this.__wbg_ptr, ptr0, len0);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * Set a hyperlink URL on a run.
     *
     * tooltip_opt is optional — pass empty string or null for no tooltip.
     * @param {string} run_id_str
     * @param {string} url
     * @param {string} tooltip_opt
     */
    insert_hyperlink(run_id_str, url, tooltip_opt) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(run_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(url, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(tooltip_opt, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        const ret = wasm.wasmdocument_insert_hyperlink(this.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Insert an image after the specified body-level node.
     *
     * Stores bytes in MediaStore, creates Paragraph → Run → Image structure.
     * Returns the paragraph node ID containing the image.
     * @param {string} after_node_str
     * @param {Uint8Array} data
     * @param {string} content_type
     * @param {number} width_pt
     * @param {number} height_pt
     * @returns {string}
     */
    insert_image(after_node_str, data, content_type, width_pt, height_pt) {
        let deferred5_0;
        let deferred5_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(after_node_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ptr1 = passArray8ToWasm0(data, wasm.__wbindgen_malloc);
            const len1 = WASM_VECTOR_LEN;
            const ptr2 = passStringToWasm0(content_type, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len2 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_insert_image(this.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2, width_pt, height_pt);
            var ptr4 = ret[0];
            var len4 = ret[1];
            if (ret[3]) {
                ptr4 = 0; len4 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred5_0 = ptr4;
            deferred5_1 = len4;
            return getStringFromWasm0(ptr4, len4);
        } finally {
            wasm.__wbindgen_free(deferred5_0, deferred5_1, 1);
        }
    }
    /**
     * Insert a line break (soft return) within a paragraph at a character offset.
     *
     * Creates a `LineBreak` node within the run at the specified offset,
     * splitting the text node if the offset falls in the middle.
     * @param {string} node_id_str
     * @param {number} char_offset
     */
    insert_line_break(node_id_str, char_offset) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(node_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        _assertNum(char_offset);
        const ret = wasm.wasmdocument_insert_line_break(this.__wbg_ptr, ptr0, len0, char_offset);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Insert a paragraph with PageBreakBefore after the given node.
     *
     * Returns the new paragraph node ID.
     * @param {string} after_node_str
     * @returns {string}
     */
    insert_page_break(after_node_str) {
        let deferred3_0;
        let deferred3_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(after_node_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_insert_page_break(this.__wbg_ptr, ptr0, len0);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * Insert a new paragraph after a given node.
     *
     * Returns the new paragraph's node ID.
     * @param {string} after_id_str
     * @param {string} text
     * @returns {string}
     */
    insert_paragraph_after(after_id_str, text) {
        let deferred4_0;
        let deferred4_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(after_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ptr1 = passStringToWasm0(text, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_insert_paragraph_after(this.__wbg_ptr, ptr0, len0, ptr1, len1);
            var ptr3 = ret[0];
            var len3 = ret[1];
            if (ret[3]) {
                ptr3 = 0; len3 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred4_0 = ptr3;
            deferred4_1 = len3;
            return getStringFromWasm0(ptr3, len3);
        } finally {
            wasm.__wbindgen_free(deferred4_0, deferred4_1, 1);
        }
    }
    /**
     * Insert a section break after the given node.
     *
     * `break_type` is one of: `"nextPage"`, `"continuous"`, `"evenPage"`, `"oddPage"`.
     *
     * This creates a new section in the document model. Content after the break
     * belongs to the new section with the specified break type.
     * Returns the new section's paragraph node ID (the first paragraph in the new section).
     * @param {string} after_node_str
     * @param {string} break_type
     * @returns {string}
     */
    insert_section_break(after_node_str, break_type) {
        let deferred4_0;
        let deferred4_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(after_node_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ptr1 = passStringToWasm0(break_type, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_insert_section_break(this.__wbg_ptr, ptr0, len0, ptr1, len1);
            var ptr3 = ret[0];
            var len3 = ret[1];
            if (ret[3]) {
                ptr3 = 0; len3 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred4_0 = ptr3;
            deferred4_1 = len3;
            return getStringFromWasm0(ptr3, len3);
        } finally {
            wasm.__wbindgen_free(deferred4_0, deferred4_1, 1);
        }
    }
    /**
     * Insert a tab node at the given character offset within a paragraph.
     *
     * Like `insert_line_break`, this inserts a `Tab` node inside the
     * appropriate run, splitting text nodes as needed. Tab nodes render
     * as `&emsp;` in HTML and as proper tab stops in layout.
     * @param {string} node_id_str
     * @param {number} char_offset
     */
    insert_tab(node_id_str, char_offset) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(node_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        _assertNum(char_offset);
        const ret = wasm.wasmdocument_insert_tab(this.__wbg_ptr, ptr0, len0, char_offset);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Insert a table after the specified body-level node.
     *
     * Creates a table with the given number of rows and columns,
     * each cell containing an empty paragraph. Returns the table node ID.
     * @param {string} after_node_str
     * @param {number} rows
     * @param {number} cols
     * @returns {string}
     */
    insert_table(after_node_str, rows, cols) {
        let deferred3_0;
        let deferred3_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(after_node_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            _assertNum(rows);
            _assertNum(cols);
            const ret = wasm.wasmdocument_insert_table(this.__wbg_ptr, ptr0, len0, rows, cols);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * Insert a column at the given index across all rows.
     * @param {string} table_id_str
     * @param {number} col_index
     */
    insert_table_column(table_id_str, col_index) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(table_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        _assertNum(col_index);
        const ret = wasm.wasmdocument_insert_table_column(this.__wbg_ptr, ptr0, len0, col_index);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Insert a Table of Contents after the given node.
     *
     * `max_level` controls the deepest heading level included (1-9, default 3).
     * If `title` is non-empty, it is set as the TOC title.
     * Returns the TOC node ID string.
     * @param {string} after_node_str
     * @param {number} max_level
     * @param {string} title
     * @returns {string}
     */
    insert_table_of_contents(after_node_str, max_level, title) {
        let deferred4_0;
        let deferred4_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(after_node_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            _assertNum(max_level);
            const ptr1 = passStringToWasm0(title, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_insert_table_of_contents(this.__wbg_ptr, ptr0, len0, max_level, ptr1, len1);
            var ptr3 = ret[0];
            var len3 = ret[1];
            if (ret[3]) {
                ptr3 = 0; len3 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred4_0 = ptr3;
            deferred4_1 = len3;
            return getStringFromWasm0(ptr3, len3);
        } finally {
            wasm.__wbindgen_free(deferred4_0, deferred4_1, 1);
        }
    }
    /**
     * Insert a row at the given index in a table.
     *
     * Creates cells matching the column count of existing rows.
     * Returns the new row's node ID.
     * @param {string} table_id_str
     * @param {number} row_index
     * @returns {string}
     */
    insert_table_row(table_id_str, row_index) {
        let deferred3_0;
        let deferred3_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(table_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            _assertNum(row_index);
            const ret = wasm.wasmdocument_insert_table_row(this.__wbg_ptr, ptr0, len0, row_index);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * Insert text at an offset in a paragraph's first text node.
     * @param {string} node_id_str
     * @param {number} offset
     * @param {string} text
     */
    insert_text_in_paragraph(node_id_str, offset, text) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(node_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        _assertNum(offset);
        const ptr1 = passStringToWasm0(text, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmdocument_insert_text_in_paragraph(this.__wbg_ptr, ptr0, len0, offset, ptr1, len1);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Check if a batch is currently active.
     * @returns {boolean}
     */
    is_batching() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wasmdocument_is_batching(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Check if this document handle is still valid.
     * @returns {boolean}
     */
    is_valid() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wasmdocument_is_valid(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Merge cells in a range by setting ColSpan/RowSpan attributes.
     * @param {string} table_id_str
     * @param {number} start_row
     * @param {number} start_col
     * @param {number} end_row
     * @param {number} end_col
     */
    merge_cells(table_id_str, start_row, start_col, end_row, end_col) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(table_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        _assertNum(start_row);
        _assertNum(start_col);
        _assertNum(end_row);
        _assertNum(end_col);
        const ret = wasm.wasmdocument_merge_cells(this.__wbg_ptr, ptr0, len0, start_row, start_col, end_row, end_col);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Merge two adjacent paragraphs.
     *
     * Moves all runs from `second_id` into `first_id` (preserving formatting),
     * then deletes the now-empty `second_id`. Used for Backspace at the start
     * of a paragraph.
     * @param {string} first_id_str
     * @param {string} second_id_str
     */
    merge_paragraphs(first_id_str, second_id_str) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(first_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(second_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmdocument_merge_paragraphs(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Get the document author (from metadata).
     * @returns {string | undefined}
     */
    metadata_author() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wasmdocument_metadata_author(this.__wbg_ptr);
        if (ret[3]) {
            throw takeFromExternrefTable0(ret[2]);
        }
        let v1;
        if (ret[0] !== 0) {
            v1 = getStringFromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        }
        return v1;
    }
    /**
     * Get the document title (from metadata).
     * @returns {string | undefined}
     */
    metadata_title() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wasmdocument_metadata_title(this.__wbg_ptr);
        if (ret[3]) {
            throw takeFromExternrefTable0(ret[2]);
        }
        let v1;
        if (ret[0] !== 0) {
            v1 = getStringFromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        }
        return v1;
    }
    /**
     * Move a node (e.g. an image paragraph) to be after another node in
     * the same parent (body). Used for drag-and-drop reordering.
     * @param {string} node_id_str
     * @param {string} after_id_str
     */
    move_node_after(node_id_str, after_id_str) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(node_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(after_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmdocument_move_node_after(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Move a node to be before another node in the same parent (body).
     * @param {string} node_id_str
     * @param {string} before_id_str
     */
    move_node_before(node_id_str, before_id_str) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(node_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(before_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmdocument_move_node_before(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Get detailed info about a node as JSON.
     *
     * Returns `{"id":"0:5","type":"Paragraph","text":"Hello","children":[...],...}`
     * @param {string} node_id_str
     * @returns {string}
     */
    node_info_json(node_id_str) {
        let deferred3_0;
        let deferred3_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(node_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_node_info_json(this.__wbg_ptr, ptr0, len0);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * Get the number of paragraphs in the document.
     * @returns {number}
     */
    paragraph_count() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wasmdocument_paragraph_count(this.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ret[0] >>> 0;
    }
    /**
     * Get top-level paragraph IDs as a JSON array of "replica:counter" strings.
     * @returns {string}
     */
    paragraph_ids_json() {
        let deferred2_0;
        let deferred2_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ret = wasm.wasmdocument_paragraph_ids_json(this.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Paste formatted text (with per-run styling) at a position in the document.
     *
     * `target_node_str` is the paragraph node ID (e.g. `"0:5"`).
     * `char_offset` is the character offset within that paragraph.
     * `runs_json` is a JSON string describing the formatted text to paste:
     *
     * ```json
     * {
     *   "paragraphs": [
     *     {
     *       "runs": [
     *         {"text": "Hello ", "bold": false},
     *         {"text": "world", "bold": true, "italic": true,
     *          "fontSize": 14, "fontFamily": "Arial",
     *          "color": "FF0000", "underline": true,
     *          "strikethrough": false}
     *       ]
     *     },
     *     {
     *       "runs": [
     *         {"text": "Second paragraph"}
     *       ]
     *     }
     *   ]
     * }
     * ```
     *
     * For a single paragraph: inserts all run text at the offset and formats
     * each run's character range. For multiple paragraphs: splits the target
     * paragraph, inserts new paragraphs between, each with formatted runs.
     * @param {string} target_node_str
     * @param {number} char_offset
     * @param {string} runs_json
     */
    paste_formatted_runs_json(target_node_str, char_offset, runs_json) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(target_node_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        _assertNum(char_offset);
        const ptr1 = passStringToWasm0(runs_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmdocument_paste_formatted_runs_json(this.__wbg_ptr, ptr0, len0, char_offset, ptr1, len1);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Insert plain text at cursor position, splitting on newlines.
     * @param {string} para_id_str
     * @param {number} offset
     * @param {string} text
     */
    paste_plain_text(para_id_str, offset, text) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(para_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        _assertNum(offset);
        const ptr1 = passStringToWasm0(text, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmdocument_paste_plain_text(this.__wbg_ptr, ptr0, len0, offset, ptr1, len1);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Redo the last undone operation. Returns true if something was redone.
     * @returns {boolean}
     */
    redo() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wasmdocument_redo(this.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ret[0] !== 0;
    }
    /**
     * Reject all tracked changes in the document.
     *
     * Insertions are removed; deletions are un-deleted; format changes
     * restore original formatting. All revision attributes are stripped.
     */
    reject_all_changes() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wasmdocument_reject_all_changes(this.__wbg_ptr);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Reject a single tracked change by node ID string ("replica:counter").
     * @param {string} node_id_str
     */
    reject_change(node_id_str) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(node_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmdocument_reject_change(this.__wbg_ptr, ptr0, len0);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Remove a hyperlink from a run.
     * @param {string} run_id_str
     */
    remove_hyperlink(run_id_str) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(run_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmdocument_remove_hyperlink(this.__wbg_ptr, ptr0, len0);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Render a single node (paragraph, table, etc.) as HTML.
     *
     * Returns the HTML string for that node only, suitable for incremental
     * DOM updates. Uses the same rendering as `to_html()`.
     * @param {string} node_id_str
     * @returns {string}
     */
    render_node_html(node_id_str) {
        let deferred3_0;
        let deferred3_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(node_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_render_node_html(this.__wbg_ptr, ptr0, len0);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * Render a table with only specific rows (for split-table pagination).
     *
     * `table_id_str` is the table node ID (e.g., "1:5").
     * `row_ids_json` is a JSON array of row node IDs to include (e.g., '["1:6","1:7"]').
     * `chunk_id` is a unique identifier for this chunk (used as data-node-id).
     * `is_continuation` indicates if this is a continuation chunk (for styling).
     * @param {string} table_id_str
     * @param {string} row_ids_json
     * @param {string} chunk_id
     * @param {boolean} is_continuation
     * @returns {string}
     */
    render_table_chunk(table_id_str, row_ids_json, chunk_id, is_continuation) {
        let deferred5_0;
        let deferred5_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(table_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ptr1 = passStringToWasm0(row_ids_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            const ptr2 = passStringToWasm0(chunk_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len2 = WASM_VECTOR_LEN;
            _assertBoolean(is_continuation);
            const ret = wasm.wasmdocument_render_table_chunk(this.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2, is_continuation);
            var ptr4 = ret[0];
            var len4 = ret[1];
            if (ret[3]) {
                ptr4 = 0; len4 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred5_0 = ptr4;
            deferred5_1 = len4;
            return getStringFromWasm0(ptr4, len4);
        } finally {
            wasm.__wbindgen_free(deferred5_0, deferred5_1, 1);
        }
    }
    /**
     * Replace all occurrences of query with replacement.
     *
     * Returns the number of replacements made. Single transaction.
     * @param {string} query
     * @param {string} replacement
     * @param {boolean} case_sensitive
     * @returns {number}
     */
    replace_all(query, replacement, case_sensitive) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(query, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(replacement, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        _assertBoolean(case_sensitive);
        const ret = wasm.wasmdocument_replace_all(this.__wbg_ptr, ptr0, len0, ptr1, len1, case_sensitive);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ret[0] >>> 0;
    }
    /**
     * Replace text at a specific location.
     * @param {string} node_id_str
     * @param {number} offset
     * @param {number} length
     * @param {string} replacement
     */
    replace_text(node_id_str, offset, length, replacement) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(node_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        _assertNum(offset);
        _assertNum(length);
        const ptr1 = passStringToWasm0(replacement, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmdocument_replace_text(this.__wbg_ptr, ptr0, len0, offset, length, ptr1, len1);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Resize an image by setting width/height attributes.
     * @param {string} image_id_str
     * @param {number} width_pt
     * @param {number} height_pt
     */
    resize_image(image_id_str, width_pt, height_pt) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(image_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmdocument_resize_image(this.__wbg_ptr, ptr0, len0, width_pt, height_pt);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Set paragraph alignment ("left", "center", "right", "justify").
     * @param {string} node_id_str
     * @param {string} alignment
     */
    set_alignment(node_id_str, alignment) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(node_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(alignment, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmdocument_set_alignment(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Set the document author (metadata).
     * @param {string} author
     */
    set_author(author) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(author, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmdocument_set_author(this.__wbg_ptr, ptr0, len0);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Set bold on a paragraph's first run.
     * @param {string} node_id_str
     * @param {boolean} bold
     */
    set_bold(node_id_str, bold) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(node_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        _assertBoolean(bold);
        const ret = wasm.wasmdocument_set_bold(this.__wbg_ptr, ptr0, len0, bold);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Set the background color of a table cell.
     * @param {string} cell_id_str
     * @param {string} hex
     */
    set_cell_background(cell_id_str, hex) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(cell_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(hex, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmdocument_set_cell_background(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Set the text content of a table cell.
     *
     * Replaces the entire cell content with the given text. Sets text in
     * the first paragraph and deletes any extra paragraphs.
     * @param {string} cell_id_str
     * @param {string} text
     */
    set_cell_text(cell_id_str, text) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(cell_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(text, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmdocument_set_cell_text(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Set text color on a paragraph's first run (hex string like "FF0000").
     * @param {string} node_id_str
     * @param {string} hex
     */
    set_color(node_id_str, hex) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(node_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(hex, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmdocument_set_color(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Set font family on a paragraph's first run.
     * @param {string} node_id_str
     * @param {string} font
     */
    set_font_family(node_id_str, font) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(node_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(font, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmdocument_set_font_family(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Set font size on a paragraph's first run (in points).
     * @param {string} node_id_str
     * @param {number} size_pt
     */
    set_font_size(node_id_str, size_pt) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(node_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmdocument_set_font_size(this.__wbg_ptr, ptr0, len0, size_pt);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Set header or footer text for a given section.
     *
     * `section_index`: 0-based section index.
     * `hf_kind`: `"header"` or `"footer"`.
     * `hf_type`: `"default"` or `"first"`.
     * `text`: Plain text content. If empty, the header/footer content is cleared.
     *
     * If the section does not have a header/footer of the specified type,
     * one is created with a new Paragraph > Run > Text structure.
     * @param {number} section_index
     * @param {string} hf_kind
     * @param {string} hf_type_str
     * @param {string} text
     */
    set_header_footer_text(section_index, hf_kind, hf_type_str, text) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        _assertNum(section_index);
        const ptr0 = passStringToWasm0(hf_kind, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(hf_type_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(text, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        const ret = wasm.wasmdocument_set_header_footer_text(this.__wbg_ptr, section_index, ptr0, len0, ptr1, len1, ptr2, len2);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Set the heading level of a paragraph.
     *
     * Level 0 removes the heading style (converts to normal paragraph).
     * Level 1-6 sets the corresponding heading style.
     * @param {string} node_id_str
     * @param {number} level
     */
    set_heading_level(node_id_str, level) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(node_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        _assertNum(level);
        const ret = wasm.wasmdocument_set_heading_level(this.__wbg_ptr, ptr0, len0, level);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Set alt text on an image.
     * @param {string} image_id_str
     * @param {string} alt
     */
    set_image_alt_text(image_id_str, alt) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(image_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(alt, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmdocument_set_image_alt_text(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Set image wrap mode.
     *
     * `mode` is one of: "inline", "wrapLeft", "wrapRight", "wrapBoth",
     * "topAndBottom", "behind", "inFront".
     * Defaults to "inline" if not set.
     * @param {string} image_id_str
     * @param {string} mode
     */
    set_image_wrap_mode(image_id_str, mode) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(image_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(mode, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmdocument_set_image_wrap_mode(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Set paragraph indentation (left, right, or first-line).
     *
     * `indent_type` is one of: "left", "right", "firstLine".
     * `value_pt` is the indent value in points.
     * @param {string} node_id_str
     * @param {string} indent_type
     * @param {number} value_pt
     */
    set_indent(node_id_str, indent_type, value_pt) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(node_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(indent_type, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmdocument_set_indent(this.__wbg_ptr, ptr0, len0, ptr1, len1, value_pt);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Set italic on a paragraph's first run.
     * @param {string} node_id_str
     * @param {boolean} italic
     */
    set_italic(node_id_str, italic) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(node_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        _assertBoolean(italic);
        const ret = wasm.wasmdocument_set_italic(this.__wbg_ptr, ptr0, len0, italic);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Set the line spacing for a paragraph.
     *
     * `spacing` is one of: "single", "1.5", "double", or a numeric multiplier (e.g. "1.15").
     * @param {string} node_id_str
     * @param {string} spacing
     */
    set_line_spacing(node_id_str, spacing) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(node_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(spacing, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmdocument_set_line_spacing(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Set list format on a paragraph.
     *
     * format: "bullet", "decimal", "none".
     * @param {string} para_id_str
     * @param {string} format
     * @param {number} level
     */
    set_list_format(para_id_str, format, level) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(para_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(format, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        _assertNum(level);
        const ret = wasm.wasmdocument_set_list_format(this.__wbg_ptr, ptr0, len0, ptr1, len1, level);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Set page setup properties for all sections from JSON.
     *
     * Accepts JSON: `{"pageWidth":612,"pageHeight":792,"marginTop":72,
     * "marginBottom":72,"marginLeft":72,"marginRight":72,"orientation":"portrait"}`
     *
     * All dimensions are in points (1 inch = 72 points).
     * Updates all sections in the document to use the new page dimensions.
     * @param {string} json
     */
    set_page_setup(json) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmdocument_set_page_setup(this.__wbg_ptr, ptr0, len0);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Set paragraph keep options (keep with next, keep lines together).
     *
     * `keep_type` is one of: "keepWithNext", "keepLinesTogether".
     * `enabled` controls whether the option is on or off.
     * @param {string} node_id_str
     * @param {string} keep_type
     * @param {boolean} enabled
     */
    set_paragraph_keep(node_id_str, keep_type, enabled) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(node_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(keep_type, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        _assertBoolean(enabled);
        const ret = wasm.wasmdocument_set_paragraph_keep(this.__wbg_ptr, ptr0, len0, ptr1, len1, enabled);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Set paragraph spacing (before and/or after) in points.
     *
     * `spacing_type` is one of: "before", "after".
     * `value_pt` is the spacing value in points.
     * @param {string} node_id_str
     * @param {string} spacing_type
     * @param {number} value_pt
     */
    set_paragraph_spacing(node_id_str, spacing_type, value_pt) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(node_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(spacing_type, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmdocument_set_paragraph_spacing(this.__wbg_ptr, ptr0, len0, ptr1, len1, value_pt);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Set the paragraph style ID on a paragraph node.
     *
     * Sets the `StyleId` attribute to any arbitrary style name
     * (e.g., "Title", "Subtitle", "Quote", "Code", "Heading1", etc.).
     * Pass an empty string to clear the style (revert to Normal).
     * @param {string} node_id_str
     * @param {string} style_id
     */
    set_paragraph_style_id(node_id_str, style_id) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(node_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(style_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmdocument_set_paragraph_style_id(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Replace the text content of a paragraph.
     *
     * For multi-run paragraphs, this first checks whether the total text
     * across all runs already matches `new_text`.  If so, it is a no-op
     * (preserving per-run formatting).  If the text has genuinely changed,
     * all extra runs are deleted and the remaining single run receives the
     * new text.
     * @param {string} node_id_str
     * @param {string} new_text
     */
    set_paragraph_text(node_id_str, new_text) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(node_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(new_text, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmdocument_set_paragraph_text(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Set the number of columns for a section.
     *
     * `section_index`: 0-based section index (0 for the default/first section).
     * `columns`: number of columns (1-6). Pass 1 for single-column layout.
     * `spacing_pt`: spacing between columns in points (default: 36.0 = 0.5in).
     * @param {number} section_index
     * @param {number} columns
     * @param {number} spacing_pt
     */
    set_section_columns(section_index, columns, spacing_pt) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        _assertNum(section_index);
        _assertNum(columns);
        const ret = wasm.wasmdocument_set_section_columns(this.__wbg_ptr, section_index, columns, spacing_pt);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Set strikethrough on a paragraph's first run.
     * @param {string} node_id_str
     * @param {boolean} strikethrough
     */
    set_strikethrough(node_id_str, strikethrough) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(node_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        _assertBoolean(strikethrough);
        const ret = wasm.wasmdocument_set_strikethrough(this.__wbg_ptr, ptr0, len0, strikethrough);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Set the document title (metadata).
     * @param {string} title
     */
    set_title(title) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(title, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmdocument_set_title(this.__wbg_ptr, ptr0, len0);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Set or clear the "different first page" flag for a section.
     *
     * When enabled, the first page of the section uses the "first" header/footer
     * instead of the "default" one.
     * @param {number} section_index
     * @param {boolean} enabled
     */
    set_title_page(section_index, enabled) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        _assertNum(section_index);
        _assertBoolean(enabled);
        const ret = wasm.wasmdocument_set_title_page(this.__wbg_ptr, section_index, enabled);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Set underline on a paragraph's first run.
     * @param {string} node_id_str
     * @param {boolean} underline
     */
    set_underline(node_id_str, underline) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(node_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        _assertBoolean(underline);
        const ret = wasm.wasmdocument_set_underline(this.__wbg_ptr, ptr0, len0, underline);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Set the maximum number of undo steps to keep.
     *
     * `max` of 0 means unlimited. Excess history is trimmed (oldest first).
     * @param {number} max
     */
    set_undo_history_cap(max) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        _assertNum(max);
        const ret = wasm.wasmdocument_set_undo_history_cap(this.__wbg_ptr, max);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Split a previously merged cell back to individual cells.
     *
     * Removes ColSpan/RowSpan attributes from the target cell and clears
     * the "continue" RowSpan from cells that were part of the merge.
     * @param {string} table_id_str
     * @param {number} row
     * @param {number} col
     */
    split_merged_cell(table_id_str, row, col) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(table_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        _assertNum(row);
        _assertNum(col);
        const ret = wasm.wasmdocument_split_merged_cell(this.__wbg_ptr, ptr0, len0, row, col);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Split a paragraph at a character offset.
     *
     * Creates a new paragraph after the current one with the tail text.
     * If the original paragraph is a heading, the new paragraph inherits
     * the same heading style.
     *
     * Returns the new paragraph's node ID as "replica:counter".
     * @param {string} node_id_str
     * @param {number} char_offset
     * @returns {string}
     */
    split_paragraph(node_id_str, char_offset) {
        let deferred3_0;
        let deferred3_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(node_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            _assertNum(char_offset);
            const ret = wasm.wasmdocument_split_paragraph(this.__wbg_ptr, ptr0, len0, char_offset);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * Split a Run node at a character offset.
     *
     * Creates a new Run after the original with the tail text, preserving
     * all formatting attributes. Returns the new run's node ID.
     * @param {string} run_id_str
     * @param {number} char_offset
     * @returns {string}
     */
    split_run(run_id_str, char_offset) {
        let deferred3_0;
        let deferred3_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(run_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            _assertNum(char_offset);
            const ret = wasm.wasmdocument_split_run(this.__wbg_ptr, ptr0, len0, char_offset);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * Render the document as HTML with formatting, images, and hyperlinks.
     * @returns {string}
     */
    to_html() {
        let deferred2_0;
        let deferred2_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ret = wasm.wasmdocument_to_html(this.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Render the document layout as structured JSON for canvas-based rendering.
     *
     * Returns a JSON object with page, block, line, and glyph run data
     * including exact positions, font information, and styling. This enables
     * pixel-accurate canvas rendering as an alternative to DOM-based HTML.
     *
     * Uses fallback font metrics (no system fonts). For more accurate layout,
     * use `to_layout_json_with_fonts()` after loading fonts via
     * `WasmFontDatabase`.
     * @returns {string}
     */
    to_layout_json() {
        let deferred2_0;
        let deferred2_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ret = wasm.wasmdocument_to_layout_json(this.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Render the document layout as structured JSON with a custom layout configuration.
     *
     * Use this to control page dimensions and margins.
     * @param {WasmLayoutConfig} config
     * @returns {string}
     */
    to_layout_json_with_config(config) {
        let deferred2_0;
        let deferred2_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            _assertClass(config, WasmLayoutConfig);
            if (config.__wbg_ptr === 0) {
                throw new Error('Attempt to use a moved value');
            }
            const ret = wasm.wasmdocument_to_layout_json_with_config(this.__wbg_ptr, config.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Render the document layout as structured JSON with loaded fonts.
     *
     * Use this when you have loaded fonts via `WasmFontDatabase` for
     * accurate text shaping and positioning.
     * @param {WasmFontDatabase} font_db
     * @returns {string}
     */
    to_layout_json_with_fonts(font_db) {
        let deferred2_0;
        let deferred2_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            _assertClass(font_db, WasmFontDatabase);
            if (font_db.__wbg_ptr === 0) {
                throw new Error('Attempt to use a moved value');
            }
            const ret = wasm.wasmdocument_to_layout_json_with_fonts(this.__wbg_ptr, font_db.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Render the document layout as structured JSON with loaded fonts and custom config.
     *
     * Combines custom page dimensions/margins with loaded font data for
     * the most accurate canvas rendering.
     * @param {WasmFontDatabase} font_db
     * @param {WasmLayoutConfig} config
     * @returns {string}
     */
    to_layout_json_with_fonts_and_config(font_db, config) {
        let deferred2_0;
        let deferred2_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            _assertClass(font_db, WasmFontDatabase);
            if (font_db.__wbg_ptr === 0) {
                throw new Error('Attempt to use a moved value');
            }
            _assertClass(config, WasmLayoutConfig);
            if (config.__wbg_ptr === 0) {
                throw new Error('Attempt to use a moved value');
            }
            const ret = wasm.wasmdocument_to_layout_json_with_fonts_and_config(this.__wbg_ptr, font_db.__wbg_ptr, config.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Render the document as paginated HTML using the layout engine.
     *
     * Produces CSS-positioned HTML with real page boundaries. Each page
     * is rendered as a separate div with absolute-positioned content.
     * Uses US Letter page size (612pt x 792pt) with 1-inch margins.
     *
     * Text is positioned using fallback font metrics (no system fonts
     * are available in WASM). For more accurate layout, use
     * `to_paginated_html_with_fonts()` after loading fonts via
     * `WasmFontDatabase`.
     * @returns {string}
     */
    to_paginated_html() {
        let deferred2_0;
        let deferred2_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ret = wasm.wasmdocument_to_paginated_html(this.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Render the document as paginated HTML with a custom layout configuration.
     *
     * Use this to control page dimensions and margins.
     * @param {WasmLayoutConfig} config
     * @returns {string}
     */
    to_paginated_html_with_config(config) {
        let deferred2_0;
        let deferred2_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            _assertClass(config, WasmLayoutConfig);
            if (config.__wbg_ptr === 0) {
                throw new Error('Attempt to use a moved value');
            }
            const ret = wasm.wasmdocument_to_paginated_html_with_config(this.__wbg_ptr, config.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Render the document as paginated HTML with loaded fonts.
     *
     * Use this when you have loaded fonts via `WasmFontDatabase` for
     * accurate text shaping and positioning.
     * @param {WasmFontDatabase} font_db
     * @returns {string}
     */
    to_paginated_html_with_fonts(font_db) {
        let deferred2_0;
        let deferred2_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            _assertClass(font_db, WasmFontDatabase);
            if (font_db.__wbg_ptr === 0) {
                throw new Error('Attempt to use a moved value');
            }
            const ret = wasm.wasmdocument_to_paginated_html_with_fonts(this.__wbg_ptr, font_db.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Render the document as paginated HTML with loaded fonts and custom config.
     *
     * Combines custom page dimensions/margins with loaded font data for
     * the most accurate layout.
     * @param {WasmFontDatabase} font_db
     * @param {WasmLayoutConfig} config
     * @returns {string}
     */
    to_paginated_html_with_fonts_and_config(font_db, config) {
        let deferred2_0;
        let deferred2_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            _assertClass(font_db, WasmFontDatabase);
            if (font_db.__wbg_ptr === 0) {
                throw new Error('Attempt to use a moved value');
            }
            _assertClass(config, WasmLayoutConfig);
            if (config.__wbg_ptr === 0) {
                throw new Error('Attempt to use a moved value');
            }
            const ret = wasm.wasmdocument_to_paginated_html_with_fonts_and_config(this.__wbg_ptr, font_db.__wbg_ptr, config.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Export the document as PDF bytes.
     *
     * Uses fallback font metrics (no system fonts). For more accurate
     * output, use `to_pdf_with_fonts()` after loading fonts via
     * `WasmFontDatabase`.
     *
     * Returns the raw PDF bytes suitable for download or embedding.
     * @returns {Uint8Array}
     */
    to_pdf() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wasmdocument_to_pdf(this.__wbg_ptr);
        if (ret[3]) {
            throw takeFromExternrefTable0(ret[2]);
        }
        var v1 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        return v1;
    }
    /**
     * Export the document as a PDF data URL.
     *
     * Returns a string like `data:application/pdf;base64,...` suitable
     * for embedding in iframes, download links, or `window.open()`.
     * @returns {string}
     */
    to_pdf_data_url() {
        let deferred2_0;
        let deferred2_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ret = wasm.wasmdocument_to_pdf_data_url(this.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Export the document as a PDF data URL using loaded fonts.
     * @param {WasmFontDatabase} font_db
     * @returns {string}
     */
    to_pdf_data_url_with_fonts(font_db) {
        let deferred2_0;
        let deferred2_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            _assertClass(font_db, WasmFontDatabase);
            if (font_db.__wbg_ptr === 0) {
                throw new Error('Attempt to use a moved value');
            }
            const ret = wasm.wasmdocument_to_pdf_data_url_with_fonts(this.__wbg_ptr, font_db.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Export the document as PDF bytes using loaded fonts.
     *
     * Use this when you have loaded fonts via `WasmFontDatabase` for
     * accurate text shaping and glyph embedding.
     * @param {WasmFontDatabase} font_db
     * @returns {Uint8Array}
     */
    to_pdf_with_fonts(font_db) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        _assertClass(font_db, WasmFontDatabase);
        if (font_db.__wbg_ptr === 0) {
            throw new Error('Attempt to use a moved value');
        }
        const ret = wasm.wasmdocument_to_pdf_with_fonts(this.__wbg_ptr, font_db.__wbg_ptr);
        if (ret[3]) {
            throw takeFromExternrefTable0(ret[2]);
        }
        var v1 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        return v1;
    }
    /**
     * Extract all text content as a plain string.
     * @returns {string}
     */
    to_plain_text() {
        let deferred2_0;
        let deferred2_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ret = wasm.wasmdocument_to_plain_text(this.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Get the number of tracked changes in the document.
     * @returns {number}
     */
    tracked_changes_count() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wasmdocument_tracked_changes_count(this.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ret[0] >>> 0;
    }
    /**
     * Get all tracked changes as a JSON array.
     *
     * Returns `[{"nodeId":"0:5","type":"Insert","author":"...","date":"..."},...]`
     * @returns {string}
     */
    tracked_changes_json() {
        let deferred2_0;
        let deferred2_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ret = wasm.wasmdocument_tracked_changes_json(this.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Undo the last editing operation. Returns true if something was undone.
     * @returns {boolean}
     */
    undo() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wasmdocument_undo(this.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ret[0] !== 0;
    }
    /**
     * Update all Table of Contents entries in the document.
     *
     * Rescans headings and regenerates TOC child paragraphs.
     */
    update_table_of_contents() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wasmdocument_update_table_of_contents(this.__wbg_ptr);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
}
if (Symbol.dispose) WasmDocument.prototype[Symbol.dispose] = WasmDocument.prototype.free;

/**
 * A fluent builder for constructing documents.
 */
export class WasmDocumentBuilder {
    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(WasmDocumentBuilder.prototype);
        obj.__wbg_ptr = ptr;
        WasmDocumentBuilderFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmDocumentBuilderFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmdocumentbuilder_free(ptr, 0);
    }
    /**
     * Set the document author.
     * @param {string} author
     * @returns {WasmDocumentBuilder}
     */
    author(author) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        const ptr = this.__destroy_into_raw();
        _assertNum(ptr);
        const ptr0 = passStringToWasm0(author, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmdocumentbuilder_author(ptr, ptr0, len0);
        return WasmDocumentBuilder.__wrap(ret);
    }
    /**
     * Build the document. Consumes the builder.
     *
     * Returns an error if the document exceeds the maximum node count
     * limit (100,000 nodes) or the maximum depth limit (100) to prevent
     * OOM in the WASM environment.
     * @returns {WasmDocument}
     */
    build() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        const ptr = this.__destroy_into_raw();
        _assertNum(ptr);
        const ret = wasm.wasmdocumentbuilder_build(ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return WasmDocument.__wrap(ret[0]);
    }
    /**
     * Add a heading at the specified level (1-6).
     * @param {number} level
     * @param {string} text
     * @returns {WasmDocumentBuilder}
     */
    heading(level, text) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        const ptr = this.__destroy_into_raw();
        _assertNum(ptr);
        _assertNum(level);
        const ptr0 = passStringToWasm0(text, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmdocumentbuilder_heading(ptr, level, ptr0, len0);
        return WasmDocumentBuilder.__wrap(ret);
    }
    /**
     * Create a new document builder.
     */
    constructor() {
        const ret = wasm.wasmdocumentbuilder_new();
        this.__wbg_ptr = ret >>> 0;
        WasmDocumentBuilderFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * Add a paragraph with plain text.
     * @param {string} text
     * @returns {WasmDocumentBuilder}
     */
    text(text) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        const ptr = this.__destroy_into_raw();
        _assertNum(ptr);
        const ptr0 = passStringToWasm0(text, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmdocumentbuilder_text(ptr, ptr0, len0);
        return WasmDocumentBuilder.__wrap(ret);
    }
    /**
     * Set the document title.
     * @param {string} title
     * @returns {WasmDocumentBuilder}
     */
    title(title) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        const ptr = this.__destroy_into_raw();
        _assertNum(ptr);
        const ptr0 = passStringToWasm0(title, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmdocumentbuilder_title(ptr, ptr0, len0);
        return WasmDocumentBuilder.__wrap(ret);
    }
}
if (Symbol.dispose) WasmDocumentBuilder.prototype[Symbol.dispose] = WasmDocumentBuilder.prototype.free;

/**
 * The main entry point for s1engine in WASM.
 */
export class WasmEngine {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmEngineFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmengine_free(ptr, 0);
    }
    /**
     * Create a new empty document.
     * @returns {WasmDocument}
     */
    create() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wasmengine_create(this.__wbg_ptr);
        return WasmDocument.__wrap(ret);
    }
    /**
     * Create a new collaborative document.
     *
     * `replica_id` must be unique per user/session (e.g., random u64).
     * @param {bigint} replica_id
     * @returns {WasmCollabDocument}
     */
    create_collab(replica_id) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        _assertBigInt(replica_id);
        const ret = wasm.wasmengine_create_collab(this.__wbg_ptr, replica_id);
        return WasmCollabDocument.__wrap(ret);
    }
    /**
     * Create a new engine instance.
     */
    constructor() {
        const ret = wasm.wasmengine_new();
        this.__wbg_ptr = ret >>> 0;
        WasmEngineFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * Open a document from bytes with auto-detected format.
     *
     * Supports DOCX, ODT, and TXT formats.
     * @param {Uint8Array} data
     * @returns {WasmDocument}
     */
    open(data) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passArray8ToWasm0(data, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmengine_open(this.__wbg_ptr, ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return WasmDocument.__wrap(ret[0]);
    }
    /**
     * Open a document from bytes with an explicit format.
     *
     * Format should be one of: "docx", "odt", "txt".
     * @param {Uint8Array} data
     * @param {string} format
     * @returns {WasmDocument}
     */
    open_as(data, format) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passArray8ToWasm0(data, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(format, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmengine_open_as(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return WasmDocument.__wrap(ret[0]);
    }
    /**
     * Open a file as a collaborative document.
     *
     * The document is loaded and wrapped in a CRDT-aware container.
     * @param {Uint8Array} data
     * @param {bigint} replica_id
     * @returns {WasmCollabDocument}
     */
    open_collab(data, replica_id) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passArray8ToWasm0(data, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        _assertBigInt(replica_id);
        const ret = wasm.wasmengine_open_collab(this.__wbg_ptr, ptr0, len0, replica_id);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return WasmCollabDocument.__wrap(ret[0]);
    }
}
if (Symbol.dispose) WasmEngine.prototype[Symbol.dispose] = WasmEngine.prototype.free;

/**
 * A font database for WASM environments.
 *
 * Since WASM has no filesystem access, fonts must be loaded manually
 * via `load_font()`.
 */
export class WasmFontDatabase {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmFontDatabaseFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmfontdatabase_free(ptr, 0);
    }
    /**
     * Get the number of loaded font faces.
     * @returns {number}
     */
    font_count() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wasmfontdatabase_font_count(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Check if a font family is available (exact or via substitution).
     * @param {string} family
     * @returns {boolean}
     */
    has_font(family) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(family, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmfontdatabase_has_font(this.__wbg_ptr, ptr0, len0);
        return ret !== 0;
    }
    /**
     * Load a font from raw bytes (TTF/OTF).
     * @param {Uint8Array} data
     */
    load_font(data) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passArray8ToWasm0(data, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.wasmfontdatabase_load_font(this.__wbg_ptr, ptr0, len0);
    }
    /**
     * Create a new empty font database.
     */
    constructor() {
        const ret = wasm.wasmfontdatabase_new();
        this.__wbg_ptr = ret >>> 0;
        WasmFontDatabaseFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
}
if (Symbol.dispose) WasmFontDatabase.prototype[Symbol.dispose] = WasmFontDatabase.prototype.free;

/**
 * Configuration for paginated HTML layout.
 *
 * Controls page dimensions and margins for the layout engine.
 * Defaults to US Letter (8.5" x 11") with 1-inch margins.
 */
export class WasmLayoutConfig {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmLayoutConfigFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmlayoutconfig_free(ptr, 0);
    }
    /**
     * Get the bottom margin in points.
     * @returns {number}
     */
    margin_bottom() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wasmlayoutconfig_margin_bottom(this.__wbg_ptr);
        return ret;
    }
    /**
     * Get the left margin in points.
     * @returns {number}
     */
    margin_left() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wasmlayoutconfig_margin_left(this.__wbg_ptr);
        return ret;
    }
    /**
     * Get the right margin in points.
     * @returns {number}
     */
    margin_right() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wasmlayoutconfig_margin_right(this.__wbg_ptr);
        return ret;
    }
    /**
     * Get the top margin in points.
     * @returns {number}
     */
    margin_top() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wasmlayoutconfig_margin_top(this.__wbg_ptr);
        return ret;
    }
    /**
     * Create a new layout configuration with US Letter defaults.
     *
     * Page: 612pt x 792pt (8.5" x 11")
     * Margins: 72pt (1") on all sides.
     */
    constructor() {
        const ret = wasm.wasmlayoutconfig_new();
        this.__wbg_ptr = ret >>> 0;
        WasmLayoutConfigFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * Get the page height in points.
     * @returns {number}
     */
    page_height() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wasmlayoutconfig_page_height(this.__wbg_ptr);
        return ret;
    }
    /**
     * Get the page width in points.
     * @returns {number}
     */
    page_width() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wasmlayoutconfig_page_width(this.__wbg_ptr);
        return ret;
    }
    /**
     * Set the bottom margin in points.
     * @param {number} margin
     */
    set_margin_bottom(margin) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        wasm.wasmlayoutconfig_set_margin_bottom(this.__wbg_ptr, margin);
    }
    /**
     * Set the left margin in points.
     * @param {number} margin
     */
    set_margin_left(margin) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        wasm.wasmlayoutconfig_set_margin_left(this.__wbg_ptr, margin);
    }
    /**
     * Set the right margin in points.
     * @param {number} margin
     */
    set_margin_right(margin) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        wasm.wasmlayoutconfig_set_margin_right(this.__wbg_ptr, margin);
    }
    /**
     * Set the top margin in points.
     * @param {number} margin
     */
    set_margin_top(margin) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        wasm.wasmlayoutconfig_set_margin_top(this.__wbg_ptr, margin);
    }
    /**
     * Set the page height in points.
     * @param {number} height
     */
    set_page_height(height) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        wasm.wasmlayoutconfig_set_page_height(this.__wbg_ptr, height);
    }
    /**
     * Set the page width in points.
     * @param {number} width
     */
    set_page_width(width) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        wasm.wasmlayoutconfig_set_page_width(this.__wbg_ptr, width);
    }
}
if (Symbol.dispose) WasmLayoutConfig.prototype[Symbol.dispose] = WasmLayoutConfig.prototype.free;

/**
 * PDF editor for reading, annotating, and modifying existing PDFs.
 */
export class WasmPdfEditor {
    constructor() {
        throw new Error('cannot invoke `new` directly');
    }
    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(WasmPdfEditor.prototype);
        obj.__wbg_ptr = ptr;
        WasmPdfEditorFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmPdfEditorFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmpdfeditor_free(ptr, 0);
    }
    /**
     * Add a free text annotation (text box).
     * @param {number} page
     * @param {number} x
     * @param {number} y
     * @param {number} width
     * @param {number} height
     * @param {string} text
     * @param {number} font_size
     */
    add_freetext_annotation(page, x, y, width, height, text, font_size) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        _assertNum(page);
        const ptr0 = passStringToWasm0(text, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmpdfeditor_add_freetext_annotation(this.__wbg_ptr, page, x, y, width, height, ptr0, len0, font_size);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Add a highlight annotation (0-indexed page, quad points as flat array).
     * @param {number} page
     * @param {Float64Array} quads
     * @param {number} r
     * @param {number} g
     * @param {number} b
     * @param {string} author
     * @param {string} content
     */
    add_highlight_annotation(page, quads, r, g, b, author, content) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        _assertNum(page);
        const ptr0 = passArrayF64ToWasm0(quads, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(author, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(content, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        const ret = wasm.wasmpdfeditor_add_highlight_annotation(this.__wbg_ptr, page, ptr0, len0, r, g, b, ptr1, len1, ptr2, len2);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Add an ink (freehand) annotation. Points is a flat array [x1,y1,x2,y2,...].
     * @param {number} page
     * @param {Float64Array} points
     * @param {number} r
     * @param {number} g
     * @param {number} b
     * @param {number} width
     */
    add_ink_annotation(page, points, r, g, b, width) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        _assertNum(page);
        const ptr0 = passArrayF64ToWasm0(points, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmpdfeditor_add_ink_annotation(this.__wbg_ptr, page, ptr0, len0, r, g, b, width);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Add a redaction annotation.
     * @param {number} page
     * @param {number} x
     * @param {number} y
     * @param {number} width
     * @param {number} height
     */
    add_redaction(page, x, y, width, height) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        _assertNum(page);
        const ret = wasm.wasmpdfeditor_add_redaction(this.__wbg_ptr, page, x, y, width, height);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Add a sticky note (text) annotation (0-indexed page).
     * @param {number} page
     * @param {number} x
     * @param {number} y
     * @param {string} author
     * @param {string} content
     */
    add_text_annotation(page, x, y, author, content) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        _assertNum(page);
        const ptr0 = passStringToWasm0(author, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(content, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmpdfeditor_add_text_annotation(this.__wbg_ptr, page, x, y, ptr0, len0, ptr1, len1);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Add text overlay on a page at a given position (0-indexed).
     * @param {number} page
     * @param {number} x
     * @param {number} y
     * @param {number} width
     * @param {number} height
     * @param {string} text
     * @param {number} font_size
     */
    add_text_overlay(page, x, y, width, height, text, font_size) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        _assertNum(page);
        const ptr0 = passStringToWasm0(text, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmpdfeditor_add_text_overlay(this.__wbg_ptr, page, x, y, width, height, ptr0, len0, font_size);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Add a white rectangle to cover content on a page (0-indexed).
     * @param {number} page
     * @param {number} x
     * @param {number} y
     * @param {number} width
     * @param {number} height
     */
    add_white_rect(page, x, y, width, height) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        _assertNum(page);
        const ret = wasm.wasmpdfeditor_add_white_rect(this.__wbg_ptr, page, x, y, width, height);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Apply all redaction annotations — permanently removes content.
     */
    apply_redactions() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wasmpdfeditor_apply_redactions(this.__wbg_ptr);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Delete a page (0-indexed).
     * @param {number} page
     */
    delete_page(page) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        _assertNum(page);
        const ret = wasm.wasmpdfeditor_delete_page(this.__wbg_ptr, page);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Duplicate a page (0-indexed).
     * @param {number} page
     */
    duplicate_page(page) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        _assertNum(page);
        const ret = wasm.wasmpdfeditor_duplicate_page(this.__wbg_ptr, page);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Extract specified pages (0-indexed) into a new PDF.
     * @param {Uint32Array} pages
     * @returns {Uint8Array}
     */
    extract_pages(pages) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passArray32ToWasm0(pages, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmpdfeditor_extract_pages(this.__wbg_ptr, ptr0, len0);
        if (ret[3]) {
            throw takeFromExternrefTable0(ret[2]);
        }
        var v2 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        return v2;
    }
    /**
     * Flatten the form.
     */
    flatten_form() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wasmpdfeditor_flatten_form(this.__wbg_ptr);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Get all form fields as JSON.
     * @returns {string}
     */
    get_form_fields() {
        let deferred2_0;
        let deferred2_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ret = wasm.wasmpdfeditor_get_form_fields(this.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Merge another PDF's pages at the end of this document.
     * @param {Uint8Array} other_data
     */
    merge(other_data) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passArray8ToWasm0(other_data, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmpdfeditor_merge(this.__wbg_ptr, ptr0, len0);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Move a page from one position to another (0-indexed).
     * @param {number} from
     * @param {number} to
     */
    move_page(from, to) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        _assertNum(from);
        _assertNum(to);
        const ret = wasm.wasmpdfeditor_move_page(this.__wbg_ptr, from, to);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Open a PDF from raw bytes.
     * @param {Uint8Array} data
     * @returns {WasmPdfEditor}
     */
    static open(data) {
        const ptr0 = passArray8ToWasm0(data, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmpdfeditor_open(ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return WasmPdfEditor.__wrap(ret[0]);
    }
    /**
     * Get the number of pages.
     * @returns {number}
     */
    page_count() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wasmpdfeditor_page_count(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Rotate a page by degrees (must be a multiple of 90).
     * @param {number} page
     * @param {number} degrees
     */
    rotate_page(page, degrees) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        _assertNum(page);
        _assertNum(degrees);
        const ret = wasm.wasmpdfeditor_rotate_page(this.__wbg_ptr, page, degrees);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Save the modified PDF to bytes.
     * @returns {Uint8Array}
     */
    save() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wasmpdfeditor_save(this.__wbg_ptr);
        if (ret[3]) {
            throw takeFromExternrefTable0(ret[2]);
        }
        var v1 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        return v1;
    }
    /**
     * Set a form field's value by name.
     * @param {string} field_name
     * @param {string} value
     */
    set_form_field_value(field_name, value) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(field_name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(value, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmpdfeditor_set_form_field_value(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
}
if (Symbol.dispose) WasmPdfEditor.prototype[Symbol.dispose] = WasmPdfEditor.prototype.free;

/**
 * Detect the format of a document from its bytes.
 *
 * Returns one of: "docx", "odt", "pdf", "txt".
 * @param {Uint8Array} data
 * @returns {string}
 */
export function detect_format(data) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passArray8ToWasm0(data, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.detect_format(ptr0, len0);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

//#endregion

//#region wasm imports

function __wbg_get_imports() {
    const import0 = {
        __proto__: null,
        __wbg_Error_83742b46f01ce22d: function() { return logError(function (arg0, arg1) {
            const ret = Error(getStringFromWasm0(arg0, arg1));
            return ret;
        }, arguments); },
        __wbg___wbindgen_throw_6ddd609b62940d55: function(arg0, arg1) {
            throw new Error(getStringFromWasm0(arg0, arg1));
        },
        __wbindgen_init_externref_table: function() {
            const table = wasm.__wbindgen_externrefs;
            const offset = table.grow(4);
            table.set(0, undefined);
            table.set(offset + 0, undefined);
            table.set(offset + 1, null);
            table.set(offset + 2, true);
            table.set(offset + 3, false);
        },
    };
    return {
        __proto__: null,
        "./s1engine_wasm_bg.js": import0,
    };
}


//#endregion
const WasmCollabDocumentFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmcollabdocument_free(ptr >>> 0, 1));
const WasmDocumentFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmdocument_free(ptr >>> 0, 1));
const WasmDocumentBuilderFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmdocumentbuilder_free(ptr >>> 0, 1));
const WasmEngineFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmengine_free(ptr >>> 0, 1));
const WasmFontDatabaseFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmfontdatabase_free(ptr >>> 0, 1));
const WasmLayoutConfigFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmlayoutconfig_free(ptr >>> 0, 1));
const WasmPdfEditorFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmpdfeditor_free(ptr >>> 0, 1));


//#region intrinsics
function _assertBigInt(n) {
    if (typeof(n) !== 'bigint') throw new Error(`expected a bigint argument, found ${typeof(n)}`);
}

function _assertBoolean(n) {
    if (typeof(n) !== 'boolean') {
        throw new Error(`expected a boolean argument, found ${typeof(n)}`);
    }
}

function _assertClass(instance, klass) {
    if (!(instance instanceof klass)) {
        throw new Error(`expected instance of ${klass.name}`);
    }
}

function _assertNum(n) {
    if (typeof(n) !== 'number') throw new Error(`expected a number argument, found ${typeof(n)}`);
}

function getArrayU8FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
}

let cachedFloat64ArrayMemory0 = null;
function getFloat64ArrayMemory0() {
    if (cachedFloat64ArrayMemory0 === null || cachedFloat64ArrayMemory0.byteLength === 0) {
        cachedFloat64ArrayMemory0 = new Float64Array(wasm.memory.buffer);
    }
    return cachedFloat64ArrayMemory0;
}

function getStringFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return decodeText(ptr, len);
}

let cachedUint32ArrayMemory0 = null;
function getUint32ArrayMemory0() {
    if (cachedUint32ArrayMemory0 === null || cachedUint32ArrayMemory0.byteLength === 0) {
        cachedUint32ArrayMemory0 = new Uint32Array(wasm.memory.buffer);
    }
    return cachedUint32ArrayMemory0;
}

let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function logError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        let error = (function () {
            try {
                return e instanceof Error ? `${e.message}\n\nStack:\n${e.stack}` : e.toString();
            } catch(_) {
                return "<failed to stringify thrown value>";
            }
        }());
        console.error("wasm-bindgen: imported JS function that was not marked as `catch` threw an error:", error);
        throw e;
    }
}

function passArray32ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 4, 4) >>> 0;
    getUint32ArrayMemory0().set(arg, ptr / 4);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}

function passArray8ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 1, 1) >>> 0;
    getUint8ArrayMemory0().set(arg, ptr / 1);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}

function passArrayF64ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 8, 8) >>> 0;
    getFloat64ArrayMemory0().set(arg, ptr / 8);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}

function passStringToWasm0(arg, malloc, realloc) {
    if (typeof(arg) !== 'string') throw new Error(`expected a string argument, found ${typeof(arg)}`);
    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }
    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = cachedTextEncoder.encodeInto(arg, view);
        if (ret.read !== arg.length) throw new Error('failed to pass whole string');
        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

function takeFromExternrefTable0(idx) {
    const value = wasm.__wbindgen_externrefs.get(idx);
    wasm.__externref_table_dealloc(idx);
    return value;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
cachedTextDecoder.decode();
const MAX_SAFARI_DECODE_BYTES = 2146435072;
let numBytesDecoded = 0;
function decodeText(ptr, len) {
    numBytesDecoded += len;
    if (numBytesDecoded >= MAX_SAFARI_DECODE_BYTES) {
        cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
        cachedTextDecoder.decode();
        numBytesDecoded = len;
    }
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

const cachedTextEncoder = new TextEncoder();

if (!('encodeInto' in cachedTextEncoder)) {
    cachedTextEncoder.encodeInto = function (arg, view) {
        const buf = cachedTextEncoder.encode(arg);
        view.set(buf);
        return {
            read: arg.length,
            written: buf.length
        };
    };
}

let WASM_VECTOR_LEN = 0;


//#endregion

//#region wasm loading
let wasmModule, wasm;
function __wbg_finalize_init(instance, module) {
    wasm = instance.exports;
    wasmModule = module;
    cachedFloat64ArrayMemory0 = null;
    cachedUint32ArrayMemory0 = null;
    cachedUint8ArrayMemory0 = null;
    wasm.__wbindgen_start();
    return wasm;
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);
            } catch (e) {
                const validResponse = module.ok && expectedResponseType(module.type);

                if (validResponse && module.headers.get('Content-Type') !== 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else { throw e; }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);
    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };
        } else {
            return instance;
        }
    }

    function expectedResponseType(type) {
        switch (type) {
            case 'basic': case 'cors': case 'default': return true;
        }
        return false;
    }
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (module !== undefined) {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();
    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }
    const instance = new WebAssembly.Instance(module, imports);
    return __wbg_finalize_init(instance, module);
}

async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;


    if (module_or_path !== undefined) {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (module_or_path === undefined) {
        module_or_path = new URL('s1engine_wasm_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync, __wbg_init as default };
//#endregion
export { wasm as __wasm }
