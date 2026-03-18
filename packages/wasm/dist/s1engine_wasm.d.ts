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
     * Export the collaborative document to a format (docx, odt, txt, md).
     */
    export(format: string): Uint8Array;
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
     * Get the size of the operation log.
     */
    op_log_size(): number;
    /**
     * Redo the last undone operation.
     */
    redo(): string;
    /**
     * Get the replica ID of this collaborative document.
     */
    replica_id(): bigint;
    /**
     * Set the local cursor position and return an awareness update for broadcast.
     */
    set_cursor(node_id: string, offset: number, user_name: string, user_color: string): string;
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
     * Begin a batch of operations that form a single undo step.
     *
     * All operations between `begin_batch()` and `end_batch()` are applied
     * individually. On `end_batch()`, they are merged into a single undo
     * unit by collapsing the undo history.
     */
    begin_batch(label: string): void;
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
     * Clear all undo/redo history.
     */
    clear_history(): void;
    /**
     * Explicitly release document memory. The document cannot be used after this.
     */
    close(): void;
    /**
     * Delete a comment and its range markers.
     */
    delete_comment(comment_id: string): void;
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
     * Find all occurrences of text in the document.
     *
     * Returns JSON array of `{"nodeId":"0:5","offset":3,"length":5}`.
     */
    find_text(query: string, case_sensitive: boolean): string;
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
     * Get page break information from the layout engine as JSON.
     *
     * Returns `{"pages": [{"pageNum":1, "nodeIds":["0:5","0:12"], "footer":"Page 1", "header":"..."}, ...]}`.
     * This tells the editor which node IDs are on which page, so the editor
     * can show visual page breaks matching the actual layout engine output.
     */
    get_page_map_json(): string;
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
     * Insert bookmark start/end around a paragraph.
     *
     * Returns the bookmark start node ID.
     */
    insert_bookmark(para_id_str: string, name: string): string;
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
     * Insert a tab node at the given character offset within a paragraph.
     *
     * Like `insert_line_break`, this inserts a `Tab` node inside the
     * appropriate run, splitting text nodes as needed. Tab nodes render
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
     * Check if this document handle is still valid.
     */
    is_valid(): boolean;
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
    /**
     * Get detailed info about a node as JSON.
     *
     * Returns `{"id":"0:5","type":"Paragraph","text":"Hello","children":[...],...}`
     */
    node_info_json(node_id_str: string): string;
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
     */
    replace_text(node_id_str: string, offset: number, length: number, replacement: string): void;
    /**
     * Resize an image by setting width/height attributes.
     */
    resize_image(image_id_str: string, width_pt: number, height_pt: number): void;
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
     */
    set_bold(node_id_str: string, bold: boolean): void;
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
     * Set font family on a paragraph's first run.
     */
    set_font_family(node_id_str: string, font: string): void;
    /**
     * Set font size on a paragraph's first run (in points).
     */
    set_font_size(node_id_str: string, size_pt: number): void;
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
     */
    set_italic(node_id_str: string, italic: boolean): void;
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
     * Replace the text content of a paragraph.
     *
     * For multi-run paragraphs, this first checks whether the total text
     * across all runs already matches `new_text`.  If so, it is a no-op
     * (preserving per-run formatting).  If the text has genuinely changed,
     * all extra runs are deleted and the remaining single run receives the
     * new text.
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
     * Set underline on a paragraph's first run.
     */
    set_underline(node_id_str: string, underline: boolean): void;
    /**
     * Set the maximum number of undo steps to keep.
     *
     * `max` of 0 means unlimited. Excess history is trimmed (oldest first).
     */
    set_undo_history_cap(max: number): void;
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
     * Update all Table of Contents entries in the document.
     *
     * Rescans headings and regenerates TOC child paragraphs.
     */
    update_table_of_contents(): void;
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
 * Detect the format of a document from its bytes.
 *
 * Returns one of: "docx", "odt", "pdf", "txt".
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
    readonly detect_format: (a: number, b: number) => [number, number];
    readonly wasmcollabdocument_apply_awareness_update: (a: number, b: number, c: number) => [number, number];
    readonly wasmcollabdocument_apply_local_delete_text: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly wasmcollabdocument_apply_local_format: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number, number, number];
    readonly wasmcollabdocument_apply_local_insert_text: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number, number];
    readonly wasmcollabdocument_apply_remote_ops: (a: number, b: number, c: number) => [number, number];
    readonly wasmcollabdocument_can_redo: (a: number) => number;
    readonly wasmcollabdocument_can_undo: (a: number) => number;
    readonly wasmcollabdocument_compact_op_log: (a: number) => [number, number];
    readonly wasmcollabdocument_export: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmcollabdocument_free_doc: (a: number) => void;
    readonly wasmcollabdocument_get_changes_since: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmcollabdocument_get_peers_json: (a: number) => [number, number, number, number];
    readonly wasmcollabdocument_get_state_vector: (a: number) => [number, number, number, number];
    readonly wasmcollabdocument_op_log_size: (a: number) => [number, number, number];
    readonly wasmcollabdocument_redo: (a: number) => [number, number, number, number];
    readonly wasmcollabdocument_replica_id: (a: number) => [bigint, number, number];
    readonly wasmcollabdocument_set_cursor: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => [number, number, number, number];
    readonly wasmcollabdocument_to_html: (a: number) => [number, number, number, number];
    readonly wasmcollabdocument_to_plain_text: (a: number) => [number, number, number, number];
    readonly wasmcollabdocument_tombstone_count: (a: number) => [number, number, number];
    readonly wasmcollabdocument_undo: (a: number) => [number, number, number, number];
    readonly wasmdocument_accept_all_changes: (a: number) => [number, number];
    readonly wasmdocument_accept_change: (a: number, b: number, c: number) => [number, number];
    readonly wasmdocument_append_heading: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly wasmdocument_append_paragraph: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmdocument_begin_batch: (a: number, b: number, c: number) => [number, number];
    readonly wasmdocument_body_children_json: (a: number) => [number, number, number, number];
    readonly wasmdocument_body_id: (a: number) => [number, number, number, number];
    readonly wasmdocument_can_redo: (a: number) => [number, number, number];
    readonly wasmdocument_can_undo: (a: number) => [number, number, number];
    readonly wasmdocument_clear_history: (a: number) => [number, number];
    readonly wasmdocument_close: (a: number) => void;
    readonly wasmdocument_delete_comment: (a: number, b: number, c: number) => [number, number];
    readonly wasmdocument_delete_image: (a: number, b: number, c: number) => [number, number];
    readonly wasmdocument_delete_node: (a: number, b: number, c: number) => [number, number];
    readonly wasmdocument_delete_selection: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number];
    readonly wasmdocument_delete_table_column: (a: number, b: number, c: number, d: number) => [number, number];
    readonly wasmdocument_delete_table_row: (a: number, b: number, c: number, d: number) => [number, number];
    readonly wasmdocument_delete_text_in_paragraph: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly wasmdocument_end_batch: (a: number) => [number, number];
    readonly wasmdocument_export: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmdocument_export_selection_html: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number, number, number];
    readonly wasmdocument_find_text: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly wasmdocument_format_run: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number];
    readonly wasmdocument_format_selection: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number) => [number, number];
    readonly wasmdocument_free: (a: number) => void;
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
    readonly wasmdocument_get_page_map_json: (a: number) => [number, number, number, number];
    readonly wasmdocument_get_page_setup_json: (a: number) => [number, number, number, number];
    readonly wasmdocument_get_paragraph_text: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmdocument_get_run_formatting_json: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmdocument_get_run_ids: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmdocument_get_run_text: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmdocument_get_section_breaks_json: (a: number) => [number, number, number, number];
    readonly wasmdocument_get_section_columns: (a: number, b: number) => [number, number, number, number];
    readonly wasmdocument_get_sections_json: (a: number) => [number, number, number, number];
    readonly wasmdocument_get_selection_formatting_json: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number, number, number];
    readonly wasmdocument_get_table_dimensions: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmdocument_get_used_fonts: (a: number) => [number, number, number, number];
    readonly wasmdocument_insert_bookmark: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly wasmdocument_insert_column_break: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmdocument_insert_comment: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number) => [number, number, number, number];
    readonly wasmdocument_insert_comment_at_range: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number) => [number, number, number, number];
    readonly wasmdocument_insert_comment_reply: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number, number, number];
    readonly wasmdocument_insert_endnote: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly wasmdocument_insert_equation: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly wasmdocument_insert_footnote: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly wasmdocument_insert_horizontal_rule: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmdocument_insert_hyperlink: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number];
    readonly wasmdocument_insert_image: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number) => [number, number, number, number];
    readonly wasmdocument_insert_line_break: (a: number, b: number, c: number, d: number) => [number, number];
    readonly wasmdocument_insert_page_break: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmdocument_insert_paragraph_after: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly wasmdocument_insert_section_break: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly wasmdocument_insert_tab: (a: number, b: number, c: number, d: number) => [number, number];
    readonly wasmdocument_insert_table: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly wasmdocument_insert_table_column: (a: number, b: number, c: number, d: number) => [number, number];
    readonly wasmdocument_insert_table_of_contents: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number, number];
    readonly wasmdocument_insert_table_row: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly wasmdocument_insert_text_in_paragraph: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number];
    readonly wasmdocument_is_batching: (a: number) => number;
    readonly wasmdocument_is_valid: (a: number) => number;
    readonly wasmdocument_merge_cells: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number];
    readonly wasmdocument_merge_paragraphs: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly wasmdocument_metadata_author: (a: number) => [number, number, number, number];
    readonly wasmdocument_metadata_title: (a: number) => [number, number, number, number];
    readonly wasmdocument_move_node_after: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly wasmdocument_move_node_before: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly wasmdocument_node_info_json: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmdocument_paragraph_count: (a: number) => [number, number, number];
    readonly wasmdocument_paragraph_ids_json: (a: number) => [number, number, number, number];
    readonly wasmdocument_paste_formatted_runs_json: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number];
    readonly wasmdocument_paste_plain_text: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number];
    readonly wasmdocument_redo: (a: number) => [number, number, number];
    readonly wasmdocument_reject_all_changes: (a: number) => [number, number];
    readonly wasmdocument_reject_change: (a: number, b: number, c: number) => [number, number];
    readonly wasmdocument_remove_hyperlink: (a: number, b: number, c: number) => [number, number];
    readonly wasmdocument_render_node_html: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmdocument_render_table_chunk: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => [number, number, number, number];
    readonly wasmdocument_replace_all: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number];
    readonly wasmdocument_replace_text: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number];
    readonly wasmdocument_resize_image: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly wasmdocument_set_alignment: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly wasmdocument_set_author: (a: number, b: number, c: number) => [number, number];
    readonly wasmdocument_set_bold: (a: number, b: number, c: number, d: number) => [number, number];
    readonly wasmdocument_set_cell_background: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly wasmdocument_set_cell_text: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly wasmdocument_set_color: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly wasmdocument_set_font_family: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly wasmdocument_set_font_size: (a: number, b: number, c: number, d: number) => [number, number];
    readonly wasmdocument_set_header_footer_text: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => [number, number];
    readonly wasmdocument_set_heading_level: (a: number, b: number, c: number, d: number) => [number, number];
    readonly wasmdocument_set_image_alt_text: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly wasmdocument_set_image_wrap_mode: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly wasmdocument_set_indent: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number];
    readonly wasmdocument_set_italic: (a: number, b: number, c: number, d: number) => [number, number];
    readonly wasmdocument_set_line_spacing: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly wasmdocument_set_list_format: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number];
    readonly wasmdocument_set_page_setup: (a: number, b: number, c: number) => [number, number];
    readonly wasmdocument_set_paragraph_keep: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number];
    readonly wasmdocument_set_paragraph_spacing: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number];
    readonly wasmdocument_set_paragraph_style_id: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly wasmdocument_set_paragraph_text: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly wasmdocument_set_section_columns: (a: number, b: number, c: number, d: number) => [number, number];
    readonly wasmdocument_set_strikethrough: (a: number, b: number, c: number, d: number) => [number, number];
    readonly wasmdocument_set_title: (a: number, b: number, c: number) => [number, number];
    readonly wasmdocument_set_title_page: (a: number, b: number, c: number) => [number, number];
    readonly wasmdocument_set_underline: (a: number, b: number, c: number, d: number) => [number, number];
    readonly wasmdocument_set_undo_history_cap: (a: number, b: number) => [number, number];
    readonly wasmdocument_split_merged_cell: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly wasmdocument_split_paragraph: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly wasmdocument_split_run: (a: number, b: number, c: number, d: number) => [number, number, number, number];
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
    readonly wasmdocument_to_pdf_data_url: (a: number) => [number, number, number, number];
    readonly wasmdocument_to_pdf_data_url_with_fonts: (a: number, b: number) => [number, number, number, number];
    readonly wasmdocument_to_pdf_with_fonts: (a: number, b: number) => [number, number, number, number];
    readonly wasmdocument_to_plain_text: (a: number) => [number, number, number, number];
    readonly wasmdocument_tracked_changes_count: (a: number) => [number, number, number];
    readonly wasmdocument_tracked_changes_json: (a: number) => [number, number, number, number];
    readonly wasmdocument_undo: (a: number) => [number, number, number];
    readonly wasmdocument_update_table_of_contents: (a: number) => [number, number];
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
