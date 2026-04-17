/* tslint:disable */
/* eslint-disable */

/**
 * A collaborative document that supports CRDT-based real-time editing.
 *
 * Each instance represents one replica. Local edits produce operations that
 * must be broadcast to other replicas. Remote operations are applied via
 * `apply_remote_ops`.
 */
export class WasmCollabDocument {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Append an empty paragraph.
     */
    append_paragraph(text: string): string;
    /**
     * Apply a remote awareness (cursor) update from another replica.
     */
    apply_awareness_update(update_json: string): void;
    /**
     * Apply a local text deletion and return serialized ops for broadcast.
     */
    apply_local_delete_text(target_id: string, offset: number, length: number): string;
    /**
     * Apply a local formatting change and return serialized ops for broadcast.
     */
    apply_local_format(target_id: string, key: string, value: string): string;
    /**
     * Apply a local text insertion and return serialized ops for broadcast.
     *
     * Returns a JSON string of the operations that must be sent to other replicas.
     */
    apply_local_insert_text(target_id: string, offset: number, text: string): string;
    /**
     * Apply remote operations received from another replica.
     *
     * Accepts a JSON string of a CRDT operation (as produced by apply_local_* methods).
     */
    apply_remote_ops(ops_json: string): void;
    /**
     * Check if redo is available.
     */
    can_redo(): boolean;
    /**
     * Check if undo is available.
     */
    can_undo(): boolean;
    /**
     * Compact the operation log (merge consecutive single-char inserts).
     */
    compact_op_log(): void;
    /**
     * Delete a node.
     */
    delete_node(node_id_str: string): string;
    /**
     * Delete a text selection (single or cross-paragraph).
     * Returns serialized CRDT operations.
     */
    delete_selection(start_node_str: string, start_offset: number, end_node_str: string, end_offset: number): string;
    /**
     * Export the collaborative document to a format (docx, odt, txt, md).
     */
    export(format: string): Uint8Array;
    /**
     * Format a selection.
     * Returns serialized CRDT operations.
     */
    format_selection(start_node_str: string, start_offset: number, end_node_str: string, end_offset: number, key: string, value: string): string;
    /**
     * Free the document (for manual memory management from JS).
     */
    free_doc(): void;
    /**
     * Get operations that have happened since a given state vector.
     *
     * Used for delta sync: peer sends their state vector, you return
     * the operations they're missing.
     */
    get_changes_since(state_vector_json: string): string;
    /**
     * Get formatting info for a node as JSON (delegates to WasmDocument).
     */
    get_formatting_json(node_id_str: string): string;
    /**
     * Get all peer cursors as JSON.
     *
     * Returns a JSON array of cursor states:
     * `[{"replicaId":2,"nodeId":"1:5","offset":3,"userName":"Alice","userColor":"#ff0000"},...]`
     */
    get_peers_json(): string;
    /**
     * Get the current state vector as JSON.
     *
     * Used for delta synchronization — send your state vector to a peer
     * to find out what operations you're missing.
     */
    get_state_vector(): string;
    /**
     * Insert horizontal rule.
     */
    insert_horizontal_rule(after_node_str: string): string;
    /**
     * Insert page break.
     */
    insert_page_break(after_node_str: string): string;
    /**
     * Insert a paragraph after a given node.
     */
    insert_paragraph_after(after_node_str: string, text: string): string;
    /**
     * Insert a table.
     */
    insert_table(after_node_str: string, rows: number, cols: number): string;
    /**
     * Insert text in a paragraph at a specific offset (CRDT-native).
     */
    insert_text_in_paragraph(node_id_str: string, offset: number, text: string): string;
    /**
     * Merge two paragraphs.
     * Returns serialized CRDT operations.
     */
    merge_paragraphs(node1_str: string, node2_str: string): string;
    /**
     * Get the size of the operation log.
     */
    op_log_size(): number;
    /**
     * Get paragraph IDs as JSON array.
     */
    paragraph_ids_json(): string;
    /**
     * Paste plain text (may create multiple paragraphs).
     */
    paste_plain_text(node_id_str: string, offset: number, text: string): string;
    /**
     * Redo the last undone operation.
     */
    redo(): string;
    /**
     * Render a single node as HTML (for incremental rendering).
     */
    render_node_html(node_id_str: string): string;
    /**
     * Get the replica ID of this collaborative document.
     */
    replica_id(): bigint;
    /**
     * Set alignment for a paragraph.
     */
    set_alignment(node_id_str: string, alignment: string): string;
    /**
     * Set the local cursor position and return an awareness update for broadcast.
     */
    set_cursor(node_id: string, offset: number, user_name: string, user_color: string): string;
    /**
     * Set heading level for a paragraph.
     */
    set_heading_level(node_id_str: string, level: number): string;
    /**
     * Set indent for a paragraph.
     */
    set_indent(node_id_str: string, side: string, value: number): string;
    /**
     * Set line spacing for a paragraph.
     */
    set_line_spacing(node_id_str: string, value: string): string;
    /**
     * Set list format for a paragraph.
     */
    set_list_format(node_id_str: string, format: string, level: number): string;
    /**
     * Set paragraph text, preserving multi-run formatting when possible.
     *
     * When the text is unchanged, this is a no-op (preserves all formatting).
     * When only a portion of the text changed, a diff-based approach is used
     * to minimize the edit and preserve run-level formatting on unchanged
     * portions. Only falls back to full delete+insert when the paragraph
     * has no existing runs.
     */
    set_paragraph_text(node_id_str: string, text: string): string;
    /**
     * Set column widths for a table.
     */
    set_table_column_widths(table_id_str: string, widths_csv: string): string;
    /**
     * Sort a table by column (delegates to WasmDocument.sort_table_by_column).
     */
    sort_table_by_column(table_id_str: string, col_index: number, ascending: boolean): string;
    /**
     * Split a paragraph at the given offset.
     * Returns JSON: { "newId": "replica:counter", "ops": [ ... ] }
     */
    split_paragraph(node_id_str: string, offset: number): string;
    /**
     * Get the document content as HTML.
     */
    to_html(): string;
    /**
     * Get the document content as plain text.
     */
    to_plain_text(): string;
    /**
     * Get the number of tombstones.
     */
    tombstone_count(): number;
    /**
     * Undo the last local operation.
     *
     * Returns JSON of the undo operation for broadcast, or null if nothing to undo.
     */
    undo(): string;
}

/**
 * A document handle for reading, editing, and exporting.
 */
export class WasmDocument {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Accept all tracked changes in the document.
     *
     * Insertions keep their content; deletions are removed; format changes
     * keep the new formatting. All revision attributes are stripped.
     */
    accept_all_changes(): void;
    /**
     * Accept a single tracked change by node ID string ("replica:counter").
     */
    accept_change(node_id_str: string): void;
    /**
     * Append a heading at the given level (1-6).
     *
     * Returns the heading paragraph's node ID.
     */
    append_heading(level: number, text: string): string;
    /**
     * Append a new paragraph with plain text at the end of the document body.
     *
     * Returns the new paragraph's node ID as "replica:counter".
     */
    append_paragraph(text: string): string;
    /**
     * Apply mail merge data to the document.
     *
     * Takes a JSON array of records: `[{"FirstName":"John","LastName":"Doe"}, ...]`
     * and a record index (0-based). Replaces all MERGEFIELD placeholders with
     * values from the specified record.
     *
     * Returns the number of fields replaced.
     */
    apply_merge_data(data_json: string, record_index: number): number;
    /**
     * Apply a predefined table style to a table.
     *
     * Available styles: "plain", "grid", "striped-blue", "striped-gray",
     * "header-blue", "header-green", "header-orange", "bordered", "minimal".
     *
     * Applies cell backgrounds and header row formatting.
     */
    apply_table_style(table_id_str: string, style_name: string): void;
    /**
     * Begin a batch of operations that form a single undo step.
     *
     * All operations between `begin_batch()` and `end_batch()` are applied
     * individually. On `end_batch()`, they are merged into a single undo
     * unit by collapsing the undo history.
     */
    begin_batch(label: string): void;
    /**
     * Begin an IME composition at the given position.
     *
     * Stores the anchor position for subsequent composition updates.
     * Returns JSON `{"status":"composing","anchor":<position>}`.
     */
    begin_composition(position_json: string): string;
    /**
     * Get all body-level node IDs with their types as JSON.
     *
     * Returns `[{"id":"0:5","type":"Paragraph"},{"id":"0:12","type":"Table"},...]`
     */
    body_children_json(): string;
    /**
     * Get the body node ID as "replica:counter" string.
     */
    body_id(): string | undefined;
    /**
     * Check if redo is available.
     */
    can_redo(): boolean;
    /**
     * Check if undo is available.
     */
    can_undo(): boolean;
    /**
     * Cancel the IME composition.
     *
     * Deletes the preview text and clears composition state.
     * Returns an EditResult JSON with cursor at the original anchor.
     */
    cancel_composition(): string;
    /**
     * Delete a canvas range (anchor + focus as text-node IDs + UTF-16 offsets).
     *
     * Resolves the range to paragraph coordinates, performs the deletion,
     * and returns an EditResult JSON string with the cursor collapsed at range start.
     */
    canvas_delete_range(range_json: string): string;
    /**
     * Insert a paragraph break at a canvas position.
     *
     * Splits the paragraph at the resolved char offset.
     * Returns an EditResult JSON with the cursor at the start of the new paragraph.
     */
    canvas_insert_paragraph_break(position_json: string): string;
    /**
     * Insert text at a canvas position (text-node ID + UTF-16 offset).
     *
     * Resolves the position to paragraph coordinates, performs the insert,
     * and returns an EditResult JSON string with the new cursor position.
     */
    canvas_insert_text(position_json: string, text: string): string;
    /**
     * Replace a canvas range with new text.
     *
     * Deletes the range, then inserts text at the start position.
     * Returns an EditResult JSON with the cursor after the inserted text.
     */
    canvas_replace_range(range_json: string, text: string): string;
    /**
     * Toggle a formatting mark on a canvas range.
     *
     * Checks the current formatting state at the anchor position and
     * toggles the specified mark. Supported marks: "bold", "italic",
     * "underline", "strikethrough".
     *
     * Returns an EditResult JSON.
     */
    canvas_toggle_mark(range_json: string, mark: string): string;
    /**
     * Get the caret rectangle for a model position.
     *
     * Returns JSON `RectPt` with page_index, x, y, width (1.0), height.
     */
    caret_rect(position_json: string): string;
    /**
     * Clear all undo/redo history.
     */
    clear_history(): void;
    /**
     * Explicitly release document memory. The document cannot be used after this.
     */
    close(): void;
    /**
     * Commit the IME composition with final text.
     *
     * Deletes the preview, inserts the final text, and clears composition state.
     * Returns an EditResult JSON.
     */
    commit_composition(text: string): string;
    /**
     * Compare this document with another and return word-level differences as JSON.
     *
     * Takes the bytes of another document, opens it, extracts plain text from both,
     * and returns a JSON array of diff operations:
     * `[{"type":"equal","text":"..."},{"type":"insert","text":"..."},{"type":"delete","text":"..."}]`
     */
    compare_with(other_bytes: Uint8Array): string;
    /**
     * Copy a canvas range as HTML.
     *
     * Resolves the range to paragraph coordinates and delegates to
     * the existing `export_selection_html` method.
     */
    copy_range_html(range_json: string): string;
    /**
     * Copy a canvas range as plain text.
     *
     * Walks text nodes from anchor to focus, joining with newlines
     * at paragraph boundaries.
     */
    copy_range_plain_text(range_json: string): string;
    /**
     * Delete a comment and its range markers.
     */
    delete_comment(comment_id: string): void;
    /**
     * Delete a comment reply by its comment ID.
     *
     * Removes the CommentBody node that has the given `reply_id` as its
     * CommentId attribute. Only deletes replies (nodes with CommentParentId).
     */
    delete_comment_reply(reply_id: string): void;
    /**
     * Delete an image node (and its containing paragraph if empty).
     */
    delete_image(image_id_str: string): void;
    /**
     * Delete a body-level node (paragraph, table, heading, etc.).
     */
    delete_node(node_id_str: string): void;
    /**
     * Delete a selection range spanning one or more paragraphs.
     *
     * If start and end are in the same paragraph, deletes the text range.
     * If they span multiple paragraphs, deletes the tail of the first,
     * all intermediate paragraphs, the head of the last, then merges
     * the first and last paragraphs.
     */
    delete_selection(start_node_str: string, start_offset: number, end_node_str: string, end_offset: number): void;
    /**
     * Delete a column at the given index across all rows.
     */
    delete_table_column(table_id_str: string, col_index: number): void;
    /**
     * Delete a row at the given index in a table.
     */
    delete_table_row(table_id_str: string, row_index: number): void;
    /**
     * Delete text in a paragraph at a given character offset.
     *
     * Correctly handles multi-run paragraphs by finding the right text node(s).
     */
    delete_text_in_paragraph(node_id_str: string, offset: number, length: number): void;
    /**
     * Return a monotonically increasing document revision number.
     *
     * Bumps on every model mutation (insert, delete, format change).
     * Uses undo_count as a proxy for revision tracking.
     */
    document_revision(): number;
    /**
     * Edit a comment's text content.
     *
     * Replaces the text in the first paragraph of the CommentBody node
     * matching `comment_id`.
     */
    edit_comment(comment_id: string, new_text: string): void;
    /**
     * Get the editor capabilities as a JSON object.
     *
     * Returns a JSON object indicating which editing features are available.
     */
    editor_capabilities(): string;
    /**
     * End a batch and merge all operations since `begin_batch()` into
     * a single undo step.
     */
    end_batch(): void;
    /**
     * Export the document to the specified format.
     *
     * Format should be one of: "docx", "odt", "txt", "pdf".
     * Returns the exported bytes.
     */
    export(format: string): Uint8Array;
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
     */
    export_selection_html(start_node_str: string, start_offset: number, end_node_str: string, end_offset: number): string;
    /**
     * Get an import fidelity report as a JSON string.
     *
     * Counts objects that could not be rendered faithfully and are shown
     * as placeholders. Returns JSON like:
     * `{"charts":2,"smartart":1,"ole":0,"missingImages":0,"total":3}`
     *
     * Consumers can use this to display "3 objects shown as placeholders"
     * after opening a document, rather than silently degrading.
     */
    fidelity_report_json(): string;
    /**
     * Find all occurrences of text in the document.
     *
     * Returns JSON array of `{"nodeId":"0:5","offset":3,"length":5}`.
     */
    find_text(query: string, case_sensitive: boolean): string;
    /**
     * Force a fresh relayout using the provided font database.
     *
     * Call this from JS on the deferred timer to get an accurate layout
     * after a batch of edits. Clears the dirty flag.
     */
    force_relayout(font_db: WasmFontDatabase): void;
    /**
     * Set a formatting attribute on a specific Run node.
     *
     * key/value are string representations parsed to AttributeKey/AttributeValue.
     * Supported keys: "bold", "italic", "underline", "strikethrough",
     * "fontSize", "fontFamily", "color", "highlightColor", "superscript", "subscript".
     */
    format_run(run_id_str: string, key: string, value: string): void;
    /**
     * Format a text range spanning one or more runs/paragraphs.
     *
     * Internally splits start/end runs as needed and applies the attribute
     * to all runs in the selection range. Single transaction (atomic undo).
     *
     * start_node/end_node are paragraph IDs, offsets are character positions.
     */
    format_selection(start_node_str: string, start_offset: number, end_node_str: string, end_offset: number, key: string, value: string): void;
    /**
     * Free the document, releasing memory.
     *
     * After calling this, all other methods will return an error.
     */
    free(): void;
    /**
     * Generate a bibliography section from all citations in the document.
     *
     * Scans for CITATION fields, extracts their JSON data, and creates
     * a formatted bibliography paragraph after `after_node_str`.
     */
    generate_bibliography(after_node_str: string): string;
    /**
     * Get page indices affected by a node, plus adjacent pages.
     *
     * Returns a JSON array of 0-based page indices, e.g. `[1,2,3]`.
     * Used by the editor to know which pages to re-render after an edit.
     *
     * Layout must already be cached (call `get_page_count*` first).
     */
    get_affected_pages(node_id_str: string): string;
    /**
     * Get the node ID of a cell at a given row/column index.
     */
    get_cell_id(table_id_str: string, row: number, col: number): string;
    /**
     * Get the text content of a table cell.
     */
    get_cell_text(cell_id_str: string): string;
    /**
     * Get all comments as a JSON array.
     */
    get_comments_json(): string;
    /**
     * Get all text in the document as a single string.
     */
    get_document_text(): string;
    /**
     * Get all endnotes as JSON array.
     *
     * Returns `[{"number":1,"text":"Endnote text"},...]`.
     */
    get_endnotes_json(): string;
    /**
     * Get all footnotes as JSON array.
     *
     * Returns `[{"number":1,"text":"Footnote text"},...]`.
     */
    get_footnotes_json(): string;
    /**
     * Get the formatting state of a paragraph as JSON.
     *
     * Returns JSON with keys: bold, italic, underline, strikethrough,
     * fontSize, fontFamily, color, alignment, headingLevel.
     * Values come from the paragraph's attributes and first run's attributes.
     */
    get_formatting_json(node_id_str: string): string;
    /**
     * Get header/footer info for a section as JSON.
     *
     * Returns JSON: `{"hasDefaultHeader":true,"hasFirstHeader":false,
     * "defaultHeaderText":"My Header","firstHeaderText":"",
     * "hasDefaultFooter":true,"hasFirstFooter":false,
     * "defaultFooterText":"Page 1","firstFooterText":"",
     * "titlePage":false}`
     */
    get_header_footer_info(section_index: number): string;
    /**
     * Get the document heading hierarchy as JSON.
     *
     * Returns a JSON array of objects: `[{"nodeId":"r:c","level":1,"text":"..."},...]`
     * Useful for building outline panels and TOC navigation.
     */
    get_headings_json(): string;
    /**
     * Get image as a data URL for display.
     */
    get_image_data_url(image_id_str: string): string;
    /**
     * Get the wrap mode for an image node.
     *
     * Returns one of: "inline", "wrapLeft", "wrapRight", "wrapBoth",
     * "topAndBottom", "behind", "inFront". Defaults to "inline".
     */
    get_image_wrap_mode(image_id_str: string): string;
    /**
     * Get layout cache statistics as JSON.
     *
     * Returns `{"hits":N,"misses":N,"entries":N}`.
     */
    get_layout_cache_stats(): string;
    /**
     * Get the total number of pages using default (empty) font metrics.
     *
     * Lazily computes and caches layout. The cache is invalidated on any
     * document mutation.
     */
    get_page_count(): number;
    /**
     * Get the total number of pages using loaded fonts for accurate metrics.
     *
     * Lazily computes and caches layout. The cache is invalidated on any
     * document mutation.
     */
    get_page_count_with_fonts(font_db: WasmFontDatabase): number;
    /**
     * Get ready-to-mount HTML for a single page using default font metrics.
     *
     * Returns document-model HTML (semantic `<p>`, `<h1>`, `<table>` with
     * `data-node-id`) filtered to the blocks on `page_index`. Split
     * paragraphs get `data-split="first"` or `data-split="continuation"`.
     *
     * Call `get_page_count()` first to ensure layout is cached.
     */
    get_page_html(page_index: number): string;
    /**
     * Get ready-to-mount HTML for a single page using loaded fonts.
     *
     * Call `get_page_count_with_fonts()` first to ensure layout is cached,
     * or this will lazily compute layout.
     */
    get_page_html_with_fonts(page_index: number, font_db: WasmFontDatabase): string;
    /**
     * Get page break information from the layout engine as JSON.
     *
     * Returns `{"pages": [{"pageNum":1, "nodeIds":["0:5","0:12"], "footer":"Page 1", "header":"..."}, ...]}`.
     * This tells the editor which node IDs are on which page, so the editor
     * can show visual page breaks matching the actual layout engine output.
     */
    get_page_map_json(): string;
    /**
     * Get page map JSON with font metrics for accurate line-level pagination.
     */
    get_page_map_json_with_fonts(font_db: WasmFontDatabase): string;
    /**
     * Get page setup properties for the first section as JSON.
     *
     * Returns JSON: `{"pageWidth":612,"pageHeight":792,"marginTop":72,
     * "marginBottom":72,"marginLeft":72,"marginRight":72,"orientation":"portrait"}`
     *
     * All dimensions are in points (1 inch = 72 points).
     */
    get_page_setup_json(): string;
    /**
     * Get the text content of a paragraph (concatenates all runs).
     */
    get_paragraph_text(node_id_str: string): string;
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
     */
    get_reference_targets_json(): string;
    /**
     * Get formatting of a specific run as JSON.
     *
     * Returns `{"bold":true,"italic":false,...}`.
     */
    get_run_formatting_json(run_id_str: string): string;
    /**
     * Get run IDs within a paragraph as a JSON array.
     *
     * Returns `["0:5","0:8",...]` — the IDs of all Run nodes in the paragraph.
     */
    get_run_ids(paragraph_id_str: string): string;
    /**
     * Get the text content of a specific run.
     */
    get_run_text(run_id_str: string): string;
    /**
     * Get section break information for all sections as JSON.
     *
     * Returns a JSON array of objects with section index, break type, and
     * page dimensions for each section.
     */
    get_section_breaks_json(): string;
    /**
     * Get the column configuration for a section as JSON.
     *
     * Returns JSON: `{"columns":2,"spacing":36.0}`
     */
    get_section_columns(section_index: number): string;
    /**
     * Get section properties as JSON.
     */
    get_sections_json(): string;
    /**
     * Get common formatting across a selection range as JSON.
     *
     * Returns JSON with `true`/`false`/`"mixed"` per property.
     * E.g., `{"bold":true,"italic":"mixed","underline":false}`.
     */
    get_selection_formatting_json(start_node_str: string, start_offset: number, end_node_str: string, end_offset: number): string;
    /**
     * Get table dimensions as JSON: `{"rows":N,"cols":M}`.
     */
    get_table_dimensions(table_id_str: string): string;
    /**
     * Get all unique font families used in the document.
     *
     * Returns a JSON array of font family names, e.g. `["Arial","Calibri","Georgia"]`.
     * Useful for determining which fonts need to be loaded before layout.
     */
    get_used_fonts(): string;
    /**
     * Hit-test a point on a page to find the nearest model position.
     *
     * Returns JSON `HitTestResult` with position, kind, node_id, and inside flag.
     */
    hit_test(page_index: number, x_pt: number, y_pt: number): string;
    /**
     * Insert bookmark start/end around a paragraph.
     *
     * Returns the bookmark start node ID.
     */
    insert_bookmark(para_id_str: string, name: string): string;
    /**
     * Insert an auto-numbered caption paragraph after a node.
     *
     * - `after_node_str`: the node (image paragraph, table, etc.) after which to insert
     * - `label`: "Figure", "Table", or "Equation"
     * - `text`: additional caption text (e.g., ": My diagram")
     *
     * The caption is numbered automatically by counting existing captions of the same label.
     * Returns the caption paragraph node ID.
     */
    insert_caption(after_node_str: string, label: string, text: string): string;
    /**
     * Insert a bibliography citation at the cursor position.
     *
     * - `para_id_str`: paragraph to insert into
     * - `citation_json`: JSON object with citation fields:
     *   `{"author":"Smith","year":"2024","title":"Paper Title","source":"Journal Name"}`
     *
     * Returns the citation field node ID.
     */
    insert_citation(para_id_str: string, citation_json: string): string;
    /**
     * Insert a column break inside the specified paragraph.
     *
     * Inserts a ColumnBreak node at the end of the paragraph's children.
     * Returns the column break node ID.
     */
    insert_column_break(para_id_str: string): string;
    /**
     * Insert a comment with range markers and body.
     *
     * Returns the comment ID string.
     */
    insert_comment(start_node_str: string, end_node_str: string, author: string, text: string): string;
    /**
     * Insert a comment with markers positioned at the selected text range.
     *
     * Unlike `insert_comment` which places markers at paragraph boundaries,
     * this positions CommentStart/CommentEnd at the correct run indices
     * based on character offsets within the paragraphs.
     */
    insert_comment_at_range(start_node_str: string, start_offset: number, end_node_str: string, end_offset: number, author: string, text: string): string;
    /**
     * Insert a reply to an existing comment.
     *
     * Returns the reply comment ID string.
     */
    insert_comment_reply(parent_comment_id: string, author: string, text: string): string;
    /**
     * Insert a cross-reference field at the cursor position.
     *
     * - `para_id_str`: paragraph to insert into
     * - `offset`: character offset within the paragraph
     * - `target_id_str`: node ID of the target (heading or bookmark)
     * - `ref_type`: "heading_text", "page_number", or "bookmark_text"
     * - `display_text`: the text to show for the cross-reference
     */
    insert_cross_reference(para_id_str: string, _offset: number, target_id_str: string, _ref_type: string, display_text: string): string;
    /**
     * Insert an endnote at the current position in a paragraph.
     *
     * Creates an endnote reference in the paragraph and an endnote body
     * at the document root. Returns the endnote body node ID.
     */
    insert_endnote(node_id_str: string, text: string): string;
    /**
     * Insert an equation (inline math) into a paragraph.
     *
     * `node_id_str` is the paragraph to insert into.
     * `latex_source` is the equation source (LaTeX or raw XML).
     * Returns the equation node ID string.
     */
    insert_equation(node_id_str: string, latex_source: string): string;
    /**
     * Insert a footnote at the current position in a paragraph.
     *
     * Creates a footnote reference in the paragraph and a footnote body
     * at the document root. Returns the footnote body node ID.
     */
    insert_footnote(node_id_str: string, text: string): string;
    /**
     * Insert a horizontal rule (thematic break) after the given node.
     *
     * Returns the new node ID.
     */
    insert_horizontal_rule(after_node_str: string): string;
    /**
     * Set a hyperlink URL on a run.
     *
     * tooltip_opt is optional — pass empty string or null for no tooltip.
     */
    insert_hyperlink(run_id_str: string, url: string, tooltip_opt: string): void;
    /**
     * Insert an image after the specified body-level node.
     *
     * Stores bytes in MediaStore, creates Paragraph → Run → Image structure.
     * Returns the paragraph node ID containing the image.
     */
    insert_image(after_node_str: string, data: Uint8Array, content_type: string, width_pt: number, height_pt: number): string;
    /**
     * Insert a line break (soft return) within a paragraph at a character offset.
     *
     * Creates a `LineBreak` node within the run at the specified offset,
     * splitting the text node if the offset falls in the middle.
     */
    insert_line_break(node_id_str: string, char_offset: number): void;
    /**
     * Insert a mail merge field placeholder in the current paragraph.
     *
     * - `para_id_str`: paragraph to insert into
     * - `field_name`: the merge field name (e.g., "FirstName", "Email")
     *
     * Returns the field node ID. The field displays as `«FieldName»` until merged.
     */
    insert_merge_field(para_id_str: string, field_name: string): string;
    /**
     * Insert a paragraph with PageBreakBefore after the given node.
     *
     * Returns the new paragraph node ID.
     */
    insert_page_break(after_node_str: string): string;
    /**
     * Insert a new paragraph after a given node.
     *
     * Returns the new paragraph's node ID.
     */
    insert_paragraph_after(after_id_str: string, text: string): string;
    /**
     * Insert a section break after the given node.
     *
     * `break_type` is one of: `"nextPage"`, `"continuous"`, `"evenPage"`, `"oddPage"`.
     *
     * This creates a new section in the document model. Content after the break
     * belongs to the new section with the specified break type.
     * Returns the new section's paragraph node ID (the first paragraph in the new section).
     */
    insert_section_break(after_node_str: string, break_type: string): string;
    /**
     * Insert a SEQ (sequence) field for auto-numbering.
     *
     * Sequence fields maintain separate counters per `seq_name` (e.g., "Figure", "Table").
     * Returns the field node ID.
     */
    insert_seq_field(para_id_str: string, seq_name: string): string;
    /**
     * Insert a shape (Drawing node) after a body-level node.
     *
     * Returns the Drawing node ID. The shape is rendered by the layout engine.
     */
    insert_shape(after_node_str: string, shape_type: string, width_pt: number, height_pt: number, _x_pt: number, _y_pt: number, fill_hex: string, stroke_hex: string, stroke_width: number): string;
    /**
     * Insert a tab node at the given character offset within a paragraph.
     *
     * Like `insert_line_break`, this inserts a `Tab` node at paragraph level,
     * splitting runs as needed. Tab nodes render
     * as `&emsp;` in HTML and as proper tab stops in layout.
     */
    insert_tab(node_id_str: string, char_offset: number): void;
    /**
     * Insert a table after the specified body-level node.
     *
     * Creates a table with the given number of rows and columns,
     * each cell containing an empty paragraph. Returns the table node ID.
     */
    insert_table(after_node_str: string, rows: number, cols: number): string;
    /**
     * Insert a column at the given index across all rows.
     */
    insert_table_column(table_id_str: string, col_index: number): void;
    /**
     * Insert a Table of Contents after the given node.
     *
     * `max_level` controls the deepest heading level included (1-9, default 3).
     * If `title` is non-empty, it is set as the TOC title.
     * Returns the TOC node ID string.
     */
    insert_table_of_contents(after_node_str: string, max_level: number, title: string): string;
    /**
     * Generate a Table of Figures from all Caption-styled paragraphs.
     *
     * Inserts a new section after `after_node_str` containing a list of all
     * captions found in the document (Figure 1: ..., Table 2: ..., etc.).
     * Returns the TOF container node ID.
     */
    insert_table_of_figures(after_node_str: string, label_filter: string): string;
    /**
     * Insert a row at the given index in a table.
     *
     * Creates cells matching the column count of existing rows.
     * Returns the new row's node ID.
     */
    insert_table_row(table_id_str: string, row_index: number): string;
    /**
     * Insert text at an offset in a paragraph's first text node.
     */
    insert_text_in_paragraph(node_id_str: string, offset: number, text: string): void;
    /**
     * Check if a batch is currently active.
     */
    is_batching(): boolean;
    /**
     * Check whether the layout cache is dirty (needs recomputation).
     */
    is_layout_dirty(): boolean;
    /**
     * Check if track changes mode is currently enabled.
     */
    is_track_changes_enabled(): boolean;
    /**
     * Check if this document handle is still valid.
     */
    is_valid(): boolean;
    /**
     * Convert LaTeX string to OMML XML.
     *
     * Handles common LaTeX commands and produces valid Office MathML.
     */
    latex_to_omml(latex: string): string;
    /**
     * Return a monotonically increasing layout revision number.
     *
     * Bumps when pagination output changes (page count, block positions).
     */
    layout_revision(): number;
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
     */
    layout_single_page_json(page_index: number): string;
    /**
     * Get the line boundary position for "start" or "end" of the line
     * containing the given position.
     *
     * Returns a PositionRef JSON.
     */
    line_boundary(position_json: string, side: string): string;
    /**
     * Merge cells in a range by setting ColSpan/RowSpan attributes.
     */
    merge_cells(table_id_str: string, start_row: number, start_col: number, end_row: number, end_col: number): void;
    /**
     * Merge two adjacent paragraphs.
     *
     * Moves all runs from `second_id` into `first_id` (preserving formatting),
     * then deletes the now-empty `second_id`. Used for Backspace at the start
     * of a paragraph.
     */
    merge_paragraphs(first_id_str: string, second_id_str: string): void;
    /**
     * Get the document author (from metadata).
     */
    metadata_author(): string | undefined;
    /**
     * Get full document metadata as JSON (title, author, custom_properties, etc.).
     */
    metadata_json(): string;
    /**
     * Get the document title (from metadata).
     */
    metadata_title(): string | undefined;
    /**
     * Move a node (e.g. an image paragraph) to be after another node in
     * the same parent (body). Used for drag-and-drop reordering.
     */
    move_node_after(node_id_str: string, after_id_str: string): void;
    /**
     * Move a node to be before another node in the same parent (body).
     */
    move_node_before(node_id_str: string, before_id_str: string): void;
    move_position(position_json: string, direction: string, granularity: string): string;
    /**
     * Move a range in a direction by a granularity.
     *
     * If extend is true, moves only the focus while keeping the anchor.
     * If extend is false, collapses the range and moves.
     * Returns a RangeRef JSON.
     */
    move_range(range_json: string, direction: string, granularity: string, extend: boolean): string;
    /**
     * Delete text at multiple cursor positions simultaneously.
     *
     * Takes a JSON array of `[{"nodeId":"0:5","offset":3,"length":1}, ...]`.
     * Applied in reverse order to preserve offsets.
     */
    multi_cursor_delete(cursors_json: string): void;
    /**
     * Insert text at multiple cursor positions simultaneously.
     *
     * Takes a JSON array of `[{"nodeId":"0:5","offset":3,"text":"x"}, ...]`.
     * Positions are sorted in reverse document order and applied back-to-front
     * so that earlier insertions don't shift later offsets.
     *
     * All insertions form a single undo step via merge_undo_entries.
     */
    multi_cursor_insert(cursors_json: string): void;
    /**
     * Get bounds for all pages containing a node.
     *
     * Returns JSON array of `RectPt`.
     */
    node_bounds(node_id_str: string): string;
    /**
     * Get detailed info about a node as JSON.
     *
     * Returns `{"id":"0:5","type":"Paragraph","text":"Hello","children":[...],...}`
     */
    node_info_json(node_id_str: string): string;
    /**
     * Convert OMML (Office MathML) XML to LaTeX string.
     *
     * Handles common OMML elements: fractions, subscripts, superscripts,
     * square roots, matrices, summations, integrals, Greek letters.
     */
    omml_to_latex(omml_xml: string): string;
    /**
     * Return a full scene for a single page.
     *
     * Returns JSON with page bounds, content rect, header/footer rects,
     * and all scene items (text runs, backgrounds, borders, images, shapes, etc.).
     */
    page_scene(page_index: number): string;
    /**
     * Return page scene using loaded fonts for accurate text shaping.
     */
    page_scene_with_fonts(font_db: WasmFontDatabase, page_index: number): string;
    /**
     * Get the number of paragraphs in the document.
     */
    paragraph_count(): number;
    /**
     * Get top-level paragraph IDs as a JSON array of "replica:counter" strings.
     */
    paragraph_ids_json(): string;
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
     */
    paste_formatted_runs_json(target_node_str: string, char_offset: number, runs_json: string): void;
    /**
     * Paste HTML at a canvas position.
     *
     * For now, strips HTML tags and inserts as plain text.
     * Returns an EditResult JSON.
     */
    paste_html(position_json: string, html: string): string;
    /**
     * Insert plain text at cursor position, splitting on newlines.
     */
    paste_plain_text(para_id_str: string, offset: number, text: string): void;
    /**
     * Redo the last undone operation. Returns true if something was redone.
     */
    redo(): boolean;
    /**
     * Reject all tracked changes in the document.
     *
     * Insertions are removed; deletions are un-deleted; format changes
     * restore original formatting. All revision attributes are stripped.
     */
    reject_all_changes(): void;
    /**
     * Reject a single tracked change by node ID string ("replica:counter").
     */
    reject_change(node_id_str: string): void;
    /**
     * Remove a hyperlink from a run.
     */
    remove_hyperlink(run_id_str: string): void;
    /**
     * Render a single node (paragraph, table, etc.) as HTML.
     *
     * Returns the HTML string for that node only, suitable for incremental
     * DOM updates. Uses the same rendering as `to_html()`.
     */
    render_node_html(node_id_str: string): string;
    /**
     * Render a paragraph node as HTML for the half-open character range
     * `[start_char, end_char)`. Used by pagination to mount page-specific
     * fragments for split paragraphs instead of rendering the full paragraph
     * and clipping it in CSS.
     */
    render_node_slice(node_id_str: string, start_char: number, end_char: number): string;
    /**
     * Render a table with only specific rows (for split-table pagination).
     *
     * `table_id_str` is the table node ID (e.g., "1:5").
     * `row_ids_json` is a JSON array of row node IDs to include (e.g., '["1:6","1:7"]').
     * `chunk_id` is a unique identifier for this chunk (used as data-node-id).
     * `is_continuation` indicates if this is a continuation chunk (for styling).
     */
    render_table_chunk(table_id_str: string, row_ids_json: string, chunk_id: string, is_continuation: boolean): string;
    /**
     * Replace all occurrences of query with replacement.
     *
     * Returns the number of replacements made. Single transaction.
     */
    replace_all(query: string, replacement: string, case_sensitive: boolean): number;
    /**
     * Replace text at a specific location.
     *
     * Note: insert_text into an existing text node inherits the parent run's
     * formatting (bold, italic, etc.) — no explicit attribute copy needed.
     */
    replace_text(node_id_str: string, offset: number, length: number, replacement: string): void;
    /**
     * Replace text in a paragraph by character range. Alias to `replace_text`.
     *
     * Preserves inline formatting (bold, italic, etc.) outside the modified range.
     */
    replace_text_range(node_id_str: string, start_offset: number, end_offset: number, replacement: string): void;
    /**
     * Resize an image by setting width/height attributes.
     */
    resize_image(image_id_str: string, width_pt: number, height_pt: number): void;
    /**
     * Set the resolved status of a comment.
     *
     * Persists the resolved state as a `CommentResolved` attribute on
     * the CommentBody node, so it survives save/load and collab sync.
     */
    resolve_comment(comment_id: string, resolved: boolean): void;
    /**
     * Return the scene protocol version supported by this build.
     */
    scene_protocol_version(): number;
    /**
     * Return a lightweight scene summary for viewport boot.
     *
     * Returns JSON: `{ "protocol_version": 1, "document_revision": N,
     * "layout_revision": N, "page_count": N, "default_page_size_pt": {...},
     * "pages": [...] }`
     */
    scene_summary(_config_json: string): string;
    /**
     * Return scene summary using loaded fonts for accurate text shaping.
     */
    scene_summary_with_fonts(font_db: WasmFontDatabase, config_json: string): string;
    /**
     * Search for text matches and return results with page rects.
     *
     * Wraps the existing `find_text` and enriches results with layout
     * position information when available.
     */
    search_matches(query: string, case_sensitive: boolean): string;
    /**
     * Return formatting state at a selection range for toolbar display.
     *
     * Returns JSON `FormattingState` with bold, italic, font info, etc.
     */
    selection_formatting(range_json: string): string;
    /**
     * Get selection rectangles for a model range.
     *
     * Returns JSON array of `RectPt` objects covering the selection.
     */
    selection_rects(range_json: string): string;
    /**
     * Set paragraph alignment ("left", "center", "right", "justify").
     */
    set_alignment(node_id_str: string, alignment: string): void;
    /**
     * Set the document author (metadata).
     */
    set_author(author: string): void;
    /**
     * Set bold on a paragraph's first run.
     *
     * For selection-aware formatting, use `format_selection()` or
     * `set_bold_range()` instead — they correctly handle mixed-format
     * paragraphs by splitting runs at selection boundaries.
     */
    set_bold(node_id_str: string, bold: boolean): void;
    /**
     * Set bold on a selection range. Preferred over `set_bold` for toolbar
     * actions when the user has an active text selection.
     */
    set_bold_range(start_node_str: string, start_offset: number, end_node_str: string, end_offset: number, bold: boolean): void;
    /**
     * Set the background color of a table cell.
     */
    set_cell_background(cell_id_str: string, hex: string): void;
    /**
     * Set the text content of a table cell.
     *
     * Replaces the entire cell content with the given text. Sets text in
     * the first paragraph and deletes any extra paragraphs.
     */
    set_cell_text(cell_id_str: string, text: string): void;
    /**
     * Set text color on a paragraph's first run (hex string like "FF0000").
     */
    set_color(node_id_str: string, hex: string): void;
    /**
     * Set text color on a selection range (hex string like "FF0000").
     */
    set_color_range(start_node_str: string, start_offset: number, end_node_str: string, end_offset: number, hex: string): void;
    /**
     * Set font family on a paragraph's first run.
     */
    set_font_family(node_id_str: string, font: string): void;
    /**
     * Set font family on a selection range.
     */
    set_font_family_range(start_node_str: string, start_offset: number, end_node_str: string, end_offset: number, font: string): void;
    /**
     * Set font size on a paragraph's first run (in points).
     */
    set_font_size(node_id_str: string, size_pt: number): void;
    /**
     * Set font size on a selection range (in points).
     */
    set_font_size_range(start_node_str: string, start_offset: number, end_node_str: string, end_offset: number, size_pt: number): void;
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
     */
    set_header_footer_text(section_index: number, hf_kind: string, hf_type_str: string, text: string): void;
    /**
     * Set the heading level of a paragraph.
     *
     * Level 0 removes the heading style (converts to normal paragraph).
     * Level 1-6 sets the corresponding heading style.
     */
    set_heading_level(node_id_str: string, level: number): void;
    /**
     * Set alt text on an image.
     */
    set_image_alt_text(image_id_str: string, alt: string): void;
    /**
     * Set image wrap mode.
     *
     * `mode` is one of: "inline", "wrapLeft", "wrapRight", "wrapBoth",
     * "topAndBottom", "behind", "inFront".
     * Defaults to "inline" if not set.
     */
    set_image_wrap_mode(image_id_str: string, mode: string): void;
    /**
     * Set paragraph indentation (left, right, or first-line).
     *
     * `indent_type` is one of: "left", "right", "firstLine".
     * `value_pt` is the indent value in points.
     */
    set_indent(node_id_str: string, indent_type: string, value_pt: number): void;
    /**
     * Set italic on a paragraph's first run.
     * For selection-aware formatting, use `set_italic_range` or `format_selection`.
     */
    set_italic(node_id_str: string, italic: boolean): void;
    /**
     * Set italic on a selection range.
     */
    set_italic_range(start_node_str: string, start_offset: number, end_node_str: string, end_offset: number, italic: boolean): void;
    /**
     * Set the line spacing for a paragraph.
     *
     * `spacing` is one of: "single", "1.5", "double", or a numeric multiplier (e.g. "1.15").
     */
    set_line_spacing(node_id_str: string, spacing: string): void;
    /**
     * Set list format on a paragraph.
     *
     * format: "bullet", "decimal", "none".
     */
    set_list_format(para_id_str: string, format: string, level: number): void;
    /**
     * Set page setup properties for all sections from JSON.
     *
     * Accepts JSON: `{"pageWidth":612,"pageHeight":792,"marginTop":72,
     * "marginBottom":72,"marginLeft":72,"marginRight":72,"orientation":"portrait"}`
     *
     * All dimensions are in points (1 inch = 72 points).
     * Updates all sections in the document to use the new page dimensions.
     */
    set_page_setup(json: string): void;
    /**
     * Set paragraph keep options (keep with next, keep lines together).
     *
     * `keep_type` is one of: "keepWithNext", "keepLinesTogether".
     * `enabled` controls whether the option is on or off.
     */
    set_paragraph_keep(node_id_str: string, keep_type: string, enabled: boolean): void;
    /**
     * Set paragraph spacing (before and/or after) in points.
     *
     * `spacing_type` is one of: "before", "after".
     * `value_pt` is the spacing value in points.
     */
    set_paragraph_spacing(node_id_str: string, spacing_type: string, value_pt: number): void;
    /**
     * Set the paragraph style ID on a paragraph node.
     *
     * Sets the `StyleId` attribute to any arbitrary style name
     * (e.g., "Title", "Subtitle", "Quote", "Code", "Heading1", etc.).
     * Pass an empty string to clear the style (revert to Normal).
     */
    set_paragraph_style_id(node_id_str: string, style_id: string): void;
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
     */
    set_paragraph_text(node_id_str: string, new_text: string): void;
    /**
     * Set the number of columns for a section.
     *
     * `section_index`: 0-based section index (0 for the default/first section).
     * `columns`: number of columns (1-6). Pass 1 for single-column layout.
     * `spacing_pt`: spacing between columns in points (default: 36.0 = 0.5in).
     */
    set_section_columns(section_index: number, columns: number, spacing_pt: number): void;
    /**
     * Set strikethrough on a paragraph's first run.
     */
    set_strikethrough(node_id_str: string, strikethrough: boolean): void;
    /**
     * Set column widths for a table. Widths should be in points (CSV string).
     */
    set_table_column_widths(table_id_str: string, widths_csv: string): void;
    /**
     * Set the document title (metadata).
     */
    set_title(title: string): void;
    /**
     * Set or clear the "different first page" flag for a section.
     *
     * When enabled, the first page of the section uses the "first" header/footer
     * instead of the "default" one.
     */
    set_title_page(section_index: number, enabled: boolean): void;
    /**
     * Enable or disable track changes mode.
     *
     * When enabled, subsequent text edits create revision markers.
     * This stores the state on the document metadata so it persists.
     */
    set_track_changes_enabled(enabled: boolean): void;
    /**
     * Set underline on a paragraph's first run.
     */
    set_underline(node_id_str: string, underline: boolean): void;
    /**
     * Set underline on a selection range.
     */
    set_underline_range(start_node_str: string, start_offset: number, end_node_str: string, end_offset: number, underline: boolean): void;
    /**
     * Set the maximum number of undo steps to keep.
     *
     * `max` of 0 means unlimited. Excess history is trimmed (oldest first).
     */
    set_undo_history_cap(max: number): void;
    /**
     * Sort a table by the text content of a specific column.
     *
     * Skips the first row (assumed header) if the table has more than one row.
     */
    sort_table_by_column(table_id_str: string, col_index: number, ascending: boolean): void;
    /**
     * Split a previously merged cell back to individual cells.
     *
     * Removes ColSpan/RowSpan attributes from the target cell and clears
     * the "continue" RowSpan from cells that were part of the merge.
     */
    split_merged_cell(table_id_str: string, row: number, col: number): void;
    /**
     * Split a paragraph at a character offset.
     *
     * Creates a new paragraph after the current one with the tail text.
     * If the original paragraph is a heading, the new paragraph inherits
     * the same heading style.
     *
     * Returns the new paragraph's node ID as "replica:counter".
     */
    split_paragraph(node_id_str: string, char_offset: number): string;
    /**
     * Split a Run node at a character offset.
     *
     * Creates a new Run after the original with the tail text, preserving
     * all formatting attributes. Returns the new run's node ID.
     */
    split_run(run_id_str: string, char_offset: number): string;
    /**
     * Check if password protection is available.
     *
     * Password protection requires server-side encryption (AES-256/AGILE).
     * Use the server API: `POST /api/documents/convert` with `format=docx&password=...`
     *
     * Returns false in WASM (server-side only feature).
     */
    supports_password_protection(): boolean;
    /**
     * Convert selected paragraphs to a table.
     *
     * Takes consecutive paragraphs and converts each into a table row.
     * Cells are split by `delimiter` ("tab", "comma", "semicolon", or "paragraph").
     * If delimiter is "paragraph", each paragraph becomes a single-cell row.
     *
     * Returns the new table node ID.
     */
    text_to_table(first_para_str: string, last_para_str: string, delimiter: string): string;
    /**
     * Convert to OnlyOffice DOCY binary format.
     *
     * Returns the wrapped DOCY payload string: `DOCY;v5;{size};{base64_data}`.
     *
     * This is currently a debug/export surface only. The current DOCY writer
     * is not yet structurally compatible with OnlyOffice for general open.
     */
    to_docy(): string;
    /**
     * Export the document as EPUB bytes.
     *
     * Generates an EPUB 3 file from the document content.
     * Returns the EPUB ZIP as a byte array.
     */
    to_epub(): Uint8Array;
    /**
     * Render the document as HTML with formatting, images, and hyperlinks.
     */
    to_html(): string;
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
     */
    to_layout_json(): string;
    /**
     * Render the document layout as structured JSON with a custom layout configuration.
     *
     * Use this to control page dimensions and margins.
     */
    to_layout_json_with_config(config: WasmLayoutConfig): string;
    /**
     * Render the document layout as structured JSON with loaded fonts.
     *
     * Use this when you have loaded fonts via `WasmFontDatabase` for
     * accurate text shaping and positioning.
     */
    to_layout_json_with_fonts(font_db: WasmFontDatabase): string;
    /**
     * Render the document layout as structured JSON with loaded fonts and custom config.
     *
     * Combines custom page dimensions/margins with loaded font data for
     * the most accurate canvas rendering.
     */
    to_layout_json_with_fonts_and_config(font_db: WasmFontDatabase, config: WasmLayoutConfig): string;
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
     */
    to_paginated_html(): string;
    /**
     * Render the document as paginated HTML with a custom layout configuration.
     *
     * Use this to control page dimensions and margins.
     */
    to_paginated_html_with_config(config: WasmLayoutConfig): string;
    /**
     * Render the document as paginated HTML with loaded fonts.
     *
     * Use this when you have loaded fonts via `WasmFontDatabase` for
     * accurate text shaping and positioning.
     */
    to_paginated_html_with_fonts(font_db: WasmFontDatabase): string;
    /**
     * Render the document as paginated HTML with loaded fonts and custom config.
     *
     * Combines custom page dimensions/margins with loaded font data for
     * the most accurate layout.
     */
    to_paginated_html_with_fonts_and_config(font_db: WasmFontDatabase, config: WasmLayoutConfig): string;
    /**
     * Export the document as PDF bytes.
     *
     * Uses fallback font metrics (no system fonts). For more accurate
     * output, use `to_pdf_with_fonts()` after loading fonts via
     * `WasmFontDatabase`.
     *
     * Returns the raw PDF bytes suitable for download or embedding.
     */
    to_pdf(): Uint8Array;
    /**
     * Export the document as PDF/A-1b bytes (ISO 19005 archival format).
     */
    to_pdf_a(): Uint8Array;
    /**
     * Export the document as a PDF/A data URL.
     */
    to_pdf_a_data_url(): string;
    /**
     * Export the document as a PDF data URL.
     *
     * Returns a string like `data:application/pdf;base64,...` suitable
     * for embedding in iframes, download links, or `window.open()`.
     */
    to_pdf_data_url(): string;
    /**
     * Export the document as a PDF data URL using loaded fonts.
     */
    to_pdf_data_url_with_fonts(font_db: WasmFontDatabase): string;
    /**
     * Export the document as PDF bytes using loaded fonts.
     *
     * Use this when you have loaded fonts via `WasmFontDatabase` for
     * accurate text shaping and glyph embedding.
     */
    to_pdf_with_fonts(font_db: WasmFontDatabase): Uint8Array;
    /**
     * Extract all text content as a plain string.
     */
    to_plain_text(): string;
    /**
     * Toggle a form checkbox's checked state.
     *
     * Returns the new checked state.
     */
    toggle_form_checkbox(node_id_str: string): boolean;
    /**
     * Get the number of tracked changes in the document.
     */
    tracked_changes_count(): number;
    /**
     * Get all tracked changes as a JSON array.
     *
     * Returns `[{"nodeId":"0:5","type":"Insert","author":"...","date":"..."},...]`
     */
    tracked_changes_json(): string;
    /**
     * Undo the last editing operation. Returns true if something was undone.
     */
    undo(): boolean;
    /**
     * Update the IME composition preview text.
     *
     * If a preview already exists, deletes it first, then inserts
     * the new preview text at the anchor.
     * Returns an EditResult JSON.
     */
    update_composition(text: string): string;
    /**
     * Update shape properties (position, size, fill, stroke).
     */
    update_shape(shape_id_str: string, width_pt: number, height_pt: number, fill_hex: string, stroke_hex: string): void;
    /**
     * Update all Table of Contents entries in the document.
     *
     * Rescans headings and regenerates TOC child paragraphs.
     */
    update_table_of_contents(): void;
    /**
     * Move a position in a direction by a granularity.
     *
     * Returns JSON `PositionRef`.
     * Validate and clamp a position to ensure the offset is within bounds.
     *
     * If the offset exceeds the text node's length, it is clamped to the end.
     * Returns the validated position as JSON.
     */
    validate_position(position_json: string): string;
    /**
     * Return scenes for a range of pages (batch fetch for viewport).
     *
     * Returns JSON: `{ "pages": [...] }`
     */
    visible_page_scenes(start_page: number, end_page: number): string;
    /**
     * Return visible page scenes using loaded fonts for accurate text shaping.
     */
    visible_page_scenes_with_fonts(font_db: WasmFontDatabase, start_page: number, end_page: number): string;
    /**
     * Get the word boundary around a position.
     *
     * Returns JSON `RangeRef` with anchor at word start and focus at word end.
     */
    word_boundary(position_json: string): string;
}

/**
 * A fluent builder for constructing documents.
 */
export class WasmDocumentBuilder {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Set the document author.
     */
    author(author: string): WasmDocumentBuilder;
    /**
     * Build the document. Consumes the builder.
     *
     * Returns an error if the document exceeds the maximum node count
     * limit (100,000 nodes) or the maximum depth limit (100) to prevent
     * OOM in the WASM environment.
     */
    build(): WasmDocument;
    /**
     * Add a heading at the specified level (1-6).
     */
    heading(level: number, text: string): WasmDocumentBuilder;
    /**
     * Create a new document builder.
     */
    constructor();
    /**
     * Add a paragraph with plain text.
     */
    text(text: string): WasmDocumentBuilder;
    /**
     * Set the document title.
     */
    title(title: string): WasmDocumentBuilder;
}

/**
 * The main entry point for s1engine in WASM.
 */
export class WasmEngine {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Create a new empty document.
     */
    create(): WasmDocument;
    /**
     * Create a new collaborative document.
     *
     * `replica_id` must be unique per user/session (e.g., random u64).
     */
    create_collab(replica_id: bigint): WasmCollabDocument;
    /**
     * Create a new engine instance.
     */
    constructor();
    /**
     * Open a document from bytes with auto-detected format.
     *
     * Supports DOCX, ODT, and TXT formats.
     */
    open(data: Uint8Array): WasmDocument;
    /**
     * Open a document from bytes with an explicit format.
     *
     * Format should be one of: "docx", "odt", "txt".
     */
    open_as(data: Uint8Array, format: string): WasmDocument;
    /**
     * Open a file as a collaborative document.
     *
     * The document is loaded and wrapped in a CRDT-aware container.
     */
    open_collab(data: Uint8Array, replica_id: bigint): WasmCollabDocument;
}

/**
 * A font database for WASM environments.
 *
 * Since WASM has no filesystem access, fonts must be loaded manually
 * via `load_font()`.
 */
export class WasmFontDatabase {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Get the number of loaded font faces.
     */
    font_count(): number;
    /**
     * Check if a font family is available (exact or via substitution).
     */
    has_font(family: string): boolean;
    /**
     * Load a font from raw bytes (TTF/OTF).
     */
    load_font(data: Uint8Array): void;
    /**
     * Create a new empty font database.
     */
    constructor();
    /**
     * Rasterize a single glyph to RGBA pixels.
     *
     * Returns a flat Uint8Array of RGBA pixels (width * height * 4 bytes)
     * plus metadata as JSON: `{"width":W,"height":H,"bearingX":X,"bearingY":Y,"advance":A}`
     *
     * This is the core API for canvas-first rendering — replaces `ctx.fillText()`.
     */
    rasterize_glyph(family: string, bold: boolean, italic: boolean, glyph_id: number, size_px: number, r: number, g: number, b: number): Uint8Array | undefined;
    /**
     * Rasterize a complete text run to RGBA pixels.
     *
     * Takes shaped glyph data (from layout engine) and produces a single
     * bitmap. Returns packed buffer: 8 bytes header (width u32, height u32)
     * followed by RGBA pixels.
     */
    rasterize_run(family: string, bold: boolean, italic: boolean, glyph_data: Uint8Array, size_px: number, r: number, g: number, b: number, total_width: number, line_height: number): Uint8Array | undefined;
}

/**
 * Configuration for paginated HTML layout.
 *
 * Controls page dimensions and margins for the layout engine.
 * Defaults to US Letter (8.5" x 11") with 1-inch margins.
 */
export class WasmLayoutConfig {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Get the bottom margin in points.
     */
    margin_bottom(): number;
    /**
     * Get the left margin in points.
     */
    margin_left(): number;
    /**
     * Get the right margin in points.
     */
    margin_right(): number;
    /**
     * Get the top margin in points.
     */
    margin_top(): number;
    /**
     * Create a new layout configuration with US Letter defaults.
     *
     * Page: 612pt x 792pt (8.5" x 11")
     * Margins: 72pt (1") on all sides.
     */
    constructor();
    /**
     * Get the page height in points.
     */
    page_height(): number;
    /**
     * Get the page width in points.
     */
    page_width(): number;
    /**
     * Set the bottom margin in points.
     */
    set_margin_bottom(margin: number): void;
    /**
     * Set the left margin in points.
     */
    set_margin_left(margin: number): void;
    /**
     * Set the right margin in points.
     */
    set_margin_right(margin: number): void;
    /**
     * Set the top margin in points.
     */
    set_margin_top(margin: number): void;
    /**
     * Set the page height in points.
     */
    set_page_height(height: number): void;
    /**
     * Set the page width in points.
     */
    set_page_width(width: number): void;
}

/**
 * PDF editor for reading, annotating, and modifying existing PDFs.
 */
export class WasmPdfEditor {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Add a free text annotation (text box).
     */
    add_freetext_annotation(page: number, x: number, y: number, width: number, height: number, text: string, font_size: number): void;
    /**
     * Add a highlight annotation (0-indexed page, quad points as flat array).
     */
    add_highlight_annotation(page: number, quads: Float64Array, r: number, g: number, b: number, author: string, content: string): void;
    /**
     * Add an ink (freehand) annotation. Points is a flat array [x1,y1,x2,y2,...].
     */
    add_ink_annotation(page: number, points: Float64Array, r: number, g: number, b: number, width: number): void;
    /**
     * Add a redaction annotation.
     */
    add_redaction(page: number, x: number, y: number, width: number, height: number): void;
    /**
     * Add a sticky note (text) annotation (0-indexed page).
     */
    add_text_annotation(page: number, x: number, y: number, author: string, content: string): void;
    /**
     * Add text overlay on a page at a given position (0-indexed).
     */
    add_text_overlay(page: number, x: number, y: number, width: number, height: number, text: string, font_size: number): void;
    /**
     * Add a white rectangle to cover content on a page (0-indexed).
     */
    add_white_rect(page: number, x: number, y: number, width: number, height: number): void;
    /**
     * Apply all redaction annotations — permanently removes content.
     */
    apply_redactions(): void;
    /**
     * Delete a page (0-indexed).
     */
    delete_page(page: number): void;
    /**
     * Duplicate a page (0-indexed).
     */
    duplicate_page(page: number): void;
    /**
     * Extract specified pages (0-indexed) into a new PDF.
     */
    extract_pages(pages: Uint32Array): Uint8Array;
    /**
     * Flatten the form.
     */
    flatten_form(): void;
    /**
     * Get all form fields as JSON.
     */
    get_form_fields(): string;
    /**
     * Merge another PDF's pages at the end of this document.
     */
    merge(other_data: Uint8Array): void;
    /**
     * Move a page from one position to another (0-indexed).
     */
    move_page(from: number, to: number): void;
    /**
     * Open a PDF from raw bytes.
     */
    static open(data: Uint8Array): WasmPdfEditor;
    /**
     * Get the number of pages.
     */
    page_count(): number;
    /**
     * Rotate a page by degrees (must be a multiple of 90).
     */
    rotate_page(page: number, degrees: number): void;
    /**
     * Save the modified PDF to bytes.
     */
    save(): Uint8Array;
    /**
     * Set a form field's value by name.
     */
    set_form_field_value(field_name: string, value: string): void;
}

/**
 * WASM bindings for spreadsheet operations (XLSX, ODS, CSV).
 *
 * Provides a JavaScript-friendly API for opening, editing, and exporting
 * spreadsheet files from the browser or Node.js.
 */
export class WasmSpreadsheet {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Add a new sheet with the given name.
     */
    add_sheet(name: string): void;
    /**
     * Delete a column and shift remaining columns left.
     */
    delete_column(sheet: number, col: number): void;
    /**
     * Delete a row and shift remaining rows up.
     */
    delete_row(sheet: number, row: number): void;
    /**
     * Delete a sheet by index.
     */
    delete_sheet(index: number): void;
    /**
     * Get dimensions (max col, max row) as JSON string: `"[cols,rows]"`.
     */
    dimensions(sheet: number): string;
    /**
     * Export a sheet as CSV string.
     */
    export_csv(sheet: number): string;
    /**
     * Export as ODS bytes.
     */
    export_ods(): Uint8Array;
    /**
     * Export as XLSX bytes.
     */
    export_xlsx(): Uint8Array;
    /**
     * Set or clear frozen panes on a sheet.
     *
     * Pass `col=0, row=0` to unfreeze.
     */
    freeze_panes(sheet: number, col: number, row: number): void;
    /**
     * Get cell value as string.
     *
     * Returns an empty string for empty or out-of-range cells.
     */
    get_cell(sheet: number, col: number, row: number): string;
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
     */
    get_visible_range_json(sheet: number, start_col: number, start_row: number, end_col: number, end_row: number): string;
    /**
     * Insert a column after the given column index.
     *
     * All columns at `after_col + 1` and beyond are shifted right.
     */
    insert_column(sheet: number, after_col: number): void;
    /**
     * Insert a row after the given row index.
     *
     * All rows at `after_row + 1` and below are shifted down.
     */
    insert_row(sheet: number, after_row: number): void;
    /**
     * Get merged cells as JSON array: `[{"start":"A1","end":"C3"}, ...]`.
     */
    merged_cells_json(sheet: number): string;
    /**
     * Create a new empty spreadsheet with one sheet.
     */
    constructor();
    /**
     * Open a spreadsheet from bytes (auto-detect XLSX, ODS, CSV).
     *
     * Detection is based on file magic bytes:
     * - XLSX/ODS: ZIP signature (PK header)
     * - CSV: plain text fallback
     */
    static open(data: Uint8Array): WasmSpreadsheet;
    /**
     * Recalculate all formulas in a sheet.
     */
    recalculate(sheet: number): void;
    /**
     * Rename a sheet by index.
     */
    rename_sheet(index: number, name: string): void;
    /**
     * Set cell value (auto-detect type: number, boolean, or text).
     */
    set_cell(sheet: number, col: number, row: number, value: string): void;
    /**
     * Set cell formula.
     */
    set_formula(sheet: number, col: number, row: number, formula: string): void;
    /**
     * Get the number of sheets.
     */
    sheet_count(): number;
    /**
     * Get sheet names as a JSON array.
     */
    sheet_names_json(): string;
    /**
     * Sort rows by a column value.
     *
     * Sorts all data rows in the sheet by the specified column.
     */
    sort_by_column(sheet: number, col: number, ascending: boolean): void;
}

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
 */
export function detect_file_type(data: Uint8Array): string;

/**
 * Detect the format of a document from its bytes.
 *
 * Returns one of: "docx", "odt", "pdf", "txt", "csv", "xlsx", "pptx", "ods", "odp", "doc".
 */
export function detect_format(data: Uint8Array): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_wasmcollabdocument_free: (a: number, b: number) => void;
    readonly __wbg_wasmdocument_free: (a: number, b: number) => void;
    readonly __wbg_wasmdocumentbuilder_free: (a: number, b: number) => void;
    readonly __wbg_wasmengine_free: (a: number, b: number) => void;
    readonly __wbg_wasmfontdatabase_free: (a: number, b: number) => void;
    readonly __wbg_wasmlayoutconfig_free: (a: number, b: number) => void;
    readonly __wbg_wasmpdfeditor_free: (a: number, b: number) => void;
    readonly __wbg_wasmspreadsheet_free: (a: number, b: number) => void;
    readonly detect_file_type: (a: number, b: number) => [number, number];
    readonly detect_format: (a: number, b: number) => [number, number];
    readonly wasmcollabdocument_append_paragraph: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmcollabdocument_apply_awareness_update: (a: number, b: number, c: number) => [number, number];
    readonly wasmcollabdocument_apply_local_delete_text: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly wasmcollabdocument_apply_local_format: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number, number, number];
    readonly wasmcollabdocument_apply_local_insert_text: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number, number];
    readonly wasmcollabdocument_apply_remote_ops: (a: number, b: number, c: number) => [number, number];
    readonly wasmcollabdocument_can_redo: (a: number) => number;
    readonly wasmcollabdocument_can_undo: (a: number) => number;
    readonly wasmcollabdocument_compact_op_log: (a: number) => [number, number];
    readonly wasmcollabdocument_delete_node: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmcollabdocument_delete_selection: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number, number, number];
    readonly wasmcollabdocument_export: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmcollabdocument_format_selection: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number) => [number, number, number, number];
    readonly wasmcollabdocument_free_doc: (a: number) => void;
    readonly wasmcollabdocument_get_changes_since: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmcollabdocument_get_formatting_json: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmcollabdocument_get_peers_json: (a: number) => [number, number, number, number];
    readonly wasmcollabdocument_get_state_vector: (a: number) => [number, number, number, number];
    readonly wasmcollabdocument_insert_horizontal_rule: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmcollabdocument_insert_page_break: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmcollabdocument_insert_paragraph_after: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly wasmcollabdocument_insert_table: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly wasmcollabdocument_insert_text_in_paragraph: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number, number];
    readonly wasmcollabdocument_merge_paragraphs: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly wasmcollabdocument_op_log_size: (a: number) => [number, number, number];
    readonly wasmcollabdocument_paragraph_ids_json: (a: number) => [number, number, number, number];
    readonly wasmcollabdocument_paste_plain_text: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number, number];
    readonly wasmcollabdocument_redo: (a: number) => [number, number, number, number];
    readonly wasmcollabdocument_render_node_html: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmcollabdocument_replica_id: (a: number) => [bigint, number, number];
    readonly wasmcollabdocument_set_alignment: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly wasmcollabdocument_set_cursor: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => [number, number, number, number];
    readonly wasmcollabdocument_set_heading_level: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly wasmcollabdocument_set_indent: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number, number];
    readonly wasmcollabdocument_set_line_spacing: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly wasmcollabdocument_set_list_format: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number, number];
    readonly wasmcollabdocument_set_paragraph_text: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly wasmcollabdocument_set_table_column_widths: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly wasmcollabdocument_sort_table_by_column: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly wasmcollabdocument_split_paragraph: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly wasmcollabdocument_to_html: (a: number) => [number, number, number, number];
    readonly wasmcollabdocument_to_plain_text: (a: number) => [number, number, number, number];
    readonly wasmcollabdocument_tombstone_count: (a: number) => [number, number, number];
    readonly wasmcollabdocument_undo: (a: number) => [number, number, number, number];
    readonly wasmdocument_accept_all_changes: (a: number) => [number, number];
    readonly wasmdocument_accept_change: (a: number, b: number, c: number) => [number, number];
    readonly wasmdocument_append_heading: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly wasmdocument_append_paragraph: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmdocument_apply_merge_data: (a: number, b: number, c: number, d: number) => [number, number, number];
    readonly wasmdocument_apply_table_style: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly wasmdocument_begin_batch: (a: number, b: number, c: number) => [number, number];
    readonly wasmdocument_begin_composition: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmdocument_body_children_json: (a: number) => [number, number, number, number];
    readonly wasmdocument_body_id: (a: number) => [number, number, number, number];
    readonly wasmdocument_can_redo: (a: number) => [number, number, number];
    readonly wasmdocument_can_undo: (a: number) => [number, number, number];
    readonly wasmdocument_cancel_composition: (a: number) => [number, number, number, number];
    readonly wasmdocument_canvas_delete_range: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmdocument_canvas_insert_paragraph_break: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmdocument_canvas_insert_text: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly wasmdocument_canvas_replace_range: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly wasmdocument_canvas_toggle_mark: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly wasmdocument_caret_rect: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmdocument_clear_history: (a: number) => [number, number];
    readonly wasmdocument_close: (a: number) => void;
    readonly wasmdocument_commit_composition: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmdocument_compare_with: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmdocument_copy_range_html: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmdocument_copy_range_plain_text: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmdocument_delete_comment: (a: number, b: number, c: number) => [number, number];
    readonly wasmdocument_delete_comment_reply: (a: number, b: number, c: number) => [number, number];
    readonly wasmdocument_delete_image: (a: number, b: number, c: number) => [number, number];
    readonly wasmdocument_delete_node: (a: number, b: number, c: number) => [number, number];
    readonly wasmdocument_delete_selection: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number];
    readonly wasmdocument_delete_table_column: (a: number, b: number, c: number, d: number) => [number, number];
    readonly wasmdocument_delete_table_row: (a: number, b: number, c: number, d: number) => [number, number];
    readonly wasmdocument_delete_text_in_paragraph: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly wasmdocument_document_revision: (a: number) => [number, number, number];
    readonly wasmdocument_edit_comment: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly wasmdocument_editor_capabilities: (a: number) => [number, number, number, number];
    readonly wasmdocument_end_batch: (a: number) => [number, number];
    readonly wasmdocument_export: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmdocument_export_selection_html: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number, number, number];
    readonly wasmdocument_fidelity_report_json: (a: number) => [number, number, number, number];
    readonly wasmdocument_find_text: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly wasmdocument_force_relayout: (a: number, b: number) => [number, number];
    readonly wasmdocument_format_run: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number];
    readonly wasmdocument_format_selection: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number) => [number, number];
    readonly wasmdocument_free: (a: number) => void;
    readonly wasmdocument_generate_bibliography: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmdocument_get_affected_pages: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmdocument_get_cell_id: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly wasmdocument_get_cell_text: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmdocument_get_comments_json: (a: number) => [number, number, number, number];
    readonly wasmdocument_get_document_text: (a: number) => [number, number, number, number];
    readonly wasmdocument_get_endnotes_json: (a: number) => [number, number, number, number];
    readonly wasmdocument_get_footnotes_json: (a: number) => [number, number, number, number];
    readonly wasmdocument_get_formatting_json: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmdocument_get_header_footer_info: (a: number, b: number) => [number, number, number, number];
    readonly wasmdocument_get_headings_json: (a: number) => [number, number, number, number];
    readonly wasmdocument_get_image_data_url: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmdocument_get_image_wrap_mode: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmdocument_get_layout_cache_stats: (a: number) => [number, number, number, number];
    readonly wasmdocument_get_page_count: (a: number) => [number, number, number];
    readonly wasmdocument_get_page_count_with_fonts: (a: number, b: number) => [number, number, number];
    readonly wasmdocument_get_page_html: (a: number, b: number) => [number, number, number, number];
    readonly wasmdocument_get_page_html_with_fonts: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmdocument_get_page_map_json: (a: number) => [number, number, number, number];
    readonly wasmdocument_get_page_map_json_with_fonts: (a: number, b: number) => [number, number, number, number];
    readonly wasmdocument_get_page_setup_json: (a: number) => [number, number, number, number];
    readonly wasmdocument_get_paragraph_text: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmdocument_get_reference_targets_json: (a: number) => [number, number, number, number];
    readonly wasmdocument_get_run_formatting_json: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmdocument_get_run_ids: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmdocument_get_run_text: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmdocument_get_section_breaks_json: (a: number) => [number, number, number, number];
    readonly wasmdocument_get_section_columns: (a: number, b: number) => [number, number, number, number];
    readonly wasmdocument_get_sections_json: (a: number) => [number, number, number, number];
    readonly wasmdocument_get_selection_formatting_json: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number, number, number];
    readonly wasmdocument_get_table_dimensions: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmdocument_get_used_fonts: (a: number) => [number, number, number, number];
    readonly wasmdocument_hit_test: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly wasmdocument_insert_bookmark: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly wasmdocument_insert_caption: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number, number, number];
    readonly wasmdocument_insert_citation: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly wasmdocument_insert_column_break: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmdocument_insert_comment: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number) => [number, number, number, number];
    readonly wasmdocument_insert_comment_at_range: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number) => [number, number, number, number];
    readonly wasmdocument_insert_comment_reply: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number, number, number];
    readonly wasmdocument_insert_cross_reference: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number) => [number, number, number, number];
    readonly wasmdocument_insert_endnote: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly wasmdocument_insert_equation: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly wasmdocument_insert_footnote: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly wasmdocument_insert_horizontal_rule: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmdocument_insert_hyperlink: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number];
    readonly wasmdocument_insert_image: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number) => [number, number, number, number];
    readonly wasmdocument_insert_line_break: (a: number, b: number, c: number, d: number) => [number, number];
    readonly wasmdocument_insert_merge_field: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly wasmdocument_insert_page_break: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmdocument_insert_paragraph_after: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly wasmdocument_insert_section_break: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly wasmdocument_insert_seq_field: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly wasmdocument_insert_shape: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number, m: number, n: number) => [number, number, number, number];
    readonly wasmdocument_insert_tab: (a: number, b: number, c: number, d: number) => [number, number];
    readonly wasmdocument_insert_table: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly wasmdocument_insert_table_column: (a: number, b: number, c: number, d: number) => [number, number];
    readonly wasmdocument_insert_table_of_contents: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number, number];
    readonly wasmdocument_insert_table_of_figures: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly wasmdocument_insert_table_row: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly wasmdocument_insert_text_in_paragraph: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number];
    readonly wasmdocument_is_batching: (a: number) => number;
    readonly wasmdocument_is_layout_dirty: (a: number) => number;
    readonly wasmdocument_is_track_changes_enabled: (a: number) => [number, number, number];
    readonly wasmdocument_is_valid: (a: number) => number;
    readonly wasmdocument_latex_to_omml: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmdocument_layout_revision: (a: number) => [number, number, number];
    readonly wasmdocument_layout_single_page_json: (a: number, b: number) => [number, number, number, number];
    readonly wasmdocument_line_boundary: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly wasmdocument_merge_cells: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number];
    readonly wasmdocument_merge_paragraphs: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly wasmdocument_metadata_author: (a: number) => [number, number, number, number];
    readonly wasmdocument_metadata_json: (a: number) => [number, number, number, number];
    readonly wasmdocument_metadata_title: (a: number) => [number, number, number, number];
    readonly wasmdocument_move_node_after: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly wasmdocument_move_node_before: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly wasmdocument_move_position: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number, number, number];
    readonly wasmdocument_move_range: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => [number, number, number, number];
    readonly wasmdocument_multi_cursor_delete: (a: number, b: number, c: number) => [number, number];
    readonly wasmdocument_multi_cursor_insert: (a: number, b: number, c: number) => [number, number];
    readonly wasmdocument_node_bounds: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmdocument_node_info_json: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmdocument_omml_to_latex: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmdocument_page_scene: (a: number, b: number) => [number, number, number, number];
    readonly wasmdocument_page_scene_with_fonts: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmdocument_paragraph_count: (a: number) => [number, number, number];
    readonly wasmdocument_paragraph_ids_json: (a: number) => [number, number, number, number];
    readonly wasmdocument_paste_formatted_runs_json: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number];
    readonly wasmdocument_paste_html: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly wasmdocument_paste_plain_text: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number];
    readonly wasmdocument_redo: (a: number) => [number, number, number];
    readonly wasmdocument_reject_all_changes: (a: number) => [number, number];
    readonly wasmdocument_reject_change: (a: number, b: number, c: number) => [number, number];
    readonly wasmdocument_remove_hyperlink: (a: number, b: number, c: number) => [number, number];
    readonly wasmdocument_render_node_html: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmdocument_render_node_slice: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly wasmdocument_render_table_chunk: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => [number, number, number, number];
    readonly wasmdocument_replace_all: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number];
    readonly wasmdocument_replace_text: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number];
    readonly wasmdocument_replace_text_range: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number];
    readonly wasmdocument_resize_image: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly wasmdocument_resolve_comment: (a: number, b: number, c: number, d: number) => [number, number];
    readonly wasmdocument_scene_protocol_version: (a: number) => number;
    readonly wasmdocument_scene_summary: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmdocument_scene_summary_with_fonts: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly wasmdocument_search_matches: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly wasmdocument_selection_formatting: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmdocument_selection_rects: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmdocument_set_alignment: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly wasmdocument_set_author: (a: number, b: number, c: number) => [number, number];
    readonly wasmdocument_set_bold: (a: number, b: number, c: number, d: number) => [number, number];
    readonly wasmdocument_set_bold_range: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => [number, number];
    readonly wasmdocument_set_cell_background: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly wasmdocument_set_cell_text: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly wasmdocument_set_color: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly wasmdocument_set_color_range: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number) => [number, number];
    readonly wasmdocument_set_font_family: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly wasmdocument_set_font_family_range: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number) => [number, number];
    readonly wasmdocument_set_font_size: (a: number, b: number, c: number, d: number) => [number, number];
    readonly wasmdocument_set_font_size_range: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => [number, number];
    readonly wasmdocument_set_header_footer_text: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => [number, number];
    readonly wasmdocument_set_heading_level: (a: number, b: number, c: number, d: number) => [number, number];
    readonly wasmdocument_set_image_alt_text: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly wasmdocument_set_image_wrap_mode: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly wasmdocument_set_indent: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number];
    readonly wasmdocument_set_italic: (a: number, b: number, c: number, d: number) => [number, number];
    readonly wasmdocument_set_italic_range: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => [number, number];
    readonly wasmdocument_set_line_spacing: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly wasmdocument_set_list_format: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number];
    readonly wasmdocument_set_page_setup: (a: number, b: number, c: number) => [number, number];
    readonly wasmdocument_set_paragraph_keep: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number];
    readonly wasmdocument_set_paragraph_spacing: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number];
    readonly wasmdocument_set_paragraph_style_id: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly wasmdocument_set_paragraph_text: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly wasmdocument_set_section_columns: (a: number, b: number, c: number, d: number) => [number, number];
    readonly wasmdocument_set_strikethrough: (a: number, b: number, c: number, d: number) => [number, number];
    readonly wasmdocument_set_table_column_widths: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly wasmdocument_set_title: (a: number, b: number, c: number) => [number, number];
    readonly wasmdocument_set_title_page: (a: number, b: number, c: number) => [number, number];
    readonly wasmdocument_set_track_changes_enabled: (a: number, b: number) => [number, number];
    readonly wasmdocument_set_underline: (a: number, b: number, c: number, d: number) => [number, number];
    readonly wasmdocument_set_underline_range: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => [number, number];
    readonly wasmdocument_set_undo_history_cap: (a: number, b: number) => [number, number];
    readonly wasmdocument_sort_table_by_column: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly wasmdocument_split_merged_cell: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly wasmdocument_split_paragraph: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly wasmdocument_split_run: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly wasmdocument_supports_password_protection: (a: number) => number;
    readonly wasmdocument_text_to_table: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number, number, number];
    readonly wasmdocument_to_docy: (a: number) => [number, number, number, number];
    readonly wasmdocument_to_epub: (a: number) => [number, number, number, number];
    readonly wasmdocument_to_html: (a: number) => [number, number, number, number];
    readonly wasmdocument_to_layout_json: (a: number) => [number, number, number, number];
    readonly wasmdocument_to_layout_json_with_config: (a: number, b: number) => [number, number, number, number];
    readonly wasmdocument_to_layout_json_with_fonts: (a: number, b: number) => [number, number, number, number];
    readonly wasmdocument_to_layout_json_with_fonts_and_config: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmdocument_to_paginated_html: (a: number) => [number, number, number, number];
    readonly wasmdocument_to_paginated_html_with_config: (a: number, b: number) => [number, number, number, number];
    readonly wasmdocument_to_paginated_html_with_fonts: (a: number, b: number) => [number, number, number, number];
    readonly wasmdocument_to_paginated_html_with_fonts_and_config: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmdocument_to_pdf: (a: number) => [number, number, number, number];
    readonly wasmdocument_to_pdf_a: (a: number) => [number, number, number, number];
    readonly wasmdocument_to_pdf_a_data_url: (a: number) => [number, number, number, number];
    readonly wasmdocument_to_pdf_data_url: (a: number) => [number, number, number, number];
    readonly wasmdocument_to_pdf_data_url_with_fonts: (a: number, b: number) => [number, number, number, number];
    readonly wasmdocument_to_pdf_with_fonts: (a: number, b: number) => [number, number, number, number];
    readonly wasmdocument_to_plain_text: (a: number) => [number, number, number, number];
    readonly wasmdocument_toggle_form_checkbox: (a: number, b: number, c: number) => [number, number, number];
    readonly wasmdocument_tracked_changes_count: (a: number) => [number, number, number];
    readonly wasmdocument_tracked_changes_json: (a: number) => [number, number, number, number];
    readonly wasmdocument_undo: (a: number) => [number, number, number];
    readonly wasmdocument_update_composition: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmdocument_update_shape: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number) => [number, number];
    readonly wasmdocument_update_table_of_contents: (a: number) => [number, number];
    readonly wasmdocument_validate_position: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmdocument_visible_page_scenes: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmdocument_visible_page_scenes_with_fonts: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly wasmdocument_word_boundary: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmdocumentbuilder_author: (a: number, b: number, c: number) => number;
    readonly wasmdocumentbuilder_build: (a: number) => [number, number, number];
    readonly wasmdocumentbuilder_heading: (a: number, b: number, c: number, d: number) => number;
    readonly wasmdocumentbuilder_new: () => number;
    readonly wasmdocumentbuilder_text: (a: number, b: number, c: number) => number;
    readonly wasmdocumentbuilder_title: (a: number, b: number, c: number) => number;
    readonly wasmengine_create: (a: number) => number;
    readonly wasmengine_create_collab: (a: number, b: bigint) => number;
    readonly wasmengine_new: () => number;
    readonly wasmengine_open: (a: number, b: number, c: number) => [number, number, number];
    readonly wasmengine_open_as: (a: number, b: number, c: number, d: number, e: number) => [number, number, number];
    readonly wasmengine_open_collab: (a: number, b: number, c: number, d: bigint) => [number, number, number];
    readonly wasmfontdatabase_font_count: (a: number) => number;
    readonly wasmfontdatabase_has_font: (a: number, b: number, c: number) => number;
    readonly wasmfontdatabase_load_font: (a: number, b: number, c: number) => void;
    readonly wasmfontdatabase_new: () => number;
    readonly wasmfontdatabase_rasterize_glyph: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number) => [number, number];
    readonly wasmfontdatabase_rasterize_run: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number, m: number) => [number, number];
    readonly wasmlayoutconfig_margin_bottom: (a: number) => number;
    readonly wasmlayoutconfig_margin_left: (a: number) => number;
    readonly wasmlayoutconfig_margin_right: (a: number) => number;
    readonly wasmlayoutconfig_margin_top: (a: number) => number;
    readonly wasmlayoutconfig_new: () => number;
    readonly wasmlayoutconfig_page_height: (a: number) => number;
    readonly wasmlayoutconfig_page_width: (a: number) => number;
    readonly wasmlayoutconfig_set_margin_bottom: (a: number, b: number) => void;
    readonly wasmlayoutconfig_set_margin_left: (a: number, b: number) => void;
    readonly wasmlayoutconfig_set_margin_right: (a: number, b: number) => void;
    readonly wasmlayoutconfig_set_margin_top: (a: number, b: number) => void;
    readonly wasmlayoutconfig_set_page_height: (a: number, b: number) => void;
    readonly wasmlayoutconfig_set_page_width: (a: number, b: number) => void;
    readonly wasmpdfeditor_add_freetext_annotation: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number) => [number, number];
    readonly wasmpdfeditor_add_highlight_annotation: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number) => [number, number];
    readonly wasmpdfeditor_add_ink_annotation: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => [number, number];
    readonly wasmpdfeditor_add_redaction: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number];
    readonly wasmpdfeditor_add_text_annotation: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => [number, number];
    readonly wasmpdfeditor_add_text_overlay: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number) => [number, number];
    readonly wasmpdfeditor_add_white_rect: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number];
    readonly wasmpdfeditor_apply_redactions: (a: number) => [number, number];
    readonly wasmpdfeditor_delete_page: (a: number, b: number) => [number, number];
    readonly wasmpdfeditor_duplicate_page: (a: number, b: number) => [number, number];
    readonly wasmpdfeditor_extract_pages: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmpdfeditor_flatten_form: (a: number) => [number, number];
    readonly wasmpdfeditor_get_form_fields: (a: number) => [number, number, number, number];
    readonly wasmpdfeditor_merge: (a: number, b: number, c: number) => [number, number];
    readonly wasmpdfeditor_move_page: (a: number, b: number, c: number) => [number, number];
    readonly wasmpdfeditor_open: (a: number, b: number) => [number, number, number];
    readonly wasmpdfeditor_page_count: (a: number) => number;
    readonly wasmpdfeditor_rotate_page: (a: number, b: number, c: number) => [number, number];
    readonly wasmpdfeditor_save: (a: number) => [number, number, number, number];
    readonly wasmpdfeditor_set_form_field_value: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly wasmspreadsheet_add_sheet: (a: number, b: number, c: number) => void;
    readonly wasmspreadsheet_delete_column: (a: number, b: number, c: number) => void;
    readonly wasmspreadsheet_delete_row: (a: number, b: number, c: number) => void;
    readonly wasmspreadsheet_delete_sheet: (a: number, b: number) => void;
    readonly wasmspreadsheet_dimensions: (a: number, b: number) => [number, number];
    readonly wasmspreadsheet_export_csv: (a: number, b: number) => [number, number];
    readonly wasmspreadsheet_export_ods: (a: number) => [number, number, number, number];
    readonly wasmspreadsheet_export_xlsx: (a: number) => [number, number, number, number];
    readonly wasmspreadsheet_freeze_panes: (a: number, b: number, c: number, d: number) => void;
    readonly wasmspreadsheet_get_cell: (a: number, b: number, c: number, d: number) => [number, number];
    readonly wasmspreadsheet_get_visible_range_json: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number];
    readonly wasmspreadsheet_insert_column: (a: number, b: number, c: number) => void;
    readonly wasmspreadsheet_insert_row: (a: number, b: number, c: number) => void;
    readonly wasmspreadsheet_merged_cells_json: (a: number, b: number) => [number, number];
    readonly wasmspreadsheet_new: () => number;
    readonly wasmspreadsheet_open: (a: number, b: number) => [number, number, number];
    readonly wasmspreadsheet_recalculate: (a: number, b: number) => void;
    readonly wasmspreadsheet_rename_sheet: (a: number, b: number, c: number, d: number) => void;
    readonly wasmspreadsheet_set_cell: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
    readonly wasmspreadsheet_set_formula: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
    readonly wasmspreadsheet_sheet_count: (a: number) => number;
    readonly wasmspreadsheet_sheet_names_json: (a: number) => [number, number];
    readonly wasmspreadsheet_sort_by_column: (a: number, b: number, c: number, d: number) => void;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
