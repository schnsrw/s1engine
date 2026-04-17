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
     * Append an empty paragraph.
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
            const ret = wasm.wasmcollabdocument_append_paragraph(this.__wbg_ptr, ptr0, len0);
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
     * Delete a node.
     * @param {string} node_id_str
     * @returns {string}
     */
    delete_node(node_id_str) {
        let deferred3_0;
        let deferred3_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(node_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmcollabdocument_delete_node(this.__wbg_ptr, ptr0, len0);
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
     * Delete a text selection (single or cross-paragraph).
     * Returns serialized CRDT operations.
     * @param {string} start_node_str
     * @param {number} start_offset
     * @param {string} end_node_str
     * @param {number} end_offset
     * @returns {string}
     */
    delete_selection(start_node_str, start_offset, end_node_str, end_offset) {
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
            const ret = wasm.wasmcollabdocument_delete_selection(this.__wbg_ptr, ptr0, len0, start_offset, ptr1, len1, end_offset);
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
     * Format a selection.
     * Returns serialized CRDT operations.
     * @param {string} start_node_str
     * @param {number} start_offset
     * @param {string} end_node_str
     * @param {number} end_offset
     * @param {string} key
     * @param {string} value
     * @returns {string}
     */
    format_selection(start_node_str, start_offset, end_node_str, end_offset, key, value) {
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
            const ptr2 = passStringToWasm0(key, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len2 = WASM_VECTOR_LEN;
            const ptr3 = passStringToWasm0(value, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len3 = WASM_VECTOR_LEN;
            const ret = wasm.wasmcollabdocument_format_selection(this.__wbg_ptr, ptr0, len0, start_offset, ptr1, len1, end_offset, ptr2, len2, ptr3, len3);
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
     * Get formatting info for a node as JSON (delegates to WasmDocument).
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
            const ret = wasm.wasmcollabdocument_get_formatting_json(this.__wbg_ptr, ptr0, len0);
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
     * Insert horizontal rule.
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
            const ret = wasm.wasmcollabdocument_insert_horizontal_rule(this.__wbg_ptr, ptr0, len0);
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
     * Insert page break.
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
            const ret = wasm.wasmcollabdocument_insert_page_break(this.__wbg_ptr, ptr0, len0);
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
     * Insert a paragraph after a given node.
     * @param {string} after_node_str
     * @param {string} text
     * @returns {string}
     */
    insert_paragraph_after(after_node_str, text) {
        let deferred4_0;
        let deferred4_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(after_node_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ptr1 = passStringToWasm0(text, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            const ret = wasm.wasmcollabdocument_insert_paragraph_after(this.__wbg_ptr, ptr0, len0, ptr1, len1);
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
     * Insert a table.
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
            const ret = wasm.wasmcollabdocument_insert_table(this.__wbg_ptr, ptr0, len0, rows, cols);
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
     * Insert text in a paragraph at a specific offset (CRDT-native).
     * @param {string} node_id_str
     * @param {number} offset
     * @param {string} text
     * @returns {string}
     */
    insert_text_in_paragraph(node_id_str, offset, text) {
        let deferred4_0;
        let deferred4_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(node_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            _assertNum(offset);
            const ptr1 = passStringToWasm0(text, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            const ret = wasm.wasmcollabdocument_insert_text_in_paragraph(this.__wbg_ptr, ptr0, len0, offset, ptr1, len1);
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
     * Merge two paragraphs.
     * Returns serialized CRDT operations.
     * @param {string} node1_str
     * @param {string} node2_str
     * @returns {string}
     */
    merge_paragraphs(node1_str, node2_str) {
        let deferred4_0;
        let deferred4_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(node1_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ptr1 = passStringToWasm0(node2_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            const ret = wasm.wasmcollabdocument_merge_paragraphs(this.__wbg_ptr, ptr0, len0, ptr1, len1);
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
     * Get paragraph IDs as JSON array.
     * @returns {string}
     */
    paragraph_ids_json() {
        let deferred2_0;
        let deferred2_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ret = wasm.wasmcollabdocument_paragraph_ids_json(this.__wbg_ptr);
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
     * Paste plain text (may create multiple paragraphs).
     * @param {string} node_id_str
     * @param {number} offset
     * @param {string} text
     * @returns {string}
     */
    paste_plain_text(node_id_str, offset, text) {
        let deferred4_0;
        let deferred4_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(node_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            _assertNum(offset);
            const ptr1 = passStringToWasm0(text, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            const ret = wasm.wasmcollabdocument_paste_plain_text(this.__wbg_ptr, ptr0, len0, offset, ptr1, len1);
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
     * Render a single node as HTML (for incremental rendering).
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
            const ret = wasm.wasmcollabdocument_render_node_html(this.__wbg_ptr, ptr0, len0);
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
     * Set alignment for a paragraph.
     * @param {string} node_id_str
     * @param {string} alignment
     * @returns {string}
     */
    set_alignment(node_id_str, alignment) {
        let deferred4_0;
        let deferred4_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(node_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ptr1 = passStringToWasm0(alignment, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            const ret = wasm.wasmcollabdocument_set_alignment(this.__wbg_ptr, ptr0, len0, ptr1, len1);
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
     * Set heading level for a paragraph.
     * @param {string} node_id_str
     * @param {number} level
     * @returns {string}
     */
    set_heading_level(node_id_str, level) {
        let deferred3_0;
        let deferred3_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(node_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            _assertNum(level);
            const ret = wasm.wasmcollabdocument_set_heading_level(this.__wbg_ptr, ptr0, len0, level);
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
     * Set indent for a paragraph.
     * @param {string} node_id_str
     * @param {string} side
     * @param {number} value
     * @returns {string}
     */
    set_indent(node_id_str, side, value) {
        let deferred4_0;
        let deferred4_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(node_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ptr1 = passStringToWasm0(side, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            const ret = wasm.wasmcollabdocument_set_indent(this.__wbg_ptr, ptr0, len0, ptr1, len1, value);
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
     * Set line spacing for a paragraph.
     * @param {string} node_id_str
     * @param {string} value
     * @returns {string}
     */
    set_line_spacing(node_id_str, value) {
        let deferred4_0;
        let deferred4_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(node_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ptr1 = passStringToWasm0(value, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            const ret = wasm.wasmcollabdocument_set_line_spacing(this.__wbg_ptr, ptr0, len0, ptr1, len1);
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
     * Set list format for a paragraph.
     * @param {string} node_id_str
     * @param {string} format
     * @param {number} level
     * @returns {string}
     */
    set_list_format(node_id_str, format, level) {
        let deferred4_0;
        let deferred4_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(node_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ptr1 = passStringToWasm0(format, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            _assertNum(level);
            const ret = wasm.wasmcollabdocument_set_list_format(this.__wbg_ptr, ptr0, len0, ptr1, len1, level);
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
     * Set paragraph text, preserving multi-run formatting when possible.
     *
     * When the text is unchanged, this is a no-op (preserves all formatting).
     * When only a portion of the text changed, a diff-based approach is used
     * to minimize the edit and preserve run-level formatting on unchanged
     * portions. Only falls back to full delete+insert when the paragraph
     * has no existing runs.
     * @param {string} node_id_str
     * @param {string} text
     * @returns {string}
     */
    set_paragraph_text(node_id_str, text) {
        let deferred4_0;
        let deferred4_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(node_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ptr1 = passStringToWasm0(text, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            const ret = wasm.wasmcollabdocument_set_paragraph_text(this.__wbg_ptr, ptr0, len0, ptr1, len1);
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
     * Set column widths for a table.
     * @param {string} table_id_str
     * @param {string} widths_csv
     * @returns {string}
     */
    set_table_column_widths(table_id_str, widths_csv) {
        let deferred4_0;
        let deferred4_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(table_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ptr1 = passStringToWasm0(widths_csv, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            const ret = wasm.wasmcollabdocument_set_table_column_widths(this.__wbg_ptr, ptr0, len0, ptr1, len1);
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
     * Sort a table by column (delegates to WasmDocument.sort_table_by_column).
     * @param {string} table_id_str
     * @param {number} col_index
     * @param {boolean} ascending
     * @returns {string}
     */
    sort_table_by_column(table_id_str, col_index, ascending) {
        let deferred3_0;
        let deferred3_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(table_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            _assertNum(col_index);
            _assertBoolean(ascending);
            const ret = wasm.wasmcollabdocument_sort_table_by_column(this.__wbg_ptr, ptr0, len0, col_index, ascending);
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
     * Split a paragraph at the given offset.
     * Returns JSON: { "newId": "replica:counter", "ops": [ ... ] }
     * @param {string} node_id_str
     * @param {number} offset
     * @returns {string}
     */
    split_paragraph(node_id_str, offset) {
        let deferred3_0;
        let deferred3_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(node_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            _assertNum(offset);
            const ret = wasm.wasmcollabdocument_split_paragraph(this.__wbg_ptr, ptr0, len0, offset);
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
     * Apply mail merge data to the document.
     *
     * Takes a JSON array of records: `[{"FirstName":"John","LastName":"Doe"}, ...]`
     * and a record index (0-based). Replaces all MERGEFIELD placeholders with
     * values from the specified record.
     *
     * Returns the number of fields replaced.
     * @param {string} data_json
     * @param {number} record_index
     * @returns {number}
     */
    apply_merge_data(data_json, record_index) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(data_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        _assertNum(record_index);
        const ret = wasm.wasmdocument_apply_merge_data(this.__wbg_ptr, ptr0, len0, record_index);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ret[0] >>> 0;
    }
    /**
     * Apply a predefined table style to a table.
     *
     * Available styles: "plain", "grid", "striped-blue", "striped-gray",
     * "header-blue", "header-green", "header-orange", "bordered", "minimal".
     *
     * Applies cell backgrounds and header row formatting.
     * @param {string} table_id_str
     * @param {string} style_name
     */
    apply_table_style(table_id_str, style_name) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(table_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(style_name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmdocument_apply_table_style(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
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
     * Begin an IME composition at the given position.
     *
     * Stores the anchor position for subsequent composition updates.
     * Returns JSON `{"status":"composing","anchor":<position>}`.
     * @param {string} position_json
     * @returns {string}
     */
    begin_composition(position_json) {
        let deferred3_0;
        let deferred3_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(position_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_begin_composition(this.__wbg_ptr, ptr0, len0);
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
     * Cancel the IME composition.
     *
     * Deletes the preview text and clears composition state.
     * Returns an EditResult JSON with cursor at the original anchor.
     * @returns {string}
     */
    cancel_composition() {
        let deferred2_0;
        let deferred2_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ret = wasm.wasmdocument_cancel_composition(this.__wbg_ptr);
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
     * Delete a canvas range (anchor + focus as text-node IDs + UTF-16 offsets).
     *
     * Resolves the range to paragraph coordinates, performs the deletion,
     * and returns an EditResult JSON string with the cursor collapsed at range start.
     * @param {string} range_json
     * @returns {string}
     */
    canvas_delete_range(range_json) {
        let deferred3_0;
        let deferred3_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(range_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_canvas_delete_range(this.__wbg_ptr, ptr0, len0);
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
     * Insert a paragraph break at a canvas position.
     *
     * Splits the paragraph at the resolved char offset.
     * Returns an EditResult JSON with the cursor at the start of the new paragraph.
     * @param {string} position_json
     * @returns {string}
     */
    canvas_insert_paragraph_break(position_json) {
        let deferred3_0;
        let deferred3_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(position_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_canvas_insert_paragraph_break(this.__wbg_ptr, ptr0, len0);
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
     * Insert text at a canvas position (text-node ID + UTF-16 offset).
     *
     * Resolves the position to paragraph coordinates, performs the insert,
     * and returns an EditResult JSON string with the new cursor position.
     * @param {string} position_json
     * @param {string} text
     * @returns {string}
     */
    canvas_insert_text(position_json, text) {
        let deferred4_0;
        let deferred4_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(position_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ptr1 = passStringToWasm0(text, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_canvas_insert_text(this.__wbg_ptr, ptr0, len0, ptr1, len1);
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
     * Replace a canvas range with new text.
     *
     * Deletes the range, then inserts text at the start position.
     * Returns an EditResult JSON with the cursor after the inserted text.
     * @param {string} range_json
     * @param {string} text
     * @returns {string}
     */
    canvas_replace_range(range_json, text) {
        let deferred4_0;
        let deferred4_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(range_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ptr1 = passStringToWasm0(text, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_canvas_replace_range(this.__wbg_ptr, ptr0, len0, ptr1, len1);
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
     * Toggle a formatting mark on a canvas range.
     *
     * Checks the current formatting state at the anchor position and
     * toggles the specified mark. Supported marks: "bold", "italic",
     * "underline", "strikethrough".
     *
     * Returns an EditResult JSON.
     * @param {string} range_json
     * @param {string} mark
     * @returns {string}
     */
    canvas_toggle_mark(range_json, mark) {
        let deferred4_0;
        let deferred4_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(range_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ptr1 = passStringToWasm0(mark, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_canvas_toggle_mark(this.__wbg_ptr, ptr0, len0, ptr1, len1);
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
     * Get the caret rectangle for a model position.
     *
     * Returns JSON `RectPt` with page_index, x, y, width (1.0), height.
     * @param {string} position_json
     * @returns {string}
     */
    caret_rect(position_json) {
        let deferred3_0;
        let deferred3_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(position_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_caret_rect(this.__wbg_ptr, ptr0, len0);
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
     * Commit the IME composition with final text.
     *
     * Deletes the preview, inserts the final text, and clears composition state.
     * Returns an EditResult JSON.
     * @param {string} text
     * @returns {string}
     */
    commit_composition(text) {
        let deferred3_0;
        let deferred3_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(text, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_commit_composition(this.__wbg_ptr, ptr0, len0);
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
     * Compare this document with another and return word-level differences as JSON.
     *
     * Takes the bytes of another document, opens it, extracts plain text from both,
     * and returns a JSON array of diff operations:
     * `[{"type":"equal","text":"..."},{"type":"insert","text":"..."},{"type":"delete","text":"..."}]`
     * @param {Uint8Array} other_bytes
     * @returns {string}
     */
    compare_with(other_bytes) {
        let deferred3_0;
        let deferred3_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passArray8ToWasm0(other_bytes, wasm.__wbindgen_malloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_compare_with(this.__wbg_ptr, ptr0, len0);
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
     * Copy a canvas range as HTML.
     *
     * Resolves the range to paragraph coordinates and delegates to
     * the existing `export_selection_html` method.
     * @param {string} range_json
     * @returns {string}
     */
    copy_range_html(range_json) {
        let deferred3_0;
        let deferred3_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(range_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_copy_range_html(this.__wbg_ptr, ptr0, len0);
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
     * Copy a canvas range as plain text.
     *
     * Walks text nodes from anchor to focus, joining with newlines
     * at paragraph boundaries.
     * @param {string} range_json
     * @returns {string}
     */
    copy_range_plain_text(range_json) {
        let deferred3_0;
        let deferred3_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(range_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_copy_range_plain_text(this.__wbg_ptr, ptr0, len0);
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
     * Delete a comment reply by its comment ID.
     *
     * Removes the CommentBody node that has the given `reply_id` as its
     * CommentId attribute. Only deletes replies (nodes with CommentParentId).
     * @param {string} reply_id
     */
    delete_comment_reply(reply_id) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(reply_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmdocument_delete_comment_reply(this.__wbg_ptr, ptr0, len0);
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
     * Return a monotonically increasing document revision number.
     *
     * Bumps on every model mutation (insert, delete, format change).
     * Uses undo_count as a proxy for revision tracking.
     * @returns {number}
     */
    document_revision() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wasmdocument_document_revision(this.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ret[0] >>> 0;
    }
    /**
     * Edit a comment's text content.
     *
     * Replaces the text in the first paragraph of the CommentBody node
     * matching `comment_id`.
     * @param {string} comment_id
     * @param {string} new_text
     */
    edit_comment(comment_id, new_text) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(comment_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(new_text, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmdocument_edit_comment(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Get the editor capabilities as a JSON object.
     *
     * Returns a JSON object indicating which editing features are available.
     * @returns {string}
     */
    editor_capabilities() {
        let deferred2_0;
        let deferred2_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ret = wasm.wasmdocument_editor_capabilities(this.__wbg_ptr);
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
     * Get an import fidelity report as a JSON string.
     *
     * Counts objects that could not be rendered faithfully and are shown
     * as placeholders. Returns JSON like:
     * `{"charts":2,"smartart":1,"ole":0,"missingImages":0,"total":3}`
     *
     * Consumers can use this to display "3 objects shown as placeholders"
     * after opening a document, rather than silently degrading.
     * @returns {string}
     */
    fidelity_report_json() {
        let deferred2_0;
        let deferred2_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ret = wasm.wasmdocument_fidelity_report_json(this.__wbg_ptr);
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
     * Force a fresh relayout using the provided font database.
     *
     * Call this from JS on the deferred timer to get an accurate layout
     * after a batch of edits. Clears the dirty flag.
     * @param {WasmFontDatabase} font_db
     */
    force_relayout(font_db) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        _assertClass(font_db, WasmFontDatabase);
        if (font_db.__wbg_ptr === 0) {
            throw new Error('Attempt to use a moved value');
        }
        const ret = wasm.wasmdocument_force_relayout(this.__wbg_ptr, font_db.__wbg_ptr);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
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
     * Generate a bibliography section from all citations in the document.
     *
     * Scans for CITATION fields, extracts their JSON data, and creates
     * a formatted bibliography paragraph after `after_node_str`.
     * @param {string} after_node_str
     * @returns {string}
     */
    generate_bibliography(after_node_str) {
        let deferred3_0;
        let deferred3_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(after_node_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_generate_bibliography(this.__wbg_ptr, ptr0, len0);
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
     * Get page indices affected by a node, plus adjacent pages.
     *
     * Returns a JSON array of 0-based page indices, e.g. `[1,2,3]`.
     * Used by the editor to know which pages to re-render after an edit.
     *
     * Layout must already be cached (call `get_page_count*` first).
     * @param {string} node_id_str
     * @returns {string}
     */
    get_affected_pages(node_id_str) {
        let deferred3_0;
        let deferred3_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(node_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_get_affected_pages(this.__wbg_ptr, ptr0, len0);
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
     * Get the total number of pages using default (empty) font metrics.
     *
     * Lazily computes and caches layout. The cache is invalidated on any
     * document mutation.
     * @returns {number}
     */
    get_page_count() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wasmdocument_get_page_count(this.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ret[0] >>> 0;
    }
    /**
     * Get the total number of pages using loaded fonts for accurate metrics.
     *
     * Lazily computes and caches layout. The cache is invalidated on any
     * document mutation.
     * @param {WasmFontDatabase} font_db
     * @returns {number}
     */
    get_page_count_with_fonts(font_db) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        _assertClass(font_db, WasmFontDatabase);
        if (font_db.__wbg_ptr === 0) {
            throw new Error('Attempt to use a moved value');
        }
        const ret = wasm.wasmdocument_get_page_count_with_fonts(this.__wbg_ptr, font_db.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ret[0] >>> 0;
    }
    /**
     * Get ready-to-mount HTML for a single page using default font metrics.
     *
     * Returns document-model HTML (semantic `<p>`, `<h1>`, `<table>` with
     * `data-node-id`) filtered to the blocks on `page_index`. Split
     * paragraphs get `data-split="first"` or `data-split="continuation"`.
     *
     * Call `get_page_count()` first to ensure layout is cached.
     * @param {number} page_index
     * @returns {string}
     */
    get_page_html(page_index) {
        let deferred2_0;
        let deferred2_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            _assertNum(page_index);
            const ret = wasm.wasmdocument_get_page_html(this.__wbg_ptr, page_index);
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
     * Get ready-to-mount HTML for a single page using loaded fonts.
     *
     * Call `get_page_count_with_fonts()` first to ensure layout is cached,
     * or this will lazily compute layout.
     * @param {number} page_index
     * @param {WasmFontDatabase} font_db
     * @returns {string}
     */
    get_page_html_with_fonts(page_index, font_db) {
        let deferred2_0;
        let deferred2_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            _assertNum(page_index);
            _assertClass(font_db, WasmFontDatabase);
            if (font_db.__wbg_ptr === 0) {
                throw new Error('Attempt to use a moved value');
            }
            const ret = wasm.wasmdocument_get_page_html_with_fonts(this.__wbg_ptr, page_index, font_db.__wbg_ptr);
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
     * Get page map JSON with font metrics for accurate line-level pagination.
     * @param {WasmFontDatabase} font_db
     * @returns {string}
     */
    get_page_map_json_with_fonts(font_db) {
        let deferred2_0;
        let deferred2_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            _assertClass(font_db, WasmFontDatabase);
            if (font_db.__wbg_ptr === 0) {
                throw new Error('Attempt to use a moved value');
            }
            const ret = wasm.wasmdocument_get_page_map_json_with_fonts(this.__wbg_ptr, font_db.__wbg_ptr);
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
     * Get available cross-reference targets as JSON.
     *
     * Returns a JSON object with `headings` and `bookmarks` arrays:
     * ```json
     * {
     *   "headings": [{"nodeId":"0:5","text":"Introduction","level":1}],
     *   "bookmarks": [{"name":"myBookmark","nodeId":"0:10"}]
     * }
     * ```
     * @returns {string}
     */
    get_reference_targets_json() {
        let deferred2_0;
        let deferred2_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ret = wasm.wasmdocument_get_reference_targets_json(this.__wbg_ptr);
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
     * Hit-test a point on a page to find the nearest model position.
     *
     * Returns JSON `HitTestResult` with position, kind, node_id, and inside flag.
     * @param {number} page_index
     * @param {number} x_pt
     * @param {number} y_pt
     * @returns {string}
     */
    hit_test(page_index, x_pt, y_pt) {
        let deferred2_0;
        let deferred2_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            _assertNum(page_index);
            const ret = wasm.wasmdocument_hit_test(this.__wbg_ptr, page_index, x_pt, y_pt);
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
     * Insert an auto-numbered caption paragraph after a node.
     *
     * - `after_node_str`: the node (image paragraph, table, etc.) after which to insert
     * - `label`: "Figure", "Table", or "Equation"
     * - `text`: additional caption text (e.g., ": My diagram")
     *
     * The caption is numbered automatically by counting existing captions of the same label.
     * Returns the caption paragraph node ID.
     * @param {string} after_node_str
     * @param {string} label
     * @param {string} text
     * @returns {string}
     */
    insert_caption(after_node_str, label, text) {
        let deferred5_0;
        let deferred5_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(after_node_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ptr1 = passStringToWasm0(label, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            const ptr2 = passStringToWasm0(text, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len2 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_insert_caption(this.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2);
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
     * Insert a bibliography citation at the cursor position.
     *
     * - `para_id_str`: paragraph to insert into
     * - `citation_json`: JSON object with citation fields:
     *   `{"author":"Smith","year":"2024","title":"Paper Title","source":"Journal Name"}`
     *
     * Returns the citation field node ID.
     * @param {string} para_id_str
     * @param {string} citation_json
     * @returns {string}
     */
    insert_citation(para_id_str, citation_json) {
        let deferred4_0;
        let deferred4_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(para_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ptr1 = passStringToWasm0(citation_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_insert_citation(this.__wbg_ptr, ptr0, len0, ptr1, len1);
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
     * Insert a cross-reference field at the cursor position.
     *
     * - `para_id_str`: paragraph to insert into
     * - `offset`: character offset within the paragraph
     * - `target_id_str`: node ID of the target (heading or bookmark)
     * - `ref_type`: "heading_text", "page_number", or "bookmark_text"
     * - `display_text`: the text to show for the cross-reference
     * @param {string} para_id_str
     * @param {number} _offset
     * @param {string} target_id_str
     * @param {string} _ref_type
     * @param {string} display_text
     * @returns {string}
     */
    insert_cross_reference(para_id_str, _offset, target_id_str, _ref_type, display_text) {
        let deferred6_0;
        let deferred6_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(para_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            _assertNum(_offset);
            const ptr1 = passStringToWasm0(target_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            const ptr2 = passStringToWasm0(_ref_type, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len2 = WASM_VECTOR_LEN;
            const ptr3 = passStringToWasm0(display_text, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len3 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_insert_cross_reference(this.__wbg_ptr, ptr0, len0, _offset, ptr1, len1, ptr2, len2, ptr3, len3);
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
     * Insert a mail merge field placeholder in the current paragraph.
     *
     * - `para_id_str`: paragraph to insert into
     * - `field_name`: the merge field name (e.g., "FirstName", "Email")
     *
     * Returns the field node ID. The field displays as `«FieldName»` until merged.
     * @param {string} para_id_str
     * @param {string} field_name
     * @returns {string}
     */
    insert_merge_field(para_id_str, field_name) {
        let deferred4_0;
        let deferred4_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(para_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ptr1 = passStringToWasm0(field_name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_insert_merge_field(this.__wbg_ptr, ptr0, len0, ptr1, len1);
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
     * Insert a SEQ (sequence) field for auto-numbering.
     *
     * Sequence fields maintain separate counters per `seq_name` (e.g., "Figure", "Table").
     * Returns the field node ID.
     * @param {string} para_id_str
     * @param {string} seq_name
     * @returns {string}
     */
    insert_seq_field(para_id_str, seq_name) {
        let deferred4_0;
        let deferred4_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(para_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ptr1 = passStringToWasm0(seq_name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_insert_seq_field(this.__wbg_ptr, ptr0, len0, ptr1, len1);
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
     * Insert a shape (Drawing node) after a body-level node.
     *
     * Returns the Drawing node ID. The shape is rendered by the layout engine.
     * @param {string} after_node_str
     * @param {string} shape_type
     * @param {number} width_pt
     * @param {number} height_pt
     * @param {number} _x_pt
     * @param {number} _y_pt
     * @param {string} fill_hex
     * @param {string} stroke_hex
     * @param {number} stroke_width
     * @returns {string}
     */
    insert_shape(after_node_str, shape_type, width_pt, height_pt, _x_pt, _y_pt, fill_hex, stroke_hex, stroke_width) {
        let deferred6_0;
        let deferred6_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(after_node_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ptr1 = passStringToWasm0(shape_type, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            const ptr2 = passStringToWasm0(fill_hex, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len2 = WASM_VECTOR_LEN;
            const ptr3 = passStringToWasm0(stroke_hex, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len3 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_insert_shape(this.__wbg_ptr, ptr0, len0, ptr1, len1, width_pt, height_pt, _x_pt, _y_pt, ptr2, len2, ptr3, len3, stroke_width);
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
     * Insert a tab node at the given character offset within a paragraph.
     *
     * Like `insert_line_break`, this inserts a `Tab` node at paragraph level,
     * splitting runs as needed. Tab nodes render
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
     * Generate a Table of Figures from all Caption-styled paragraphs.
     *
     * Inserts a new section after `after_node_str` containing a list of all
     * captions found in the document (Figure 1: ..., Table 2: ..., etc.).
     * Returns the TOF container node ID.
     * @param {string} after_node_str
     * @param {string} label_filter
     * @returns {string}
     */
    insert_table_of_figures(after_node_str, label_filter) {
        let deferred4_0;
        let deferred4_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(after_node_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ptr1 = passStringToWasm0(label_filter, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_insert_table_of_figures(this.__wbg_ptr, ptr0, len0, ptr1, len1);
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
     * Check whether the layout cache is dirty (needs recomputation).
     * @returns {boolean}
     */
    is_layout_dirty() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wasmdocument_is_layout_dirty(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Check if track changes mode is currently enabled.
     * @returns {boolean}
     */
    is_track_changes_enabled() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wasmdocument_is_track_changes_enabled(this.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ret[0] !== 0;
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
     * Convert LaTeX string to OMML XML.
     *
     * Handles common LaTeX commands and produces valid Office MathML.
     * @param {string} latex
     * @returns {string}
     */
    latex_to_omml(latex) {
        let deferred3_0;
        let deferred3_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(latex, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_latex_to_omml(this.__wbg_ptr, ptr0, len0);
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
     * Return a monotonically increasing layout revision number.
     *
     * Bumps when pagination output changes (page count, block positions).
     * @returns {number}
     */
    layout_revision() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wasmdocument_layout_revision(this.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ret[0] >>> 0;
    }
    /**
     * Return layout JSON for a single page using the CACHED layout.
     *
     * This is fast because it does NOT recompute layout — it uses whatever
     * is in the cache (possibly stale after edits). The JS side uses this
     * for immediate visual feedback after typing, then does a full relayout
     * on a deferred timer.
     *
     * Returns `null` JSON string if the cache is empty or the page index is
     * out of range.
     * @param {number} page_index
     * @returns {string}
     */
    layout_single_page_json(page_index) {
        let deferred2_0;
        let deferred2_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            _assertNum(page_index);
            const ret = wasm.wasmdocument_layout_single_page_json(this.__wbg_ptr, page_index);
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
     * Get the line boundary position for "start" or "end" of the line
     * containing the given position.
     *
     * Returns a PositionRef JSON.
     * @param {string} position_json
     * @param {string} side
     * @returns {string}
     */
    line_boundary(position_json, side) {
        let deferred4_0;
        let deferred4_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(position_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ptr1 = passStringToWasm0(side, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_line_boundary(this.__wbg_ptr, ptr0, len0, ptr1, len1);
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
     * Get full document metadata as JSON (title, author, custom_properties, etc.).
     * @returns {string}
     */
    metadata_json() {
        let deferred2_0;
        let deferred2_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ret = wasm.wasmdocument_metadata_json(this.__wbg_ptr);
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
     * @param {string} position_json
     * @param {string} direction
     * @param {string} granularity
     * @returns {string}
     */
    move_position(position_json, direction, granularity) {
        let deferred5_0;
        let deferred5_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(position_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ptr1 = passStringToWasm0(direction, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            const ptr2 = passStringToWasm0(granularity, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len2 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_move_position(this.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2);
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
     * Move a range in a direction by a granularity.
     *
     * If extend is true, moves only the focus while keeping the anchor.
     * If extend is false, collapses the range and moves.
     * Returns a RangeRef JSON.
     * @param {string} range_json
     * @param {string} direction
     * @param {string} granularity
     * @param {boolean} extend
     * @returns {string}
     */
    move_range(range_json, direction, granularity, extend) {
        let deferred5_0;
        let deferred5_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(range_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ptr1 = passStringToWasm0(direction, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            const ptr2 = passStringToWasm0(granularity, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len2 = WASM_VECTOR_LEN;
            _assertBoolean(extend);
            const ret = wasm.wasmdocument_move_range(this.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2, extend);
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
     * Delete text at multiple cursor positions simultaneously.
     *
     * Takes a JSON array of `[{"nodeId":"0:5","offset":3,"length":1}, ...]`.
     * Applied in reverse order to preserve offsets.
     * @param {string} cursors_json
     */
    multi_cursor_delete(cursors_json) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(cursors_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmdocument_multi_cursor_delete(this.__wbg_ptr, ptr0, len0);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Insert text at multiple cursor positions simultaneously.
     *
     * Takes a JSON array of `[{"nodeId":"0:5","offset":3,"text":"x"}, ...]`.
     * Positions are sorted in reverse document order and applied back-to-front
     * so that earlier insertions don't shift later offsets.
     *
     * All insertions form a single undo step via merge_undo_entries.
     * @param {string} cursors_json
     */
    multi_cursor_insert(cursors_json) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(cursors_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmdocument_multi_cursor_insert(this.__wbg_ptr, ptr0, len0);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Get bounds for all pages containing a node.
     *
     * Returns JSON array of `RectPt`.
     * @param {string} node_id_str
     * @returns {string}
     */
    node_bounds(node_id_str) {
        let deferred3_0;
        let deferred3_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(node_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_node_bounds(this.__wbg_ptr, ptr0, len0);
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
     * Convert OMML (Office MathML) XML to LaTeX string.
     *
     * Handles common OMML elements: fractions, subscripts, superscripts,
     * square roots, matrices, summations, integrals, Greek letters.
     * @param {string} omml_xml
     * @returns {string}
     */
    omml_to_latex(omml_xml) {
        let deferred3_0;
        let deferred3_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(omml_xml, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_omml_to_latex(this.__wbg_ptr, ptr0, len0);
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
     * Return a full scene for a single page.
     *
     * Returns JSON with page bounds, content rect, header/footer rects,
     * and all scene items (text runs, backgrounds, borders, images, shapes, etc.).
     * @param {number} page_index
     * @returns {string}
     */
    page_scene(page_index) {
        let deferred2_0;
        let deferred2_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            _assertNum(page_index);
            const ret = wasm.wasmdocument_page_scene(this.__wbg_ptr, page_index);
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
     * Return page scene using loaded fonts for accurate text shaping.
     * @param {WasmFontDatabase} font_db
     * @param {number} page_index
     * @returns {string}
     */
    page_scene_with_fonts(font_db, page_index) {
        let deferred2_0;
        let deferred2_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            _assertClass(font_db, WasmFontDatabase);
            if (font_db.__wbg_ptr === 0) {
                throw new Error('Attempt to use a moved value');
            }
            _assertNum(page_index);
            const ret = wasm.wasmdocument_page_scene_with_fonts(this.__wbg_ptr, font_db.__wbg_ptr, page_index);
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
     * Paste HTML at a canvas position.
     *
     * For now, strips HTML tags and inserts as plain text.
     * Returns an EditResult JSON.
     * @param {string} position_json
     * @param {string} html
     * @returns {string}
     */
    paste_html(position_json, html) {
        let deferred4_0;
        let deferred4_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(position_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ptr1 = passStringToWasm0(html, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_paste_html(this.__wbg_ptr, ptr0, len0, ptr1, len1);
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
     * Render a paragraph node as HTML for the half-open character range
     * `[start_char, end_char)`. Used by pagination to mount page-specific
     * fragments for split paragraphs instead of rendering the full paragraph
     * and clipping it in CSS.
     * @param {string} node_id_str
     * @param {number} start_char
     * @param {number} end_char
     * @returns {string}
     */
    render_node_slice(node_id_str, start_char, end_char) {
        let deferred3_0;
        let deferred3_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(node_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            _assertNum(start_char);
            _assertNum(end_char);
            const ret = wasm.wasmdocument_render_node_slice(this.__wbg_ptr, ptr0, len0, start_char, end_char);
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
     *
     * Note: insert_text into an existing text node inherits the parent run's
     * formatting (bold, italic, etc.) — no explicit attribute copy needed.
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
     * Replace text in a paragraph by character range. Alias to `replace_text`.
     *
     * Preserves inline formatting (bold, italic, etc.) outside the modified range.
     * @param {string} node_id_str
     * @param {number} start_offset
     * @param {number} end_offset
     * @param {string} replacement
     */
    replace_text_range(node_id_str, start_offset, end_offset, replacement) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(node_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        _assertNum(start_offset);
        _assertNum(end_offset);
        const ptr1 = passStringToWasm0(replacement, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmdocument_replace_text_range(this.__wbg_ptr, ptr0, len0, start_offset, end_offset, ptr1, len1);
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
     * Set the resolved status of a comment.
     *
     * Persists the resolved state as a `CommentResolved` attribute on
     * the CommentBody node, so it survives save/load and collab sync.
     * @param {string} comment_id
     * @param {boolean} resolved
     */
    resolve_comment(comment_id, resolved) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(comment_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        _assertBoolean(resolved);
        const ret = wasm.wasmdocument_resolve_comment(this.__wbg_ptr, ptr0, len0, resolved);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Return the scene protocol version supported by this build.
     * @returns {number}
     */
    scene_protocol_version() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wasmdocument_scene_protocol_version(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Return a lightweight scene summary for viewport boot.
     *
     * Returns JSON: `{ "protocol_version": 1, "document_revision": N,
     * "layout_revision": N, "page_count": N, "default_page_size_pt": {...},
     * "pages": [...] }`
     * @param {string} _config_json
     * @returns {string}
     */
    scene_summary(_config_json) {
        let deferred3_0;
        let deferred3_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(_config_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_scene_summary(this.__wbg_ptr, ptr0, len0);
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
     * Return scene summary using loaded fonts for accurate text shaping.
     * @param {WasmFontDatabase} font_db
     * @param {string} config_json
     * @returns {string}
     */
    scene_summary_with_fonts(font_db, config_json) {
        let deferred3_0;
        let deferred3_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            _assertClass(font_db, WasmFontDatabase);
            if (font_db.__wbg_ptr === 0) {
                throw new Error('Attempt to use a moved value');
            }
            const ptr0 = passStringToWasm0(config_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_scene_summary_with_fonts(this.__wbg_ptr, font_db.__wbg_ptr, ptr0, len0);
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
     * Search for text matches and return results with page rects.
     *
     * Wraps the existing `find_text` and enriches results with layout
     * position information when available.
     * @param {string} query
     * @param {boolean} case_sensitive
     * @returns {string}
     */
    search_matches(query, case_sensitive) {
        let deferred3_0;
        let deferred3_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(query, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            _assertBoolean(case_sensitive);
            const ret = wasm.wasmdocument_search_matches(this.__wbg_ptr, ptr0, len0, case_sensitive);
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
     * Return formatting state at a selection range for toolbar display.
     *
     * Returns JSON `FormattingState` with bold, italic, font info, etc.
     * @param {string} range_json
     * @returns {string}
     */
    selection_formatting(range_json) {
        let deferred3_0;
        let deferred3_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(range_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_selection_formatting(this.__wbg_ptr, ptr0, len0);
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
     * Get selection rectangles for a model range.
     *
     * Returns JSON array of `RectPt` objects covering the selection.
     * @param {string} range_json
     * @returns {string}
     */
    selection_rects(range_json) {
        let deferred3_0;
        let deferred3_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(range_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_selection_rects(this.__wbg_ptr, ptr0, len0);
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
     *
     * For selection-aware formatting, use `format_selection()` or
     * `set_bold_range()` instead — they correctly handle mixed-format
     * paragraphs by splitting runs at selection boundaries.
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
     * Set bold on a selection range. Preferred over `set_bold` for toolbar
     * actions when the user has an active text selection.
     * @param {string} start_node_str
     * @param {number} start_offset
     * @param {string} end_node_str
     * @param {number} end_offset
     * @param {boolean} bold
     */
    set_bold_range(start_node_str, start_offset, end_node_str, end_offset, bold) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(start_node_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        _assertNum(start_offset);
        const ptr1 = passStringToWasm0(end_node_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        _assertNum(end_offset);
        _assertBoolean(bold);
        const ret = wasm.wasmdocument_set_bold_range(this.__wbg_ptr, ptr0, len0, start_offset, ptr1, len1, end_offset, bold);
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
     * Set text color on a selection range (hex string like "FF0000").
     * @param {string} start_node_str
     * @param {number} start_offset
     * @param {string} end_node_str
     * @param {number} end_offset
     * @param {string} hex
     */
    set_color_range(start_node_str, start_offset, end_node_str, end_offset, hex) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(start_node_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        _assertNum(start_offset);
        const ptr1 = passStringToWasm0(end_node_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        _assertNum(end_offset);
        const ptr2 = passStringToWasm0(hex, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        const ret = wasm.wasmdocument_set_color_range(this.__wbg_ptr, ptr0, len0, start_offset, ptr1, len1, end_offset, ptr2, len2);
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
     * Set font family on a selection range.
     * @param {string} start_node_str
     * @param {number} start_offset
     * @param {string} end_node_str
     * @param {number} end_offset
     * @param {string} font
     */
    set_font_family_range(start_node_str, start_offset, end_node_str, end_offset, font) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(start_node_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        _assertNum(start_offset);
        const ptr1 = passStringToWasm0(end_node_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        _assertNum(end_offset);
        const ptr2 = passStringToWasm0(font, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        const ret = wasm.wasmdocument_set_font_family_range(this.__wbg_ptr, ptr0, len0, start_offset, ptr1, len1, end_offset, ptr2, len2);
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
     * Set font size on a selection range (in points).
     * @param {string} start_node_str
     * @param {number} start_offset
     * @param {string} end_node_str
     * @param {number} end_offset
     * @param {number} size_pt
     */
    set_font_size_range(start_node_str, start_offset, end_node_str, end_offset, size_pt) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(start_node_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        _assertNum(start_offset);
        const ptr1 = passStringToWasm0(end_node_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        _assertNum(end_offset);
        const ret = wasm.wasmdocument_set_font_size_range(this.__wbg_ptr, ptr0, len0, start_offset, ptr1, len1, end_offset, size_pt);
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
     * For selection-aware formatting, use `set_italic_range` or `format_selection`.
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
     * Set italic on a selection range.
     * @param {string} start_node_str
     * @param {number} start_offset
     * @param {string} end_node_str
     * @param {number} end_offset
     * @param {boolean} italic
     */
    set_italic_range(start_node_str, start_offset, end_node_str, end_offset, italic) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(start_node_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        _assertNum(start_offset);
        const ptr1 = passStringToWasm0(end_node_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        _assertNum(end_offset);
        _assertBoolean(italic);
        const ret = wasm.wasmdocument_set_italic_range(this.__wbg_ptr, ptr0, len0, start_offset, ptr1, len1, end_offset, italic);
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
     * Set the entire text content of a paragraph.
     *
     * # Formatting preservation behavior
     *
     * - **No change**: If `new_text` matches the existing text across all runs,
     *   this is a no-op — per-run formatting is fully preserved.
     * - **Single-run edit**: If the diff falls within a single run, a targeted
     *   insert/delete is used and that run's formatting is preserved.
     * - **Cross-run edit**: When the edit spans multiple runs, extra runs are
     *   deleted and the surviving run receives the new text. **This collapses
     *   inline formatting** (bold, italic, links, font changes, etc.) to a
     *   single formatting context.
     *
     * # Preferred alternatives
     *
     * For DOM-driven edits from the editor, prefer range-aware operations:
     * - `insert_text_in_paragraph()` — insert at a specific offset
     * - `delete_text_in_paragraph()` — delete a range within a paragraph
     * - `format_selection()` — apply formatting to a character range
     * - `replace_text()` — replace text in a range (preserves surrounding formatting)
     *
     * These operations work at the character/run level and never collapse
     * formatting outside the edited range. `set_paragraph_text` should be
     * reserved for sync/convergence scenarios where the full paragraph text
     * needs to be force-set (e.g., non-CRDT collaboration fallback).
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
     * Set column widths for a table. Widths should be in points (CSV string).
     * @param {string} table_id_str
     * @param {string} widths_csv
     */
    set_table_column_widths(table_id_str, widths_csv) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(table_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(widths_csv, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmdocument_set_table_column_widths(this.__wbg_ptr, ptr0, len0, ptr1, len1);
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
     * Enable or disable track changes mode.
     *
     * When enabled, subsequent text edits create revision markers.
     * This stores the state on the document metadata so it persists.
     * @param {boolean} enabled
     */
    set_track_changes_enabled(enabled) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        _assertBoolean(enabled);
        const ret = wasm.wasmdocument_set_track_changes_enabled(this.__wbg_ptr, enabled);
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
     * Set underline on a selection range.
     * @param {string} start_node_str
     * @param {number} start_offset
     * @param {string} end_node_str
     * @param {number} end_offset
     * @param {boolean} underline
     */
    set_underline_range(start_node_str, start_offset, end_node_str, end_offset, underline) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(start_node_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        _assertNum(start_offset);
        const ptr1 = passStringToWasm0(end_node_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        _assertNum(end_offset);
        _assertBoolean(underline);
        const ret = wasm.wasmdocument_set_underline_range(this.__wbg_ptr, ptr0, len0, start_offset, ptr1, len1, end_offset, underline);
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
     * Sort a table by the text content of a specific column.
     *
     * Skips the first row (assumed header) if the table has more than one row.
     * @param {string} table_id_str
     * @param {number} col_index
     * @param {boolean} ascending
     */
    sort_table_by_column(table_id_str, col_index, ascending) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(table_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        _assertNum(col_index);
        _assertBoolean(ascending);
        const ret = wasm.wasmdocument_sort_table_by_column(this.__wbg_ptr, ptr0, len0, col_index, ascending);
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
     * Check if password protection is available.
     *
     * Password protection requires server-side encryption (AES-256/AGILE).
     * Use the server API: `POST /api/documents/convert` with `format=docx&password=...`
     *
     * Returns false in WASM (server-side only feature).
     * @returns {boolean}
     */
    supports_password_protection() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wasmdocument_supports_password_protection(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Convert selected paragraphs to a table.
     *
     * Takes consecutive paragraphs and converts each into a table row.
     * Cells are split by `delimiter` ("tab", "comma", "semicolon", or "paragraph").
     * If delimiter is "paragraph", each paragraph becomes a single-cell row.
     *
     * Returns the new table node ID.
     * @param {string} first_para_str
     * @param {string} last_para_str
     * @param {string} delimiter
     * @returns {string}
     */
    text_to_table(first_para_str, last_para_str, delimiter) {
        let deferred5_0;
        let deferred5_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(first_para_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ptr1 = passStringToWasm0(last_para_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            const ptr2 = passStringToWasm0(delimiter, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len2 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_text_to_table(this.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2);
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
     * Convert to OnlyOffice DOCY binary format.
     *
     * Returns the wrapped DOCY payload string: `DOCY;v5;{size};{base64_data}`.
     *
     * This is currently a debug/export surface only. The current DOCY writer
     * is not yet structurally compatible with OnlyOffice for general open.
     * @returns {string}
     */
    to_docy() {
        let deferred2_0;
        let deferred2_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ret = wasm.wasmdocument_to_docy(this.__wbg_ptr);
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
     * Export the document as EPUB bytes.
     *
     * Generates an EPUB 3 file from the document content.
     * Returns the EPUB ZIP as a byte array.
     * @returns {Uint8Array}
     */
    to_epub() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wasmdocument_to_epub(this.__wbg_ptr);
        if (ret[3]) {
            throw takeFromExternrefTable0(ret[2]);
        }
        var v1 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        return v1;
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
     * Export the document as PDF/A-1b bytes (ISO 19005 archival format).
     * @returns {Uint8Array}
     */
    to_pdf_a() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wasmdocument_to_pdf_a(this.__wbg_ptr);
        if (ret[3]) {
            throw takeFromExternrefTable0(ret[2]);
        }
        var v1 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        return v1;
    }
    /**
     * Export the document as a PDF/A data URL.
     * @returns {string}
     */
    to_pdf_a_data_url() {
        let deferred2_0;
        let deferred2_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ret = wasm.wasmdocument_to_pdf_a_data_url(this.__wbg_ptr);
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
     * Toggle a form checkbox's checked state.
     *
     * Returns the new checked state.
     * @param {string} node_id_str
     * @returns {boolean}
     */
    toggle_form_checkbox(node_id_str) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(node_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmdocument_toggle_form_checkbox(this.__wbg_ptr, ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ret[0] !== 0;
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
     * Update the IME composition preview text.
     *
     * If a preview already exists, deletes it first, then inserts
     * the new preview text at the anchor.
     * Returns an EditResult JSON.
     * @param {string} text
     * @returns {string}
     */
    update_composition(text) {
        let deferred3_0;
        let deferred3_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(text, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_update_composition(this.__wbg_ptr, ptr0, len0);
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
     * Update shape properties (position, size, fill, stroke).
     * @param {string} shape_id_str
     * @param {number} width_pt
     * @param {number} height_pt
     * @param {string} fill_hex
     * @param {string} stroke_hex
     */
    update_shape(shape_id_str, width_pt, height_pt, fill_hex, stroke_hex) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(shape_id_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(fill_hex, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(stroke_hex, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        const ret = wasm.wasmdocument_update_shape(this.__wbg_ptr, ptr0, len0, width_pt, height_pt, ptr1, len1, ptr2, len2);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
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
    /**
     * Move a position in a direction by a granularity.
     *
     * Returns JSON `PositionRef`.
     * Validate and clamp a position to ensure the offset is within bounds.
     *
     * If the offset exceeds the text node's length, it is clamped to the end.
     * Returns the validated position as JSON.
     * @param {string} position_json
     * @returns {string}
     */
    validate_position(position_json) {
        let deferred3_0;
        let deferred3_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(position_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_validate_position(this.__wbg_ptr, ptr0, len0);
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
     * Return scenes for a range of pages (batch fetch for viewport).
     *
     * Returns JSON: `{ "pages": [...] }`
     * @param {number} start_page
     * @param {number} end_page
     * @returns {string}
     */
    visible_page_scenes(start_page, end_page) {
        let deferred2_0;
        let deferred2_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            _assertNum(start_page);
            _assertNum(end_page);
            const ret = wasm.wasmdocument_visible_page_scenes(this.__wbg_ptr, start_page, end_page);
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
     * Return visible page scenes using loaded fonts for accurate text shaping.
     * @param {WasmFontDatabase} font_db
     * @param {number} start_page
     * @param {number} end_page
     * @returns {string}
     */
    visible_page_scenes_with_fonts(font_db, start_page, end_page) {
        let deferred2_0;
        let deferred2_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            _assertClass(font_db, WasmFontDatabase);
            if (font_db.__wbg_ptr === 0) {
                throw new Error('Attempt to use a moved value');
            }
            _assertNum(start_page);
            _assertNum(end_page);
            const ret = wasm.wasmdocument_visible_page_scenes_with_fonts(this.__wbg_ptr, font_db.__wbg_ptr, start_page, end_page);
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
     * Get the word boundary around a position.
     *
     * Returns JSON `RangeRef` with anchor at word start and focus at word end.
     * @param {string} position_json
     * @returns {string}
     */
    word_boundary(position_json) {
        let deferred3_0;
        let deferred3_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ptr0 = passStringToWasm0(position_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmdocument_word_boundary(this.__wbg_ptr, ptr0, len0);
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
    /**
     * Rasterize a single glyph to RGBA pixels.
     *
     * Returns a flat Uint8Array of RGBA pixels (width * height * 4 bytes)
     * plus metadata as JSON: `{"width":W,"height":H,"bearingX":X,"bearingY":Y,"advance":A}`
     *
     * This is the core API for canvas-first rendering — replaces `ctx.fillText()`.
     * @param {string} family
     * @param {boolean} bold
     * @param {boolean} italic
     * @param {number} glyph_id
     * @param {number} size_px
     * @param {number} r
     * @param {number} g
     * @param {number} b
     * @returns {Uint8Array | undefined}
     */
    rasterize_glyph(family, bold, italic, glyph_id, size_px, r, g, b) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(family, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        _assertBoolean(bold);
        _assertBoolean(italic);
        _assertNum(glyph_id);
        _assertNum(r);
        _assertNum(g);
        _assertNum(b);
        const ret = wasm.wasmfontdatabase_rasterize_glyph(this.__wbg_ptr, ptr0, len0, bold, italic, glyph_id, size_px, r, g, b);
        let v2;
        if (ret[0] !== 0) {
            v2 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        }
        return v2;
    }
    /**
     * Rasterize a complete text run to RGBA pixels.
     *
     * Takes shaped glyph data (from layout engine) and produces a single
     * bitmap. Returns packed buffer: 8 bytes header (width u32, height u32)
     * followed by RGBA pixels.
     * @param {string} family
     * @param {boolean} bold
     * @param {boolean} italic
     * @param {Uint8Array} glyph_data
     * @param {number} size_px
     * @param {number} r
     * @param {number} g
     * @param {number} b
     * @param {number} total_width
     * @param {number} line_height
     * @returns {Uint8Array | undefined}
     */
    rasterize_run(family, bold, italic, glyph_data, size_px, r, g, b, total_width, line_height) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(family, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        _assertBoolean(bold);
        _assertBoolean(italic);
        const ptr1 = passArray8ToWasm0(glyph_data, wasm.__wbindgen_malloc);
        const len1 = WASM_VECTOR_LEN;
        _assertNum(r);
        _assertNum(g);
        _assertNum(b);
        const ret = wasm.wasmfontdatabase_rasterize_run(this.__wbg_ptr, ptr0, len0, bold, italic, ptr1, len1, size_px, r, g, b, total_width, line_height);
        let v3;
        if (ret[0] !== 0) {
            v3 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        }
        return v3;
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
 * WASM bindings for spreadsheet operations (XLSX, ODS, CSV).
 *
 * Provides a JavaScript-friendly API for opening, editing, and exporting
 * spreadsheet files from the browser or Node.js.
 */
export class WasmSpreadsheet {
    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(WasmSpreadsheet.prototype);
        obj.__wbg_ptr = ptr;
        WasmSpreadsheetFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmSpreadsheetFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmspreadsheet_free(ptr, 0);
    }
    /**
     * Add a new sheet with the given name.
     * @param {string} name
     */
    add_sheet(name) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.wasmspreadsheet_add_sheet(this.__wbg_ptr, ptr0, len0);
    }
    /**
     * Delete a column and shift remaining columns left.
     * @param {number} sheet
     * @param {number} col
     */
    delete_column(sheet, col) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        _assertNum(sheet);
        _assertNum(col);
        wasm.wasmspreadsheet_delete_column(this.__wbg_ptr, sheet, col);
    }
    /**
     * Delete a row and shift remaining rows up.
     * @param {number} sheet
     * @param {number} row
     */
    delete_row(sheet, row) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        _assertNum(sheet);
        _assertNum(row);
        wasm.wasmspreadsheet_delete_row(this.__wbg_ptr, sheet, row);
    }
    /**
     * Delete a sheet by index.
     * @param {number} index
     */
    delete_sheet(index) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        _assertNum(index);
        wasm.wasmspreadsheet_delete_sheet(this.__wbg_ptr, index);
    }
    /**
     * Get dimensions (max col, max row) as JSON string: `"[cols,rows]"`.
     * @param {number} sheet
     * @returns {string}
     */
    dimensions(sheet) {
        let deferred1_0;
        let deferred1_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            _assertNum(sheet);
            const ret = wasm.wasmspreadsheet_dimensions(this.__wbg_ptr, sheet);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Export a sheet as CSV string.
     * @param {number} sheet
     * @returns {string}
     */
    export_csv(sheet) {
        let deferred1_0;
        let deferred1_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            _assertNum(sheet);
            const ret = wasm.wasmspreadsheet_export_csv(this.__wbg_ptr, sheet);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Export as ODS bytes.
     * @returns {Uint8Array}
     */
    export_ods() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wasmspreadsheet_export_ods(this.__wbg_ptr);
        if (ret[3]) {
            throw takeFromExternrefTable0(ret[2]);
        }
        var v1 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        return v1;
    }
    /**
     * Export as XLSX bytes.
     * @returns {Uint8Array}
     */
    export_xlsx() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wasmspreadsheet_export_xlsx(this.__wbg_ptr);
        if (ret[3]) {
            throw takeFromExternrefTable0(ret[2]);
        }
        var v1 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        return v1;
    }
    /**
     * Set or clear frozen panes on a sheet.
     *
     * Pass `col=0, row=0` to unfreeze.
     * @param {number} sheet
     * @param {number} col
     * @param {number} row
     */
    freeze_panes(sheet, col, row) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        _assertNum(sheet);
        _assertNum(col);
        _assertNum(row);
        wasm.wasmspreadsheet_freeze_panes(this.__wbg_ptr, sheet, col, row);
    }
    /**
     * Get cell value as string.
     *
     * Returns an empty string for empty or out-of-range cells.
     * @param {number} sheet
     * @param {number} col
     * @param {number} row
     * @returns {string}
     */
    get_cell(sheet, col, row) {
        let deferred1_0;
        let deferred1_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            _assertNum(sheet);
            _assertNum(col);
            _assertNum(row);
            const ret = wasm.wasmspreadsheet_get_cell(this.__wbg_ptr, sheet, col, row);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Get a visible range of cells as JSON for rendering.
     *
     * Returns a JSON object:
     * ```json
     * {
     *   "cells": [{"col":0,"row":0,"value":"Hello","formula":null,"styleId":0}, ...],
     *   "colWidths": [8.43, 15.0, ...],
     *   "rowHeights": [15.0, 20.0, ...]
     * }
     * ```
     * @param {number} sheet
     * @param {number} start_col
     * @param {number} start_row
     * @param {number} end_col
     * @param {number} end_row
     * @returns {string}
     */
    get_visible_range_json(sheet, start_col, start_row, end_col, end_row) {
        let deferred1_0;
        let deferred1_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            _assertNum(sheet);
            _assertNum(start_col);
            _assertNum(start_row);
            _assertNum(end_col);
            _assertNum(end_row);
            const ret = wasm.wasmspreadsheet_get_visible_range_json(this.__wbg_ptr, sheet, start_col, start_row, end_col, end_row);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Insert a column after the given column index.
     *
     * All columns at `after_col + 1` and beyond are shifted right.
     * @param {number} sheet
     * @param {number} after_col
     */
    insert_column(sheet, after_col) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        _assertNum(sheet);
        _assertNum(after_col);
        wasm.wasmspreadsheet_insert_column(this.__wbg_ptr, sheet, after_col);
    }
    /**
     * Insert a row after the given row index.
     *
     * All rows at `after_row + 1` and below are shifted down.
     * @param {number} sheet
     * @param {number} after_row
     */
    insert_row(sheet, after_row) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        _assertNum(sheet);
        _assertNum(after_row);
        wasm.wasmspreadsheet_insert_row(this.__wbg_ptr, sheet, after_row);
    }
    /**
     * Get merged cells as JSON array: `[{"start":"A1","end":"C3"}, ...]`.
     * @param {number} sheet
     * @returns {string}
     */
    merged_cells_json(sheet) {
        let deferred1_0;
        let deferred1_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            _assertNum(sheet);
            const ret = wasm.wasmspreadsheet_merged_cells_json(this.__wbg_ptr, sheet);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Create a new empty spreadsheet with one sheet.
     */
    constructor() {
        const ret = wasm.wasmspreadsheet_new();
        this.__wbg_ptr = ret >>> 0;
        WasmSpreadsheetFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * Open a spreadsheet from bytes (auto-detect XLSX, ODS, CSV).
     *
     * Detection is based on file magic bytes:
     * - XLSX/ODS: ZIP signature (PK header)
     * - CSV: plain text fallback
     * @param {Uint8Array} data
     * @returns {WasmSpreadsheet}
     */
    static open(data) {
        const ptr0 = passArray8ToWasm0(data, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmspreadsheet_open(ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return WasmSpreadsheet.__wrap(ret[0]);
    }
    /**
     * Recalculate all formulas in a sheet.
     * @param {number} sheet
     */
    recalculate(sheet) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        _assertNum(sheet);
        wasm.wasmspreadsheet_recalculate(this.__wbg_ptr, sheet);
    }
    /**
     * Rename a sheet by index.
     * @param {number} index
     * @param {string} name
     */
    rename_sheet(index, name) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        _assertNum(index);
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.wasmspreadsheet_rename_sheet(this.__wbg_ptr, index, ptr0, len0);
    }
    /**
     * Set cell value (auto-detect type: number, boolean, or text).
     * @param {number} sheet
     * @param {number} col
     * @param {number} row
     * @param {string} value
     */
    set_cell(sheet, col, row, value) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        _assertNum(sheet);
        _assertNum(col);
        _assertNum(row);
        const ptr0 = passStringToWasm0(value, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.wasmspreadsheet_set_cell(this.__wbg_ptr, sheet, col, row, ptr0, len0);
    }
    /**
     * Set cell formula.
     * @param {number} sheet
     * @param {number} col
     * @param {number} row
     * @param {string} formula
     */
    set_formula(sheet, col, row, formula) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        _assertNum(sheet);
        _assertNum(col);
        _assertNum(row);
        const ptr0 = passStringToWasm0(formula, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.wasmspreadsheet_set_formula(this.__wbg_ptr, sheet, col, row, ptr0, len0);
    }
    /**
     * Get the number of sheets.
     * @returns {number}
     */
    sheet_count() {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        const ret = wasm.wasmspreadsheet_sheet_count(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Get sheet names as a JSON array.
     * @returns {string}
     */
    sheet_names_json() {
        let deferred1_0;
        let deferred1_1;
        try {
            if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
            _assertNum(this.__wbg_ptr);
            const ret = wasm.wasmspreadsheet_sheet_names_json(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Sort rows by a column value.
     *
     * Sorts all data rows in the sheet by the specified column.
     * @param {number} sheet
     * @param {number} col
     * @param {boolean} ascending
     */
    sort_by_column(sheet, col, ascending) {
        if (this.__wbg_ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.__wbg_ptr);
        _assertNum(sheet);
        _assertNum(col);
        _assertBoolean(ascending);
        wasm.wasmspreadsheet_sort_by_column(this.__wbg_ptr, sheet, col, ascending);
    }
}
if (Symbol.dispose) WasmSpreadsheet.prototype[Symbol.dispose] = WasmSpreadsheet.prototype.free;

/**
 * Detect the file type from bytes with extended metadata.
 *
 * Returns a JSON string with fields:
 * - `type`: file extension (e.g., "docx", "xlsx", "pptx")
 * - `label`: human-readable label (e.g., "Excel Spreadsheet")
 * - `mime`: MIME type
 * - `isDocument`: boolean
 * - `isSpreadsheet`: boolean
 * - `isPresentation`: boolean
 * - `isSupported`: whether s1engine can open this file
 * @param {Uint8Array} data
 * @returns {string}
 */
export function detect_file_type(data) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passArray8ToWasm0(data, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.detect_file_type(ptr0, len0);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * Detect the format of a document from its bytes.
 *
 * Returns one of: "docx", "odt", "pdf", "txt", "csv", "xlsx", "pptx", "ods", "odp", "doc".
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
const WasmSpreadsheetFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmspreadsheet_free(ptr >>> 0, 1));


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
