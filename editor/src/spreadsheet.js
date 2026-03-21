// ─── Spreadsheet Grid ──────────────────────────────────
// Canvas-based virtual scrolling grid for XLSX/CSV editing.
// Renders only visible cells + buffer for 60fps performance.

import { state, $ } from './state.js';

const CELL_WIDTH = 100;      // Default column width in px
const CELL_HEIGHT = 24;      // Default row height in px
const HEADER_HEIGHT = 24;    // Column header (A, B, C...)
const ROW_HEADER_WIDTH = 50; // Row header (1, 2, 3...)
const BUFFER_CELLS = 5;      // Extra cells rendered beyond viewport
const MAX_COLS = 16384;       // Max columns (XFD)
const MAX_ROWS = 1048576;     // Max rows (Excel limit)
const FILL_HANDLE_SIZE = 6;  // Drag fill handle square size in px

// ─── Workbook data model (in-memory) ───────────────────
class Sheet {
    constructor(name) {
        this.name = name;
        this.cells = {};         // 'col,row' -> { value, formula, display, type, style }
        this.colWidths = {};     // col index -> px
        this.rowHeights = {};    // row index -> px
        this.merges = [];        // [{ startCol, startRow, endCol, endRow }]
        this.maxCol = 0;
        this.maxRow = 0;
    }

    getCell(col, row) {
        return this.cells[`${col},${row}`] || null;
    }

    setCell(col, row, cell) {
        this.cells[`${col},${row}`] = cell;
        if (col > this.maxCol) this.maxCol = col;
        if (row > this.maxRow) this.maxRow = row;
    }

    deleteCell(col, row) {
        delete this.cells[`${col},${row}`];
    }
}

class Workbook {
    constructor() {
        this.sheets = [new Sheet('Sheet1')];
    }
}

// ─── CSV Parser ────────────────────────────────────────
function parseCSV(text) {
    const rows = [];
    let currentRow = [];
    let current = '';
    let inQuotes = false;
    for (let i = 0; i < text.length; i++) {
        const ch = text[i];
        if (inQuotes) {
            if (ch === '"') {
                if (i + 1 < text.length && text[i + 1] === '"') {
                    current += '"';
                    i++;
                } else {
                    inQuotes = false;
                }
            } else {
                current += ch;
            }
        } else {
            if (ch === '"') {
                inQuotes = true;
            } else if (ch === ',') {
                currentRow.push(current);
                current = '';
            } else if (ch === '\n') {
                currentRow.push(current);
                current = '';
                rows.push(currentRow);
                currentRow = [];
            } else if (ch === '\r') {
                // skip, handle \r\n
            } else {
                current += ch;
            }
        }
    }
    if (current || currentRow.length > 0) {
        currentRow.push(current);
        rows.push(currentRow);
    }
    return rows;
}

function generateCSV(sheet) {
    const lines = [];
    for (let r = 0; r <= sheet.maxRow; r++) {
        const cells = [];
        for (let c = 0; c <= sheet.maxCol; c++) {
            const cell = sheet.getCell(c, r);
            let val = cell ? (cell.formula ? cell.formula : String(cell.value ?? '')) : '';
            // Escape CSV
            if (val.includes(',') || val.includes('"') || val.includes('\n')) {
                val = '"' + val.replace(/"/g, '""') + '"';
            }
            cells.push(val);
        }
        lines.push(cells.join(','));
    }
    return lines.join('\n');
}

// ─── Simple formula evaluator ──────────────────────────
function evaluateFormula(formula, sheet) {
    if (!formula || formula[0] !== '=') return formula;
    const expr = formula.slice(1).trim();
    try {
        // Replace cell references (A1, B2, etc.) with their values
        const replaced = expr.replace(/\b([A-Z]{1,3})(\d{1,7})\b/gi, (_match, colLetter, rowNum) => {
            const col = colLetterToIndex(colLetter.toUpperCase());
            const row = parseInt(rowNum, 10) - 1;
            const cell = sheet.getCell(col, row);
            if (!cell) return '0';
            const v = cell.value;
            if (v === null || v === undefined || v === '') return '0';
            if (typeof v === 'number') return String(v);
            const n = Number(v);
            return isNaN(n) ? '0' : String(n);
        });

        // Handle SUM(range)
        const sumMatch = replaced.match(/^SUM\(([^)]+)\)$/i);
        if (sumMatch) {
            return evaluateSum(sumMatch[1], sheet);
        }
        // Handle AVERAGE(range)
        const avgMatch = replaced.match(/^AVERAGE\(([^)]+)\)$/i);
        if (avgMatch) {
            return evaluateAverage(avgMatch[1], sheet);
        }
        // Handle COUNT(range)
        const countMatch = replaced.match(/^COUNT\(([^)]+)\)$/i);
        if (countMatch) {
            return evaluateCount(countMatch[1], sheet);
        }
        // Handle MIN(range)
        const minMatch = replaced.match(/^MIN\(([^)]+)\)$/i);
        if (minMatch) {
            return evaluateMinMax(minMatch[1], sheet, 'min');
        }
        // Handle MAX(range)
        const maxMatch = replaced.match(/^MAX\(([^)]+)\)$/i);
        if (maxMatch) {
            return evaluateMinMax(maxMatch[1], sheet, 'max');
        }

        // Simple arithmetic evaluation (safe subset)
        // Only allow numbers, operators, parentheses
        if (/^[\d\s+\-*/.()]+$/.test(replaced)) {
            // eslint-disable-next-line no-new-func
            const result = Function('"use strict"; return (' + replaced + ')')();
            if (typeof result === 'number' && isFinite(result)) {
                return Math.round(result * 1e10) / 1e10; // avoid floating point noise
            }
            return '#VALUE!';
        }
        return '#VALUE!';
    } catch (_e) {
        return '#ERROR!';
    }
}

function colLetterToIndex(letters) {
    let idx = 0;
    for (let i = 0; i < letters.length; i++) {
        idx = idx * 26 + (letters.charCodeAt(i) - 64);
    }
    return idx - 1;
}

function parseRange(rangeStr, sheet) {
    // Parse A1:B5 style ranges
    const match = rangeStr.trim().match(/^([A-Z]{1,3})(\d{1,7}):([A-Z]{1,3})(\d{1,7})$/i);
    if (!match) return [];
    const startCol = colLetterToIndex(match[1].toUpperCase());
    const startRow = parseInt(match[2], 10) - 1;
    const endCol = colLetterToIndex(match[3].toUpperCase());
    const endRow = parseInt(match[4], 10) - 1;
    const values = [];
    for (let r = startRow; r <= endRow; r++) {
        for (let c = startCol; c <= endCol; c++) {
            const cell = sheet.getCell(c, r);
            if (cell && cell.value !== null && cell.value !== undefined && cell.value !== '') {
                const n = Number(cell.value);
                if (!isNaN(n)) values.push(n);
            }
        }
    }
    return values;
}

function evaluateSum(rangeStr, sheet) {
    const values = parseRange(rangeStr, sheet);
    return values.reduce((a, b) => a + b, 0);
}

function evaluateAverage(rangeStr, sheet) {
    const values = parseRange(rangeStr, sheet);
    if (values.length === 0) return '#DIV/0!';
    return values.reduce((a, b) => a + b, 0) / values.length;
}

function evaluateCount(rangeStr, sheet) {
    const values = parseRange(rangeStr, sheet);
    return values.length;
}

function evaluateMinMax(rangeStr, sheet, mode) {
    const values = parseRange(rangeStr, sheet);
    if (values.length === 0) return 0;
    return mode === 'min' ? Math.min(...values) : Math.max(...values);
}

// ─── Undo / Redo stack ─────────────────────────────────
class UndoManager {
    constructor() {
        this._undoStack = [];
        this._redoStack = [];
    }

    push(action) {
        // action: { type, col, row, oldValue, newValue, sheetIndex, ... }
        this._undoStack.push(action);
        this._redoStack = [];
        if (this._undoStack.length > 500) this._undoStack.shift();
    }

    undo(workbook) {
        const action = this._undoStack.pop();
        if (!action) return null;
        this._redoStack.push(action);
        this._applyInverse(action, workbook);
        return action;
    }

    redo(workbook) {
        const action = this._redoStack.pop();
        if (!action) return null;
        this._undoStack.push(action);
        this._applyForward(action, workbook);
        return action;
    }

    _applyInverse(action, workbook) {
        const sheet = workbook.sheets[action.sheetIndex];
        if (!sheet) return;
        if (action.type === 'edit') {
            if (action.oldValue === null) {
                sheet.deleteCell(action.col, action.row);
            } else {
                sheet.setCell(action.col, action.row, action.oldValue);
            }
        } else if (action.type === 'insertRow') {
            this._deleteRowData(sheet, action.row);
        } else if (action.type === 'deleteRow') {
            this._insertRowData(sheet, action.row, action.rowData);
        } else if (action.type === 'insertCol') {
            this._deleteColData(sheet, action.col);
        } else if (action.type === 'deleteCol') {
            this._insertColData(sheet, action.col, action.colData);
        } else if (action.type === 'sort') {
            // Restore pre-sort cell state
            const currentCells = JSON.parse(JSON.stringify(sheet.cells));
            action.postSortCells = currentCells; // save for redo
            sheet.cells = {};
            sheet.maxCol = 0;
            sheet.maxRow = 0;
            const prev = action.previousCells;
            for (const key of Object.keys(prev)) {
                sheet.cells[key] = prev[key];
                const [c, r] = key.split(',').map(Number);
                if (c > sheet.maxCol) sheet.maxCol = c;
                if (r > sheet.maxRow) sheet.maxRow = r;
            }
        }
    }

    _applyForward(action, workbook) {
        const sheet = workbook.sheets[action.sheetIndex];
        if (!sheet) return;
        if (action.type === 'edit') {
            if (action.newValue === null) {
                sheet.deleteCell(action.col, action.row);
            } else {
                sheet.setCell(action.col, action.row, action.newValue);
            }
        } else if (action.type === 'insertRow') {
            this._insertRowData(sheet, action.row, null);
        } else if (action.type === 'deleteRow') {
            this._deleteRowData(sheet, action.row);
        } else if (action.type === 'insertCol') {
            this._insertColData(sheet, action.col, null);
        } else if (action.type === 'deleteCol') {
            this._deleteColData(sheet, action.col);
        } else if (action.type === 'sort') {
            // Redo: restore post-sort cells
            if (action.postSortCells) {
                sheet.cells = {};
                sheet.maxCol = 0;
                sheet.maxRow = 0;
                const post = action.postSortCells;
                for (const key of Object.keys(post)) {
                    sheet.cells[key] = post[key];
                    const [c, r] = key.split(',').map(Number);
                    if (c > sheet.maxCol) sheet.maxCol = c;
                    if (r > sheet.maxRow) sheet.maxRow = r;
                }
            }
        }
    }

    _deleteRowData(sheet, row) {
        // Shift rows up
        for (let r = row; r <= sheet.maxRow; r++) {
            for (let c = 0; c <= sheet.maxCol; c++) {
                const below = sheet.getCell(c, r + 1);
                if (below) {
                    sheet.setCell(c, r, { ...below });
                } else {
                    sheet.deleteCell(c, r);
                }
            }
        }
        if (sheet.maxRow > 0) sheet.maxRow--;
    }

    _insertRowData(sheet, row, rowData) {
        // Shift rows down
        for (let r = sheet.maxRow; r >= row; r--) {
            for (let c = 0; c <= sheet.maxCol; c++) {
                const cell = sheet.getCell(c, r);
                if (cell) {
                    sheet.setCell(c, r + 1, { ...cell });
                } else {
                    sheet.deleteCell(c, r + 1);
                }
            }
        }
        // Clear the inserted row
        for (let c = 0; c <= sheet.maxCol; c++) {
            if (rowData && rowData[c]) {
                sheet.setCell(c, row, { ...rowData[c] });
            } else {
                sheet.deleteCell(c, row);
            }
        }
        sheet.maxRow++;
    }

    _deleteColData(sheet, col) {
        for (let c = col; c <= sheet.maxCol; c++) {
            for (let r = 0; r <= sheet.maxRow; r++) {
                const right = sheet.getCell(c + 1, r);
                if (right) {
                    sheet.setCell(c, r, { ...right });
                } else {
                    sheet.deleteCell(c, r);
                }
            }
        }
        if (sheet.maxCol > 0) sheet.maxCol--;
    }

    _insertColData(sheet, col, colData) {
        for (let c = sheet.maxCol; c >= col; c--) {
            for (let r = 0; r <= sheet.maxRow; r++) {
                const cell = sheet.getCell(c, r);
                if (cell) {
                    sheet.setCell(c + 1, r, { ...cell });
                } else {
                    sheet.deleteCell(c + 1, r);
                }
            }
        }
        for (let r = 0; r <= sheet.maxRow; r++) {
            if (colData && colData[r]) {
                sheet.setCell(col, r, { ...colData[r] });
            } else {
                sheet.deleteCell(col, r);
            }
        }
        sheet.maxCol++;
    }
}

// ─── SpreadsheetView class ─────────────────────────────
export class SpreadsheetView {
    constructor(container) {
        this.container = container;
        this.canvas = null;
        this.ctx = null;
        this.workbook = null;
        this.activeSheet = 0;
        this.scrollX = 0;
        this.scrollY = 0;
        this.selectedCell = { col: 0, row: 0 };
        this.selectionRange = null;   // { startCol, startRow, endCol, endRow }
        this.editingCell = null;      // { col, row }
        this.editInput = null;        // <input> element for cell editing
        this.formulaInput = null;     // formula bar input
        this.cellRefLabel = null;     // cell ref label
        this.tabBar = null;
        this.frozenCols = 0;
        this.frozenRows = 0;
        this.sortState = {};
        this.filterState = {};
        this.hiddenRows = new Set();
        this._undoManager = new UndoManager();
        this._clipboard = null;       // { cells, startCol, startRow, endCol, endRow, cut }
        this._dragging = false;
        this._dragStart = null;
        this._resizingCol = null;     // { col, startX, startWidth }
        this._resizingRow = null;     // { row, startY, startHeight }
        this._rafId = null;
        this._contextMenu = null;
        this._filterDropdown = null;
        this._dpr = window.devicePixelRatio || 1;

        this.setupDOM();
        this.setupEvents();
    }

    // ─── DOM setup ───────────────────────────────
    setupDOM() {
        this.container.innerHTML = '';
        this.container.className = 'spreadsheet-container';

        // Formula bar
        const formulaBar = document.createElement('div');
        formulaBar.className = 'ss-formula-bar';
        const cellRefLabel = document.createElement('span');
        cellRefLabel.className = 'ss-cell-ref-label';
        cellRefLabel.textContent = 'A1';
        this.cellRefLabel = cellRefLabel;
        const fxLabel = document.createElement('span');
        fxLabel.className = 'ss-fx-label';
        fxLabel.textContent = 'fx';
        const formulaInput = document.createElement('input');
        formulaInput.className = 'ss-formula-input';
        formulaInput.placeholder = 'Enter value or formula...';
        formulaInput.title = 'Formula bar — type a value or formula (e.g. =SUM(A1:A10))';
        this.formulaInput = formulaInput;
        formulaBar.appendChild(cellRefLabel);
        formulaBar.appendChild(fxLabel);
        formulaBar.appendChild(formulaInput);
        this.container.appendChild(formulaBar);

        // Canvas wrapper (for scrolling)
        const canvasWrap = document.createElement('div');
        canvasWrap.className = 'ss-canvas-wrap';
        this.canvasWrap = canvasWrap;

        this.canvas = document.createElement('canvas');
        this.canvas.className = 'ss-canvas';
        this.canvas.tabIndex = 0;
        this.ctx = this.canvas.getContext('2d');
        canvasWrap.appendChild(this.canvas);
        this.container.appendChild(canvasWrap);

        // Cell editor overlay
        this.editInput = document.createElement('input');
        this.editInput.className = 'ss-cell-editor';
        this.editInput.style.display = 'none';
        canvasWrap.appendChild(this.editInput);

        // Context menu container
        this._contextMenu = document.createElement('div');
        this._contextMenu.className = 'ss-context-menu';
        this._contextMenu.style.display = 'none';
        canvasWrap.appendChild(this._contextMenu);

        // Filter dropdown container
        this._filterDropdown = document.createElement('div');
        this._filterDropdown.className = 'ss-filter-dropdown';
        this._filterDropdown.style.display = 'none';
        canvasWrap.appendChild(this._filterDropdown);

        // Sheet tabs
        const tabBar = document.createElement('div');
        tabBar.className = 'ss-tab-bar';
        this.tabBar = tabBar;
        this.container.appendChild(tabBar);

        this.resizeCanvas();
    }

    // ─── Events ──────────────────────────────────
    setupEvents() {
        // Canvas events
        this.canvas.addEventListener('mousedown', (e) => this.handleMouseDown(e));
        this.canvas.addEventListener('mousemove', (e) => this.handleMouseMove(e));
        this.canvas.addEventListener('mouseup', (e) => this.handleMouseUp(e));
        this.canvas.addEventListener('dblclick', (e) => this.handleDoubleClick(e));
        this.canvas.addEventListener('wheel', (e) => this.handleScroll(e), { passive: false });
        this.canvas.addEventListener('keydown', (e) => this.handleKeyDown(e));
        this.canvas.addEventListener('contextmenu', (e) => this.handleContextMenu(e));

        // Formula bar events
        this.formulaInput.addEventListener('keydown', (e) => {
            if (e.key === 'Enter') {
                e.preventDefault();
                const sheet = this._sheet();
                const { col, row } = this.selectedCell;
                const val = this.formulaInput.value;
                this._setCellValue(col, row, val);
                this.canvas.focus();
                this.render();
            } else if (e.key === 'Escape') {
                this.formulaInput.value = this._getCellDisplay(this.selectedCell.col, this.selectedCell.row);
                this.canvas.focus();
            }
        });

        // Edit input events
        this.editInput.addEventListener('keydown', (e) => {
            if (e.key === 'Enter') {
                e.preventDefault();
                this.commitEdit();
                // Move down
                if (this.selectedCell.row < MAX_ROWS - 1) {
                    this.selectedCell.row++;
                    this._ensureVisible(this.selectedCell.col, this.selectedCell.row);
                }
                this.render();
            } else if (e.key === 'Escape') {
                this.cancelEdit();
            } else if (e.key === 'Tab') {
                e.preventDefault();
                this.commitEdit();
                if (e.shiftKey) {
                    if (this.selectedCell.col > 0) this.selectedCell.col--;
                } else {
                    if (this.selectedCell.col < MAX_COLS - 1) this.selectedCell.col++;
                }
                this._ensureVisible(this.selectedCell.col, this.selectedCell.row);
                this.render();
            }
        });

        this.editInput.addEventListener('blur', () => {
            if (this.editingCell) {
                this.commitEdit();
            }
        });

        // Resize observer
        this._resizeObserver = new ResizeObserver(() => {
            this.resizeCanvas();
            this.render();
        });
        this._resizeObserver.observe(this.container);

        // Close context menu on click outside
        document.addEventListener('mousedown', (e) => {
            if (this._contextMenu && !this._contextMenu.contains(e.target)) {
                this._contextMenu.style.display = 'none';
            }
            if (this._filterDropdown && !this._filterDropdown.contains(e.target)) {
                this._filterDropdown.style.display = 'none';
            }
        });
    }

    // ─── Data loading ────────────────────────────
    loadWorkbook(data, filename) {
        const ext = filename?.split('.').pop()?.toLowerCase();
        this.workbook = new Workbook();

        if (ext === 'csv' || (typeof data === 'string') || this._isTextData(data)) {
            // CSV parsing
            const text = typeof data === 'string' ? data : new TextDecoder('utf-8').decode(data);
            const rows = parseCSV(text);
            const sheet = this.workbook.sheets[0];
            sheet.name = (filename || 'Sheet1').replace(/\.[^.]+$/, '');
            for (let r = 0; r < rows.length; r++) {
                for (let c = 0; c < rows[r].length; c++) {
                    const raw = rows[r][c];
                    if (raw === '') continue;
                    const cell = this._parseRawValue(raw);
                    sheet.setCell(c, r, cell);
                }
            }
        } else {
            // Attempt XLSX parsing (simple: just load as single-sheet placeholder)
            // Full XLSX parsing would require a library like SheetJS
            try {
                this._parseXLSXBytes(data);
            } catch (e) {
                console.warn('[spreadsheet] XLSX parse failed, treating as CSV:', e);
                const text = new TextDecoder('utf-8').decode(data);
                const rows = parseCSV(text);
                const sheet = this.workbook.sheets[0];
                for (let r = 0; r < rows.length; r++) {
                    for (let c = 0; c < rows[r].length; c++) {
                        const raw = rows[r][c];
                        if (raw === '') continue;
                        sheet.setCell(c, r, this._parseRawValue(raw));
                    }
                }
            }
        }

        this.activeSheet = 0;
        this.selectedCell = { col: 0, row: 0 };
        this.selectionRange = null;
        this.scrollX = 0;
        this.scrollY = 0;
        this._undoManager = new UndoManager();

        this.updateSheetTabs();
        this._updateFormulaBar();
        this.render();
    }

    _isTextData(data) {
        // Check if data looks like text (CSV) vs binary (XLSX)
        if (!(data instanceof Uint8Array)) return false;
        // XLSX starts with PK (ZIP magic bytes)
        if (data.length >= 4 && data[0] === 0x50 && data[1] === 0x4B) return false;
        // Check first 100 bytes for text-like content
        const sample = data.slice(0, Math.min(data.length, 100));
        for (const b of sample) {
            if (b < 9 || (b > 13 && b < 32 && b !== 27)) return false;
        }
        return true;
    }

    _parseRawValue(raw) {
        // Detect type: number, boolean, formula, string
        if (raw.startsWith('=')) {
            return { value: raw, formula: raw, display: '', type: 'formula', style: null };
        }
        if (raw.toLowerCase() === 'true' || raw.toLowerCase() === 'false') {
            return { value: raw.toLowerCase() === 'true', formula: null, display: String(raw), type: 'boolean', style: null };
        }
        const num = Number(raw);
        if (raw !== '' && !isNaN(num)) {
            return { value: num, formula: null, display: String(num), type: 'number', style: null };
        }
        return { value: raw, formula: null, display: raw, type: 'string', style: null };
    }

    _parseXLSXBytes(data) {
        // Minimal XLSX parser — reads shared strings and sheet data from the ZIP
        // This is a simplified implementation. For full XLSX support, use SheetJS.
        const zip = this._readZip(data);
        if (!zip) throw new Error('Not a valid XLSX file');

        // Read shared strings
        const sharedStrings = [];
        const sst = zip['xl/sharedStrings.xml'];
        if (sst) {
            const sstText = new TextDecoder('utf-8').decode(sst);
            const siRegex = /<si[^>]*>([\s\S]*?)<\/si>/gi;
            let siMatch;
            while ((siMatch = siRegex.exec(sstText)) !== null) {
                const tRegex = /<t[^>]*>([\s\S]*?)<\/t>/gi;
                let combined = '';
                let tMatch;
                while ((tMatch = tRegex.exec(siMatch[1])) !== null) {
                    combined += tMatch[1];
                }
                sharedStrings.push(this._unescapeXml(combined));
            }
        }

        // Read workbook to get sheet names
        const wbXml = zip['xl/workbook.xml'];
        const sheetNames = [];
        if (wbXml) {
            const wbText = new TextDecoder('utf-8').decode(wbXml);
            const sheetRegex = /<sheet\s+name="([^"]*)"[^/]*\/?>/gi;
            let sm;
            while ((sm = sheetRegex.exec(wbText)) !== null) {
                sheetNames.push(this._unescapeXml(sm[1]));
            }
        }

        // Read each sheet
        this.workbook = new Workbook();
        this.workbook.sheets = [];
        let sheetIndex = 1;
        while (true) {
            const path = `xl/worksheets/sheet${sheetIndex}.xml`;
            const sheetData = zip[path];
            if (!sheetData) break;

            const name = sheetNames[sheetIndex - 1] || `Sheet${sheetIndex}`;
            const sheet = new Sheet(name);
            const sheetText = new TextDecoder('utf-8').decode(sheetData);

            // Parse rows and cells
            const rowRegex = /<row[^>]*>([\s\S]*?)<\/row>/gi;
            let rowMatch;
            while ((rowMatch = rowRegex.exec(sheetText)) !== null) {
                const cellRegex = /<c\s+r="([A-Z]+)(\d+)"([^>]*)>([\s\S]*?)<\/c>/gi;
                let cellMatch;
                while ((cellMatch = cellRegex.exec(rowMatch[1])) !== null) {
                    const colStr = cellMatch[1];
                    const rowNum = parseInt(cellMatch[2], 10) - 1;
                    const attrs = cellMatch[3];
                    const inner = cellMatch[4];
                    const col = colLetterToIndex(colStr);

                    // Get value
                    const vMatch = inner.match(/<v[^>]*>([\s\S]*?)<\/v>/);
                    let value = vMatch ? vMatch[1] : '';

                    // Check type
                    const tAttr = attrs.match(/t="([^"]*)"/);
                    const type = tAttr ? tAttr[1] : '';

                    if (type === 's' && sharedStrings[parseInt(value, 10)] !== undefined) {
                        value = sharedStrings[parseInt(value, 10)];
                        sheet.setCell(col, rowNum, { value, formula: null, display: value, type: 'string', style: null });
                    } else if (type === 'b') {
                        sheet.setCell(col, rowNum, { value: value === '1', formula: null, display: value === '1' ? 'TRUE' : 'FALSE', type: 'boolean', style: null });
                    } else {
                        const num = Number(value);
                        if (!isNaN(num) && value !== '') {
                            sheet.setCell(col, rowNum, { value: num, formula: null, display: String(num), type: 'number', style: null });
                        } else {
                            sheet.setCell(col, rowNum, { value, formula: null, display: value, type: 'string', style: null });
                        }
                    }

                    // Check for formula
                    const fMatch = inner.match(/<f[^>]*>([\s\S]*?)<\/f>/);
                    if (fMatch) {
                        const formula = '=' + fMatch[1];
                        const cell = sheet.getCell(col, rowNum);
                        if (cell) {
                            cell.formula = formula;
                            cell.type = 'formula';
                        }
                    }
                }
            }

            this.workbook.sheets.push(sheet);
            sheetIndex++;
        }

        if (this.workbook.sheets.length === 0) {
            this.workbook.sheets.push(new Sheet('Sheet1'));
        }
    }

    _readZip(data) {
        // Minimal ZIP reader for XLSX — reads local file entries
        const view = new DataView(data.buffer || data);
        const files = {};
        let offset = 0;
        const len = data.length;

        while (offset < len - 4) {
            const sig = view.getUint32(offset, true);
            if (sig !== 0x04034B50) break; // PK\x03\x04 local file header

            const compressionMethod = view.getUint16(offset + 8, true);
            const compressedSize = view.getUint32(offset + 18, true);
            const uncompressedSize = view.getUint32(offset + 22, true);
            const nameLen = view.getUint16(offset + 26, true);
            const extraLen = view.getUint16(offset + 28, true);
            const nameBytes = data.slice(offset + 30, offset + 30 + nameLen);
            const name = new TextDecoder('utf-8').decode(nameBytes);
            const dataStart = offset + 30 + nameLen + extraLen;

            if (compressionMethod === 0) {
                // Stored (no compression)
                files[name] = data.slice(dataStart, dataStart + compressedSize);
            } else if (compressionMethod === 8) {
                // Deflate — try using DecompressionStream if available
                try {
                    const compressed = data.slice(dataStart, dataStart + compressedSize);
                    // Use raw inflate (browser built-in not available synchronously,
                    // so we skip compressed entries for now)
                    files[name] = compressed; // Will be garbled but at least we try
                } catch (_) {
                    // Skip compressed entries
                }
            }
            offset = dataStart + compressedSize;
        }

        return Object.keys(files).length > 0 ? files : null;
    }

    _unescapeXml(str) {
        return str
            .replace(/&amp;/g, '&')
            .replace(/&lt;/g, '<')
            .replace(/&gt;/g, '>')
            .replace(/&quot;/g, '"')
            .replace(/&apos;/g, "'");
    }

    // ─── Sheet access ────────────────────────────
    _sheet() {
        if (!this.workbook) return null;
        return this.workbook.sheets[this.activeSheet] || null;
    }

    // ─── Cell value helpers ──────────────────────
    _getCellDisplay(col, row) {
        const sheet = this._sheet();
        if (!sheet) return '';
        const cell = sheet.getCell(col, row);
        if (!cell) return '';
        if (cell.formula) {
            const result = evaluateFormula(cell.formula, sheet);
            return String(result);
        }
        return cell.display || String(cell.value ?? '');
    }

    _getCellRawValue(col, row) {
        const sheet = this._sheet();
        if (!sheet) return '';
        const cell = sheet.getCell(col, row);
        if (!cell) return '';
        if (cell.formula) return cell.formula;
        return String(cell.value ?? '');
    }

    _setCellValue(col, row, rawValue) {
        const sheet = this._sheet();
        if (!sheet) return;
        const oldCell = sheet.getCell(col, row);
        const oldCopy = oldCell ? { ...oldCell } : null;

        if (rawValue === '' || rawValue === null || rawValue === undefined) {
            sheet.deleteCell(col, row);
            this._undoManager.push({
                type: 'edit', col, row, sheetIndex: this.activeSheet,
                oldValue: oldCopy, newValue: null
            });
        } else {
            const newCell = this._parseRawValue(rawValue);
            if (newCell.formula) {
                const result = evaluateFormula(newCell.formula, sheet);
                newCell.display = String(result);
            }
            sheet.setCell(col, row, newCell);
            this._undoManager.push({
                type: 'edit', col, row, sheetIndex: this.activeSheet,
                oldValue: oldCopy, newValue: { ...newCell }
            });
        }
    }

    // ─── Rendering ───────────────────────────────
    render() {
        if (this._rafId) cancelAnimationFrame(this._rafId);
        this._rafId = requestAnimationFrame(() => this._renderImpl());
    }

    _renderImpl() {
        const ctx = this.ctx;
        const canvas = this.canvas;
        const dpr = this._dpr;
        const w = canvas.width / dpr;
        const h = canvas.height / dpr;

        ctx.save();
        ctx.scale(dpr, dpr);
        ctx.clearRect(0, 0, w, h);

        // Background
        ctx.fillStyle = '#fff';
        ctx.fillRect(0, 0, w, h);

        const sheet = this._sheet();
        if (!sheet) {
            ctx.restore();
            return;
        }

        // Calculate visible range
        const visibleCols = this._getVisibleCols();
        const visibleRows = this._getVisibleRows();

        // Draw grid cells
        this._renderCells(ctx, visibleCols, visibleRows);

        // Draw grid lines
        this._renderGridLines(ctx, visibleCols, visibleRows, w, h);

        // Draw frozen pane separators
        this._renderFrozenPanes(ctx, w, h);

        // Draw headers (on top of grid)
        this._renderHeaders(ctx, visibleCols, visibleRows, w, h);

        // Draw selection
        this._renderSelection(ctx);

        // Draw fill handle
        this._renderFillHandle(ctx);

        ctx.restore();
    }

    _getVisibleCols() {
        const w = this.canvas.width / this._dpr;
        const cols = [];
        let x = ROW_HEADER_WIDTH - this.scrollX;
        // Frozen columns first
        for (let c = 0; c < this.frozenCols; c++) {
            const cw = this.getColumnWidth(c);
            if (x + cw > ROW_HEADER_WIDTH && x < w) {
                cols.push({ col: c, x, width: cw, frozen: true });
            }
            x += cw;
        }
        const frozenWidth = x - (ROW_HEADER_WIDTH - this.scrollX);
        // Scrollable columns
        x = ROW_HEADER_WIDTH + frozenWidth - this.scrollX;
        const startCol = Math.max(this.frozenCols, this._colAtX(this.scrollX) - BUFFER_CELLS);
        for (let c = startCol; c < MAX_COLS; c++) {
            if (c < this.frozenCols) continue;
            const cx = this._colX(c) - this.scrollX + ROW_HEADER_WIDTH;
            const cw = this.getColumnWidth(c);
            if (cx > w + cw) break;
            if (cx + cw > ROW_HEADER_WIDTH) {
                cols.push({ col: c, x: cx, width: cw, frozen: false });
            }
        }
        return cols;
    }

    _getVisibleRows() {
        const h = this.canvas.height / this._dpr;
        const rows = [];
        let y = HEADER_HEIGHT;
        // Frozen rows first
        for (let r = 0; r < this.frozenRows; r++) {
            if (this.hiddenRows.has(r)) continue;
            const rh = this.getRowHeight(r);
            if (y + rh > HEADER_HEIGHT && y < h) {
                rows.push({ row: r, y, height: rh, frozen: true });
            }
            y += rh;
        }
        const frozenHeight = y - HEADER_HEIGHT;
        // Scrollable rows
        const startRow = Math.max(this.frozenRows, this._rowAtY(this.scrollY) - BUFFER_CELLS);
        for (let r = startRow; r < MAX_ROWS; r++) {
            if (r < this.frozenRows) continue;
            if (this.hiddenRows.has(r)) continue;
            const ry = this._rowY(r) - this.scrollY + HEADER_HEIGHT + frozenHeight;
            const rh = this.getRowHeight(r);
            if (ry > h + rh) break;
            if (ry + rh > HEADER_HEIGHT) {
                rows.push({ row: r, y: ry, height: rh, frozen: false });
            }
        }
        return rows;
    }

    _renderCells(ctx, visibleCols, visibleRows) {
        const sheet = this._sheet();
        if (!sheet) return;

        ctx.font = '13px Arial, sans-serif';
        ctx.textBaseline = 'middle';

        for (const vc of visibleCols) {
            for (const vr of visibleRows) {
                const cell = sheet.getCell(vc.col, vr.row);
                if (cell) {
                    this.renderCell(ctx, vc.col, vr.row, vc.x, vr.y, vc.width, vr.height, cell);
                }
            }
        }
    }

    renderCell(ctx, col, row, x, y, w, h, cell) {
        if (!cell) return;

        const sheet = this._sheet();
        const display = cell.formula
            ? String(evaluateFormula(cell.formula, sheet))
            : (cell.display || String(cell.value ?? ''));

        // Cell background
        if (cell.style?.bgColor) {
            ctx.fillStyle = cell.style.bgColor;
            ctx.fillRect(x, y, w, h);
        }

        // Cell text
        ctx.save();
        ctx.beginPath();
        ctx.rect(x + 2, y, w - 4, h);
        ctx.clip();

        const isError = typeof display === 'string' && display.startsWith('#');
        const isNumber = cell.type === 'number' || (cell.type === 'formula' && !isNaN(Number(display)));

        if (isError) {
            ctx.fillStyle = '#d93025';
        } else if (cell.style?.color) {
            ctx.fillStyle = cell.style.color;
        } else {
            ctx.fillStyle = '#202124';
        }

        if (cell.style?.bold) {
            ctx.font = 'bold 13px Arial, sans-serif';
        } else {
            ctx.font = '13px Arial, sans-serif';
        }

        ctx.textAlign = isNumber && !isError ? 'right' : 'left';
        const textX = isNumber && !isError ? x + w - 4 : x + 4;
        ctx.fillText(display, textX, y + h / 2);
        ctx.restore();
    }

    _renderGridLines(ctx, visibleCols, visibleRows, w, h) {
        ctx.strokeStyle = '#e0e0e0';
        ctx.lineWidth = 1;

        // Vertical lines
        for (const vc of visibleCols) {
            const x = Math.round(vc.x) + 0.5;
            ctx.beginPath();
            ctx.moveTo(x, HEADER_HEIGHT);
            ctx.lineTo(x, h);
            ctx.stroke();
            // Right edge
            const xr = Math.round(vc.x + vc.width) + 0.5;
            ctx.beginPath();
            ctx.moveTo(xr, HEADER_HEIGHT);
            ctx.lineTo(xr, h);
            ctx.stroke();
        }

        // Horizontal lines
        for (const vr of visibleRows) {
            const y = Math.round(vr.y) + 0.5;
            ctx.beginPath();
            ctx.moveTo(ROW_HEADER_WIDTH, y);
            ctx.lineTo(w, y);
            ctx.stroke();
            // Bottom edge
            const yb = Math.round(vr.y + vr.height) + 0.5;
            ctx.beginPath();
            ctx.moveTo(ROW_HEADER_WIDTH, yb);
            ctx.lineTo(w, yb);
            ctx.stroke();
        }
    }

    renderHeaders(ctx, visibleCols, visibleRows, w, h) {
        this._renderHeaders(ctx, visibleCols, visibleRows, w, h);
    }

    _renderHeaders(ctx, visibleCols, visibleRows, w, h) {
        // Column headers background
        ctx.fillStyle = '#f8f9fa';
        ctx.fillRect(0, 0, w, HEADER_HEIGHT);
        ctx.fillRect(0, 0, ROW_HEADER_WIDTH, h);

        // Corner cell
        ctx.fillStyle = '#e8eaed';
        ctx.fillRect(0, 0, ROW_HEADER_WIDTH, HEADER_HEIGHT);
        ctx.strokeStyle = '#c4c7c5';
        ctx.lineWidth = 1;
        ctx.strokeRect(0.5, 0.5, ROW_HEADER_WIDTH - 1, HEADER_HEIGHT - 1);

        // Column headers
        ctx.font = '500 12px Arial, sans-serif';
        ctx.textAlign = 'center';
        ctx.textBaseline = 'middle';
        ctx.fillStyle = '#5f6368';

        for (const vc of visibleCols) {
            const isSelected = this._isColSelected(vc.col);
            if (isSelected) {
                ctx.fillStyle = '#d2e3fc';
                ctx.fillRect(vc.x, 0, vc.width, HEADER_HEIGHT);
                ctx.fillStyle = '#1967d2';
            } else {
                ctx.fillStyle = '#5f6368';
            }
            ctx.strokeStyle = '#c4c7c5';
            ctx.strokeRect(Math.round(vc.x) + 0.5, 0.5, vc.width - 1, HEADER_HEIGHT - 1);
            ctx.fillText(this.getCellA1Col(vc.col), vc.x + vc.width / 2, HEADER_HEIGHT / 2);

            // Filter indicator
            if (this.filterState[vc.col]) {
                ctx.fillStyle = '#1a73e8';
                ctx.font = '10px Arial';
                ctx.fillText('\u25BC', vc.x + vc.width - 10, HEADER_HEIGHT / 2);
                ctx.font = '500 12px Arial, sans-serif';
            }
        }

        // Row headers
        ctx.textAlign = 'center';
        for (const vr of visibleRows) {
            const isSelected = this._isRowSelected(vr.row);
            if (isSelected) {
                ctx.fillStyle = '#d2e3fc';
                ctx.fillRect(0, vr.y, ROW_HEADER_WIDTH, vr.height);
                ctx.fillStyle = '#1967d2';
            } else {
                ctx.fillStyle = '#5f6368';
            }
            ctx.strokeStyle = '#c4c7c5';
            ctx.strokeRect(0.5, Math.round(vr.y) + 0.5, ROW_HEADER_WIDTH - 1, vr.height - 1);
            ctx.fillText(String(vr.row + 1), ROW_HEADER_WIDTH / 2, vr.y + vr.height / 2);
        }
    }

    _isColSelected(col) {
        if (this.selectionRange) {
            const { startCol, endCol } = this._normalizeRange(this.selectionRange);
            return col >= startCol && col <= endCol;
        }
        return col === this.selectedCell.col;
    }

    _isRowSelected(row) {
        if (this.selectionRange) {
            const { startRow, endRow } = this._normalizeRange(this.selectionRange);
            return row >= startRow && row <= endRow;
        }
        return row === this.selectedCell.row;
    }

    renderSelection(ctx) {
        this._renderSelection(ctx);
    }

    _renderSelection(ctx) {
        const frozenColsW = this._frozenColsWidth();
        const frozenRowsH = this._frozenRowsHeight();

        // Helper: get the correct screen X for a column, accounting for frozen panes
        const getScreenX = (c) => {
            if (c < this.frozenCols) {
                // Frozen column — no scroll offset
                let x = ROW_HEADER_WIDTH;
                for (let i = 0; i < c; i++) x += this.getColumnWidth(i);
                return x;
            }
            return this._colX(c) - this.scrollX + ROW_HEADER_WIDTH;
        };
        const getScreenY = (r) => {
            if (r < this.frozenRows) {
                // Frozen row — no scroll offset
                let y = HEADER_HEIGHT;
                for (let i = 0; i < r; i++) y += this.getRowHeight(i);
                return y;
            }
            return this._rowY(r) - this.scrollY + HEADER_HEIGHT + frozenRowsH;
        };

        // Selection range
        if (this.selectionRange) {
            const { startCol, startRow, endCol, endRow } = this._normalizeRange(this.selectionRange);
            const x1 = getScreenX(startCol);
            const y1 = getScreenY(startRow);
            const x2 = getScreenX(endCol) + this.getColumnWidth(endCol);
            const y2 = getScreenY(endRow) + this.getRowHeight(endRow);

            // Range fill
            ctx.fillStyle = 'rgba(26, 115, 232, 0.08)';
            ctx.fillRect(x1, y1, x2 - x1, y2 - y1);

            // Range border
            ctx.strokeStyle = '#1a73e8';
            ctx.lineWidth = 2;
            ctx.strokeRect(x1, y1, x2 - x1, y2 - y1);
        }

        // Active cell
        const { col, row } = this.selectedCell;
        const sx = getScreenX(col);
        const sy = getScreenY(row);
        const sw = this.getColumnWidth(col);
        const sh = this.getRowHeight(row);

        // White background for active cell
        ctx.fillStyle = '#fff';
        ctx.fillRect(sx, sy, sw, sh);

        // Re-render cell content on top
        const sheet = this._sheet();
        if (sheet) {
            const cell = sheet.getCell(col, row);
            if (cell) {
                this.renderCell(ctx, col, row, sx, sy, sw, sh, cell);
            }
        }

        // Active cell border (thick blue)
        ctx.strokeStyle = '#1a73e8';
        ctx.lineWidth = 2;
        ctx.strokeRect(sx, sy, sw, sh);
    }

    _renderFillHandle(ctx) {
        if (this.editingCell) return;
        const { col, row } = this.selectionRange
            ? { col: this._normalizeRange(this.selectionRange).endCol, row: this._normalizeRange(this.selectionRange).endRow }
            : this.selectedCell;
        const x = this._cellScreenX(col) + this.getColumnWidth(col) - FILL_HANDLE_SIZE / 2;
        const y = this._cellScreenY(row) + this.getRowHeight(row) - FILL_HANDLE_SIZE / 2;
        ctx.fillStyle = '#1a73e8';
        ctx.fillRect(x, y, FILL_HANDLE_SIZE, FILL_HANDLE_SIZE);
    }

    renderFrozenPanes(ctx, w, h) {
        this._renderFrozenPanes(ctx, w, h);
    }

    _renderFrozenPanes(ctx, w, h) {
        if (this.frozenCols > 0) {
            const fx = this._frozenColsWidth() + ROW_HEADER_WIDTH;
            ctx.strokeStyle = '#9aa0a6';
            ctx.lineWidth = 2;
            ctx.beginPath();
            ctx.moveTo(fx, 0);
            ctx.lineTo(fx, h);
            ctx.stroke();
        }
        if (this.frozenRows > 0) {
            const fy = this._frozenRowsHeight() + HEADER_HEIGHT;
            ctx.strokeStyle = '#9aa0a6';
            ctx.lineWidth = 2;
            ctx.beginPath();
            ctx.moveTo(0, fy);
            ctx.lineTo(w, fy);
            ctx.stroke();
        }
    }

    // ─── Coordinate helpers ──────────────────────
    _colX(col) {
        let x = 0;
        for (let c = 0; c < col; c++) x += this.getColumnWidth(c);
        return x;
    }

    _rowY(row) {
        let y = 0;
        for (let r = 0; r < row; r++) {
            if (this.hiddenRows.has(r)) continue;
            y += this.getRowHeight(r);
        }
        return y;
    }

    _colAtX(scrollX) {
        let x = 0;
        for (let c = 0; c < MAX_COLS; c++) {
            x += this.getColumnWidth(c);
            if (x > scrollX) return c;
        }
        return 0;
    }

    _rowAtY(scrollY) {
        let y = 0;
        for (let r = 0; r < MAX_ROWS; r++) {
            y += this.getRowHeight(r);
            if (y > scrollY) return r;
        }
        return 0;
    }

    _cellScreenX(col) {
        return this._colX(col) - this.scrollX + ROW_HEADER_WIDTH;
    }

    _cellScreenY(row) {
        return this._rowY(row) - this.scrollY + HEADER_HEIGHT;
    }

    _frozenColsWidth() {
        let w = 0;
        for (let c = 0; c < this.frozenCols; c++) w += this.getColumnWidth(c);
        return w;
    }

    _frozenRowsHeight() {
        let h = 0;
        for (let r = 0; r < this.frozenRows; r++) h += this.getRowHeight(r);
        return h;
    }

    getCellAt(canvasX, canvasY) {
        if (canvasX < ROW_HEADER_WIDTH || canvasY < HEADER_HEIGHT) return null;

        const frozenColsW = this._frozenColsWidth();
        const frozenRowsH = this._frozenRowsHeight();

        // Find column
        let col = -1;
        if (this.frozenCols > 0 && canvasX < ROW_HEADER_WIDTH + frozenColsW) {
            // Click is in frozen column region — no scroll offset
            let x = ROW_HEADER_WIDTH;
            for (let c = 0; c < this.frozenCols; c++) {
                const cw = this.getColumnWidth(c);
                if (canvasX >= x && canvasX < x + cw) { col = c; break; }
                x += cw;
            }
        } else {
            // Scrollable column region — apply scroll offset
            let x = ROW_HEADER_WIDTH + frozenColsW;
            const startCol = Math.max(this.frozenCols, this._colAtX(this.scrollX));
            for (let c = startCol; c < MAX_COLS; c++) {
                const cx = this._colX(c) - this.scrollX + ROW_HEADER_WIDTH;
                const cw = this.getColumnWidth(c);
                if (canvasX >= cx && canvasX < cx + cw) { col = c; break; }
                if (cx > canvasX + CELL_WIDTH * 10) break;
            }
        }

        // Find row
        let row = -1;
        if (this.frozenRows > 0 && canvasY < HEADER_HEIGHT + frozenRowsH) {
            // Click is in frozen row region — no scroll offset
            let y = HEADER_HEIGHT;
            for (let r = 0; r < this.frozenRows; r++) {
                const rh = this.getRowHeight(r);
                if (canvasY >= y && canvasY < y + rh) { row = r; break; }
                y += rh;
            }
        } else {
            // Scrollable row region — apply scroll offset
            const startRow = Math.max(this.frozenRows, this._rowAtY(this.scrollY));
            for (let r = startRow; r < MAX_ROWS; r++) {
                const ry = this._rowY(r) - this.scrollY + HEADER_HEIGHT + frozenRowsH;
                const rh = this.getRowHeight(r);
                if (canvasY >= ry && canvasY < ry + rh) { row = r; break; }
                if (ry > canvasY + CELL_HEIGHT * 10) break;
            }
        }

        if (col < 0 || row < 0) return null;
        return { col, row };
    }

    getColumnWidth(col) {
        const sheet = this._sheet();
        return sheet?.colWidths?.[col] || CELL_WIDTH;
    }

    getRowHeight(row) {
        const sheet = this._sheet();
        return sheet?.rowHeights?.[row] || CELL_HEIGHT;
    }

    // ─── Mouse handling ──────────────────────────
    handleMouseDown(e) {
        const rect = this.canvas.getBoundingClientRect();
        const canvasX = (e.clientX - rect.left);
        const canvasY = (e.clientY - rect.top);

        // Check for column resize handle
        if (canvasY < HEADER_HEIGHT) {
            const resizeCol = this._hitColumnResizeHandle(canvasX);
            if (resizeCol !== null) {
                this._resizingCol = { col: resizeCol, startX: e.clientX, startWidth: this.getColumnWidth(resizeCol) };
                e.preventDefault();
                return;
            }
        }

        // Check for row resize handle
        if (canvasX < ROW_HEADER_WIDTH) {
            const resizeRow = this._hitRowResizeHandle(canvasY);
            if (resizeRow !== null) {
                this._resizingRow = { row: resizeRow, startY: e.clientY, startHeight: this.getRowHeight(resizeRow) };
                e.preventDefault();
                return;
            }
        }

        const cell = this.getCellAt(canvasX, canvasY);
        if (!cell) return;

        if (this.editingCell) {
            this.commitEdit();
        }

        this.selectedCell = { col: cell.col, row: cell.row };
        this.selectionRange = null;
        this._dragging = true;
        this._dragStart = { col: cell.col, row: cell.row };

        if (e.shiftKey) {
            // Extend selection
            this.selectionRange = {
                startCol: this._dragStart.col, startRow: this._dragStart.row,
                endCol: cell.col, endRow: cell.row
            };
        }

        this._updateFormulaBar();
        this.render();
    }

    handleMouseMove(e) {
        const rect = this.canvas.getBoundingClientRect();
        const canvasX = e.clientX - rect.left;
        const canvasY = e.clientY - rect.top;

        // Column resize
        if (this._resizingCol) {
            const delta = e.clientX - this._resizingCol.startX;
            const newWidth = Math.max(30, this._resizingCol.startWidth + delta);
            this.resizeColumn(this._resizingCol.col, newWidth);
            return;
        }

        // Row resize
        if (this._resizingRow) {
            const delta = e.clientY - this._resizingRow.startY;
            const newHeight = Math.max(16, this._resizingRow.startHeight + delta);
            this.resizeRow(this._resizingRow.row, newHeight);
            return;
        }

        // Update cursor for resize handles
        if (canvasY < HEADER_HEIGHT && this._hitColumnResizeHandle(canvasX) !== null) {
            this.canvas.style.cursor = 'col-resize';
        } else if (canvasX < ROW_HEADER_WIDTH && this._hitRowResizeHandle(canvasY) !== null) {
            this.canvas.style.cursor = 'row-resize';
        } else {
            this.canvas.style.cursor = 'cell';
        }

        // Drag selection
        if (this._dragging && this._dragStart) {
            const cell = this.getCellAt(canvasX, canvasY);
            if (cell) {
                this.selectionRange = {
                    startCol: this._dragStart.col, startRow: this._dragStart.row,
                    endCol: cell.col, endRow: cell.row
                };
                this.selectedCell = { col: cell.col, row: cell.row };
                this.render();
            }
        }
    }

    handleMouseUp(_e) {
        this._dragging = false;
        this._dragStart = null;
        this._resizingCol = null;
        this._resizingRow = null;
    }

    _hitColumnResizeHandle(canvasX) {
        // Check if canvasX is near a column header border (within 4px)
        let x = ROW_HEADER_WIDTH - this.scrollX;
        for (let c = 0; c < MAX_COLS; c++) {
            x += this.getColumnWidth(c);
            if (x > this.canvas.width / this._dpr + 100) break;
            if (Math.abs(canvasX - x) < 4) return c;
        }
        return null;
    }

    _hitRowResizeHandle(canvasY) {
        let y = HEADER_HEIGHT - this.scrollY;
        for (let r = 0; r < MAX_ROWS; r++) {
            y += this.getRowHeight(r);
            if (y > this.canvas.height / this._dpr + 100) break;
            if (Math.abs(canvasY - y) < 4) return r;
        }
        return null;
    }

    handleClick(e) {
        // Handled by mousedown
    }

    handleDoubleClick(e) {
        const rect = this.canvas.getBoundingClientRect();
        const canvasX = e.clientX - rect.left;
        const canvasY = e.clientY - rect.top;

        // Double-click column header border = auto-fit
        if (canvasY < HEADER_HEIGHT) {
            const col = this._hitColumnResizeHandle(canvasX);
            if (col !== null) {
                this._autoFitColumn(col);
                return;
            }
        }

        const cell = this.getCellAt(canvasX, canvasY);
        if (cell) {
            this.startEdit(cell.col, cell.row);
        }
    }

    handleContextMenu(e) {
        e.preventDefault();
        const rect = this.canvas.getBoundingClientRect();
        const canvasX = e.clientX - rect.left;
        const canvasY = e.clientY - rect.top;
        const cell = this.getCellAt(canvasX, canvasY);
        if (!cell) return;

        this.selectedCell = { col: cell.col, row: cell.row };
        this._updateFormulaBar();
        this.render();
        this._showContextMenu(e.clientX - rect.left, e.clientY - rect.top, cell);
    }

    _showContextMenu(x, y, cell) {
        const menu = this._contextMenu;
        menu.innerHTML = '';
        const items = [
            { label: 'Cut', action: () => this.cutCells(), shortcut: 'Ctrl+X' },
            { label: 'Copy', action: () => this.copyCells(), shortcut: 'Ctrl+C' },
            { label: 'Paste', action: () => this.pasteCells(), shortcut: 'Ctrl+V' },
            { label: '---' },
            { label: 'Insert row above', action: () => this.insertRow(cell.row) },
            { label: 'Insert row below', action: () => this.insertRow(cell.row + 1) },
            { label: 'Delete row', action: () => this.deleteRow(cell.row) },
            { label: '---' },
            { label: 'Insert column left', action: () => this.insertColumn(cell.col) },
            { label: 'Insert column right', action: () => this.insertColumn(cell.col + 1) },
            { label: 'Delete column', action: () => this.deleteColumn(cell.col) },
            { label: '---' },
            { label: 'Sort A-Z', action: () => this.sort(cell.col, true) },
            { label: 'Sort Z-A', action: () => this.sort(cell.col, false) },
            { label: '---' },
            { label: this.filterState[cell.col] ? 'Remove filter' : 'Add filter', action: () => {
                if (this.filterState[cell.col]) this.removeFilter(cell.col);
                else this.addFilter(cell.col);
            }},
            { label: '---' },
            { label: `Freeze at ${this.getCellA1(cell.col, cell.row)}`, action: () => this.freezePanes(cell.col, cell.row) },
            { label: 'Unfreeze', action: () => this.freezePanes(0, 0) },
        ];

        for (const item of items) {
            if (item.label === '---') {
                const sep = document.createElement('div');
                sep.className = 'ss-ctx-sep';
                menu.appendChild(sep);
            } else {
                const btn = document.createElement('button');
                btn.className = 'ss-ctx-item';
                btn.textContent = item.label;
                if (item.shortcut) {
                    const sc = document.createElement('span');
                    sc.className = 'ss-ctx-shortcut';
                    sc.textContent = item.shortcut;
                    btn.appendChild(sc);
                }
                btn.addEventListener('click', () => {
                    menu.style.display = 'none';
                    item.action();
                });
                menu.appendChild(btn);
            }
        }

        menu.style.display = 'block';
        menu.style.left = x + 'px';
        menu.style.top = y + 'px';

        // Clamp to viewport
        requestAnimationFrame(() => {
            const mr = menu.getBoundingClientRect();
            const wr = this.canvasWrap.getBoundingClientRect();
            if (mr.right > wr.right) menu.style.left = (x - mr.width) + 'px';
            if (mr.bottom > wr.bottom) menu.style.top = (y - mr.height) + 'px';
        });
    }

    // ─── Keyboard handling ───────────────────────
    handleKeyDown(e) {
        // Ignore if editing cell
        if (this.editingCell) return;

        const { col, row } = this.selectedCell;

        if (e.key === 'ArrowDown') {
            e.preventDefault();
            if (e.ctrlKey || e.metaKey) {
                // Jump to next non-empty cell or last used row
                const sheet = this._sheet();
                const maxR = sheet ? sheet.maxRow : row;
                let newRow = row + 1;
                if (sheet) {
                    const currentCell = sheet.getCell(col, row);
                    const currentEmpty = !currentCell || currentCell.value === '' || currentCell.value === null || currentCell.value === undefined;
                    if (currentEmpty) {
                        // Jump to next non-empty cell
                        for (let r = row + 1; r <= maxR; r++) {
                            const c = sheet.getCell(col, r);
                            if (c && c.value !== '' && c.value !== null && c.value !== undefined) { newRow = r; break; }
                        }
                        if (newRow === row + 1 && (!sheet.getCell(col, newRow) || !sheet.getCell(col, newRow)?.value)) newRow = maxR;
                    } else {
                        // Jump to last non-empty cell in contiguous block, or next empty then next non-empty
                        newRow = maxR;
                        for (let r = row + 1; r <= maxR; r++) {
                            const c = sheet.getCell(col, r);
                            if (!c || c.value === '' || c.value === null || c.value === undefined) { newRow = r > row + 1 ? r - 1 : r; break; }
                        }
                    }
                }
                this.selectedCell.row = Math.min(newRow, MAX_ROWS - 1);
                this.selectionRange = null;
            } else if (e.shiftKey) {
                this._extendSelection(col, Math.min(row + 1, MAX_ROWS - 1));
            } else {
                this.selectedCell.row = Math.min(row + 1, MAX_ROWS - 1);
                this.selectionRange = null;
            }
            this._ensureVisible(this.selectedCell.col, this.selectedCell.row);
            this._updateFormulaBar();
            this.render();
        } else if (e.key === 'ArrowUp') {
            e.preventDefault();
            if (e.ctrlKey || e.metaKey) {
                const sheet = this._sheet();
                let newRow = Math.max(row - 1, 0);
                if (sheet) {
                    const currentCell = sheet.getCell(col, row);
                    const currentEmpty = !currentCell || currentCell.value === '' || currentCell.value === null || currentCell.value === undefined;
                    if (currentEmpty) {
                        newRow = 0;
                        for (let r = row - 1; r >= 0; r--) {
                            const c = sheet.getCell(col, r);
                            if (c && c.value !== '' && c.value !== null && c.value !== undefined) { newRow = r; break; }
                        }
                    } else {
                        newRow = 0;
                        for (let r = row - 1; r >= 0; r--) {
                            const c = sheet.getCell(col, r);
                            if (!c || c.value === '' || c.value === null || c.value === undefined) { newRow = r < row - 1 ? r + 1 : r; break; }
                        }
                    }
                }
                this.selectedCell.row = Math.max(newRow, 0);
                this.selectionRange = null;
            } else if (e.shiftKey) {
                this._extendSelection(col, Math.max(row - 1, 0));
            } else {
                this.selectedCell.row = Math.max(row - 1, 0);
                this.selectionRange = null;
            }
            this._ensureVisible(this.selectedCell.col, this.selectedCell.row);
            this._updateFormulaBar();
            this.render();
        } else if (e.key === 'ArrowRight') {
            e.preventDefault();
            if (e.ctrlKey || e.metaKey) {
                const sheet = this._sheet();
                const maxC = sheet ? sheet.maxCol : col;
                let newCol = col + 1;
                if (sheet) {
                    const currentCell = sheet.getCell(col, row);
                    const currentEmpty = !currentCell || currentCell.value === '' || currentCell.value === null || currentCell.value === undefined;
                    if (currentEmpty) {
                        newCol = maxC;
                        for (let c = col + 1; c <= maxC; c++) {
                            const cell = sheet.getCell(c, row);
                            if (cell && cell.value !== '' && cell.value !== null && cell.value !== undefined) { newCol = c; break; }
                        }
                    } else {
                        newCol = maxC;
                        for (let c = col + 1; c <= maxC; c++) {
                            const cell = sheet.getCell(c, row);
                            if (!cell || cell.value === '' || cell.value === null || cell.value === undefined) { newCol = c > col + 1 ? c - 1 : c; break; }
                        }
                    }
                }
                this.selectedCell.col = Math.min(newCol, MAX_COLS - 1);
                this.selectionRange = null;
            } else if (e.shiftKey) {
                this._extendSelection(Math.min(col + 1, MAX_COLS - 1), row);
            } else {
                this.selectedCell.col = Math.min(col + 1, MAX_COLS - 1);
                this.selectionRange = null;
            }
            this._ensureVisible(this.selectedCell.col, this.selectedCell.row);
            this._updateFormulaBar();
            this.render();
        } else if (e.key === 'ArrowLeft') {
            e.preventDefault();
            if (e.ctrlKey || e.metaKey) {
                const sheet = this._sheet();
                let newCol = Math.max(col - 1, 0);
                if (sheet) {
                    const currentCell = sheet.getCell(col, row);
                    const currentEmpty = !currentCell || currentCell.value === '' || currentCell.value === null || currentCell.value === undefined;
                    if (currentEmpty) {
                        newCol = 0;
                        for (let c = col - 1; c >= 0; c--) {
                            const cell = sheet.getCell(c, row);
                            if (cell && cell.value !== '' && cell.value !== null && cell.value !== undefined) { newCol = c; break; }
                        }
                    } else {
                        newCol = 0;
                        for (let c = col - 1; c >= 0; c--) {
                            const cell = sheet.getCell(c, row);
                            if (!cell || cell.value === '' || cell.value === null || cell.value === undefined) { newCol = c < col - 1 ? c + 1 : c; break; }
                        }
                    }
                }
                this.selectedCell.col = Math.max(newCol, 0);
                this.selectionRange = null;
            } else if (e.shiftKey) {
                this._extendSelection(Math.max(col - 1, 0), row);
            } else {
                this.selectedCell.col = Math.max(col - 1, 0);
                this.selectionRange = null;
            }
            this._ensureVisible(this.selectedCell.col, this.selectedCell.row);
            this._updateFormulaBar();
            this.render();
        } else if (e.key === 'Tab') {
            e.preventDefault();
            if (e.shiftKey) {
                this.selectedCell.col = Math.max(col - 1, 0);
            } else {
                this.selectedCell.col = Math.min(col + 1, MAX_COLS - 1);
            }
            this.selectionRange = null;
            this._ensureVisible(this.selectedCell.col, this.selectedCell.row);
            this._updateFormulaBar();
            this.render();
        } else if (e.key === 'Enter') {
            e.preventDefault();
            if (e.shiftKey) {
                this.selectedCell.row = Math.max(row - 1, 0);
            } else {
                this.selectedCell.row = Math.min(row + 1, MAX_ROWS - 1);
            }
            this.selectionRange = null;
            this._ensureVisible(this.selectedCell.col, this.selectedCell.row);
            this._updateFormulaBar();
            this.render();
        } else if (e.key === 'F2') {
            e.preventDefault();
            this.startEdit(col, row);
        } else if (e.key === 'Delete' || e.key === 'Backspace') {
            e.preventDefault();
            if (this.selectionRange) {
                const nr = this._normalizeRange(this.selectionRange);
                for (let c = nr.startCol; c <= nr.endCol; c++) {
                    for (let r = nr.startRow; r <= nr.endRow; r++) {
                        this._setCellValue(c, r, '');
                    }
                }
            } else {
                this._setCellValue(col, row, '');
            }
            this._updateFormulaBar();
            this.render();
        } else if (e.key === 'PageDown') {
            e.preventDefault();
            const viewportRows = Math.floor((this.canvas.height / this._dpr - HEADER_HEIGHT) / CELL_HEIGHT);
            this.selectedCell.row = Math.min(this.selectedCell.row + viewportRows, MAX_ROWS - 1);
            this.selectionRange = null;
            this._ensureVisible(this.selectedCell.col, this.selectedCell.row);
            this._updateFormulaBar();
            this.render();
        } else if (e.key === 'PageUp') {
            e.preventDefault();
            const viewportRows2 = Math.floor((this.canvas.height / this._dpr - HEADER_HEIGHT) / CELL_HEIGHT);
            this.selectedCell.row = Math.max(this.selectedCell.row - viewportRows2, 0);
            this.selectionRange = null;
            this._ensureVisible(this.selectedCell.col, this.selectedCell.row);
            this._updateFormulaBar();
            this.render();
        } else if ((e.ctrlKey || e.metaKey) && e.key === 'Home') {
            e.preventDefault();
            this.selectedCell = { col: 0, row: 0 };
            this.selectionRange = null;
            this.scrollX = 0;
            this.scrollY = 0;
            this._updateFormulaBar();
            this.render();
        } else if ((e.ctrlKey || e.metaKey) && e.key === 'End') {
            e.preventDefault();
            const sheet2 = this._sheet();
            if (sheet2) {
                this.selectedCell = { col: sheet2.maxCol, row: sheet2.maxRow };
                this.selectionRange = null;
                this._ensureVisible(this.selectedCell.col, this.selectedCell.row);
                this._updateFormulaBar();
            }
            this.render();
        } else if (e.key === 'Escape') {
            this.selectionRange = null;
            this.render();
        } else if ((e.ctrlKey || e.metaKey) && e.key === 'c') {
            e.preventDefault();
            this.copyCells();
        } else if ((e.ctrlKey || e.metaKey) && e.key === 'x') {
            e.preventDefault();
            this.cutCells();
        } else if ((e.ctrlKey || e.metaKey) && e.key === 'v') {
            e.preventDefault();
            this.pasteCells();
        } else if ((e.ctrlKey || e.metaKey) && e.key === 'z') {
            e.preventDefault();
            this.undo();
        } else if ((e.ctrlKey || e.metaKey) && (e.key === 'y' || (e.shiftKey && e.key === 'Z'))) {
            e.preventDefault();
            this.redo();
        } else if ((e.ctrlKey || e.metaKey) && e.key === 'a') {
            e.preventDefault();
            // Select all
            const sheet = this._sheet();
            if (sheet) {
                this.selectionRange = { startCol: 0, startRow: 0, endCol: sheet.maxCol, endRow: sheet.maxRow };
                this.render();
            }
        } else if (e.key.length === 1 && !e.ctrlKey && !e.metaKey && !e.altKey) {
            // Start editing with typed character
            this.startEdit(col, row, e.key);
        }
    }

    _extendSelection(col, row) {
        if (!this.selectionRange) {
            this.selectionRange = {
                startCol: this.selectedCell.col, startRow: this.selectedCell.row,
                endCol: col, endRow: row
            };
        } else {
            this.selectionRange.endCol = col;
            this.selectionRange.endRow = row;
        }
        this.selectedCell = { col, row };
    }

    // ─── Scroll ──────────────────────────────────
    handleScroll(e) {
        e.preventDefault();
        const dx = e.deltaX || 0;
        const dy = e.deltaY || 0;

        this.scrollX = Math.max(0, this.scrollX + dx);
        this.scrollY = Math.max(0, this.scrollY + dy);

        this.render();
    }

    _ensureVisible(col, row) {
        const canvasW = this.canvas.width / this._dpr;
        const canvasH = this.canvas.height / this._dpr;
        const cellX = this._colX(col);
        const cellY = this._rowY(row);
        const cellW = this.getColumnWidth(col);
        const cellH = this.getRowHeight(row);

        // Horizontal
        if (cellX - this.scrollX + ROW_HEADER_WIDTH < ROW_HEADER_WIDTH) {
            this.scrollX = cellX;
        } else if (cellX + cellW - this.scrollX + ROW_HEADER_WIDTH > canvasW) {
            this.scrollX = cellX + cellW - canvasW + ROW_HEADER_WIDTH;
        }

        // Vertical
        if (cellY - this.scrollY + HEADER_HEIGHT < HEADER_HEIGHT) {
            this.scrollY = cellY;
        } else if (cellY + cellH - this.scrollY + HEADER_HEIGHT > canvasH) {
            this.scrollY = cellY + cellH - canvasH + HEADER_HEIGHT;
        }
    }

    // ─── Selection ───────────────────────────────
    selectRange(startCol, startRow, endCol, endRow) {
        this.selectionRange = { startCol, startRow, endCol, endRow };
        this.selectedCell = { col: endCol, row: endRow };
        this._updateFormulaBar();
        this.render();
    }

    _normalizeRange(range) {
        return {
            startCol: Math.min(range.startCol, range.endCol),
            startRow: Math.min(range.startRow, range.endRow),
            endCol: Math.max(range.startCol, range.endCol),
            endRow: Math.max(range.startRow, range.endRow),
        };
    }

    // ─── Editing ─────────────────────────────────
    startEdit(col, row, initialChar) {
        this.editingCell = { col, row };
        this.selectedCell = { col, row };
        this.selectionRange = null;

        const sx = this._cellScreenX(col);
        const sy = this._cellScreenY(row);
        const sw = this.getColumnWidth(col);
        const sh = this.getRowHeight(row);

        const wrapRect = this.canvasWrap.getBoundingClientRect();

        this.editInput.style.display = 'block';
        this.editInput.style.left = sx + 'px';
        this.editInput.style.top = sy + 'px';
        this.editInput.style.width = Math.max(sw, 60) + 'px';
        this.editInput.style.height = sh + 'px';
        this.editInput.style.fontSize = '13px';

        if (initialChar !== undefined) {
            this.editInput.value = initialChar;
        } else {
            this.editInput.value = this._getCellRawValue(col, row);
        }

        this.editInput.focus();
        if (initialChar !== undefined) {
            // Place cursor at end
            this.editInput.selectionStart = this.editInput.selectionEnd = this.editInput.value.length;
        } else {
            this.editInput.select();
        }
    }

    commitEdit() {
        if (!this.editingCell) return;
        const { col, row } = this.editingCell;
        const val = this.editInput.value;
        this._setCellValue(col, row, val);
        this.editingCell = null;
        this.editInput.style.display = 'none';
        this._updateFormulaBar();
        this.render();
    }

    cancelEdit() {
        this.editingCell = null;
        this.editInput.style.display = 'none';
        this.canvas.focus();
        this.render();
    }

    // ─── Resize ──────────────────────────────────
    resizeColumn(col, width) {
        const sheet = this._sheet();
        if (!sheet) return;
        sheet.colWidths[col] = Math.max(30, width);
        this.render();
    }

    resizeRow(row, height) {
        const sheet = this._sheet();
        if (!sheet) return;
        sheet.rowHeights[row] = Math.max(16, height);
        this.render();
    }

    _autoFitColumn(col) {
        const sheet = this._sheet();
        if (!sheet) return;
        const ctx = this.ctx;
        ctx.font = '13px Arial, sans-serif';
        let maxWidth = 40;
        for (let r = 0; r <= sheet.maxRow; r++) {
            const cell = sheet.getCell(col, r);
            if (cell) {
                const display = cell.formula
                    ? String(evaluateFormula(cell.formula, sheet))
                    : (cell.display || String(cell.value ?? ''));
                const tw = ctx.measureText(display).width + 12;
                if (tw > maxWidth) maxWidth = tw;
            }
        }
        // Also measure header
        const headerW = ctx.measureText(this.getCellA1Col(col)).width + 20;
        if (headerW > maxWidth) maxWidth = headerW;
        sheet.colWidths[col] = Math.ceil(maxWidth);
        this.render();
    }

    resizeCanvas() {
        const dpr = window.devicePixelRatio || 1;
        this._dpr = dpr;
        const w = this.canvasWrap?.clientWidth || this.container.clientWidth;
        const h = this.canvasWrap?.clientHeight || (this.container.clientHeight - 60);
        this.canvas.width = w * dpr;
        this.canvas.height = h * dpr;
        this.canvas.style.width = w + 'px';
        this.canvas.style.height = h + 'px';
    }

    // ─── Row/Column operations ───────────────────
    insertRow(afterRow) {
        const sheet = this._sheet();
        if (!sheet) return;
        // Shift all rows down from afterRow
        for (let r = sheet.maxRow; r >= afterRow; r--) {
            for (let c = 0; c <= sheet.maxCol; c++) {
                const cell = sheet.getCell(c, r);
                if (cell) {
                    sheet.setCell(c, r + 1, { ...cell });
                } else {
                    sheet.deleteCell(c, r + 1);
                }
            }
        }
        // Clear the new row
        for (let c = 0; c <= sheet.maxCol; c++) {
            sheet.deleteCell(c, afterRow);
        }
        sheet.maxRow++;
        this._undoManager.push({ type: 'insertRow', row: afterRow, sheetIndex: this.activeSheet });
        this.render();
    }

    deleteRow(row) {
        const sheet = this._sheet();
        if (!sheet) return;
        // Save row data for undo
        const rowData = {};
        for (let c = 0; c <= sheet.maxCol; c++) {
            const cell = sheet.getCell(c, row);
            if (cell) rowData[c] = { ...cell };
        }
        // Shift rows up
        for (let r = row; r <= sheet.maxRow; r++) {
            for (let c = 0; c <= sheet.maxCol; c++) {
                const below = sheet.getCell(c, r + 1);
                if (below) {
                    sheet.setCell(c, r, { ...below });
                } else {
                    sheet.deleteCell(c, r);
                }
            }
        }
        if (sheet.maxRow > 0) sheet.maxRow--;
        this._undoManager.push({ type: 'deleteRow', row, rowData, sheetIndex: this.activeSheet });
        if (this.selectedCell.row > sheet.maxRow) this.selectedCell.row = sheet.maxRow;
        this.render();
    }

    insertColumn(afterCol) {
        const sheet = this._sheet();
        if (!sheet) return;
        for (let c = sheet.maxCol; c >= afterCol; c--) {
            for (let r = 0; r <= sheet.maxRow; r++) {
                const cell = sheet.getCell(c, r);
                if (cell) {
                    sheet.setCell(c + 1, r, { ...cell });
                } else {
                    sheet.deleteCell(c + 1, r);
                }
            }
        }
        for (let r = 0; r <= sheet.maxRow; r++) {
            sheet.deleteCell(afterCol, r);
        }
        sheet.maxCol++;
        this._undoManager.push({ type: 'insertCol', col: afterCol, sheetIndex: this.activeSheet });
        this.render();
    }

    deleteColumn(col) {
        const sheet = this._sheet();
        if (!sheet) return;
        const colData = {};
        for (let r = 0; r <= sheet.maxRow; r++) {
            const cell = sheet.getCell(col, r);
            if (cell) colData[r] = { ...cell };
        }
        for (let c = col; c <= sheet.maxCol; c++) {
            for (let r = 0; r <= sheet.maxRow; r++) {
                const right = sheet.getCell(c + 1, r);
                if (right) {
                    sheet.setCell(c, r, { ...right });
                } else {
                    sheet.deleteCell(c, r);
                }
            }
        }
        if (sheet.maxCol > 0) sheet.maxCol--;
        this._undoManager.push({ type: 'deleteCol', col, colData, sheetIndex: this.activeSheet });
        if (this.selectedCell.col > sheet.maxCol) this.selectedCell.col = sheet.maxCol;
        this.render();
    }

    // ─── Sort ────────────────────────────────────
    sort(col, ascending, hasHeader = true) {
        const sheet = this._sheet();
        if (!sheet) return;

        // Save pre-sort state for undo
        const previousCells = JSON.parse(JSON.stringify(sheet.cells));

        // Collect all row data, optionally skipping header row
        const startRow = hasHeader ? 1 : 0;
        const rows = [];
        for (let r = startRow; r <= sheet.maxRow; r++) {
            const rowData = {};
            for (let c = 0; c <= sheet.maxCol; c++) {
                const cell = sheet.getCell(c, r);
                if (cell) rowData[c] = { ...cell };
            }
            const sortCell = sheet.getCell(col, r);
            const sortVal = sortCell ? (sortCell.value ?? '') : '';
            rows.push({ index: r, data: rowData, sortVal });
        }

        rows.sort((a, b) => {
            let va = a.sortVal, vb = b.sortVal;
            if (typeof va === 'number' && typeof vb === 'number') {
                return ascending ? va - vb : vb - va;
            }
            va = String(va).toLowerCase();
            vb = String(vb).toLowerCase();
            if (va < vb) return ascending ? -1 : 1;
            if (va > vb) return ascending ? 1 : -1;
            return 0;
        });

        // Write back sorted rows (header row stays at row 0 if hasHeader)
        for (let i = 0; i < rows.length; i++) {
            const r = startRow + i;
            for (let c = 0; c <= sheet.maxCol; c++) {
                if (rows[i].data[c]) {
                    sheet.setCell(c, r, rows[i].data[c]);
                } else {
                    sheet.deleteCell(c, r);
                }
            }
        }

        // Push undo entry
        this._undoManager.push({
            type: 'sort',
            sheetIndex: this.activeSheet,
            previousCells: previousCells
        });

        this.sortState = { col, asc: ascending };
        this.render();
    }

    // ─── Filter ──────────────────────────────────
    addFilter(col) {
        const sheet = this._sheet();
        if (!sheet) return;
        // Collect unique values (skip header row 0)
        const values = new Set();
        for (let r = 1; r <= sheet.maxRow; r++) {
            const cell = sheet.getCell(col, r);
            values.add(cell ? String(cell.value ?? '') : '');
        }
        this.filterState[col] = { values, active: new Set(values) };
        this._recomputeHiddenRows();
        this.render();
    }

    removeFilter(col) {
        delete this.filterState[col];
        this._recomputeHiddenRows();
        this.render();
    }

    _recomputeHiddenRows() {
        this.hiddenRows = new Set();
        const filterCols = Object.keys(this.filterState).map(Number);
        if (filterCols.length === 0) return;

        const sheet = this._sheet();
        if (!sheet) return;

        // Row 0 is header, never hidden
        for (let r = 1; r <= sheet.maxRow; r++) {
            let visible = true;
            for (const col of filterCols) {
                const filter = this.filterState[col];
                if (!filter || !filter.active) continue;
                const cell = sheet.getCell(col, r);
                const val = cell ? String(cell.value ?? '') : '';
                if (!filter.active.has(val)) {
                    visible = false;
                    break;
                }
            }
            if (!visible) {
                this.hiddenRows.add(r);
            }
        }
    }

    // ─── Freeze Panes ───────────────────────────
    freezePanes(col, row) {
        this.frozenCols = col;
        this.frozenRows = row;
        this.render();
    }

    // ─── Sheet Tab Management ────────────────────
    addSheet() {
        if (!this.workbook) return;
        const name = 'Sheet' + (this.workbook.sheets.length + 1);
        this.workbook.sheets.push(new Sheet(name));
        this.activeSheet = this.workbook.sheets.length - 1;
        this.selectedCell = { col: 0, row: 0 };
        this.selectionRange = null;
        this.scrollX = 0;
        this.scrollY = 0;
        this.updateSheetTabs();
        this.render();
    }

    deleteSheet(index) {
        if (!this.workbook || this.workbook.sheets.length <= 1) return;
        this.workbook.sheets.splice(index, 1);
        if (this.activeSheet >= this.workbook.sheets.length) {
            this.activeSheet = this.workbook.sheets.length - 1;
        }
        this.selectedCell = { col: 0, row: 0 };
        this.selectionRange = null;
        this.scrollX = 0;
        this.scrollY = 0;
        this.updateSheetTabs();
        this.render();
    }

    renameSheet(index, name) {
        if (!this.workbook || !this.workbook.sheets[index]) return;
        this.workbook.sheets[index].name = name;
        this.updateSheetTabs();
    }

    updateSheetTabs() {
        if (!this.tabBar || !this.workbook) return;
        this.tabBar.innerHTML = '';

        for (let i = 0; i < this.workbook.sheets.length; i++) {
            const sheet = this.workbook.sheets[i];
            const tab = document.createElement('button');
            tab.className = 'ss-tab' + (i === this.activeSheet ? ' active' : '');
            tab.textContent = sheet.name;
            tab.title = `Switch to ${sheet.name}`;
            tab.addEventListener('click', () => {
                if (this.editingCell) this.commitEdit();
                this.activeSheet = i;
                this.selectedCell = { col: 0, row: 0 };
                this.selectionRange = null;
                this.scrollX = 0;
                this.scrollY = 0;
                this.updateSheetTabs();
                this._updateFormulaBar();
                this.render();
            });
            tab.addEventListener('dblclick', () => {
                const newName = prompt('Rename sheet:', sheet.name);
                if (newName && newName.trim()) {
                    this.renameSheet(i, newName.trim());
                }
            });
            tab.addEventListener('contextmenu', (e) => {
                e.preventDefault();
                if (this.workbook.sheets.length > 1) {
                    if (confirm(`Delete sheet "${sheet.name}"?`)) {
                        this.deleteSheet(i);
                    }
                }
            });
            this.tabBar.appendChild(tab);
        }

        const addBtn = document.createElement('button');
        addBtn.className = 'ss-tab-add';
        addBtn.textContent = '+';
        addBtn.title = 'Add new sheet';
        addBtn.addEventListener('click', () => this.addSheet());
        this.tabBar.appendChild(addBtn);
    }

    // ─── Cell reference helpers ──────────────────
    getCellA1(col, row) {
        return this.getCellA1Col(col) + String(row + 1);
    }

    getCellA1Col(col) {
        let result = '';
        let c = col;
        while (c >= 0) {
            result = String.fromCharCode(65 + (c % 26)) + result;
            c = Math.floor(c / 26) - 1;
        }
        return result;
    }

    // ─── Formula bar update ──────────────────────
    _updateFormulaBar() {
        if (this.cellRefLabel) {
            this.cellRefLabel.textContent = this.getCellA1(this.selectedCell.col, this.selectedCell.row);
        }
        if (this.formulaInput) {
            this.formulaInput.value = this._getCellRawValue(this.selectedCell.col, this.selectedCell.row);
        }
    }

    // ─── Clipboard ───────────────────────────────
    copyCells() {
        const sheet = this._sheet();
        if (!sheet) return;

        const range = this.selectionRange
            ? this._normalizeRange(this.selectionRange)
            : { startCol: this.selectedCell.col, startRow: this.selectedCell.row, endCol: this.selectedCell.col, endRow: this.selectedCell.row };

        const cells = {};
        for (let c = range.startCol; c <= range.endCol; c++) {
            for (let r = range.startRow; r <= range.endRow; r++) {
                const cell = sheet.getCell(c, r);
                if (cell) cells[`${c - range.startCol},${r - range.startRow}`] = { ...cell };
            }
        }

        this._clipboard = {
            cells,
            width: range.endCol - range.startCol + 1,
            height: range.endRow - range.startRow + 1,
            cut: false
        };

        // Also write plain text to system clipboard
        const lines = [];
        for (let r = range.startRow; r <= range.endRow; r++) {
            const cols = [];
            for (let c = range.startCol; c <= range.endCol; c++) {
                cols.push(this._getCellDisplay(c, r));
            }
            lines.push(cols.join('\t'));
        }
        try {
            navigator.clipboard.writeText(lines.join('\n'));
        } catch (_) { /* clipboard API may not be available */ }
    }

    cutCells() {
        this.copyCells();
        if (this._clipboard) this._clipboard.cut = true;
        // Clear source cells
        const range = this.selectionRange
            ? this._normalizeRange(this.selectionRange)
            : { startCol: this.selectedCell.col, startRow: this.selectedCell.row, endCol: this.selectedCell.col, endRow: this.selectedCell.row };

        for (let c = range.startCol; c <= range.endCol; c++) {
            for (let r = range.startRow; r <= range.endRow; r++) {
                this._setCellValue(c, r, '');
            }
        }
        this.render();
    }

    pasteCells() {
        if (!this._clipboard) {
            // Try system clipboard
            navigator.clipboard.readText().then(text => {
                if (!text) return;
                const lines = text.split('\n');
                const { col, row } = this.selectedCell;
                for (let r = 0; r < lines.length; r++) {
                    const cols = lines[r].split('\t');
                    for (let c = 0; c < cols.length; c++) {
                        this._setCellValue(col + c, row + r, cols[c]);
                    }
                }
                this.render();
            }).catch(() => {});
            return;
        }

        const { col, row } = this.selectedCell;
        const { cells, width, height } = this._clipboard;

        for (let c = 0; c < width; c++) {
            for (let r = 0; r < height; r++) {
                const src = cells[`${c},${r}`];
                if (src) {
                    const rawVal = src.formula || String(src.value ?? '');
                    this._setCellValue(col + c, row + r, rawVal);
                } else {
                    this._setCellValue(col + c, row + r, '');
                }
            }
        }
        this.render();
    }

    // ─── Undo / Redo ─────────────────────────────
    undo() {
        this._undoManager.undo(this.workbook);
        this._updateFormulaBar();
        this.render();
    }

    redo() {
        this._undoManager.redo(this.workbook);
        this._updateFormulaBar();
        this.render();
    }

    // ─── Auto Fill ───────────────────────────────
    autoFill(range, direction) {
        const sheet = this._sheet();
        if (!sheet) return;

        const nr = this._normalizeRange(range);
        // Simple auto-fill: repeat existing cell values in the direction
        if (direction === 'down') {
            const srcRow = nr.startRow;
            for (let r = nr.startRow + 1; r <= nr.endRow; r++) {
                for (let c = nr.startCol; c <= nr.endCol; c++) {
                    const srcCell = sheet.getCell(c, srcRow);
                    if (srcCell && srcCell.type === 'number') {
                        // Increment
                        const increment = r - srcRow;
                        this._setCellValue(c, r, String(srcCell.value + increment));
                    } else if (srcCell) {
                        this._setCellValue(c, r, String(srcCell.value ?? ''));
                    }
                }
            }
        } else if (direction === 'right') {
            const srcCol = nr.startCol;
            for (let c = nr.startCol + 1; c <= nr.endCol; c++) {
                for (let r = nr.startRow; r <= nr.endRow; r++) {
                    const srcCell = sheet.getCell(srcCol, r);
                    if (srcCell && srcCell.type === 'number') {
                        const increment = c - srcCol;
                        this._setCellValue(c, r, String(srcCell.value + increment));
                    } else if (srcCell) {
                        this._setCellValue(c, r, String(srcCell.value ?? ''));
                    }
                }
            }
        }
        this.render();
    }

    // ─── Export ───────────────────────────────────
    exportCSV() {
        const sheet = this._sheet();
        if (!sheet) return '';
        return generateCSV(sheet);
    }

    downloadCSV(filename) {
        const csv = this.exportCSV();
        const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = (filename || 'spreadsheet') + '.csv';
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
    }

    // ─── Destroy ─────────────────────────────────
    destroy() {
        if (this._resizeObserver) {
            this._resizeObserver.disconnect();
            this._resizeObserver = null;
        }
        if (this._rafId) {
            cancelAnimationFrame(this._rafId);
            this._rafId = null;
        }
        this.container.innerHTML = '';
    }
}
