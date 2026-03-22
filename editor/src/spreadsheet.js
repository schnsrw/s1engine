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

// ─── Custom modal helpers (replace browser prompt/confirm) ──
function ssPrompt(message, defaultValue) {
    return new Promise(function(resolve) {
        var overlay = document.createElement('div');
        overlay.className = 'modal-overlay show';
        var modal = document.createElement('div');
        modal.className = 'modal';
        // Build DOM safely — use textContent for user-provided message to prevent XSS
        var h3 = document.createElement('h3');
        h3.textContent = message;
        var fieldDiv = document.createElement('div');
        fieldDiv.className = 'modal-field';
        var input = document.createElement('input');
        input.type = 'text';
        input.className = 'ss-modal-input';
        input.value = defaultValue || '';
        input.style.cssText = 'width:100%;padding:6px 8px;border:1px solid #dadce0;border-radius:4px;font-size:13px;';
        fieldDiv.appendChild(input);
        var actionsDiv = document.createElement('div');
        actionsDiv.className = 'modal-actions';
        var cancelBtn = document.createElement('button');
        cancelBtn.className = 'ss-modal-cancel';
        cancelBtn.textContent = 'Cancel';
        var okBtn = document.createElement('button');
        okBtn.className = 'ss-modal-ok primary';
        okBtn.textContent = 'OK';
        actionsDiv.appendChild(cancelBtn);
        actionsDiv.appendChild(okBtn);
        modal.appendChild(h3);
        modal.appendChild(fieldDiv);
        modal.appendChild(actionsDiv);
        overlay.appendChild(modal);
        document.body.appendChild(overlay);
        input.focus();
        input.select();
        function close(val) { document.body.removeChild(overlay); resolve(val); }
        cancelBtn.onclick = function() { close(null); };
        okBtn.onclick = function() { close(input.value); };
        input.onkeydown = function(e) { if (e.key === 'Enter') close(input.value); if (e.key === 'Escape') close(null); };
        overlay.onclick = function(e) { if (e.target === overlay) close(null); };
    });
}
function ssAlert(title, message) {
    return new Promise(function(resolve) {
        var overlay = document.createElement('div');
        overlay.className = 'modal-overlay show';
        var modal = document.createElement('div');
        modal.className = 'modal';
        var safeTitle = String(title).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
        // Render basic markdown: bold, code, bullets, newlines
        var rendered = String(message)
            .replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
            .replace(/```(\w*)\n([\s\S]*?)```/g, '<pre style="background:#f1f3f4;padding:8px;border-radius:4px;font-size:12px;overflow-x:auto;margin:6px 0"><code>$2</code></pre>')
            .replace(/`([^`]+)`/g, '<code style="background:#f1f3f4;padding:1px 4px;border-radius:3px;font-size:12px">$1</code>')
            .replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
            .replace(/^\s*[-*]\s+(.+)/gm, '<li style="margin-left:16px;list-style:disc">$1</li>')
            .replace(/^\s*(\d+)\.\s+(.+)/gm, '<li style="margin-left:16px;list-style:decimal">$2</li>')
            .replace(/\n/g, '<br>');
        modal.innerHTML = '<h3>' + safeTitle + '</h3>' +
            '<div class="modal-field" style="max-height:300px;overflow-y:auto;padding:10px 12px;background:var(--bg-app,#f8f9fa);border:1px solid var(--border-light,#dadce0);color:var(--text-primary,#202124);border-radius:4px;font-size:13px;line-height:1.6;user-select:text;cursor:text;">' + rendered + '</div>' +
            '<div class="modal-actions"><button class="ss-modal-cancel" style="margin-right:auto">Copy</button><button class="ss-modal-ok primary">OK</button></div>';
        overlay.appendChild(modal);
        document.body.appendChild(overlay);
        function close() { document.body.removeChild(overlay); resolve(); }
        modal.querySelector('.ss-modal-ok').onclick = close;
        modal.querySelector('.ss-modal-cancel').onclick = function() {
            navigator.clipboard.writeText(String(message)).then(function() {
                var btn = modal.querySelector('.ss-modal-cancel');
                btn.textContent = 'Copied';
                setTimeout(function() { btn.textContent = 'Copy'; }, 1500);
            });
        };
        overlay.onclick = function(e) { if (e.target === overlay) close(); };
        modal.querySelector('.ss-modal-ok').focus();
    });
}
function ssConfirm(message) {
    return new Promise(function(resolve) {
        var overlay = document.createElement('div');
        overlay.className = 'modal-overlay show';
        var modal = document.createElement('div');
        modal.className = 'modal';
        // Build DOM safely — use textContent for user-provided message to prevent XSS
        var h3 = document.createElement('h3');
        h3.textContent = message;
        var actionsDiv = document.createElement('div');
        actionsDiv.className = 'modal-actions';
        var cancelBtn = document.createElement('button');
        cancelBtn.className = 'ss-modal-cancel';
        cancelBtn.textContent = 'Cancel';
        var okBtn = document.createElement('button');
        okBtn.className = 'ss-modal-ok primary';
        okBtn.textContent = 'OK';
        actionsDiv.appendChild(cancelBtn);
        actionsDiv.appendChild(okBtn);
        modal.appendChild(h3);
        modal.appendChild(actionsDiv);
        overlay.appendChild(modal);
        document.body.appendChild(overlay);
        function close(val) { document.body.removeChild(overlay); resolve(val); }
        cancelBtn.onclick = function() { close(false); };
        okBtn.onclick = function() { close(true); };
        overlay.onclick = function(e) { if (e.target === overlay) close(false); };
        okBtn.focus();
    });
}

// ─── Workbook data model (in-memory) ───────────────────
class Sheet {
    constructor(name) {
        this.name = name;
        this.cells = {};         // 'col,row' -> { value, formula, display, type, style }
        this.colWidths = {};     // col index -> px
        this.rowHeights = {};    // row index -> px
        this.merges = [];        // [{ startCol, startRow, endCol, endRow }]
        this.charts = [];        // chart objects managed by spreadsheet-charts module
        this.namedRanges = {};   // name -> 'A1:A100' range string
        this.images = [];        // [{ data, x, y, width, height, id }]
        this.shapes = [];        // [{ id, type, x, y, width, height, text, style }]
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

// ─── Safe arithmetic parser (recursive descent — no eval/Function) ──
function safeEvalArithmetic(expr) {
    let pos = 0;
    const str = expr.replace(/\s+/g, '');

    function parseExpr() {
        let left = parseTerm();
        while (pos < str.length && (str[pos] === '+' || str[pos] === '-')) {
            const op = str[pos++];
            const right = parseTerm();
            left = op === '+' ? left + right : left - right;
        }
        return left;
    }

    function parseTerm() {
        let left = parseFactor();
        while (pos < str.length && (str[pos] === '*' || str[pos] === '/')) {
            const op = str[pos++];
            const right = parseFactor();
            left = op === '*' ? left * right : (right !== 0 ? left / right : '#DIV/0!');
        }
        return left;
    }

    function parseFactor() {
        if (str[pos] === '-') { pos++; return -parseFactor(); }
        if (str[pos] === '+') { pos++; return parseFactor(); }
        if (str[pos] === '(') {
            pos++; // skip (
            const val = parseExpr();
            if (str[pos] === ')') pos++; // skip )
            return val;
        }
        // Parse number (including decimals)
        let numStr = '';
        while (pos < str.length && (str[pos] >= '0' && str[pos] <= '9' || str[pos] === '.')) {
            numStr += str[pos++];
        }
        return numStr ? parseFloat(numStr) : NaN;
    }

    const result = parseExpr();
    return pos === str.length ? result : NaN;
}

// ─── Simple formula evaluator ──────────────────────────
function evaluateFormula(formula, sheet, allSheets, visitedCells) {
    if (!formula) return formula;
    if (!visitedCells) visitedCells = new Set();
    if (visitedCells.size > 1000) return '#CIRC!'; // Safety net for deep chains
    // S5.4: Array formula support — {=FORMULA} syntax
    let isArray = false;
    let workFormula = formula;
    if (workFormula.startsWith('{=') && workFormula.endsWith('}')) {
        isArray = true;
        workFormula = '=' + workFormula.slice(2, -1);
    }
    if (workFormula[0] !== '=') return formula;
    const expr = workFormula.slice(1).trim();
    try {
        // Handle SPARKLINE formula: =SPARKLINE(A1:A10, "line") or =SPARKLINE(A1:A10, "bar")
        const sparklineMatch = expr.match(/^SPARKLINE\(([^,]+),\s*"(line|bar)"\)$/i);
        if (sparklineMatch) {
            // Return a sparkline descriptor (rendered at draw time)
            return { __sparkline: true, range: sparklineMatch[1].trim(), type: sparklineMatch[2].toLowerCase() };
        }

        // Handle cross-sheet references: Sheet1!A1:B5 or Sheet1!A1
        let resolvedExpr = expr;
        if (allSheets) {
            resolvedExpr = resolvedExpr.replace(/([A-Za-z_][\w]*)!([A-Z]{1,3}\d{1,7}(?::[A-Z]{1,3}\d{1,7})?)/gi, (_match, sheetName, ref) => {
                const targetSheet = allSheets.find(s => s.name === sheetName);
                if (!targetSheet) return '0';
                // If it's a range, return the range evaluated on the target sheet
                const rangeMatch = ref.match(/^([A-Z]{1,3})(\d{1,7}):([A-Z]{1,3})(\d{1,7})$/i);
                if (rangeMatch) {
                    const vals = parseRange(ref, targetSheet);
                    return vals.length > 0 ? vals.reduce((a, b) => a + b, 0) : '0';
                }
                // Single cell
                const cellMatch = ref.match(/^([A-Z]{1,3})(\d{1,7})$/i);
                if (cellMatch) {
                    const col = colLetterToIndex(cellMatch[1].toUpperCase());
                    const row = parseInt(cellMatch[2], 10) - 1;
                    const cell = targetSheet.getCell(col, row);
                    if (!cell) return '0';
                    // Recursively evaluate formula with circular reference detection
                    if (cell.formula) {
                        const cellKey = `${sheetName}!${cellMatch[1].toUpperCase()}${cellMatch[2]}`;
                        if (visitedCells.has(cellKey)) return '#CIRC!';
                        visitedCells.add(cellKey);
                        const result = evaluateFormula(cell.formula, targetSheet, allSheets, visitedCells);
                        visitedCells.delete(cellKey);
                        if (typeof result === 'number') return String(result);
                        const rn = Number(result);
                        return isNaN(rn) ? '0' : String(rn);
                    }
                    const v = cell.value;
                    if (v === null || v === undefined || v === '') return '0';
                    if (typeof v === 'number') return String(v);
                    const n = Number(v);
                    return isNaN(n) ? '0' : String(n);
                }
                return '0';
            });
        }

        // Replace named ranges with their cell range strings (with cycle detection)
        if (sheet && sheet.namedRanges) {
            const visited = new Set();
            let changed = true;
            let iterations = 0;
            while (changed && iterations < 10) {
                changed = false;
                iterations++;
                for (const [name, rangeStr] of Object.entries(sheet.namedRanges)) {
                    if (visited.has(name)) continue;
                    const re = new RegExp('\\b' + name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&') + '\\b', 'gi');
                    if (re.test(resolvedExpr)) {
                        visited.add(name);
                        resolvedExpr = resolvedExpr.replace(re, rangeStr);
                        changed = true;
                    }
                }
            }
        }

        // Replace cell references (A1, B2, etc.) with their values
        const replaced = resolvedExpr.replace(/\b([A-Z]{1,3})(\d{1,7})\b/gi, (_match, colLetter, rowNum) => {
            const col = colLetterToIndex(colLetter.toUpperCase());
            const row = parseInt(rowNum, 10) - 1;
            const cell = sheet.getCell(col, row);
            if (!cell) return '0';
            // Recursively evaluate formula with circular reference detection
            if (cell.formula) {
                const cellKey = `!${colLetter.toUpperCase()}${rowNum}`;
                if (visitedCells.has(cellKey)) return '#CIRC!';
                visitedCells.add(cellKey);
                const result = evaluateFormula(cell.formula, sheet, allSheets, visitedCells);
                visitedCells.delete(cellKey);
                if (typeof result === 'number') return String(result);
                const rn = Number(result);
                return isNaN(rn) ? '0' : String(rn);
            }
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

        // Simple arithmetic evaluation (safe recursive descent parser)
        // Only allow numbers, operators, parentheses — no Function() constructor
        if (/^[\d\s+\-*/.()]+$/.test(replaced)) {
            const result = safeEvalArithmetic(replaced);
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

// ─── Number format helper ─────────────────────────────
function formatCellValue(value, format) {
    if (value === null || value === undefined || value === '') return '';
    if (!format || format === 'general') return String(value);
    const num = Number(value);
    if (format === 'number') {
        if (isNaN(num)) return String(value);
        return num.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 });
    }
    if (format === 'currency') {
        if (isNaN(num)) return String(value);
        return '$' + num.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 });
    }
    if (format === 'percentage') {
        if (isNaN(num)) return String(value);
        return (num * 100).toFixed(1) + '%';
    }
    if (format === 'date') {
        if (isNaN(num)) return String(value);
        // S13: Excel 1900 date system bug — serial 60 = Feb 29 1900 (doesn't exist).
        // Excel incorrectly treats 1900 as a leap year, so serials > 59 are off by one day.
        let adjusted = num;
        if (num > 59) adjusted -= 1;
        const d = new Date((adjusted - 25569) * 86400000);
        return d.toLocaleDateString();
    }
    if (format === 'time') {
        if (isNaN(num)) return String(value);
        // Fractional day to HH:MM:SS
        const totalSeconds = Math.round((num % 1) * 86400);
        const hh = Math.floor(totalSeconds / 3600);
        const mm = Math.floor((totalSeconds % 3600) / 60);
        const ss = totalSeconds % 60;
        return String(hh).padStart(2, '0') + ':' + String(mm).padStart(2, '0') + ':' + String(ss).padStart(2, '0');
    }
    if (format === 'scientific') {
        if (isNaN(num)) return String(value);
        return num.toExponential(2);
    }
    return String(value);
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
        if (this._undoStack.length > 50) this._undoStack.shift(); // S11: Cap at 50 to reduce memory bloat from sort undo entries
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
        } else if (action.type === 'batch') {
            // Undo all sub-actions in reverse order
            for (let i = action.actions.length - 1; i >= 0; i--) {
                this._applyInverse(action.actions[i], workbook);
            }
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
        } else if (action.type === 'batch') {
            // Redo all sub-actions in order
            for (const sub of action.actions) {
                this._applyForward(sub, workbook);
            }
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
        this._dragMode = null;        // 'cell', 'column', or 'row'
        this._selectionAnchor = null; // anchor cell for shift+click extend
        this._resizingCol = null;     // { col, startX, startWidth }
        this._resizingRow = null;     // { row, startY, startHeight }
        this._rafId = null;
        this._contextMenu = null;
        this._filterDropdown = null;
        this._dpr = window.devicePixelRatio || 1;
        this._charts = [];              // active chart objects for current sheet

        this.setupDOM();
        this.setupEvents();
        this._setupNameBoxDropdown();
        this._initZoom();
        this._initFormulaAutocomplete();
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

        // S4.5: Formula syntax highlight overlay
        const formulaWrap = document.createElement('div');
        formulaWrap.className = 'ss-formula-wrap';
        const formulaHighlight = document.createElement('div');
        formulaHighlight.className = 'ss-formula-highlight';
        this._formulaHighlight = formulaHighlight;
        formulaWrap.appendChild(formulaHighlight);
        formulaWrap.appendChild(formulaInput);

        formulaBar.appendChild(cellRefLabel);
        formulaBar.appendChild(fxLabel);
        formulaBar.appendChild(formulaWrap);
        this.container.appendChild(formulaBar);

        // Canvas wrapper (for scrolling)
        const canvasWrap = document.createElement('div');
        canvasWrap.className = 'ss-canvas-wrap';
        // S4.7: Accessibility — grid role on wrapper
        canvasWrap.setAttribute('role', 'grid');
        canvasWrap.setAttribute('aria-label', 'Spreadsheet');
        canvasWrap.setAttribute('aria-rowcount', String(MAX_ROWS));
        canvasWrap.setAttribute('aria-colcount', String(MAX_COLS));
        this.canvasWrap = canvasWrap;

        this.canvas = document.createElement('canvas');
        this.canvas.className = 'ss-canvas';
        this.canvas.tabIndex = 0;
        this.canvas.setAttribute('role', 'presentation');
        this.ctx = this.canvas.getContext('2d');
        canvasWrap.appendChild(this.canvas);
        this.container.appendChild(canvasWrap);

        // S4.7: Accessibility — live region for announcing cell changes
        this._ariaLive = document.createElement('div');
        this._ariaLive.setAttribute('aria-live', 'polite');
        this._ariaLive.setAttribute('role', 'status');
        this._ariaLive.className = 'ss-aria-live';
        this._ariaLive.style.cssText = 'position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0,0,0,0);white-space:nowrap;border:0;';
        this.container.appendChild(this._ariaLive);

        // Cell editor overlay
        this.editInput = document.createElement('input');
        this.editInput.className = 'ss-cell-editor';
        this.editInput.style.display = 'none';
        this.editInput.setAttribute('aria-label', 'Cell editor');
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

        // S5.7: Touch events for mobile/tablet
        this.canvas.addEventListener('touchstart', (e) => this._handleTouchStart(e), { passive: false });
        this.canvas.addEventListener('touchmove', (e) => this._handleTouchMove(e), { passive: false });
        this.canvas.addEventListener('touchend', (e) => this._handleTouchEnd(e));

        // Formula bar events
        this.formulaInput.addEventListener('keydown', (e) => {
            // S5.4: Array formula — Ctrl+Shift+Enter from formula bar
            if (e.key === 'Enter' && e.ctrlKey && e.shiftKey) {
                e.preventDefault();
                // Use selectedCell as the editing context for formula bar
                this.editingCell = { col: this.selectedCell.col, row: this.selectedCell.row };
                this.editInput.value = this.formulaInput.value;
                this._commitArrayFormula();
                this.canvas.focus();
                return;
            }
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

        // S4.5: Re-render on formula bar input to update cell reference highlights
        this.formulaInput.addEventListener('input', () => { this._updateFormulaHighlight(); this.render(); });
        this.formulaInput.addEventListener('scroll', () => {
            if (this._formulaHighlight) this._formulaHighlight.scrollLeft = this.formulaInput.scrollLeft;
        });
        this.formulaInput.addEventListener('focus', () => this._updateFormulaHighlight());
        this.formulaInput.addEventListener('blur', () => {
            if (this._formulaHighlight) this._formulaHighlight.style.display = 'none';
            this.formulaInput.style.color = '';
            // Clear AI formula hint on blur
            clearTimeout(this._aiFormulaHintTimer);
            const hintEl = this.container.querySelector('.ss-ai-formula-hint');
            if (hintEl) hintEl.style.display = 'none';
        });

        // AI formula hint: show when user types "=" and pauses
        this._aiFormulaHintTimer = null;
        this.formulaInput.addEventListener('input', () => {
            clearTimeout(this._aiFormulaHintTimer);
            const val = this.formulaInput.value.trim();
            if (val === '=' || (val.startsWith('=') && val.length < 4)) {
                this._aiFormulaHintTimer = setTimeout(() => {
                    this._showFormulaAIHint();
                }, 2000);
            }
        });

        // Edit input events
        this.editInput.addEventListener('keydown', (e) => {
            // S5.4: Array formula — Ctrl+Shift+Enter
            if (e.key === 'Enter' && e.ctrlKey && e.shiftKey) {
                e.preventDefault();
                this._commitArrayFormula();
                return;
            }
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

        // S4.5: Re-render on cell edit input to update formula reference highlights
        this.editInput.addEventListener('input', () => { this._updateFormulaHighlight(); this.render(); });

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
            const text = typeof data === 'string' ? data : new TextDecoder(this._detectEncoding(data)).decode(data);
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
                const text = new TextDecoder(this._detectEncoding(data)).decode(data);
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

        // Warn about large datasets that may affect performance
        const loadedSheet = this.workbook.sheets[this.workbook.sheets.length > 0 ? 0 : -1];
        if (loadedSheet && Object.keys(loadedSheet.cells).length > 50000) {
            console.warn('[spreadsheet] Large dataset: ' + Object.keys(loadedSheet.cells).length + ' cells. Performance may be affected.');
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

    _detectEncoding(bytes) {
        // Detect encoding from BOM (byte order mark), fallback to UTF-8
        if (bytes.length >= 3 && bytes[0] === 0xEF && bytes[1] === 0xBB && bytes[2] === 0xBF) {
            return 'utf-8'; // UTF-8 BOM
        } else if (bytes.length >= 2 && bytes[0] === 0xFF && bytes[1] === 0xFE) {
            return 'utf-16le';
        } else if (bytes.length >= 2 && bytes[0] === 0xFE && bytes[1] === 0xFF) {
            return 'utf-16be';
        }
        return 'utf-8';
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
        // S5.4: Array formula support — {=FORMULA} syntax
        if (raw.startsWith('{=') && raw.endsWith('}')) {
            return { value: raw, formula: raw, display: '', type: 'formula', style: null, isArrayFormula: true };
        }
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

        // Re-evaluate all formula cells after import so display values are fresh
        for (const sheet of this.workbook.sheets) {
            const allSheets = this.workbook.sheets;
            for (const key in sheet.cells) {
                const cell = sheet.cells[key];
                if (cell && cell.formula) {
                    try {
                        const result = evaluateFormula(cell.formula, sheet, allSheets);
                        cell.display = (result && typeof result === 'object' && result.__sparkline)
                            ? 'SPARKLINE'
                            : (result !== undefined && result !== null ? String(result) : '');
                    } catch (_) {}
                }
            }
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
            const allSheets = this.workbook ? this.workbook.sheets : null;
            const result = evaluateFormula(cell.formula, sheet, allSheets);
            if (result && typeof result === 'object' && result.__sparkline) return 'SPARKLINE';
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

        // Preserve existing validation and comment when setting value
        const existingValidation = oldCell?.validation;
        const existingComment = oldCell?.comment;

        if (rawValue === '' || rawValue === null || rawValue === undefined) {
            sheet.deleteCell(col, row);
            // Restore validation/comment on empty cell if they exist
            if (existingValidation || existingComment) {
                let cell = sheet.getCell(col, row);
                if (!cell) {
                    cell = { value: '', formula: null, display: '', type: 'string', style: null };
                    sheet.setCell(col, row, cell);
                }
                if (existingValidation) cell.validation = existingValidation;
                if (existingComment) cell.comment = existingComment;
            }
            this._undoManager.push({
                type: 'edit', col, row, sheetIndex: this.activeSheet,
                oldValue: oldCopy, newValue: null
            });
        } else {
            // S12: Validate before setting — reject invalid input
            if (oldCell?.validation) {
                const result = this.validateCell(col, row, rawValue);
                if (!result.valid) {
                    this._showToast(result.message || 'Invalid input');
                    return; // Reject the value
                }
            }

            const newCell = this._parseRawValue(rawValue);
            if (newCell.formula) {
                const allSheets = this.workbook ? this.workbook.sheets : null;
                const result = evaluateFormula(newCell.formula, sheet, allSheets);
                newCell.display = (result && typeof result === 'object' && result.__sparkline) ? 'SPARKLINE' : String(result);
            }
            // Preserve validation and comment
            if (existingValidation) newCell.validation = existingValidation;
            if (existingComment) newCell.comment = existingComment;
            // Preserve style from old cell if new cell doesn't have one
            if (oldCell?.style && !newCell.style) newCell.style = oldCell.style;

            sheet.setCell(col, row, newCell);
            this._undoManager.push({
                type: 'edit', col, row, sheetIndex: this.activeSheet,
                oldValue: oldCopy, newValue: { ...newCell }
            });
        }
        // Refresh any charts that may depend on changed data
        this._refreshCharts();
        // S4.1: Broadcast cell edit to collaboration peers
        this.broadcastCellEdit(this.activeSheet, col, row, rawValue, null);
    }

    // ─── Rendering ───────────────────────────────
    render() {
        if (this._rafId) cancelAnimationFrame(this._rafId);
        this._rafId = requestAnimationFrame(() => this._renderImpl());
    }

    _isDarkMode() {
        return document.body.classList.contains('dark-mode') || document.documentElement.getAttribute('data-theme') === 'dark';
    }

    _getThemeColors() {
        const dark = this._isDarkMode();
        return {
            bg: dark ? '#1e1e1e' : '#fff',
            text: dark ? '#e0e0e0' : '#202124',
            textSecondary: dark ? '#9aa0a6' : '#5f6368',
            gridLine: dark ? '#444' : '#e0e0e0',
            headerBg: dark ? '#2d2d2d' : '#f8f9fa',
            headerText: dark ? '#e0e0e0' : '#5f6368',
            headerActiveBg: dark ? '#3a4a5c' : '#d2e3fc',
            headerActiveText: dark ? '#8ab4f8' : '#1a73e8',
            headerBorder: dark ? '#555' : '#c4c7c5',
            cornerBg: dark ? '#333' : '#e8eaed',
            selectionFill: dark ? 'rgba(138, 180, 248, 0.15)' : 'rgba(26, 115, 232, 0.08)',
            selectionBorder: dark ? '#8ab4f8' : '#1a73e8',
            activeCellBg: dark ? '#1e1e1e' : '#fff',
            frozenSep: dark ? '#666' : '#9aa0a6',
            errorText: dark ? '#f28b82' : '#d93025',
            linkText: dark ? '#8ab4f8' : '#1a73e8',
            commentIndicator: dark ? '#f28b82' : '#d93025',
            sparklinePositive: dark ? '#8ab4f8' : '#1a73e8',
            sparklineNegative: dark ? '#f28b82' : '#d93025',
            sparklineArea: dark ? 'rgba(138, 180, 248, 0.15)' : 'rgba(26, 115, 232, 0.1)',
            peerCursorLabel: dark ? '#1e1e1e' : '#fff',
        };
    }

    _getVisibleCols() {
        const w = this.canvas.width / this._dpr;
        const cols = [];
        let x = ROW_HEADER_WIDTH;
        // Frozen columns first — not affected by scrollX
        for (let c = 0; c < this.frozenCols; c++) {
            const cw = this.getColumnWidth(c);
            if (x + cw > ROW_HEADER_WIDTH && x < w) {
                cols.push({ col: c, x, width: cw, frozen: true });
            }
            x += cw;
        }
        const frozenWidth = x - ROW_HEADER_WIDTH;
        // Scrollable columns — start after frozen region, offset by frozenWidth
        const startCol = Math.max(this.frozenCols, this._colAtX(this.scrollX) - BUFFER_CELLS);
        for (let c = startCol; c < MAX_COLS; c++) {
            if (c < this.frozenCols) continue;
            const cx = this._colX(c) - this.scrollX + ROW_HEADER_WIDTH + frozenWidth;
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

        // Build set of cells that are part of a merge but NOT the top-left origin
        const mergedNonOrigin = new Set();
        const mergeOrigins = new Map(); // "col,row" -> merge descriptor
        if (sheet.merges && sheet.merges.length > 0) {
            for (const m of sheet.merges) {
                mergeOrigins.set(`${m.startCol},${m.startRow}`, m);
                for (let c = m.startCol; c <= m.endCol; c++) {
                    for (let r = m.startRow; r <= m.endRow; r++) {
                        if (c === m.startCol && r === m.startRow) continue;
                        mergedNonOrigin.add(`${c},${r}`);
                    }
                }
            }
        }

        for (const vc of visibleCols) {
            for (const vr of visibleRows) {
                const key = `${vc.col},${vr.row}`;
                // Skip non-origin cells of a merge (the merge origin covers them)
                if (mergedNonOrigin.has(key)) continue;

                const cell = sheet.getCell(vc.col, vr.row);
                if (!cell) continue;

                const merge = mergeOrigins.get(key);
                if (merge) {
                    // Calculate merged area dimensions
                    let mergeW = 0;
                    for (let c = merge.startCol; c <= merge.endCol; c++) mergeW += this.getColumnWidth(c);
                    let mergeH = 0;
                    for (let r = merge.startRow; r <= merge.endRow; r++) mergeH += this.getRowHeight(r);

                    const mx = this._cellScreenX(merge.startCol);
                    const my = this._cellScreenY(merge.startRow);

                    // Clear the merged area background
                    const mergeColors = this._getThemeColors();
                    ctx.fillStyle = mergeColors.bg;
                    ctx.fillRect(mx, my, mergeW, mergeH);

                    // Render cell content spanning the full merge
                    this.renderCell(ctx, vc.col, vr.row, mx, my, mergeW, mergeH, cell);

                    // Draw merge border
                    ctx.strokeStyle = mergeColors.gridLine;
                    ctx.lineWidth = 1;
                    ctx.strokeRect(mx, my, mergeW, mergeH);
                } else {
                    this.renderCell(ctx, vc.col, vr.row, vc.x, vr.y, vc.width, vr.height, cell);
                }
            }
        }
    }

    _renderValidationIndicators(ctx, visibleCols, visibleRows) {
        const sheet = this._sheet();
        if (!sheet) return;

        for (const vc of visibleCols) {
            for (const vr of visibleRows) {
                const cell = sheet.getCell(vc.col, vr.row);
                if (cell && cell.validation && cell.validation.type === 'list') {
                    // Draw a small dropdown arrow
                    const ax = vc.x + vc.width - 14;
                    const ay = vr.y + vr.height / 2;
                    ctx.fillStyle = this._getThemeColors().textSecondary;
                    ctx.beginPath();
                    ctx.moveTo(ax, ay - 3);
                    ctx.lineTo(ax + 8, ay - 3);
                    ctx.lineTo(ax + 4, ay + 3);
                    ctx.closePath();
                    ctx.fill();
                }
            }
        }
    }

    renderCell(ctx, col, row, x, y, w, h, cell) {
        if (!cell) return;

        const sheet = this._sheet();
        const style = cell.style || {};

        // Apply conditional formatting overlay
        const cfStyle = this._evaluateConditionalRules(col, row, cell);
        if (cfStyle && cfStyle.fill) {
            ctx.fillStyle = cfStyle.fill;
            ctx.fillRect(x, y, w, h);
        }

        // Compute display text with number format
        const allSheets = this.workbook ? this.workbook.sheets : null;
        let formulaResult = cell.formula ? evaluateFormula(cell.formula, sheet, allSheets) : null;

        // Handle sparkline rendering
        if (formulaResult && typeof formulaResult === 'object' && formulaResult.__sparkline) {
            this._renderSparkline(ctx, x, y, w, h, formulaResult, sheet);
            return;
        }

        let rawDisplay = formulaResult !== null
            ? String(formulaResult)
            : (cell.display || String(cell.value ?? ''));
        const displayText = style.numberFormat
            ? formatCellValue(formulaResult !== null ? formulaResult : cell.value, style.numberFormat)
            : rawDisplay;

        // Cell background fill
        if (style.fill) {
            ctx.fillStyle = style.fill;
            ctx.fillRect(x, y, w, h);
        } else if (style.bgColor) {
            ctx.fillStyle = style.bgColor;
            ctx.fillRect(x, y, w, h);
        }

        // Cell text
        ctx.save();
        ctx.beginPath();
        ctx.rect(x + 2, y, w - 4, h);
        ctx.clip();

        const isError = typeof displayText === 'string' && displayText.startsWith('#');
        const isNumber = cell.type === 'number' || (cell.type === 'formula' && !isNaN(Number(rawDisplay)));

        // Build font string
        const fontSize = style.fontSize || 13;
        const fontFamily = style.fontFamily || 'Arial, sans-serif';
        let fontStr = `${fontSize}px ${fontFamily}`;
        if (style.bold) fontStr = 'bold ' + fontStr;
        if (style.italic) fontStr = 'italic ' + fontStr;
        ctx.font = fontStr;

        // Text color (hyperlinks get blue)
        const cellColors = this._getThemeColors();
        if (isError) {
            ctx.fillStyle = cellColors.errorText;
        } else if (cell.hyperlink) {
            ctx.fillStyle = cellColors.linkText;
        } else if (style.color) {
            ctx.fillStyle = style.color;
        } else {
            ctx.fillStyle = cellColors.text;
        }

        // Alignment
        const align = style.align || (isNumber && !isError ? 'right' : 'left');
        ctx.textAlign = align;
        let textX;
        if (align === 'center') {
            textX = x + w / 2;
        } else if (align === 'right') {
            textX = x + w - 4;
        } else {
            textX = x + 4;
        }

        ctx.fillText(displayText, textX, y + h / 2);

        // Hyperlink underline (always if cell has a hyperlink)
        if (cell.hyperlink && !style.underline) {
            const metrics = ctx.measureText(displayText);
            const textWidth = metrics.width;
            let lineX;
            if (align === 'center') {
                lineX = textX - textWidth / 2;
            } else if (align === 'right') {
                lineX = textX - textWidth;
            } else {
                lineX = textX;
            }
            ctx.strokeStyle = cellColors.linkText;
            ctx.lineWidth = 1;
            ctx.beginPath();
            ctx.moveTo(lineX, y + h / 2 + fontSize * 0.3);
            ctx.lineTo(lineX + textWidth, y + h / 2 + fontSize * 0.3);
            ctx.stroke();
        }

        // Underline
        if (style.underline) {
            const metrics = ctx.measureText(displayText);
            const textWidth = metrics.width;
            let lineX;
            if (align === 'center') {
                lineX = textX - textWidth / 2;
            } else if (align === 'right') {
                lineX = textX - textWidth;
            } else {
                lineX = textX;
            }
            ctx.strokeStyle = style.color || cellColors.text;
            ctx.lineWidth = 1;
            ctx.beginPath();
            ctx.moveTo(lineX, y + h / 2 + fontSize * 0.3);
            ctx.lineTo(lineX + textWidth, y + h / 2 + fontSize * 0.3);
            ctx.stroke();
        }

        // Strikethrough
        if (style.strikethrough) {
            const metrics = ctx.measureText(displayText);
            const textWidth = metrics.width;
            let lineX;
            if (align === 'center') {
                lineX = textX - textWidth / 2;
            } else if (align === 'right') {
                lineX = textX - textWidth;
            } else {
                lineX = textX;
            }
            ctx.strokeStyle = style.color || cellColors.text;
            ctx.lineWidth = 1;
            ctx.beginPath();
            ctx.moveTo(lineX, y + h / 2);
            ctx.lineTo(lineX + textWidth, y + h / 2);
            ctx.stroke();
        }

        ctx.restore();

        // Borders (drawn outside clip region)
        if (style.borderBottom) {
            ctx.save();
            ctx.strokeStyle = style.borderBottom;
            ctx.lineWidth = 1;
            ctx.beginPath();
            ctx.moveTo(x, y + h - 0.5);
            ctx.lineTo(x + w, y + h - 0.5);
            ctx.stroke();
            ctx.restore();
        }
        if (style.borderTop) {
            ctx.save();
            ctx.strokeStyle = style.borderTop;
            ctx.lineWidth = 1;
            ctx.beginPath();
            ctx.moveTo(x, y + 0.5);
            ctx.lineTo(x + w, y + 0.5);
            ctx.stroke();
            ctx.restore();
        }
        if (style.borderLeft) {
            ctx.save();
            ctx.strokeStyle = style.borderLeft;
            ctx.lineWidth = 1;
            ctx.beginPath();
            ctx.moveTo(x + 0.5, y);
            ctx.lineTo(x + 0.5, y + h);
            ctx.stroke();
            ctx.restore();
        }
        if (style.borderRight) {
            ctx.save();
            ctx.strokeStyle = style.borderRight;
            ctx.lineWidth = 1;
            ctx.beginPath();
            ctx.moveTo(x + w - 0.5, y);
            ctx.lineTo(x + w - 0.5, y + h);
            ctx.stroke();
            ctx.restore();
        }
    }

    _renderGridLines(ctx, visibleCols, visibleRows, w, h) {
        ctx.strokeStyle = this._getThemeColors().gridLine;
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
        for (let vi = 0; vi < visibleRows.length; vi++) {
            const vr = visibleRows[vi];
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

            // S14: Hidden row indicator on grid — dashed line across full width
            if (this.hiddenRows && this.hiddenRows.has(vr.row + 1)) {
                ctx.save();
                ctx.strokeStyle = '#1a73e8';
                ctx.lineWidth = 2;
                ctx.setLineDash([3, 3]);
                ctx.beginPath();
                ctx.moveTo(ROW_HEADER_WIDTH, vr.y + vr.height);
                ctx.lineTo(w, vr.y + vr.height);
                ctx.stroke();
                ctx.restore();
            }
        }
    }

    renderHeaders(ctx, visibleCols, visibleRows, w, h) {
        this._renderHeaders(ctx, visibleCols, visibleRows, w, h);
    }

    _renderHeaders(ctx, visibleCols, visibleRows, w, h) {
        const hc = this._getThemeColors();

        // Column headers background
        ctx.fillStyle = hc.headerBg;
        ctx.fillRect(0, 0, w, HEADER_HEIGHT);
        ctx.fillRect(0, 0, ROW_HEADER_WIDTH, h);

        // Corner cell
        ctx.fillStyle = hc.cornerBg;
        ctx.fillRect(0, 0, ROW_HEADER_WIDTH, HEADER_HEIGHT);
        ctx.strokeStyle = hc.headerBorder;
        ctx.lineWidth = 1;
        ctx.strokeRect(0.5, 0.5, ROW_HEADER_WIDTH - 1, HEADER_HEIGHT - 1);

        // Column headers
        ctx.font = '500 12px Arial, sans-serif';
        ctx.textAlign = 'center';
        ctx.textBaseline = 'middle';
        ctx.fillStyle = hc.headerText;

        for (const vc of visibleCols) {
            const isSelected = this._isColSelected(vc.col);
            if (isSelected) {
                ctx.fillStyle = hc.headerActiveBg;
                ctx.fillRect(vc.x, 0, vc.width, HEADER_HEIGHT);
                ctx.fillStyle = hc.headerActiveText;
                ctx.font = 'bold 12px Arial, sans-serif';
            } else {
                ctx.fillStyle = hc.headerText;
                ctx.font = '500 12px Arial, sans-serif';
            }
            ctx.strokeStyle = hc.headerBorder;
            ctx.strokeRect(Math.round(vc.x) + 0.5, 0.5, vc.width - 1, HEADER_HEIGHT - 1);
            ctx.fillText(this.getCellA1Col(vc.col), vc.x + vc.width / 2, HEADER_HEIGHT / 2);

            // Filter indicator
            if (this.filterState[vc.col]) {
                ctx.fillStyle = hc.headerActiveText;
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
                ctx.fillStyle = hc.headerActiveBg;
                ctx.fillRect(0, vr.y, ROW_HEADER_WIDTH, vr.height);
                ctx.fillStyle = hc.headerActiveText;
                ctx.font = 'bold 12px Arial, sans-serif';
            } else {
                ctx.fillStyle = hc.headerText;
                ctx.font = '500 12px Arial, sans-serif';
            }
            ctx.strokeStyle = hc.headerBorder;
            ctx.strokeRect(0.5, Math.round(vr.y) + 0.5, ROW_HEADER_WIDTH - 1, vr.height - 1);
            ctx.fillText(String(vr.row + 1), ROW_HEADER_WIDTH / 2, vr.y + vr.height / 2);

            // S14: Hidden row indicator — draw dashed line between visible rows
            // when the next row(s) are hidden, so user can tell rows are missing
            if (this.hiddenRows && this.hiddenRows.has(vr.row + 1)) {
                ctx.save();
                ctx.strokeStyle = '#1a73e8';
                ctx.lineWidth = 2;
                ctx.setLineDash([3, 3]);
                ctx.beginPath();
                const indicatorY = vr.y + vr.height;
                ctx.moveTo(0, indicatorY);
                ctx.lineTo(ROW_HEADER_WIDTH, indicatorY);
                ctx.stroke();
                ctx.restore();
            }
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
            return this._colX(c) - this.scrollX + ROW_HEADER_WIDTH + frozenColsW;
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
        const selColors = this._getThemeColors();
        if (this.selectionRange) {
            const { startCol, startRow, endCol, endRow } = this._normalizeRange(this.selectionRange);
            const x1 = getScreenX(startCol);
            const y1 = getScreenY(startRow);
            const x2 = getScreenX(endCol) + this.getColumnWidth(endCol);
            const y2 = getScreenY(endRow) + this.getRowHeight(endRow);

            // Range fill
            ctx.fillStyle = selColors.selectionFill;
            ctx.fillRect(x1, y1, x2 - x1, y2 - y1);

            // Range border
            ctx.strokeStyle = selColors.selectionBorder;
            ctx.lineWidth = 2;
            ctx.strokeRect(x1, y1, x2 - x1, y2 - y1);
        }

        // Active cell
        const { col, row } = this.selectedCell;
        const sx = getScreenX(col);
        const sy = getScreenY(row);
        const sw = this.getColumnWidth(col);
        const sh = this.getRowHeight(row);

        // Background for active cell
        ctx.fillStyle = selColors.activeCellBg;
        ctx.fillRect(sx, sy, sw, sh);

        // Re-render cell content on top
        const sheet = this._sheet();
        if (sheet) {
            const cell = sheet.getCell(col, row);
            if (cell) {
                this.renderCell(ctx, col, row, sx, sy, sw, sh, cell);
            }
        }

        // Active cell border (thick accent)
        ctx.strokeStyle = selColors.selectionBorder;
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
        ctx.fillStyle = this._getThemeColors().selectionBorder;
        ctx.fillRect(x, y, FILL_HANDLE_SIZE, FILL_HANDLE_SIZE);
    }

    // S4.5 ── Formula Syntax Highlighting ─────────
    // Colors for highlighting different cell references in formulas
    _formulaRefColors = ['#4285f4', '#ea4335', '#34a853', '#fbbc04', '#ff6d01', '#46bdc6', '#7b1fa2', '#c2185b'];

    _renderFormulaRefHighlights(ctx) {
        // Only highlight when actively editing a formula
        const editVal = this.editingCell
            ? (this.editInput.value || '')
            : (document.activeElement === this.formulaInput ? (this.formulaInput.value || '') : null);
        if (!editVal || (!editVal.startsWith('=') && !editVal.startsWith('{='))) return;

        // Parse all cell references and ranges from the formula
        const refs = [];
        const rangeRegex = /([A-Z]{1,3})(\d{1,7}):([A-Z]{1,3})(\d{1,7})/gi;
        const singleRegex = /\b([A-Z]{1,3})(\d{1,7})\b/gi;
        const usedPositions = new Set();

        // First find ranges (so we skip their individual cells)
        let m;
        while ((m = rangeRegex.exec(editVal)) !== null) {
            const sc = colLetterToIndex(m[1].toUpperCase());
            const sr = parseInt(m[2], 10) - 1;
            const ec = colLetterToIndex(m[3].toUpperCase());
            const er = parseInt(m[4], 10) - 1;
            refs.push({ type: 'range', startCol: Math.min(sc, ec), startRow: Math.min(sr, er), endCol: Math.max(sc, ec), endRow: Math.max(sr, er) });
            usedPositions.add(m.index);
            usedPositions.add(m.index + m[1].length + m[2].length + 1); // position after ':'
        }

        // Then find single cell refs not part of a range
        while ((m = singleRegex.exec(editVal)) !== null) {
            if (usedPositions.has(m.index)) continue;
            // Skip function names (e.g., SUM, AVERAGE)
            if (/^[A-Z]{2,}$/i.test(m[1]) && FORMULA_CATALOG.some(f => f.name === m[1].toUpperCase())) continue;
            const c = colLetterToIndex(m[1].toUpperCase());
            const r = parseInt(m[2], 10) - 1;
            refs.push({ type: 'cell', startCol: c, startRow: r, endCol: c, endRow: r });
        }

        // Draw colored borders for each ref
        for (let i = 0; i < refs.length; i++) {
            const ref = refs[i];
            const color = this._formulaRefColors[i % this._formulaRefColors.length];
            const x1 = this._cellScreenX(ref.startCol);
            const y1 = this._cellScreenY(ref.startRow);
            let x2 = x1, y2 = y1;
            for (let c = ref.startCol; c <= ref.endCol; c++) x2 = this._cellScreenX(c) + this.getColumnWidth(c);
            for (let r = ref.startRow; r <= ref.endRow; r++) y2 = this._cellScreenY(r) + this.getRowHeight(r);

            // Fill with translucent color
            ctx.fillStyle = color + '18';
            ctx.fillRect(x1, y1, x2 - x1, y2 - y1);

            // Border
            ctx.strokeStyle = color;
            ctx.lineWidth = 2;
            ctx.setLineDash([4, 2]);
            ctx.strokeRect(x1, y1, x2 - x1, y2 - y1);
            ctx.setLineDash([]);
        }
    }

    renderFrozenPanes(ctx, w, h) {
        this._renderFrozenPanes(ctx, w, h);
    }

    _renderFrozenPanes(ctx, w, h) {
        const fpColors = this._getThemeColors();
        if (this.frozenCols > 0) {
            const fx = this._frozenColsWidth() + ROW_HEADER_WIDTH;
            ctx.strokeStyle = fpColors.frozenSep;
            ctx.lineWidth = 2;
            ctx.beginPath();
            ctx.moveTo(fx, 0);
            ctx.lineTo(fx, h);
            ctx.stroke();
        }
        if (this.frozenRows > 0) {
            const fy = this._frozenRowsHeight() + HEADER_HEIGHT;
            ctx.strokeStyle = fpColors.frozenSep;
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
        const sheet = this._sheet();
        const limit = Math.min((sheet?.maxCol || 26) + 10, MAX_COLS);
        for (let c = 0; c < limit; c++) {
            x += this.getColumnWidth(c);
            if (x > scrollX) return c;
        }
        return Math.min(limit - 1, MAX_COLS - 1);
    }

    _rowAtY(scrollY) {
        let y = 0;
        const sheet = this._sheet();
        const limit = Math.min((sheet?.maxRow || 100) + 50, MAX_ROWS);
        for (let r = 0; r < limit; r++) {
            if (this.hiddenRows?.has(r)) continue;
            y += this.getRowHeight(r);
            if (y > scrollY) return r;
        }
        return Math.min(limit - 1, MAX_ROWS - 1);
    }

    _cellScreenX(col) {
        if (col < this.frozenCols) {
            // Frozen column — not affected by scrollX
            let x = ROW_HEADER_WIDTH;
            for (let i = 0; i < col; i++) x += this.getColumnWidth(i);
            return x;
        }
        return this._colX(col) - this.scrollX + ROW_HEADER_WIDTH + this._frozenColsWidth();
    }

    _cellScreenY(row) {
        if (row < this.frozenRows) {
            // Frozen row — not affected by scrollY
            let y = HEADER_HEIGHT;
            for (let i = 0; i < row; i++) {
                if (!this.hiddenRows.has(i)) y += this.getRowHeight(i);
            }
            return y;
        }
        return this._rowY(row) - this.scrollY + HEADER_HEIGHT + this._frozenRowsHeight();
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
            // Scrollable column region — apply scroll offset, offset by frozen width
            let x = ROW_HEADER_WIDTH + frozenColsW;
            const startCol = Math.max(this.frozenCols, this._colAtX(this.scrollX));
            for (let c = startCol; c < MAX_COLS; c++) {
                const cx = this._colX(c) - this.scrollX + ROW_HEADER_WIDTH + frozenColsW;
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

    // ─── Helper: get column index from canvas X in header area ──
    _getColAtX(canvasX) {
        const frozenColsW = this._frozenColsWidth();
        if (this.frozenCols > 0 && canvasX < ROW_HEADER_WIDTH + frozenColsW) {
            let x = ROW_HEADER_WIDTH;
            for (let c = 0; c < this.frozenCols; c++) {
                const cw = this.getColumnWidth(c);
                if (canvasX >= x && canvasX < x + cw) return c;
                x += cw;
            }
            return -1;
        }
        const startCol = Math.max(this.frozenCols, this._colAtX(this.scrollX));
        for (let c = startCol; c < MAX_COLS; c++) {
            const cx = this._colX(c) - this.scrollX + ROW_HEADER_WIDTH + frozenColsW;
            const cw = this.getColumnWidth(c);
            if (canvasX >= cx && canvasX < cx + cw) return c;
            if (cx > canvasX + CELL_WIDTH * 10) break;
        }
        return -1;
    }

    // ─── Helper: get row index from canvas Y in header area ──
    _getRowAtY(canvasY) {
        const frozenRowsH = this._frozenRowsHeight();
        if (this.frozenRows > 0 && canvasY < HEADER_HEIGHT + frozenRowsH) {
            let y = HEADER_HEIGHT;
            for (let r = 0; r < this.frozenRows; r++) {
                if (this.hiddenRows.has(r)) continue;
                const rh = this.getRowHeight(r);
                if (canvasY >= y && canvasY < y + rh) return r;
                y += rh;
            }
            return -1;
        }
        const startRow = Math.max(this.frozenRows, this._rowAtY(this.scrollY));
        for (let r = startRow; r < MAX_ROWS; r++) {
            if (this.hiddenRows.has(r)) continue;
            const ry = this._rowY(r) - this.scrollY + HEADER_HEIGHT + frozenRowsH;
            const rh = this.getRowHeight(r);
            if (canvasY >= ry && canvasY < ry + rh) return r;
            if (ry > canvasY + CELL_HEIGHT * 10) break;
        }
        return -1;
    }

    // ─── Helper: get max data row for selections ──
    getMaxRow() {
        const sheet = this._sheet();
        return sheet ? Math.max(sheet.maxRow, 99) : 99;
    }

    // ─── Helper: get max data col for selections ──
    getMaxCol() {
        const sheet = this._sheet();
        return sheet ? Math.max(sheet.maxCol, 25) : 25;
    }

    // ─── Mouse handling ──────────────────────────
    handleMouseDown(e) {
        const rect = this.canvas.getBoundingClientRect();
        const canvasX = (e.clientX - rect.left);
        const canvasY = (e.clientY - rect.top);

        // Check for column resize handle (in header area)
        if (canvasY < HEADER_HEIGHT && canvasX > ROW_HEADER_WIDTH) {
            const resizeCol = this._hitColumnResizeHandle(canvasX);
            if (resizeCol !== null) {
                this._resizingCol = { col: resizeCol, startX: e.clientX, startWidth: this.getColumnWidth(resizeCol) };
                e.preventDefault();
                return;
            }
        }

        // Check for row resize handle (in row header area)
        if (canvasX < ROW_HEADER_WIDTH && canvasY > HEADER_HEIGHT) {
            const resizeRow = this._hitRowResizeHandle(canvasY);
            if (resizeRow !== null) {
                this._resizingRow = { row: resizeRow, startY: e.clientY, startHeight: this.getRowHeight(resizeRow) };
                e.preventDefault();
                return;
            }
        }

        // Click on top-left corner = select all cells
        if (canvasX < ROW_HEADER_WIDTH && canvasY < HEADER_HEIGHT) {
            if (this.editingCell) this.commitEdit();
            this._selectionAnchor = { col: 0, row: 0 };
            this.selectRange(0, 0, this.getMaxCol(), this.getMaxRow());
            this.render();
            return;
        }

        // Click on column header = select entire column
        if (canvasY < HEADER_HEIGHT && canvasX > ROW_HEADER_WIDTH) {
            if (this.editingCell) this.commitEdit();
            const col = this._getColAtX(canvasX);
            if (col >= 0) {
                if (e.shiftKey && this._selectionAnchor) {
                    // Shift+click extends column selection
                    const startCol = Math.min(this._selectionAnchor.col, col);
                    const endCol = Math.max(this._selectionAnchor.col, col);
                    this.selectRange(startCol, 0, endCol, this.getMaxRow());
                } else {
                    this._selectionAnchor = { col, row: 0 };
                    this.selectRange(col, 0, col, this.getMaxRow());
                }
                this._dragging = true;
                this._dragMode = 'column';
                this._dragStart = { col, row: 0 };
                this.render();
            }
            return;
        }

        // Click on row header = select entire row
        if (canvasX < ROW_HEADER_WIDTH && canvasY > HEADER_HEIGHT) {
            if (this.editingCell) this.commitEdit();
            const row = this._getRowAtY(canvasY);
            if (row >= 0) {
                if (e.shiftKey && this._selectionAnchor) {
                    // Shift+click extends row selection
                    const startRow = Math.min(this._selectionAnchor.row, row);
                    const endRow = Math.max(this._selectionAnchor.row, row);
                    this.selectRange(0, startRow, this.getMaxCol(), endRow);
                } else {
                    this._selectionAnchor = { col: 0, row };
                    this.selectRange(0, row, this.getMaxCol(), row);
                }
                this._dragging = true;
                this._dragMode = 'row';
                this._dragStart = { col: 0, row };
                this.render();
            }
            return;
        }

        // Click on a cell in the grid
        const cell = this.getCellAt(canvasX, canvasY);
        if (!cell) return;

        // Ctrl+Click on hyperlinks
        if ((e.ctrlKey || e.metaKey) && !e.shiftKey) {
            const sheet = this._sheet();
            if (sheet) {
                const cellData = sheet.getCell(cell.col, cell.row);
                if (cellData && cellData.hyperlink) {
                    window.open(cellData.hyperlink, '_blank');
                    return;
                }
            }
        }

        if (this.editingCell) {
            this.commitEdit();
        }

        // Check for validation dropdown click
        {
            const sheet = this._sheet();
            if (sheet) {
                const cellData = sheet.getCell(cell.col, cell.row);
                if (cellData && cellData.validation && cellData.validation.type === 'list') {
                    const cellRight = this._cellScreenX(cell.col) + this.getColumnWidth(cell.col);
                    if (canvasX >= cellRight - 16) {
                        this.selectedCell = { col: cell.col, row: cell.row };
                        this._updateFormulaBar();
                        this.render();
                        this._showValidationDropdown(cell.col, cell.row);
                        return;
                    }
                }
            }
        }

        this._dragMode = 'cell';

        if (e.shiftKey && this._selectionAnchor) {
            // Shift+Click extends selection from anchor to clicked cell
            this.selectionRange = {
                startCol: this._selectionAnchor.col, startRow: this._selectionAnchor.row,
                endCol: cell.col, endRow: cell.row
            };
            this.selectedCell = { col: cell.col, row: cell.row };
        } else {
            this.selectedCell = { col: cell.col, row: cell.row };
            this.selectionRange = null;
            this._selectionAnchor = { col: cell.col, row: cell.row };
        }

        this._dragging = true;
        this._dragStart = { col: cell.col, row: cell.row };

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
            // Select-all + resize: apply to ALL columns
            if (this._isAllSelected()) {
                const sheet = this._sheet();
                if (sheet) {
                    const maxC = Math.max(this.getMaxCol(), 25);
                    for (let c = 0; c <= maxC; c++) {
                        sheet.colWidths[c] = newWidth;
                    }
                    this.render();
                }
            } else {
                this.resizeColumn(this._resizingCol.col, newWidth);
            }
            return;
        }

        // Row resize
        if (this._resizingRow) {
            const delta = e.clientY - this._resizingRow.startY;
            const newHeight = Math.max(16, this._resizingRow.startHeight + delta);
            // Select-all + resize: apply to ALL rows
            if (this._isAllSelected()) {
                const sheet = this._sheet();
                if (sheet) {
                    const maxR = Math.max(this.getMaxRow(), 99);
                    for (let r = 0; r <= maxR; r++) {
                        sheet.rowHeights[r] = newHeight;
                    }
                    this.render();
                }
            } else {
                this.resizeRow(this._resizingRow.row, newHeight);
            }
            return;
        }

        // Update cursor for resize handles
        if (canvasY < HEADER_HEIGHT && canvasX > ROW_HEADER_WIDTH && this._hitColumnResizeHandle(canvasX) !== null) {
            this.canvas.style.cursor = 'col-resize';
        } else if (canvasX < ROW_HEADER_WIDTH && canvasY > HEADER_HEIGHT && this._hitRowResizeHandle(canvasY) !== null) {
            this.canvas.style.cursor = 'row-resize';
        } else {
            this.canvas.style.cursor = 'cell';
        }

        // Comment tooltip on hover
        if (!this._dragging) {
            const hoverCell = this.getCellAt(canvasX, canvasY);
            if (hoverCell) {
                const sheet = this._sheet();
                const cellData = sheet ? sheet.getCell(hoverCell.col, hoverCell.row) : null;
                if (cellData && cellData.comment) {
                    if (!this._commentTooltip || this._commentTooltipCell?.col !== hoverCell.col || this._commentTooltipCell?.row !== hoverCell.row) {
                        this._commentTooltipCell = { col: hoverCell.col, row: hoverCell.row };
                        this._showCommentTooltip(hoverCell.col, hoverCell.row, canvasX, canvasY);
                    }
                } else {
                    this._hideCommentTooltip();
                    this._commentTooltipCell = null;
                }
            } else {
                this._hideCommentTooltip();
                this._commentTooltipCell = null;
            }
        }

        // Drag selection in column header — select multiple columns
        if (this._dragging && this._dragMode === 'column') {
            const col = this._getColAtX(canvasX);
            if (col >= 0 && this._dragStart) {
                const startCol = Math.min(this._dragStart.col, col);
                const endCol = Math.max(this._dragStart.col, col);
                this.selectRange(startCol, 0, endCol, this.getMaxRow());
                this.render();
            }
            return;
        }

        // Drag selection in row header — select multiple rows
        if (this._dragging && this._dragMode === 'row') {
            const row = this._getRowAtY(canvasY);
            if (row >= 0 && this._dragStart) {
                const startRow = Math.min(this._dragStart.row, row);
                const endRow = Math.max(this._dragStart.row, row);
                this.selectRange(0, startRow, this.getMaxCol(), endRow);
                this.render();
            }
            return;
        }

        // Drag selection in cells
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
        this._dragMode = null;
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
        const sheet = this._sheet();
        const cellData = sheet ? sheet.getCell(cell.col, cell.row) : null;
        const hasComment = cellData && cellData.comment;
        const items = [
            { label: 'Cut', action: () => this.cutCells(), shortcut: 'Ctrl+X' },
            { label: 'Copy', action: () => this.copyCells(), shortcut: 'Ctrl+C' },
            { label: 'Paste', action: () => this.pasteCells(), shortcut: 'Ctrl+V' },
            { label: 'Paste Special...', action: () => this.showPasteSpecialDialog() },
            { label: '---' },
            { label: 'Insert row above', action: () => this.insertRow(cell.row) },
            { label: 'Insert row below', action: () => this.insertRow(cell.row + 1) },
            { label: 'Delete row', action: () => this.deleteRow(cell.row) },
            { label: '---' },
            { label: 'Insert column left', action: () => this.insertColumn(cell.col) },
            { label: 'Insert column right', action: () => this.insertColumn(cell.col + 1) },
            { label: 'Delete column', action: () => this.deleteColumn(cell.col) },
            { label: '---' },
            { label: hasComment ? 'Edit Comment' : 'Insert Comment', action: () => {
                const existingText = hasComment ? cellData.comment.text : '';
                ssPrompt(hasComment ? 'Edit comment:' : 'Add comment:', existingText).then((text) => {
                    if (text !== null) this.setCellComment(cell.col, cell.row, text);
                });
            }},
            ...(hasComment ? [{ label: 'Delete Comment', action: () => this.setCellComment(cell.col, cell.row, null) }] : []),
            { label: '---' },
            { label: 'Sort A-Z', action: () => this.sort(cell.col, true) },
            { label: 'Sort Z-A', action: () => this.sort(cell.col, false) },
            { label: '---' },
            { label: this.filterState[cell.col] ? 'Remove filter' : 'Add filter', action: () => {
                if (this.filterState[cell.col]) this.removeFilter(cell.col);
                else this.addFilter(cell.col);
            }},
            { label: '---' },
            { label: cellData?.hyperlink ? 'Edit Link' : 'Insert Link', action: () => {
                const existingUrl = cellData?.hyperlink || '';
                ssPrompt(cellData?.hyperlink ? 'Edit hyperlink URL:' : 'Enter hyperlink URL:', existingUrl).then((url) => {
                    if (url !== null) this.setCellHyperlink(cell.col, cell.row, url || null);
                });
            }},
            ...(cellData?.hyperlink ? [{ label: 'Remove Link', action: () => this.setCellHyperlink(cell.col, cell.row, null) }] : []),
            { label: '---' },
            { label: 'Insert Shape...', action: () => this.showInsertShapeDialog() },
            { label: '---' },
            { label: `Freeze at ${this.getCellA1(cell.col, cell.row)}`, action: () => this.freezePanes(cell.col, cell.row) },
            { label: 'Unfreeze', action: () => this.freezePanes(0, 0) },
            ...(window.S1_CONFIG?.enableAI ? [
                { label: '---' },
                { label: 'Ask AI...', action: () => this._showAIPromptForCell(cell) },
                ...(cellData?.formula ? [{ label: 'Explain Formula', action: () => this._explainFormula(cell, cellData.formula) }] : []),
                { label: 'Analyze with AI', action: () => this._analyzeWithAI() },
            ] : []),
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
        } else if ((e.ctrlKey || e.metaKey) && e.key === 'f') {
            e.preventDefault();
            this.openFindBar(false);
        } else if ((e.ctrlKey || e.metaKey) && e.key === 'h') {
            e.preventDefault();
            this.openFindBar(true);
        } else if ((e.ctrlKey || e.metaKey) && e.key === 'b') {
            e.preventDefault();
            this.toggleFormat('bold');
        } else if ((e.ctrlKey || e.metaKey) && e.key === 'i') {
            e.preventDefault();
            this.toggleFormat('italic');
        } else if ((e.ctrlKey || e.metaKey) && e.key === 'u') {
            e.preventDefault();
            this.toggleFormat('underline');
        } else if ((e.ctrlKey || e.metaKey) && e.code === 'Space') {
            // Ctrl+Space = select entire column
            e.preventDefault();
            this._selectionAnchor = { col, row: 0 };
            this.selectRange(col, 0, col, this.getMaxRow());
            this.render();
        } else if (e.shiftKey && e.code === 'Space') {
            // Shift+Space = select entire row
            e.preventDefault();
            this._selectionAnchor = { col: 0, row };
            this.selectRange(0, row, this.getMaxCol(), row);
            this.render();
        } else if ((e.ctrlKey || e.metaKey) && e.key === '/') {
            // S4.6: Keyboard shortcuts help
            e.preventDefault();
            this.showKeyboardShortcutsHelp();
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
        // S4.3: Ctrl+mousewheel = zoom
        if (e.ctrlKey || e.metaKey) {
            if (e.deltaY < 0) this.zoomIn();
            else if (e.deltaY > 0) this.zoomOut();
            return;
        }
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
        this._updateFormulaBar(); // S10: Sync formula bar immediately on edit entry
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

    // S5.4 ── Array Formulas (Ctrl+Shift+Enter) ────
    _commitArrayFormula() {
        if (!this.editingCell) return;
        const { col, row } = this.editingCell;
        let val = this.editInput.value;

        // Wrap in array braces if it's a formula
        if (val.startsWith('=') && !val.startsWith('{=')) {
            val = '{' + val + '}';
        }

        const range = this.selectionRange ? this._normalizeRange(this.selectionRange)
            : { startCol: col, startRow: row, endCol: col, endRow: row };

        // Set the array formula on all cells in the selection
        const sheet = this._sheet();
        if (!sheet) return;

        for (let c = range.startCol; c <= range.endCol; c++) {
            for (let r = range.startRow; r <= range.endRow; r++) {
                this._setCellValue(c, r, val);
                const cell = sheet.getCell(c, r);
                if (cell) {
                    cell.arrayFormula = true;
                    cell.arrayRange = {
                        startCol: range.startCol, startRow: range.startRow,
                        endCol: range.endCol, endRow: range.endRow
                    };
                }
            }
        }

        this.editingCell = null;
        this.editInput.style.display = 'none';
        this._updateFormulaBar();
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
        let maxWidth = 40;
        for (let r = 0; r <= sheet.maxRow; r++) {
            const cell = sheet.getCell(col, r);
            if (cell) {
                const style = cell.style || {};
                const fontSize = style.fontSize || 13;
                const fontFamily = style.fontFamily || 'Arial, sans-serif';
                let fontStr = `${fontSize}px ${fontFamily}`;
                if (style.bold) fontStr = 'bold ' + fontStr;
                if (style.italic) fontStr = 'italic ' + fontStr;
                ctx.font = fontStr;

                let display = cell.formula
                    ? String(evaluateFormula(cell.formula, sheet))
                    : (cell.display || String(cell.value ?? ''));
                if (style.numberFormat) {
                    display = formatCellValue(
                        cell.formula ? evaluateFormula(cell.formula, sheet) : cell.value,
                        style.numberFormat
                    );
                }
                const tw = ctx.measureText(display).width + 12;
                if (tw > maxWidth) maxWidth = tw;
            }
        }
        // Also measure header
        ctx.font = '500 12px Arial, sans-serif';
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
        this.broadcastSheetSync();
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
        this.broadcastSheetSync();
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
        this.broadcastSheetSync();
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
        this.broadcastSheetSync();
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
        this.broadcastSheetSync();
        this.render();
    }

    // ─── Filter (S2.5 Advanced) ────────────────────
    addFilter(col) {
        const sheet = this._sheet();
        if (!sheet) return;
        // Show the advanced filter dialog
        this._showAdvancedFilterDialog(col);
    }

    removeFilter(col) {
        delete this.filterState[col];
        this._recomputeHiddenRows();
        this.render();
    }

    _showAdvancedFilterDialog(col) {
        const sheet = this._sheet();
        if (!sheet) return;

        // Collect unique values for the value list (skip header row 0)
        const allValues = new Set();
        for (let r = 1; r <= sheet.maxRow; r++) {
            const cell = sheet.getCell(col, r);
            allValues.add(cell ? String(cell.value ?? '') : '');
        }

        const existing = this.filterState[col];

        const overlay = document.createElement('div');
        overlay.className = 'modal-overlay show';
        const modal = document.createElement('div');
        modal.className = 'modal';
        modal.style.minWidth = '420px';

        let valueChecks = '';
        for (const v of allValues) {
            const checked = !existing || !existing.active || existing.active.has(v);
            const safeVal = String(v).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
            valueChecks += '<label style="display:flex;align-items:center;gap:8px;font-size:13px;padding:3px 0;cursor:pointer;">'
                + '<input type="checkbox" class="flt-val-check" data-val="' + safeVal + '"' + (checked ? ' checked' : '') + '> '
                + (v === '' ? '(Blank)' : this._xmlEsc(v)) + '</label>';
        }

        modal.innerHTML = '<h3>Filter Column ' + this.getCellA1Col(col) + '</h3>'
            + '<div style="display:flex;flex-direction:column;gap:10px;padding:8px 0;">'
            + '<div class="modal-field"><label style="font-size:12px;color:#5f6368;margin-bottom:4px;display:block;">Filter type</label>'
            + '<select id="fltType" style="width:100%;padding:6px 8px;border:1px solid #dadce0;border-radius:4px;font-size:13px;">'
            + '<option value="values"' + (!existing?.advancedType ? ' selected' : '') + '>By values</option>'
            + '<option value="textContains"' + (existing?.advancedType === 'textContains' ? ' selected' : '') + '>Text: Contains</option>'
            + '<option value="textNotContains"' + (existing?.advancedType === 'textNotContains' ? ' selected' : '') + '>Text: Does not contain</option>'
            + '<option value="textStartsWith"' + (existing?.advancedType === 'textStartsWith' ? ' selected' : '') + '>Text: Starts with</option>'
            + '<option value="textEndsWith"' + (existing?.advancedType === 'textEndsWith' ? ' selected' : '') + '>Text: Ends with</option>'
            + '<option value="numGreaterThan"' + (existing?.advancedType === 'numGreaterThan' ? ' selected' : '') + '>Number: Greater than</option>'
            + '<option value="numLessThan"' + (existing?.advancedType === 'numLessThan' ? ' selected' : '') + '>Number: Less than</option>'
            + '<option value="numBetween"' + (existing?.advancedType === 'numBetween' ? ' selected' : '') + '>Number: Between</option>'
            + '<option value="numTop10"' + (existing?.advancedType === 'numTop10' ? ' selected' : '') + '>Number: Top 10</option>'
            + '<option value="dateBefore"' + (existing?.advancedType === 'dateBefore' ? ' selected' : '') + '>Date: Before</option>'
            + '<option value="dateAfter"' + (existing?.advancedType === 'dateAfter' ? ' selected' : '') + '>Date: After</option>'
            + '<option value="dateBetween"' + (existing?.advancedType === 'dateBetween' ? ' selected' : '') + '>Date: Between</option>'
            + '</select></div>'
            + '<div id="fltValuesPanel" style="max-height:180px;overflow-y:auto;border:1px solid #dadce0;border-radius:4px;padding:8px;">'
            + '<div style="margin-bottom:6px;"><label style="font-size:12px;cursor:pointer;"><input type="checkbox" id="fltSelectAll" checked> Select All</label></div>'
            + valueChecks + '</div>'
            + '<div id="fltInputPanel" style="display:none;"><div class="modal-field"><label style="font-size:12px;color:#5f6368;margin-bottom:4px;display:block;">Value</label>'
            + '<input type="text" id="fltInput1" value="' + (existing?.filterValue1 || '') + '" style="width:100%;padding:6px 8px;border:1px solid #dadce0;border-radius:4px;font-size:13px;"></div>'
            + '<div class="modal-field" id="fltInput2Row" style="display:none;"><label style="font-size:12px;color:#5f6368;margin-bottom:4px;display:block;">Value 2 (upper bound)</label>'
            + '<input type="text" id="fltInput2" value="' + (existing?.filterValue2 || '') + '" style="width:100%;padding:6px 8px;border:1px solid #dadce0;border-radius:4px;font-size:13px;"></div></div>'
            + '</div>'
            + '<div class="modal-actions"><button class="ss-modal-cancel">Cancel</button><button class="ss-modal-ok primary">Apply</button></div>';
        overlay.appendChild(modal);
        document.body.appendChild(overlay);

        const typeSelect = modal.querySelector('#fltType');
        const valuesPanel = modal.querySelector('#fltValuesPanel');
        const inputPanel = modal.querySelector('#fltInputPanel');
        const input2Row = modal.querySelector('#fltInput2Row');
        const selectAll = modal.querySelector('#fltSelectAll');

        const updatePanels = () => {
            const t = typeSelect.value;
            if (t === 'values') {
                valuesPanel.style.display = '';
                inputPanel.style.display = 'none';
            } else {
                valuesPanel.style.display = 'none';
                inputPanel.style.display = '';
                const needTwo = (t === 'numBetween' || t === 'dateBetween');
                input2Row.style.display = needTwo ? '' : 'none';
            }
        };
        typeSelect.addEventListener('change', updatePanels);
        updatePanels();

        selectAll.addEventListener('change', () => {
            modal.querySelectorAll('.flt-val-check').forEach(cb => { cb.checked = selectAll.checked; });
        });

        const close = () => { document.body.removeChild(overlay); };
        modal.querySelector('.ss-modal-cancel').onclick = close;
        modal.querySelector('.ss-modal-ok').onclick = () => {
            const filterType = typeSelect.value;
            if (filterType === 'values') {
                const active = new Set();
                modal.querySelectorAll('.flt-val-check:checked').forEach(cb => { active.add(cb.dataset.val); });
                this.filterState[col] = { values: allValues, active, advancedType: null };
            } else {
                const val1 = modal.querySelector('#fltInput1').value;
                const val2 = modal.querySelector('#fltInput2').value;
                this.filterState[col] = { advancedType: filterType, filterValue1: val1, filterValue2: val2 };
            }
            this._recomputeHiddenRows();
            this.render();
            close();
        };
        overlay.onclick = (e) => { if (e.target === overlay) close(); };
    }

    _recomputeHiddenRows() {
        this.hiddenRows = new Set();
        const filterCols = Object.keys(this.filterState).map(Number);
        if (filterCols.length === 0) return;

        const sheet = this._sheet();
        if (!sheet) return;

        // Row 0 is header, never hidden (AND logic across all filter columns)
        for (let r = 1; r <= sheet.maxRow; r++) {
            let visible = true;
            for (const col of filterCols) {
                const filter = this.filterState[col];
                if (!filter) continue;
                const cell = sheet.getCell(col, r);
                const val = cell ? String(cell.value ?? '') : '';
                const numVal = Number(val);

                if (filter.advancedType) {
                    // Advanced filter mode
                    const fv1 = filter.filterValue1 || '';
                    const fv2 = filter.filterValue2 || '';
                    switch (filter.advancedType) {
                        case 'textContains':
                            if (!val.toLowerCase().includes(fv1.toLowerCase())) visible = false;
                            break;
                        case 'textNotContains':
                            if (val.toLowerCase().includes(fv1.toLowerCase())) visible = false;
                            break;
                        case 'textStartsWith':
                            if (!val.toLowerCase().startsWith(fv1.toLowerCase())) visible = false;
                            break;
                        case 'textEndsWith':
                            if (!val.toLowerCase().endsWith(fv1.toLowerCase())) visible = false;
                            break;
                        case 'numGreaterThan':
                            if (isNaN(numVal) || numVal <= Number(fv1)) visible = false;
                            break;
                        case 'numLessThan':
                            if (isNaN(numVal) || numVal >= Number(fv1)) visible = false;
                            break;
                        case 'numBetween':
                            if (isNaN(numVal) || numVal < Number(fv1) || numVal > Number(fv2)) visible = false;
                            break;
                        case 'numTop10': {
                            // Collect all numeric values in this column and check if val is in top 10
                            const nums = [];
                            for (let rr = 1; rr <= sheet.maxRow; rr++) {
                                const cc = sheet.getCell(col, rr);
                                const nv = cc ? Number(cc.value) : NaN;
                                if (!isNaN(nv)) nums.push(nv);
                            }
                            nums.sort((a, b) => b - a);
                            const topN = new Set(nums.slice(0, 10));
                            if (isNaN(numVal) || !topN.has(numVal)) visible = false;
                            break;
                        }
                        case 'dateBefore': {
                            const d1 = new Date(fv1).getTime();
                            const dv = new Date(val).getTime();
                            if (isNaN(dv) || isNaN(d1) || dv >= d1) visible = false;
                            break;
                        }
                        case 'dateAfter': {
                            const d1b = new Date(fv1).getTime();
                            const dvb = new Date(val).getTime();
                            if (isNaN(dvb) || isNaN(d1b) || dvb <= d1b) visible = false;
                            break;
                        }
                        case 'dateBetween': {
                            const db1 = new Date(fv1).getTime();
                            const db2 = new Date(fv2).getTime();
                            const dvc = new Date(val).getTime();
                            if (isNaN(dvc) || isNaN(db1) || isNaN(db2) || dvc < db1 || dvc > db2) visible = false;
                            break;
                        }
                    }
                } else if (filter.active) {
                    // Value-based filter
                    if (!filter.active.has(val)) visible = false;
                }
                if (!visible) break;
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
            if (this._hiddenSheets && this._hiddenSheets.has(i)) continue;
            const sheet = this.workbook.sheets[i];
            const tab = document.createElement('button');
            tab.className = 'ss-tab' + (i === this.activeSheet ? ' active' : '');
            tab.textContent = sheet.name;
            tab.title = `Switch to ${sheet.name}`;
            tab.draggable = true;
            tab.dataset.sheetIndex = i;
            if (sheet.tabColor) tab.style.borderBottom = '3px solid ' + sheet.tabColor;
            tab.addEventListener('click', () => {
                if (this.editingCell) this.commitEdit();
                this.activeSheet = i;
                this.selectedCell = { col: 0, row: 0 };
                this.selectionRange = null;
                this.scrollX = 0;
                this.scrollY = 0;
                if (typeof this._showActiveSheetCharts === 'function') this._showActiveSheetCharts();
                if (typeof this._showActiveSheetImages === 'function') this._showActiveSheetImages();
                if (typeof this._showActiveSheetShapes === 'function') this._showActiveSheetShapes();
                this.updateSheetTabs();
                this._updateFormulaBar();
                this.render();
            });
            tab.addEventListener('dblclick', (e) => { e.preventDefault(); e.stopPropagation(); this._startInlineRename(tab, i); });
            tab.addEventListener('contextmenu', (e) => { e.preventDefault(); e.stopPropagation(); this._showTabContextMenu(e.clientX, e.clientY, i); });
            tab.addEventListener('dragstart', (e) => { e.dataTransfer.setData('text/plain', String(i)); e.dataTransfer.effectAllowed = 'move'; tab.classList.add('dragging'); });
            tab.addEventListener('dragend', () => { tab.classList.remove('dragging'); });
            tab.addEventListener('dragover', (e) => { e.preventDefault(); e.dataTransfer.dropEffect = 'move'; tab.classList.add('drag-over'); });
            tab.addEventListener('dragleave', () => { tab.classList.remove('drag-over'); });
            tab.addEventListener('drop', (e) => { e.preventDefault(); tab.classList.remove('drag-over'); const fi = parseInt(e.dataTransfer.getData('text/plain'), 10); if (fi !== i) this.moveSheet(fi, i); });
            this.tabBar.appendChild(tab);
        }
        const addBtn = document.createElement('button');
        addBtn.className = 'ss-tab-add';
        addBtn.textContent = '+';
        addBtn.title = 'Add new sheet';
        addBtn.addEventListener('click', () => this.addSheet());
        this.tabBar.appendChild(addBtn);
    }

    _startInlineRename(tab, index) {
        const sh = this.workbook.sheets[index];
        const input = document.createElement('input');
        input.value = sh.name;
        input.style.cssText = 'width:' + Math.max(tab.offsetWidth, 60) + 'px;height:100%;border:1px solid #1a73e8;border-radius:3px;padding:2px 6px;font-size:12px;outline:none;background:#fff;';
        tab.textContent = '';
        tab.appendChild(input);
        input.focus();
        input.select();
        const commit = () => { const n = input.value.trim(); if (n && n !== sh.name) this.renameSheet(index, n); else this.updateSheetTabs(); };
        input.addEventListener('blur', commit);
        input.addEventListener('keydown', (e) => { if (e.key === 'Enter') { e.preventDefault(); input.blur(); } if (e.key === 'Escape') { input.value = sh.name; input.blur(); } });
    }

    _showTabContextMenu(clientX, clientY, index) {
        if (this._tabContextMenu) this._tabContextMenu.remove();
        const menu = document.createElement('div');
        menu.className = 'ss-context-menu';
        menu.style.position = 'fixed';
        menu.style.left = clientX + 'px';
        menu.style.top = clientY + 'px';
        menu.style.zIndex = '200';
        const hasHidden = this._hiddenSheets && this._hiddenSheets.size > 0;
        const items = [
            { label: 'Rename', action: () => { const tabs = this.tabBar.querySelectorAll('.ss-tab'); for (const t of tabs) { if (parseInt(t.dataset.sheetIndex, 10) === index) { this._startInlineRename(t, index); break; } } }},
            { label: 'Duplicate', action: () => this.duplicateSheet(index) },
            { label: '---' },
            { label: 'Move Left', action: () => { if (index > 0) this.moveSheet(index, index - 1); }, disabled: index === 0 },
            { label: 'Move Right', action: () => { if (index < this.workbook.sheets.length - 1) this.moveSheet(index, index + 1); }, disabled: index >= this.workbook.sheets.length - 1 },
            { label: '---' },
            { label: 'Hide', action: () => this.hideSheet(index), disabled: this.workbook.sheets.length <= 1 },
            ...(hasHidden ? [{ label: 'Unhide...', action: () => this._showUnhideDialog() }] : []),
            { label: '---' },
            { label: 'Tab Color', action: () => { const ci = document.createElement('input'); ci.type = 'color'; ci.value = this.workbook.sheets[index].tabColor || '#1a73e8'; ci.style.cssText = 'position:fixed;opacity:0;pointer-events:none;'; document.body.appendChild(ci); ci.addEventListener('input', () => this.setSheetTabColor(index, ci.value)); ci.addEventListener('change', () => { this.setSheetTabColor(index, ci.value); ci.remove(); }); ci.click(); }},
            { label: '---' },
            { label: 'Delete', action: () => { if (this.workbook.sheets.length > 1) ssConfirm('Delete sheet "' + this.workbook.sheets[index].name + '"?').then((ok) => { if (ok) this.deleteSheet(index); }); }, disabled: this.workbook.sheets.length <= 1 },
        ];
        for (const item of items) {
            if (item.label === '---') { const sep = document.createElement('div'); sep.className = 'ss-ctx-sep'; menu.appendChild(sep); }
            else { const btn = document.createElement('button'); btn.className = 'ss-ctx-item'; if (item.disabled) btn.style.opacity = '0.4'; btn.textContent = item.label; btn.addEventListener('click', () => { menu.remove(); this._tabContextMenu = null; if (!item.disabled) item.action(); }); menu.appendChild(btn); }
        }
        document.body.appendChild(menu);
        this._tabContextMenu = menu;
        const closeHandler = (e) => { if (!menu.contains(e.target)) { menu.remove(); this._tabContextMenu = null; document.removeEventListener('mousedown', closeHandler); } };
        setTimeout(() => document.addEventListener('mousedown', closeHandler), 0);
    }

    _showUnhideDialog() {
        if (!this._hiddenSheets || this._hiddenSheets.size === 0) return;
        const overlay = document.createElement('div'); overlay.className = 'modal-overlay show';
        const modal = document.createElement('div'); modal.className = 'modal'; modal.style.minWidth = '300px';
        let list = '';
        for (const idx of this._hiddenSheets) { const sh = this.workbook.sheets[idx]; if (sh) list += '<label style="display:flex;align-items:center;gap:8px;font-size:13px;padding:4px 0;cursor:pointer;"><input type="checkbox" class="unhide-check" data-idx="' + idx + '"> ' + this._xmlEsc(sh.name) + '</label>'; }
        modal.innerHTML = '<h3>Unhide Sheets</h3><div style="padding:8px 0;">' + list + '</div><div class="modal-actions"><button class="ss-modal-cancel">Cancel</button><button class="ss-modal-ok primary">Unhide</button></div>';
        overlay.appendChild(modal); document.body.appendChild(overlay);
        const close = () => { document.body.removeChild(overlay); };
        modal.querySelector('.ss-modal-cancel').onclick = close;
        modal.querySelector('.ss-modal-ok').onclick = () => { modal.querySelectorAll('.unhide-check:checked').forEach(cb => this.unhideSheet(parseInt(cb.dataset.idx, 10))); close(); };
        overlay.onclick = (e) => { if (e.target === overlay) close(); };
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
        // S4.5: Update formula bar syntax highlight when formula bar value changes
        this._updateFormulaHighlight();
        // Update properties panel if open
        this._updatePropsPanel();
        // S4.7: Accessibility — announce selected cell via live region
        if (this._ariaLive) {
            const ref = this.getCellA1(this.selectedCell.col, this.selectedCell.row);
            const display = this._getCellDisplay(this.selectedCell.col, this.selectedCell.row);
            this._ariaLive.textContent = display ? `Cell ${ref}, value: ${display}` : `Cell ${ref}, empty`;
        }
        // S4.7: Update aria-activedescendant
        if (this.canvasWrap) {
            this.canvasWrap.setAttribute('aria-activedescendant', 'ss-cell-' + this.selectedCell.col + '-' + this.selectedCell.row);
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
        // Clear source cells with a single batch undo entry
        const range = this.selectionRange
            ? this._normalizeRange(this.selectionRange)
            : { startCol: this.selectedCell.col, startRow: this.selectedCell.row, endCol: this.selectedCell.col, endRow: this.selectedCell.row };

        const sheet = this._sheet();
        if (!sheet) return;
        const batchActions = [];
        for (let c = range.startCol; c <= range.endCol; c++) {
            for (let r = range.startRow; r <= range.endRow; r++) {
                const oldCell = sheet.getCell(c, r);
                if (!oldCell) continue;
                const oldCopy = { ...oldCell };
                sheet.deleteCell(c, r);
                batchActions.push({
                    type: 'edit', col: c, row: r, sheetIndex: this.activeSheet,
                    oldValue: oldCopy, newValue: null
                });
            }
        }
        if (batchActions.length > 0) {
            this._undoManager.push({ type: 'batch', sheetIndex: this.activeSheet, actions: batchActions });
        }
        this.render();
    }

    pasteCells() {
        if (!this._clipboard) {
            // U12: Try HTML clipboard first for table detection, then plain text
            if (navigator.clipboard.read) {
                navigator.clipboard.read().then(items => {
                    for (const item of items) {
                        if (item.types.includes('text/html')) {
                            item.getType('text/html').then(blob => blob.text()).then(html => {
                                const parsed = this._parseHtmlTable(html);
                                if (parsed) {
                                    this._pasteGrid(parsed);
                                } else {
                                    // Fallback: try plain text
                                    item.getType('text/plain').then(blob => blob.text()).then(text => {
                                        this._pasteGrid(this._detectTableInText(text));
                                    }).catch(() => {});
                                }
                            }).catch(() => {});
                            return;
                        }
                    }
                    // No HTML — try plain text
                    navigator.clipboard.readText().then(text => {
                        if (text) this._pasteGrid(this._detectTableInText(text));
                    }).catch(() => {});
                }).catch(() => {
                    // clipboard.read() not supported — fallback to readText
                    navigator.clipboard.readText().then(text => {
                        if (text) this._pasteGrid(this._detectTableInText(text));
                    }).catch(() => {});
                });
            } else {
                navigator.clipboard.readText().then(text => {
                    if (text) this._pasteGrid(this._detectTableInText(text));
                }).catch(() => {});
            }
            return;
        }

        const sheet = this._sheet();
        if (!sheet) return;
        const { col, row } = this.selectedCell;
        const { cells, width, height } = this._clipboard;

        for (let c = 0; c < width; c++) {
            for (let r = 0; r < height; r++) {
                const src = cells[`${c},${r}`];
                if (src) {
                    const rawVal = src.formula || String(src.value ?? '');
                    this._setCellValue(col + c, row + r, rawVal);
                    // U11: Preserve style from source cell on paste
                    if (src.style && Object.keys(src.style).length > 0) {
                        const targetCell = sheet.getCell(col + c, row + r);
                        if (targetCell) {
                            targetCell.style = { ...src.style };
                        }
                    }
                } else {
                    this._setCellValue(col + c, row + r, '');
                }
            }
        }
        this.render();
    }

    // U12 ── Table Detection Helpers ──────────────

    _pasteGrid(grid) {
        if (!grid || grid.rows.length === 0) return;
        const { col, row } = this.selectedCell;
        const sheet = this._sheet();
        if (!sheet) return;
        for (let r = 0; r < grid.rows.length; r++) {
            const rowData = grid.rows[r];
            for (let c = 0; c < rowData.length; c++) {
                const val = rowData[c];
                if (val && val.value !== undefined) {
                    this._setCellValue(col + c, row + r, String(val.value));
                    if (val.style) {
                        const cell = sheet.getCell(col + c, row + r);
                        if (cell) cell.style = { ...cell.style, ...val.style };
                    }
                } else if (typeof val === 'string') {
                    this._setCellValue(col + c, row + r, val);
                }
            }
        }
        // Auto-fit column widths based on content
        if (grid.rows.length > 0) {
            const maxCols = Math.max(...grid.rows.map(r => r.length));
            for (let c = 0; c < maxCols; c++) {
                let maxLen = 8;
                for (let r = 0; r < Math.min(grid.rows.length, 50); r++) {
                    const v = grid.rows[r][c];
                    const text = typeof v === 'string' ? v : (v?.value ?? '');
                    if (String(text).length > maxLen) maxLen = String(text).length;
                }
                const width = Math.min(Math.max(maxLen * 8 + 16, 60), 300);
                if (width > this.getColumnWidth(col + c)) {
                    sheet.colWidths[col + c] = width;
                }
            }
        }
        this.render();
    }

    _parseHtmlTable(html) {
        // Parse HTML clipboard data for <table> elements
        const parser = new DOMParser();
        const doc = parser.parseFromString(html, 'text/html');
        const table = doc.querySelector('table');
        if (!table) return null;
        const rows = [];
        for (const tr of table.querySelectorAll('tr')) {
            const row = [];
            for (const td of tr.querySelectorAll('td, th')) {
                const style = {};
                if (td.tagName === 'TH' || td.style.fontWeight === 'bold' || td.querySelector('b, strong')) {
                    style.bold = true;
                }
                if (td.style.fontStyle === 'italic' || td.querySelector('i, em')) {
                    style.italic = true;
                }
                if (td.style.backgroundColor) {
                    style.fill = td.style.backgroundColor;
                }
                if (td.style.color) {
                    style.color = td.style.color;
                }
                if (td.style.textAlign) {
                    style.align = td.style.textAlign;
                }
                const text = td.textContent.trim();
                row.push(Object.keys(style).length > 0 ? { value: text, style } : text);
            }
            if (row.length > 0) rows.push(row);
        }
        return rows.length > 0 ? { rows, isTable: true } : null;
    }

    _detectTableInText(text) {
        if (!text) return { rows: [] };
        const lines = text.split('\n').filter(l => l.trim() !== '');
        if (lines.length === 0) return { rows: [] };

        // Detect delimiter: tab > pipe > semicolon > comma
        const firstLine = lines[0];
        let delimiter = '\t';
        if (firstLine.includes('\t')) {
            delimiter = '\t';
        } else if (firstLine.includes('|') && lines.every(l => l.includes('|'))) {
            delimiter = '|';
        } else if (firstLine.includes(';') && lines.every(l => l.includes(';'))) {
            delimiter = ';';
        } else if (firstLine.includes(',') && lines.every(l => l.includes(','))) {
            delimiter = ',';
        }

        const rows = [];
        for (const line of lines) {
            // For pipe-separated, strip leading/trailing pipes
            let cleaned = line;
            if (delimiter === '|') {
                cleaned = cleaned.replace(/^\||\|$/g, '');
            }
            // Skip markdown separator rows (---|---|---)
            if (delimiter === '|' && /^[\s\-:|]+$/.test(cleaned)) continue;

            const cols = delimiter === ','
                ? this._splitCSVLine(cleaned)
                : cleaned.split(delimiter).map(s => s.trim());
            rows.push(cols);
        }

        // Detect if first row is a header (all string values, different pattern from data rows)
        if (rows.length > 1) {
            const firstRow = rows[0];
            const secondRow = rows[1];
            const firstAllText = firstRow.every(c => isNaN(Number(c)) || c === '');
            const secondHasNums = secondRow.some(c => !isNaN(Number(c)) && c !== '');
            if (firstAllText && secondHasNums) {
                // Apply bold to header row
                rows[0] = firstRow.map(v => ({ value: v, style: { bold: true } }));
            }
        }

        return { rows, isTable: rows.length > 1 && rows[0].length > 1 };
    }

    _splitCSVLine(line) {
        const result = [];
        let current = '';
        let inQuotes = false;
        for (let i = 0; i < line.length; i++) {
            const ch = line[i];
            if (inQuotes) {
                if (ch === '"' && i + 1 < line.length && line[i + 1] === '"') {
                    current += '"'; i++;
                } else if (ch === '"') {
                    inQuotes = false;
                } else {
                    current += ch;
                }
            } else {
                if (ch === '"') { inQuotes = true; }
                else if (ch === ',') { result.push(current.trim()); current = ''; }
                else { current += ch; }
            }
        }
        result.push(current.trim());
        return result;
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
        // S16: Helper to adjust cell references in formulas by row/col offset
        const adjustFormulaRefs = (formula, rowDelta, colDelta) => {
            return formula.replace(/(\$?)([A-Z]{1,3})(\$?)(\d+)/g, (_m, colLock, colStr, rowLock, rowStr) => {
                let newCol = colStr;
                let newRow = parseInt(rowStr);
                if (!colLock && colDelta) {
                    let colNum = 0;
                    for (let i = 0; i < colStr.length; i++) colNum = colNum * 26 + (colStr.charCodeAt(i) - 64);
                    colNum += colDelta;
                    if (colNum < 1) colNum = 1;
                    newCol = '';
                    let cn = colNum;
                    while (cn > 0) { cn--; newCol = String.fromCharCode(65 + (cn % 26)) + newCol; cn = Math.floor(cn / 26); }
                }
                if (!rowLock) { newRow += rowDelta; if (newRow < 1) newRow = 1; }
                return (colLock || '') + newCol + (rowLock || '') + newRow;
            });
        };
        // Auto-fill: repeat existing cell values in the direction
        if (direction === 'down') {
            const srcRow = nr.startRow;
            for (let r = nr.startRow + 1; r <= nr.endRow; r++) {
                for (let c = nr.startCol; c <= nr.endCol; c++) {
                    const srcCell = sheet.getCell(c, srcRow);
                    if (srcCell && srcCell.formula) {
                        const adjusted = adjustFormulaRefs(srcCell.formula, r - srcRow, 0);
                        this._setCellValue(c, r, adjusted);
                    } else if (srcCell && srcCell.type === 'number') {
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
                    if (srcCell && srcCell.formula) {
                        const adjusted = adjustFormulaRefs(srcCell.formula, 0, c - srcCol);
                        this._setCellValue(c, r, adjusted);
                    } else if (srcCell && srcCell.type === 'number') {
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

    // ─── Cell Formatting ─────────────────────────
    getSelectionRange() {
        if (this.selectionRange) {
            return this._normalizeRange(this.selectionRange);
        }
        return {
            startCol: this.selectedCell.col,
            startRow: this.selectedCell.row,
            endCol: this.selectedCell.col,
            endRow: this.selectedCell.row
        };
    }

    // Returns true if all cells are selected (via top-left corner click)
    _isAllSelected() {
        if (!this.selectionRange) return false;
        const nr = this._normalizeRange(this.selectionRange);
        return nr.startCol === 0 && nr.startRow === 0 &&
               nr.endCol >= this.getMaxCol() && nr.endRow >= this.getMaxRow();
    }

    getActiveStyle() {
        // Returns the style of the active cell (for toggling toolbar state)
        const sheet = this._sheet();
        if (!sheet) return {};
        const cell = sheet.getCell(this.selectedCell.col, this.selectedCell.row);
        return cell?.style || {};
    }

    setFormat(key, value) {
        const sheet = this._sheet();
        if (!sheet) return;
        const range = this.getSelectionRange();
        for (let c = range.startCol; c <= range.endCol; c++) {
            for (let r = range.startRow; r <= range.endRow; r++) {
                let cell = sheet.getCell(c, r);
                if (!cell) {
                    cell = { value: '', formula: null, display: '', type: 'string', style: {} };
                    sheet.setCell(c, r, cell);
                }
                if (!cell.style) cell.style = {};
                if (value === null || value === undefined) {
                    delete cell.style[key];
                } else {
                    cell.style[key] = value;
                }
                // S4.1: Broadcast format change
                this.broadcastFormatChange(this.activeSheet, c, r, cell.style);
            }
        }
        this.render();
    }

    toggleFormat(key) {
        const style = this.getActiveStyle();
        this.setFormat(key, !style[key]);
    }

    // ─── Find & Replace ─────────────────────────
    openFindBar(replaceMode = false) {
        if (this._findBar) {
            // Already open — just toggle replace mode if needed
            if (replaceMode && this._findReplaceRow) {
                this._findReplaceRow.style.display = 'flex';
            }
            this._findInput.focus();
            this._findInput.select();
            return;
        }

        const bar = document.createElement('div');
        bar.className = 'ss-find-bar';

        // Find row
        const findRow = document.createElement('div');
        findRow.className = 'ss-find-row';

        const findInput = document.createElement('input');
        findInput.className = 'ss-find-input';
        findInput.placeholder = 'Find in sheet...';
        findInput.title = 'Search text';
        this._findInput = findInput;

        const findInfo = document.createElement('span');
        findInfo.className = 'ss-find-info';
        findInfo.textContent = '';
        this._findInfo = findInfo;

        const prevBtn = document.createElement('button');
        prevBtn.className = 'ss-find-btn';
        prevBtn.innerHTML = '<span class="msi">keyboard_arrow_up</span>';
        prevBtn.title = 'Previous match (Shift+Enter)';
        prevBtn.addEventListener('click', () => this._findPrev());

        const nextBtn = document.createElement('button');
        nextBtn.className = 'ss-find-btn';
        nextBtn.innerHTML = '<span class="msi">keyboard_arrow_down</span>';
        nextBtn.title = 'Next match (Enter)';
        nextBtn.addEventListener('click', () => this._findNext());

        const closeBtn = document.createElement('button');
        closeBtn.className = 'ss-find-btn';
        closeBtn.innerHTML = '<span class="msi">close</span>';
        closeBtn.title = 'Close find bar (Escape)';
        closeBtn.addEventListener('click', () => this.closeFindBar());

        findRow.appendChild(findInput);
        findRow.appendChild(findInfo);
        findRow.appendChild(prevBtn);
        findRow.appendChild(nextBtn);
        findRow.appendChild(closeBtn);
        bar.appendChild(findRow);

        // Replace row
        const replaceRow = document.createElement('div');
        replaceRow.className = 'ss-find-row';
        replaceRow.style.display = replaceMode ? 'flex' : 'none';
        this._findReplaceRow = replaceRow;

        const replaceInput = document.createElement('input');
        replaceInput.className = 'ss-find-input';
        replaceInput.placeholder = 'Replace with...';
        replaceInput.title = 'Replacement text';
        this._replaceInput = replaceInput;

        const replaceBtn = document.createElement('button');
        replaceBtn.className = 'ss-find-btn ss-find-btn-text';
        replaceBtn.textContent = 'Replace';
        replaceBtn.title = 'Replace current match';
        replaceBtn.addEventListener('click', () => this._replaceCurrent());

        const replaceAllBtn = document.createElement('button');
        replaceAllBtn.className = 'ss-find-btn ss-find-btn-text';
        replaceAllBtn.textContent = 'Replace All';
        replaceAllBtn.title = 'Replace all matches';
        replaceAllBtn.addEventListener('click', () => this._replaceAll());

        replaceRow.appendChild(replaceInput);
        replaceRow.appendChild(replaceBtn);
        replaceRow.appendChild(replaceAllBtn);
        bar.appendChild(replaceRow);

        // Insert before canvas wrap
        this.container.insertBefore(bar, this.canvasWrap);
        this._findBar = bar;

        // Search state
        this._findMatches = [];
        this._findIndex = -1;

        // Events on find input
        findInput.addEventListener('input', () => this._performSearch());
        findInput.addEventListener('keydown', (e) => {
            if (e.key === 'Enter') {
                e.preventDefault();
                if (e.shiftKey) {
                    this._findPrev();
                } else {
                    this._findNext();
                }
            } else if (e.key === 'Escape') {
                this.closeFindBar();
            }
        });

        replaceInput.addEventListener('keydown', (e) => {
            if (e.key === 'Escape') {
                this.closeFindBar();
            }
        });

        findInput.focus();
    }

    closeFindBar() {
        if (this._findBar) {
            this._findBar.remove();
            this._findBar = null;
            this._findInput = null;
            this._findInfo = null;
            this._findReplaceRow = null;
            this._replaceInput = null;
            this._findMatches = [];
            this._findIndex = -1;
            this.canvas.focus();
            this.render();
        }
    }

    _performSearch() {
        const query = this._findInput?.value?.toLowerCase();
        this._findMatches = [];
        this._findIndex = -1;

        if (!query) {
            if (this._findInfo) this._findInfo.textContent = '';
            this.render();
            return;
        }

        const sheet = this._sheet();
        if (!sheet) return;

        // Search all cells row by row, column by column
        for (let r = 0; r <= sheet.maxRow; r++) {
            for (let c = 0; c <= sheet.maxCol; c++) {
                const cell = sheet.getCell(c, r);
                if (!cell) continue;
                const display = cell.formula
                    ? String(evaluateFormula(cell.formula, sheet))
                    : String(cell.value ?? '');
                if (display.toLowerCase().includes(query)) {
                    this._findMatches.push({ col: c, row: r });
                }
            }
        }

        if (this._findMatches.length > 0) {
            this._findIndex = 0;
            this._goToMatch(0);
        }
        this._updateFindInfo();
        this.render();
    }

    _findNext() {
        if (this._findMatches.length === 0) {
            this._performSearch();
            return;
        }
        this._findIndex = (this._findIndex + 1) % this._findMatches.length;
        this._goToMatch(this._findIndex);
        this._updateFindInfo();
        this.render();
    }

    _findPrev() {
        if (this._findMatches.length === 0) {
            this._performSearch();
            return;
        }
        this._findIndex = (this._findIndex - 1 + this._findMatches.length) % this._findMatches.length;
        this._goToMatch(this._findIndex);
        this._updateFindInfo();
        this.render();
    }

    _goToMatch(index) {
        const match = this._findMatches[index];
        if (!match) return;
        this.selectedCell = { col: match.col, row: match.row };
        this.selectionRange = null;
        this._ensureVisible(match.col, match.row);
        this._updateFormulaBar();
    }

    _updateFindInfo() {
        if (!this._findInfo) return;
        if (this._findMatches.length === 0) {
            const query = this._findInput?.value;
            this._findInfo.textContent = query ? 'No results' : '';
        } else {
            this._findInfo.textContent = `${this._findIndex + 1} of ${this._findMatches.length}`;
        }
    }

    _replaceCurrent() {
        if (this._findMatches.length === 0 || this._findIndex < 0) return;
        const match = this._findMatches[this._findIndex];
        const sheet = this._sheet();
        if (!sheet) return;

        const cell = sheet.getCell(match.col, match.row);
        if (!cell) return;

        const query = this._findInput?.value || '';
        const replacement = this._replaceInput?.value || '';
        const currentVal = cell.formula || String(cell.value ?? '');
        const newVal = currentVal.replace(new RegExp(query.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), 'gi'), replacement);
        this._setCellValue(match.col, match.row, newVal);

        // Re-search to update matches
        this._performSearch();
        this.render();
    }

    _replaceAll() {
        if (this._findMatches.length === 0) return;
        const query = this._findInput?.value || '';
        const replacement = this._replaceInput?.value || '';
        if (!query) return;

        const sheet = this._sheet();
        if (!sheet) return;

        const regex = new RegExp(query.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), 'gi');
        let count = 0;

        for (const match of this._findMatches) {
            const cell = sheet.getCell(match.col, match.row);
            if (!cell) continue;
            const currentVal = cell.formula || String(cell.value ?? '');
            const newVal = currentVal.replace(regex, replacement);
            if (newVal !== currentVal) {
                this._setCellValue(match.col, match.row, newVal);
                count++;
            }
        }

        // Re-search
        this._performSearch();
        this.render();
    }

    // Override render to also draw find highlights
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
        ctx.fillStyle = this._getThemeColors().bg;
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

        // Draw find highlights
        if (this._findMatches && this._findMatches.length > 0) {
            this._renderFindHighlights(ctx, visibleCols, visibleRows);
        }

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

        // S4.5: Draw formula reference highlights when editing a formula
        this._renderFormulaRefHighlights(ctx);

        // Draw comment indicators
        this._renderCommentIndicators(ctx, visibleCols, visibleRows);

        // Draw validation dropdown arrows
        this._renderValidationIndicators(ctx, visibleCols, visibleRows);

        // S4.1: Draw peer cursors (collaboration)
        this._renderPeerCursors(ctx);

        ctx.restore();
    }

    _renderFindHighlights(ctx, visibleCols, visibleRows) {
        if (!this._findMatches || this._findMatches.length === 0) return;

        // Build a set for quick lookup of visible cells
        const visColSet = new Set(visibleCols.map(vc => vc.col));
        const visRowSet = new Set(visibleRows.map(vr => vr.row));

        for (let i = 0; i < this._findMatches.length; i++) {
            const match = this._findMatches[i];
            if (!visColSet.has(match.col) || !visRowSet.has(match.row)) continue;

            const sx = this._cellScreenX(match.col);
            const sy = this._cellScreenY(match.row);
            const sw = this.getColumnWidth(match.col);
            const sh = this.getRowHeight(match.row);

            if (i === this._findIndex) {
                // Current match — orange highlight
                ctx.fillStyle = 'rgba(255, 152, 0, 0.35)';
                ctx.fillRect(sx, sy, sw, sh);
                ctx.strokeStyle = '#ff9800';
                ctx.lineWidth = 2;
                ctx.strokeRect(sx, sy, sw, sh);
            } else {
                // Other matches — yellow highlight
                ctx.fillStyle = 'rgba(255, 235, 59, 0.3)';
                ctx.fillRect(sx, sy, sw, sh);
            }
        }
    }

    // ─── Merge Cells (S1.7) ────────────────────────
    mergeCells() {
        const sheet = this._sheet();
        if (!sheet) return;
        const range = this.getSelectionRange();
        if (range.startCol === range.endCol && range.startRow === range.endRow) return;
        const existingIdx = sheet.merges.findIndex(m =>
            m.startCol === range.startCol && m.startRow === range.startRow &&
            m.endCol === range.endCol && m.endRow === range.endRow
        );
        if (existingIdx >= 0) {
            sheet.merges.splice(existingIdx, 1);
        } else {
            for (let c = range.startCol; c <= range.endCol; c++) {
                for (let r = range.startRow; r <= range.endRow; r++) {
                    if (c === range.startCol && r === range.startRow) continue;
                    this._setCellValue(c, r, '');
                }
            }
            sheet.merges.push({ startCol: range.startCol, startRow: range.startRow, endCol: range.endCol, endRow: range.endRow });
        }
        this.render();
    }

    isSelectionMerged() {
        const range = this.getSelectionRange();
        const sheet = this._sheet();
        if (!sheet) return false;
        return sheet.merges.some(m =>
            m.startCol === range.startCol && m.startRow === range.startRow &&
            m.endCol === range.endCol && m.endRow === range.endRow
        );
    }

    // ─── Number Format ──────────────────────────────
    setNumberFormat(format) {
        const sheet = this._sheet();
        if (!sheet) return;
        const range = this.getSelectionRange();
        for (let c = range.startCol; c <= range.endCol; c++) {
            for (let r = range.startRow; r <= range.endRow; r++) {
                let cell = sheet.getCell(c, r);
                if (!cell) { cell = { value: '', formula: null, display: '', type: 'string', style: {} }; sheet.setCell(c, r, cell); }
                if (!cell.style) cell.style = {};
                cell.style.numberFormat = format;
                if (cell.type === 'number' || typeof cell.value === 'number') {
                    cell.display = this._formatNumber(cell.value, format);
                }
            }
        }
        this.render();
    }

    _formatNumber(value, format) {
        const num = Number(value);
        if (isNaN(num)) return String(value);
        switch (format) {
            case 'number': return num.toFixed(2);
            case 'currency': return '$' + num.toFixed(2);
            case 'percentage': return (num * 100).toFixed(1) + '%';
            case 'date': { const adj = num > 59 ? num - 1 : num; const d = new Date((adj - 25569) * 86400000); return isNaN(d.getTime()) ? String(num) : d.toLocaleDateString(); }
            case 'time': { const frac = num % 1; const ts = Math.round(frac * 86400); return String(Math.floor(ts/3600)).padStart(2,'0')+':'+String(Math.floor((ts%3600)/60)).padStart(2,'0')+':'+String(ts%60).padStart(2,'0'); }
            default: return String(value);
        }
    }

    // ─── XLSX Export (S1.3) ──────────────────────────
    exportXLSX() {
        const wb = this.workbook;
        if (!wb) return new Uint8Array(0);
        const files = {};
        let ctSheets = '';
        for (let i = 0; i < wb.sheets.length; i++) {
            ctSheets += '<Override PartName="/xl/worksheets/sheet'+(i+1)+'.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>';
        }
        files['[Content_Types].xml'] = '<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>'+ctSheets+'<Override PartName="/xl/sharedStrings.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sharedStrings+xml"/></Types>';
        files['_rels/.rels'] = '<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>';
        let wbRels = '<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">';
        for (let i = 0; i < wb.sheets.length; i++) wbRels += '<Relationship Id="rId'+(i+1)+'" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet'+(i+1)+'.xml"/>';
        wbRels += '<Relationship Id="rId'+(wb.sheets.length+1)+'" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings" Target="sharedStrings.xml"/></Relationships>';
        files['xl/_rels/workbook.xml.rels'] = wbRels;
        const sharedStrings = []; const ssMap = new Map();
        for (const sheet of wb.sheets) {
            for (const key of Object.keys(sheet.cells)) {
                const cell = sheet.cells[key];
                if (cell && cell.type === 'string' && cell.value !== '' && cell.value != null) {
                    const sv = String(cell.value);
                    if (!ssMap.has(sv)) { ssMap.set(sv, sharedStrings.length); sharedStrings.push(sv); }
                }
            }
        }
        let ssXml = '<?xml version="1.0" encoding="UTF-8" standalone="yes"?><sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" count="'+sharedStrings.length+'" uniqueCount="'+sharedStrings.length+'">';
        for (const s of sharedStrings) ssXml += '<si><t>'+this._xmlEsc(s)+'</t></si>';
        ssXml += '</sst>';
        files['xl/sharedStrings.xml'] = ssXml;
        let wbXml = '<?xml version="1.0" encoding="UTF-8" standalone="yes"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets>';
        for (let i = 0; i < wb.sheets.length; i++) wbXml += '<sheet name="'+this._xmlEsc(wb.sheets[i].name)+'" sheetId="'+(i+1)+'" r:id="rId'+(i+1)+'"/>';
        wbXml += '</sheets></workbook>';
        files['xl/workbook.xml'] = wbXml;
        for (let si = 0; si < wb.sheets.length; si++) {
            const sheet = wb.sheets[si];
            let sx = '<?xml version="1.0" encoding="UTF-8" standalone="yes"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">';
            if (sheet.merges && sheet.merges.length > 0) {
                sx += '<mergeCells count="'+sheet.merges.length+'">';
                for (const m of sheet.merges) sx += '<mergeCell ref="'+this.getCellA1(m.startCol,m.startRow)+':'+this.getCellA1(m.endCol,m.endRow)+'"/>';
                sx += '</mergeCells>';
            }
            sx += '<sheetData>';
            for (let r = 0; r <= sheet.maxRow; r++) {
                let rHas = false; let rx = '<row r="'+(r+1)+'">';
                for (let c = 0; c <= sheet.maxCol; c++) {
                    const cell = sheet.cells[c+','+r]; if (!cell) continue; rHas = true;
                    const ref = this.getCellA1Col(c)+String(r+1);
                    if (cell.formula) { const res = evaluateFormula(cell.formula, sheet); rx += '<c r="'+ref+'"><f>'+this._xmlEsc(cell.formula.slice(1))+'</f><v>'+this._xmlEsc(String(res))+'</v></c>'; }
                    else if (cell.type === 'number' || typeof cell.value === 'number') rx += '<c r="'+ref+'"><v>'+cell.value+'</v></c>';
                    else if (cell.type === 'boolean') rx += '<c r="'+ref+'" t="b"><v>'+(cell.value?'1':'0')+'</v></c>';
                    else { const sv = String(cell.value??''); const idx = ssMap.get(sv); if (idx!==undefined) rx += '<c r="'+ref+'" t="s"><v>'+idx+'</v></c>'; else rx += '<c r="'+ref+'" t="inlineStr"><is><t>'+this._xmlEsc(sv)+'</t></is></c>'; }
                }
                rx += '</row>'; if (rHas) sx += rx;
            }
            sx += '</sheetData></worksheet>';
            files['xl/worksheets/sheet'+(si+1)+'.xml'] = sx;
        }
        return this._buildZip(files);
    }

    _xmlEsc(str) { return String(str).replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/"/g,'&quot;'); }

    _buildZip(files) {
        const entries = []; const enc = new TextEncoder();
        for (const [name, content] of Object.entries(files)) entries.push({ name: enc.encode(name), data: typeof content === 'string' ? enc.encode(content) : content });
        let totalSize = 0; const offsets = [];
        for (const e of entries) { offsets.push(totalSize); totalSize += 30 + e.name.length + e.data.length; }
        const cdOffset = totalSize;
        for (const e of entries) totalSize += 46 + e.name.length;
        totalSize += 22;
        const buf = new Uint8Array(totalSize); const dv = new DataView(buf.buffer); let off = 0;
        for (let i = 0; i < entries.length; i++) {
            const e = entries[i]; const crc = this._crc32(e.data);
            dv.setUint32(off,0x04034B50,true);off+=4;dv.setUint16(off,20,true);off+=2;dv.setUint16(off,0,true);off+=2;dv.setUint16(off,0,true);off+=2;
            dv.setUint16(off,0,true);off+=2;dv.setUint16(off,0,true);off+=2;dv.setUint32(off,crc,true);off+=4;
            dv.setUint32(off,e.data.length,true);off+=4;dv.setUint32(off,e.data.length,true);off+=4;
            dv.setUint16(off,e.name.length,true);off+=2;dv.setUint16(off,0,true);off+=2;
            buf.set(e.name,off);off+=e.name.length;buf.set(e.data,off);off+=e.data.length;
        }
        const cdStart = off;
        for (let i = 0; i < entries.length; i++) {
            const e = entries[i]; const crc = this._crc32(e.data);
            dv.setUint32(off,0x02014B50,true);off+=4;dv.setUint16(off,20,true);off+=2;dv.setUint16(off,20,true);off+=2;dv.setUint16(off,0,true);off+=2;
            dv.setUint16(off,0,true);off+=2;dv.setUint16(off,0,true);off+=2;dv.setUint16(off,0,true);off+=2;dv.setUint32(off,crc,true);off+=4;
            dv.setUint32(off,e.data.length,true);off+=4;dv.setUint32(off,e.data.length,true);off+=4;
            dv.setUint16(off,e.name.length,true);off+=2;dv.setUint16(off,0,true);off+=2;dv.setUint16(off,0,true);off+=2;
            dv.setUint16(off,0,true);off+=2;dv.setUint16(off,0,true);off+=2;dv.setUint32(off,0,true);off+=4;
            dv.setUint32(off,offsets[i],true);off+=4;buf.set(e.name,off);off+=e.name.length;
        }
        const cdSize = off - cdStart;
        dv.setUint32(off,0x06054B50,true);off+=4;dv.setUint16(off,0,true);off+=2;dv.setUint16(off,0,true);off+=2;
        dv.setUint16(off,entries.length,true);off+=2;dv.setUint16(off,entries.length,true);off+=2;
        dv.setUint32(off,cdSize,true);off+=4;dv.setUint32(off,cdStart,true);off+=4;dv.setUint16(off,0,true);off+=2;
        return buf;
    }

    _crc32(data) {
        let crc = 0xFFFFFFFF;
        for (let i = 0; i < data.length; i++) { crc ^= data[i]; for (let j = 0; j < 8; j++) crc = (crc>>>1)^(crc&1?0xEDB88320:0); }
        return (crc ^ 0xFFFFFFFF) >>> 0;
    }

    // ─── Print to PDF (S1.6) ────────────────────────
    printToPDF() {
        const sheet = this._sheet();
        if (!sheet) return;
        const wb = this.workbook;
        if (!wb) return;

        let html = '<!DOCTYPE html><html><head><meta charset="utf-8"><title>Print Spreadsheet</title>';
        html += '<style>';
        html += 'body { font-family: Arial, sans-serif; font-size: 11px; margin: 20px; color: #202124; }';
        html += 'h2 { font-size: 14px; margin: 20px 0 8px 0; color: #333; }';
        html += 'table { border-collapse: collapse; width: 100%; page-break-inside: auto; margin-bottom: 24px; }';
        html += 'tr { page-break-inside: avoid; page-break-after: auto; }';
        html += 'th, td { border: 1px solid #bbb; padding: 4px 8px; text-align: left; vertical-align: top; }';
        html += 'th { background: #f0f0f0; font-weight: 600; text-align: center; }';
        html += 'td.num { text-align: right; }';
        html += '.sheet-break { page-break-before: always; }';
        html += '@media print { body { margin: 10px; } }';
        html += '</style></head><body>';

        for (let si = 0; si < wb.sheets.length; si++) {
            const s = wb.sheets[si];
            if (si > 0) html += '<div class="sheet-break"></div>';
            html += '<h2>' + this._xmlEsc(s.name) + '</h2>';
            html += '<table>';
            // Column headers
            html += '<thead><tr><th></th>';
            for (let c = 0; c <= s.maxCol; c++) {
                html += '<th>' + this.getCellA1Col(c) + '</th>';
            }
            html += '</tr></thead><tbody>';
            // Data rows
            for (let r = 0; r <= s.maxRow; r++) {
                html += '<tr><th>' + (r + 1) + '</th>';
                for (let c = 0; c <= s.maxCol; c++) {
                    const cell = s.getCell(c, r);
                    if (!cell) {
                        html += '<td></td>';
                        continue;
                    }
                    let val = cell.formula
                        ? String(evaluateFormula(cell.formula, s))
                        : (cell.display || String(cell.value ?? ''));
                    if (cell.style && cell.style.numberFormat) {
                        val = formatCellValue(
                            cell.formula ? evaluateFormula(cell.formula, s) : cell.value,
                            cell.style.numberFormat
                        );
                    }
                    const isNum = cell.type === 'number' || (typeof cell.value === 'number');
                    let styleStr = '';
                    if (cell.style) {
                        if (cell.style.bold) styleStr += 'font-weight:bold;';
                        if (cell.style.italic) styleStr += 'font-style:italic;';
                        if (cell.style.underline) styleStr += 'text-decoration:underline;';
                        if (cell.style.color) styleStr += 'color:' + cell.style.color + ';';
                        if (cell.style.fill) styleStr += 'background:' + cell.style.fill + ';';
                        if (cell.style.bgColor) styleStr += 'background:' + cell.style.bgColor + ';';
                        if (cell.style.align) styleStr += 'text-align:' + cell.style.align + ';';
                    }
                    const cls = isNum && !cell.style?.align ? ' class="num"' : '';
                    html += '<td' + cls + (styleStr ? ' style="' + styleStr + '"' : '') + '>' + this._xmlEsc(String(val)) + '</td>';
                }
                html += '</tr>';
            }
            html += '</tbody></table>';
        }

        html += '</body></html>';

        const printWin = window.open('', '_blank', 'width=900,height=700');
        if (printWin) {
            printWin.document.write(html);
            printWin.document.close();
            printWin.focus();
            setTimeout(() => { printWin.print(); }, 300);
        }
    }

    // ─── Paste Special (S1.8) ────────────────────────
    pasteSpecial(mode) {
        if (!this._clipboard) return;
        const sheet = this._sheet();
        if (!sheet) return;
        const { col, row } = this.selectedCell;
        const { cells, width, height } = this._clipboard;

        if (mode === 'values') {
            for (let c = 0; c < width; c++) {
                for (let r = 0; r < height; r++) {
                    const src = cells[`${c},${r}`];
                    if (src) {
                        // Paste computed value, no formula
                        let val = src.value;
                        if (src.formula) {
                            val = evaluateFormula(src.formula, sheet);
                        }
                        this._setCellValue(col + c, row + r, String(val ?? ''));
                    } else {
                        this._setCellValue(col + c, row + r, '');
                    }
                }
            }
        } else if (mode === 'formulas') {
            // Paste formulas, adjusting references by the offset
            const colOffset = col - 0; // clipboard is 0-based relative
            const rowOffset = row - 0;
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
        } else if (mode === 'formatting') {
            // Paste only styles, not values
            for (let c = 0; c < width; c++) {
                for (let r = 0; r < height; r++) {
                    const src = cells[`${c},${r}`];
                    if (src && src.style) {
                        let cell = sheet.getCell(col + c, row + r);
                        if (!cell) {
                            cell = { value: '', formula: null, display: '', type: 'string', style: {} };
                            sheet.setCell(col + c, row + r, cell);
                        }
                        cell.style = { ...(src.style) };
                    }
                }
            }
        } else if (mode === 'transpose') {
            // Swap rows and columns
            for (let c = 0; c < width; c++) {
                for (let r = 0; r < height; r++) {
                    const src = cells[`${c},${r}`];
                    if (src) {
                        const rawVal = src.formula || String(src.value ?? '');
                        // Transposed: row becomes col, col becomes row
                        this._setCellValue(col + r, row + c, rawVal);
                    } else {
                        this._setCellValue(col + r, row + c, '');
                    }
                }
            }
        }
        this.render();
    }

    showPasteSpecialDialog() {
        const overlay = document.createElement('div');
        overlay.className = 'modal-overlay show';
        const modal = document.createElement('div');
        modal.className = 'modal';
        modal.style.minWidth = '320px';
        modal.innerHTML = '<h3>Paste Special</h3>'
            + '<div style="display:flex;flex-direction:column;gap:8px;padding:12px 0;">'
            + '<label style="display:flex;align-items:center;gap:8px;cursor:pointer;font-size:13px;"><input type="radio" name="pasteMode" value="values" checked> Values only</label>'
            + '<label style="display:flex;align-items:center;gap:8px;cursor:pointer;font-size:13px;"><input type="radio" name="pasteMode" value="formulas"> Formulas</label>'
            + '<label style="display:flex;align-items:center;gap:8px;cursor:pointer;font-size:13px;"><input type="radio" name="pasteMode" value="formatting"> Formatting only</label>'
            + '<label style="display:flex;align-items:center;gap:8px;cursor:pointer;font-size:13px;"><input type="radio" name="pasteMode" value="transpose"> Transpose</label>'
            + '</div>'
            + '<div class="modal-actions"><button class="ss-modal-cancel">Cancel</button><button class="ss-modal-ok primary">OK</button></div>';
        overlay.appendChild(modal);
        document.body.appendChild(overlay);
        const close = () => { document.body.removeChild(overlay); };
        modal.querySelector('.ss-modal-cancel').onclick = close;
        modal.querySelector('.ss-modal-ok').onclick = () => {
            const checked = modal.querySelector('input[name="pasteMode"]:checked');
            if (checked) this.pasteSpecial(checked.value);
            close();
        };
        overlay.onclick = (e) => { if (e.target === overlay) close(); };
    }

    // ─── Conditional Formatting (S2.1) ──────────────
    addConditionalRule(rule) {
        const sheet = this._sheet();
        if (!sheet) return;
        if (!sheet.conditionalRules) sheet.conditionalRules = [];
        sheet.conditionalRules.push(rule);
        this.render();
    }

    removeConditionalRule(index) {
        const sheet = this._sheet();
        if (!sheet || !sheet.conditionalRules) return;
        sheet.conditionalRules.splice(index, 1);
        this.render();
    }

    _evaluateConditionalRules(col, row, cell) {
        const sheet = this._sheet();
        if (!sheet || !sheet.conditionalRules || sheet.conditionalRules.length === 0) return null;

        const cellVal = cell.formula ? evaluateFormula(cell.formula, sheet) : cell.value;
        const numVal = Number(cellVal);

        for (const rule of sheet.conditionalRules) {
            // Check if cell is in the rule's range
            if (col < rule.range.startCol || col > rule.range.endCol ||
                row < rule.range.startRow || row > rule.range.endRow) continue;

            let match = false;
            switch (rule.type) {
                case 'greaterThan':
                    match = !isNaN(numVal) && numVal > Number(rule.condition);
                    break;
                case 'lessThan':
                    match = !isNaN(numVal) && numVal < Number(rule.condition);
                    break;
                case 'equalTo':
                    match = String(cellVal) === String(rule.condition) || (!isNaN(numVal) && numVal === Number(rule.condition));
                    break;
                case 'between':
                    if (rule.condition2 !== undefined) {
                        match = !isNaN(numVal) && numVal >= Number(rule.condition) && numVal <= Number(rule.condition2);
                    }
                    break;
                case 'textContains':
                    match = String(cellVal).toLowerCase().includes(String(rule.condition).toLowerCase());
                    break;
                case 'top10': {
                    // Collect all numeric values in the range
                    const values = [];
                    for (let r = rule.range.startRow; r <= rule.range.endRow; r++) {
                        for (let c = rule.range.startCol; c <= rule.range.endCol; c++) {
                            const cc = sheet.getCell(c, r);
                            if (cc) {
                                const v = cc.formula ? evaluateFormula(cc.formula, sheet) : cc.value;
                                const n = Number(v);
                                if (!isNaN(n)) values.push(n);
                            }
                        }
                    }
                    values.sort((a, b) => b - a);
                    const topN = values.slice(0, 10);
                    match = !isNaN(numVal) && topN.includes(numVal);
                    break;
                }
                case 'colorScale': {
                    // Color scale returns a style directly
                    const values = [];
                    for (let r = rule.range.startRow; r <= rule.range.endRow; r++) {
                        for (let c = rule.range.startCol; c <= rule.range.endCol; c++) {
                            const cc = sheet.getCell(c, r);
                            if (cc) {
                                const v = cc.formula ? evaluateFormula(cc.formula, sheet) : cc.value;
                                const n = Number(v);
                                if (!isNaN(n)) values.push(n);
                            }
                        }
                    }
                    if (!isNaN(numVal) && values.length > 0) {
                        const mn = Math.min(...values);
                        const mx = Math.max(...values);
                        const ratio = mx > mn ? (numVal - mn) / (mx - mn) : 0.5;
                        // Green (low) to Red (high)
                        const r2 = Math.round(255 * ratio);
                        const g2 = Math.round(255 * (1 - ratio));
                        return { fill: `rgb(${r2},${g2},100)` };
                    }
                    break;
                }
            }
            if (match) {
                return rule.style || {};
            }
        }
        return null;
    }

    showConditionalFormatDialog() {
        const range = this.getSelectionRange();
        const rangeStr = this.getCellA1(range.startCol, range.startRow) + ':' + this.getCellA1(range.endCol, range.endRow);

        const overlay = document.createElement('div');
        overlay.className = 'modal-overlay show';
        const modal = document.createElement('div');
        modal.className = 'modal';
        modal.style.minWidth = '400px';
        modal.innerHTML = '<h3>Conditional Formatting</h3>'
            + '<div style="display:flex;flex-direction:column;gap:10px;padding:12px 0;">'
            + '<div class="modal-field"><label style="font-size:12px;color:#5f6368;margin-bottom:4px;display:block;">Range</label>'
            + '<input type="text" class="ss-modal-input" id="cfRange" value="' + rangeStr + '" style="width:100%;padding:6px 8px;border:1px solid #dadce0;border-radius:4px;font-size:13px;"></div>'
            + '<div class="modal-field"><label style="font-size:12px;color:#5f6368;margin-bottom:4px;display:block;">Rule Type</label>'
            + '<select id="cfType" style="width:100%;padding:6px 8px;border:1px solid #dadce0;border-radius:4px;font-size:13px;">'
            + '<option value="greaterThan">Greater than</option>'
            + '<option value="lessThan">Less than</option>'
            + '<option value="equalTo">Equal to</option>'
            + '<option value="between">Between</option>'
            + '<option value="textContains">Text contains</option>'
            + '<option value="top10">Top 10</option>'
            + '<option value="colorScale">Color scale</option>'
            + '</select></div>'
            + '<div class="modal-field" id="cfValueRow"><label style="font-size:12px;color:#5f6368;margin-bottom:4px;display:block;">Value</label>'
            + '<input type="text" class="ss-modal-input" id="cfValue" placeholder="Enter value..." style="width:100%;padding:6px 8px;border:1px solid #dadce0;border-radius:4px;font-size:13px;"></div>'
            + '<div class="modal-field" id="cfValue2Row" style="display:none;"><label style="font-size:12px;color:#5f6368;margin-bottom:4px;display:block;">Value 2 (upper bound)</label>'
            + '<input type="text" class="ss-modal-input" id="cfValue2" placeholder="Upper bound..." style="width:100%;padding:6px 8px;border:1px solid #dadce0;border-radius:4px;font-size:13px;"></div>'
            + '<div class="modal-field"><label style="font-size:12px;color:#5f6368;margin-bottom:4px;display:block;">Highlight Color</label>'
            + '<input type="color" id="cfColor" value="#ffeb3b" style="width:60px;height:30px;border:1px solid #dadce0;border-radius:4px;cursor:pointer;"></div>'
            + '</div>'
            + '<div class="modal-actions"><button class="ss-modal-cancel">Cancel</button><button class="ss-modal-ok primary">OK</button></div>';
        overlay.appendChild(modal);
        document.body.appendChild(overlay);

        const typeSelect = modal.querySelector('#cfType');
        const valueRow = modal.querySelector('#cfValueRow');
        const value2Row = modal.querySelector('#cfValue2Row');
        typeSelect.addEventListener('change', () => {
            const t = typeSelect.value;
            valueRow.style.display = (t === 'top10' || t === 'colorScale') ? 'none' : '';
            value2Row.style.display = t === 'between' ? '' : 'none';
        });

        const close = () => { document.body.removeChild(overlay); };
        modal.querySelector('.ss-modal-cancel').onclick = close;
        modal.querySelector('.ss-modal-ok').onclick = () => {
            const rangeInput = modal.querySelector('#cfRange').value.trim();
            const type = typeSelect.value;
            const condVal = modal.querySelector('#cfValue').value;
            const condVal2 = modal.querySelector('#cfValue2').value;
            const color = modal.querySelector('#cfColor').value;

            // Parse range
            const parsed = this._parseA1Range(rangeInput);
            if (!parsed) { close(); return; }

            const rule = {
                range: parsed,
                type: type,
                condition: condVal,
                style: { fill: color }
            };
            if (type === 'between') rule.condition2 = condVal2;
            this.addConditionalRule(rule);
            close();
        };
        overlay.onclick = (e) => { if (e.target === overlay) close(); };
    }

    _parseA1Range(str) {
        const match = str.trim().match(/^([A-Z]{1,3})(\d{1,7}):([A-Z]{1,3})(\d{1,7})$/i);
        if (!match) return null;
        return {
            startCol: colLetterToIndex(match[1].toUpperCase()),
            startRow: parseInt(match[2], 10) - 1,
            endCol: colLetterToIndex(match[3].toUpperCase()),
            endRow: parseInt(match[4], 10) - 1
        };
    }

    // ─── Data Validation (S2.2) ─────────────────────
    setCellValidation(col, row, validation) {
        const sheet = this._sheet();
        if (!sheet) return;
        let cell = sheet.getCell(col, row);
        if (!cell) {
            cell = { value: '', formula: null, display: '', type: 'string', style: null };
            sheet.setCell(col, row, cell);
        }
        cell.validation = validation;
        this.render();
    }

    validateCell(col, row, value) {
        const sheet = this._sheet();
        if (!sheet) return { valid: true };
        const cell = sheet.getCell(col, row);
        if (!cell || !cell.validation) return { valid: true };

        const v = cell.validation;
        switch (v.type) {
            case 'list':
                if (v.values && !v.values.includes(String(value))) {
                    return { valid: false, message: v.message || 'Value must be one of: ' + v.values.join(', ') };
                }
                break;
            case 'number': {
                const num = Number(value);
                if (isNaN(num)) return { valid: false, message: v.message || 'Value must be a number' };
                if (v.min !== undefined && num < v.min) return { valid: false, message: v.message || 'Value must be >= ' + v.min };
                if (v.max !== undefined && num > v.max) return { valid: false, message: v.message || 'Value must be <= ' + v.max };
                break;
            }
            case 'textLength': {
                const len = String(value).length;
                if (v.min !== undefined && len < v.min) return { valid: false, message: v.message || 'Text must be at least ' + v.min + ' characters' };
                if (v.max !== undefined && len > v.max) return { valid: false, message: v.message || 'Text must be at most ' + v.max + ' characters' };
                break;
            }
            case 'custom':
                // Custom formula validation - basic support
                if (v.formula) {
                    try {
                        const result = evaluateFormula(v.formula, sheet);
                        if (result === false || result === 0 || result === '#ERROR!' || result === '#VALUE!') {
                            return { valid: false, message: v.message || 'Custom validation failed' };
                        }
                    } catch (_e) {
                        return { valid: false, message: v.message || 'Custom validation error' };
                    }
                }
                break;
        }
        return { valid: true };
    }

    _showValidationDropdown(col, row) {
        const sheet = this._sheet();
        if (!sheet) return;
        const cell = sheet.getCell(col, row);
        if (!cell || !cell.validation || cell.validation.type !== 'list') return;

        // Remove any existing dropdown
        this._closeValidationDropdown();

        const sx = this._cellScreenX(col);
        const sy = this._cellScreenY(row) + this.getRowHeight(row);

        const dd = document.createElement('div');
        dd.className = 'ss-validation-dropdown';
        dd.style.left = sx + 'px';
        dd.style.top = sy + 'px';
        dd.style.minWidth = this.getColumnWidth(col) + 'px';

        for (const val of cell.validation.values) {
            const opt = document.createElement('div');
            opt.className = 'ss-validation-option';
            opt.textContent = val;
            opt.addEventListener('click', () => {
                this._setCellValue(col, row, val);
                this._closeValidationDropdown();
                this._updateFormulaBar();
                this.render();
            });
            dd.appendChild(opt);
        }

        this.canvasWrap.appendChild(dd);
        this._validationDropdown = dd;

        // Close on outside click
        const closeHandler = (e) => {
            if (!dd.contains(e.target)) {
                this._closeValidationDropdown();
                document.removeEventListener('mousedown', closeHandler);
            }
        };
        setTimeout(() => document.addEventListener('mousedown', closeHandler), 0);
    }

    _closeValidationDropdown() {
        if (this._validationDropdown) {
            this._validationDropdown.remove();
            this._validationDropdown = null;
        }
    }

    showDataValidationDialog() {
        const { col, row } = this.selectedCell;
        const sheet = this._sheet();
        if (!sheet) return;

        const existing = sheet.getCell(col, row)?.validation;

        const overlay = document.createElement('div');
        overlay.className = 'modal-overlay show';
        const modal = document.createElement('div');
        modal.className = 'modal';
        modal.style.minWidth = '400px';
        modal.innerHTML = '<h3>Data Validation</h3>'
            + '<div style="display:flex;flex-direction:column;gap:10px;padding:12px 0;">'
            + '<div class="modal-field"><label style="font-size:12px;color:#5f6368;margin-bottom:4px;display:block;">Cell: ' + this.getCellA1(col, row) + '</label></div>'
            + '<div class="modal-field"><label style="font-size:12px;color:#5f6368;margin-bottom:4px;display:block;">Criteria</label>'
            + '<select id="dvType" style="width:100%;padding:6px 8px;border:1px solid #dadce0;border-radius:4px;font-size:13px;">'
            + '<option value="none">None</option>'
            + '<option value="list"' + (existing?.type === 'list' ? ' selected' : '') + '>List (dropdown)</option>'
            + '<option value="number"' + (existing?.type === 'number' ? ' selected' : '') + '>Number (min/max)</option>'
            + '<option value="textLength"' + (existing?.type === 'textLength' ? ' selected' : '') + '>Text length</option>'
            + '<option value="custom"' + (existing?.type === 'custom' ? ' selected' : '') + '>Custom formula</option>'
            + '</select></div>'
            + '<div class="modal-field" id="dvListRow" style="display:' + (existing?.type === 'list' ? '' : 'none') + ';"><label style="font-size:12px;color:#5f6368;margin-bottom:4px;display:block;">List values (comma-separated)</label>'
            + '<input type="text" id="dvListValues" value="' + (existing?.values?.join(',') || '') + '" style="width:100%;padding:6px 8px;border:1px solid #dadce0;border-radius:4px;font-size:13px;" placeholder="Option1,Option2,Option3"></div>'
            + '<div class="modal-field" id="dvMinRow" style="display:' + (existing?.type === 'number' || existing?.type === 'textLength' ? '' : 'none') + ';"><label style="font-size:12px;color:#5f6368;margin-bottom:4px;display:block;">Minimum</label>'
            + '<input type="number" id="dvMin" value="' + (existing?.min ?? '') + '" style="width:100%;padding:6px 8px;border:1px solid #dadce0;border-radius:4px;font-size:13px;"></div>'
            + '<div class="modal-field" id="dvMaxRow" style="display:' + (existing?.type === 'number' || existing?.type === 'textLength' ? '' : 'none') + ';"><label style="font-size:12px;color:#5f6368;margin-bottom:4px;display:block;">Maximum</label>'
            + '<input type="number" id="dvMax" value="' + (existing?.max ?? '') + '" style="width:100%;padding:6px 8px;border:1px solid #dadce0;border-radius:4px;font-size:13px;"></div>'
            + '<div class="modal-field" id="dvFormulaRow" style="display:' + (existing?.type === 'custom' ? '' : 'none') + ';"><label style="font-size:12px;color:#5f6368;margin-bottom:4px;display:block;">Formula</label>'
            + '<input type="text" id="dvFormula" value="' + (existing?.formula || '') + '" style="width:100%;padding:6px 8px;border:1px solid #dadce0;border-radius:4px;font-size:13px;" placeholder="=A1>0"></div>'
            + '<div class="modal-field"><label style="font-size:12px;color:#5f6368;margin-bottom:4px;display:block;">Error message</label>'
            + '<input type="text" id="dvMessage" value="' + (existing?.message || '') + '" style="width:100%;padding:6px 8px;border:1px solid #dadce0;border-radius:4px;font-size:13px;" placeholder="Invalid input"></div>'
            + '</div>'
            + '<div class="modal-actions"><button class="ss-modal-cancel">Cancel</button>'
            + (existing ? '<button class="ss-modal-remove" style="background:#d93025;color:#fff;border:none;padding:6px 16px;border-radius:4px;cursor:pointer;">Remove</button>' : '')
            + '<button class="ss-modal-ok primary">OK</button></div>';
        overlay.appendChild(modal);
        document.body.appendChild(overlay);

        const typeSelect = modal.querySelector('#dvType');
        typeSelect.addEventListener('change', () => {
            const t = typeSelect.value;
            modal.querySelector('#dvListRow').style.display = t === 'list' ? '' : 'none';
            modal.querySelector('#dvMinRow').style.display = (t === 'number' || t === 'textLength') ? '' : 'none';
            modal.querySelector('#dvMaxRow').style.display = (t === 'number' || t === 'textLength') ? '' : 'none';
            modal.querySelector('#dvFormulaRow').style.display = t === 'custom' ? '' : 'none';
        });

        const close = () => { document.body.removeChild(overlay); };
        modal.querySelector('.ss-modal-cancel').onclick = close;
        const removeBtn = modal.querySelector('.ss-modal-remove');
        if (removeBtn) {
            removeBtn.onclick = () => {
                const cell = sheet.getCell(col, row);
                if (cell) delete cell.validation;
                this.render();
                close();
            };
        }
        modal.querySelector('.ss-modal-ok').onclick = () => {
            const type = typeSelect.value;
            if (type === 'none') {
                const cell = sheet.getCell(col, row);
                if (cell) delete cell.validation;
                this.render();
                close();
                return;
            }
            const validation = { type, message: modal.querySelector('#dvMessage').value };
            if (type === 'list') {
                validation.values = modal.querySelector('#dvListValues').value.split(',').map(s => s.trim()).filter(Boolean);
            } else if (type === 'number' || type === 'textLength') {
                const minV = modal.querySelector('#dvMin').value;
                const maxV = modal.querySelector('#dvMax').value;
                if (minV !== '') validation.min = Number(minV);
                if (maxV !== '') validation.max = Number(maxV);
            } else if (type === 'custom') {
                validation.formula = modal.querySelector('#dvFormula').value;
            }
            // Apply validation to current selection range
            const range = this.getSelectionRange();
            for (let c = range.startCol; c <= range.endCol; c++) {
                for (let r = range.startRow; r <= range.endRow; r++) {
                    this.setCellValidation(c, r, { ...validation });
                }
            }
            close();
        };
        overlay.onclick = (e) => { if (e.target === overlay) close(); };
    }

    _showToast(message) {
        const toast = document.createElement('div');
        toast.className = 'ss-toast';
        toast.textContent = message;
        document.body.appendChild(toast);
        requestAnimationFrame(() => toast.classList.add('show'));
        setTimeout(() => {
            toast.classList.remove('show');
            setTimeout(() => toast.remove(), 300);
        }, 3000);
    }

    // ─── Cell Comments (S2.3) ───────────────────────
    setCellComment(col, row, text, author) {
        const sheet = this._sheet();
        if (!sheet) return;
        // Validate comment length
        if (text && text.length > 10000) {
            ssAlert('Comment too long (max 10,000 characters)');
            return;
        }
        let cell = sheet.getCell(col, row);
        if (!cell) {
            cell = { value: '', formula: null, display: '', type: 'string', style: null };
            sheet.setCell(col, row, cell);
        }
        if (text) {
            cell.comment = { text, author: author || 'User', timestamp: new Date().toISOString() };
        } else {
            delete cell.comment;
        }
        this.render();
    }

    _renderCommentIndicators(ctx, visibleCols, visibleRows) {
        const sheet = this._sheet();
        if (!sheet) return;

        for (const vc of visibleCols) {
            for (const vr of visibleRows) {
                const cell = sheet.getCell(vc.col, vr.row);
                if (cell && cell.comment) {
                    // Draw red triangle in top-right corner
                    ctx.fillStyle = this._getThemeColors().commentIndicator;
                    ctx.beginPath();
                    ctx.moveTo(vc.x + vc.width - 8, vr.y);
                    ctx.lineTo(vc.x + vc.width, vr.y);
                    ctx.lineTo(vc.x + vc.width, vr.y + 8);
                    ctx.closePath();
                    ctx.fill();
                }
            }
        }
    }

    _showCommentTooltip(col, row, canvasX, canvasY) {
        this._hideCommentTooltip();
        const sheet = this._sheet();
        if (!sheet) return;
        const cell = sheet.getCell(col, row);
        if (!cell || !cell.comment) return;

        const tooltip = document.createElement('div');
        tooltip.className = 'ss-comment-tooltip';
        tooltip.innerHTML = '<div class="ss-comment-author">' + this._xmlEsc(cell.comment.author)
            + ' <span class="ss-comment-time">' + new Date(cell.comment.timestamp).toLocaleString() + '</span></div>'
            + '<div class="ss-comment-text">' + this._xmlEsc(cell.comment.text) + '</div>';
        tooltip.style.left = (canvasX + 12) + 'px';
        tooltip.style.top = (canvasY + 12) + 'px';
        this.canvasWrap.appendChild(tooltip);
        this._commentTooltip = tooltip;
    }

    _hideCommentTooltip() {
        if (this._commentTooltip) {
            this._commentTooltip.remove();
            this._commentTooltip = null;
        }
    }

    showCommentsPanel() {
        if (this._commentsPanel) {
            this._commentsPanel.remove();
            this._commentsPanel = null;
            this.render();
            return;
        }

        const sheet = this._sheet();
        if (!sheet) return;

        const panel = document.createElement('div');
        panel.className = 'ss-comments-panel';

        const header = document.createElement('div');
        header.className = 'ss-comments-header';
        header.innerHTML = '<span style="font-weight:500;font-size:14px;">Comments</span>';
        const closeBtn = document.createElement('button');
        closeBtn.className = 'ss-find-btn';
        closeBtn.innerHTML = '<span class="msi">close</span>';
        closeBtn.title = 'Close comments panel';
        closeBtn.addEventListener('click', () => { panel.remove(); this._commentsPanel = null; });
        header.appendChild(closeBtn);
        panel.appendChild(header);

        const list = document.createElement('div');
        list.className = 'ss-comments-list';

        // Collect all comments
        const comments = [];
        for (const key of Object.keys(sheet.cells)) {
            const cell = sheet.cells[key];
            if (cell && cell.comment) {
                const [c, r] = key.split(',').map(Number);
                comments.push({ col: c, row: r, comment: cell.comment, ref: this.getCellA1(c, r) });
            }
        }

        if (comments.length === 0) {
            list.innerHTML = '<div style="padding:16px;color:#5f6368;font-size:13px;">No comments in this sheet.</div>';
        } else {
            for (const item of comments) {
                const entry = document.createElement('div');
                entry.className = 'ss-comment-entry';
                entry.innerHTML = '<div class="ss-comment-ref">' + item.ref + '</div>'
                    + '<div class="ss-comment-author">' + this._xmlEsc(item.comment.author)
                    + ' <span class="ss-comment-time">' + new Date(item.comment.timestamp).toLocaleString() + '</span></div>'
                    + '<div class="ss-comment-text">' + this._xmlEsc(item.comment.text) + '</div>';
                entry.addEventListener('click', () => {
                    this.selectedCell = { col: item.col, row: item.row };
                    this.selectionRange = null;
                    this._ensureVisible(item.col, item.row);
                    this._updateFormulaBar();
                    this.render();
                });
                list.appendChild(entry);
            }
        }

        panel.appendChild(list);
        this.canvasWrap.appendChild(panel);
        this._commentsPanel = panel;
    }

    // ─── Cell Properties Panel ────────────────────────
    showPropertiesPanel() {
        if (this._propsPanel) {
            this._propsPanel.remove();
            this._propsPanel = null;
            return;
        }

        const panel = document.createElement('div');
        panel.className = 'ss-comments-panel'; // reuse styling
        panel.style.width = '280px';

        const header = document.createElement('div');
        header.className = 'ss-comments-header';
        header.innerHTML = '<span style="font-weight:500;font-size:14px;">Cell Properties</span>';
        const closeBtn = document.createElement('button');
        closeBtn.className = 'ss-find-btn';
        closeBtn.innerHTML = '<span class="msi">close</span>';
        closeBtn.title = 'Close properties panel';
        closeBtn.addEventListener('click', () => { panel.remove(); this._propsPanel = null; });
        header.appendChild(closeBtn);
        panel.appendChild(header);

        const body = document.createElement('div');
        body.style.cssText = 'padding:12px;font-size:13px;font-family:Arial,sans-serif;overflow-y:auto;flex:1;';
        this._propsPanelBody = body;
        panel.appendChild(body);

        this.canvasWrap.appendChild(panel);
        this._propsPanel = panel;
        this._updatePropsPanel();
    }

    _updatePropsPanel() {
        if (!this._propsPanel || !this._propsPanelBody) return;
        const sheet = this._sheet();
        const { col, row } = this.selectedCell;
        const cell = sheet ? sheet.getCell(col, row) : null;
        const ref = this.getCellA1(col, row);
        const style = cell?.style || {};
        const val = cell ? (cell.formula || String(cell.value ?? '')) : '';
        const display = this._getCellDisplay(col, row);

        let html = '<div style="margin-bottom:12px;">';
        html += '<div style="font-weight:600;color:#1a73e8;margin-bottom:4px;">' + ref + '</div>';
        html += '<div style="color:#5f6368;margin-bottom:8px;">Value: ' + this._xmlEsc(display || '(empty)') + '</div>';
        if (cell?.formula) {
            html += '<div style="color:#5f6368;margin-bottom:8px;">Formula: ' + this._xmlEsc(cell.formula) + '</div>';
        }
        html += '</div>';

        // Format section
        html += '<div style="font-weight:600;margin-bottom:6px;border-top:1px solid #e0e0e0;padding-top:8px;">Format</div>';
        const propRows = [
            ['Bold', style.bold ? 'Yes' : 'No'],
            ['Italic', style.italic ? 'Yes' : 'No'],
            ['Underline', style.underline ? 'Yes' : 'No'],
            ['Font', style.fontFamily || 'Arial'],
            ['Size', (style.fontSize || 13) + 'px'],
            ['Color', style.color || '#202124'],
            ['Fill', style.fill || style.bgColor || 'None'],
            ['Alignment', style.align || 'Auto'],
            ['Number Format', style.numberFormat || 'General'],
        ];
        html += '<table style="width:100%;font-size:12px;border-collapse:collapse;">';
        for (const [label, value] of propRows) {
            const isColor = label === 'Color' || label === 'Fill';
            const swatch = isColor && value !== 'None' && value !== 'Auto'
                ? '<span style="display:inline-block;width:12px;height:12px;border:1px solid #ccc;border-radius:2px;background:' + value + ';vertical-align:middle;margin-right:4px;"></span>'
                : '';
            html += '<tr><td style="padding:3px 0;color:#5f6368;">' + label + '</td><td style="padding:3px 0;text-align:right;">' + swatch + this._xmlEsc(String(value)) + '</td></tr>';
        }
        html += '</table>';

        // Validation section
        if (cell?.validation) {
            html += '<div style="font-weight:600;margin:10px 0 6px;border-top:1px solid #e0e0e0;padding-top:8px;">Validation</div>';
            html += '<div style="font-size:12px;color:#5f6368;">Type: ' + cell.validation.type + '</div>';
            if (cell.validation.values) {
                html += '<div style="font-size:12px;color:#5f6368;">Values: ' + this._xmlEsc(cell.validation.values.join(', ')) + '</div>';
            }
        }

        // Comment section
        if (cell?.comment) {
            html += '<div style="font-weight:600;margin:10px 0 6px;border-top:1px solid #e0e0e0;padding-top:8px;">Comment</div>';
            html += '<div style="font-size:12px;color:#5f6368;">' + this._xmlEsc(cell.comment.author) + ': ' + this._xmlEsc(cell.comment.text) + '</div>';
        }

        // Merge info
        if (sheet?.merges) {
            for (const m of sheet.merges) {
                if (col >= m.startCol && col <= m.endCol && row >= m.startRow && row <= m.endRow) {
                    html += '<div style="font-weight:600;margin:10px 0 6px;border-top:1px solid #e0e0e0;padding-top:8px;">Merge</div>';
                    html += '<div style="font-size:12px;color:#5f6368;">'
                        + this.getCellA1(m.startCol, m.startRow) + ':' + this.getCellA1(m.endCol, m.endRow) + '</div>';
                    break;
                }
            }
        }

        this._propsPanelBody.innerHTML = html;
    }

    // ─── Multi-level Sort (S2.4) ────────────────────
    sortMulti(criteria, hasHeader = true) {
        const sheet = this._sheet();
        if (!sheet) return;

        // Save pre-sort state
        const previousCells = JSON.parse(JSON.stringify(sheet.cells));
        const startRow = hasHeader ? 1 : 0;
        const rows = [];

        for (let r = startRow; r <= sheet.maxRow; r++) {
            const rowData = {};
            for (let c = 0; c <= sheet.maxCol; c++) {
                const cell = sheet.getCell(c, r);
                if (cell) rowData[c] = { ...cell };
            }
            rows.push({ index: r, data: rowData });
        }

        rows.sort((a, b) => {
            for (const crit of criteria) {
                const cellA = a.data[crit.col];
                const cellB = b.data[crit.col];
                let va = cellA ? (cellA.value ?? '') : '';
                let vb = cellB ? (cellB.value ?? '') : '';

                let cmp = 0;
                if (typeof va === 'number' && typeof vb === 'number') {
                    cmp = va - vb;
                } else {
                    va = String(va).toLowerCase();
                    vb = String(vb).toLowerCase();
                    if (va < vb) cmp = -1;
                    else if (va > vb) cmp = 1;
                }
                if (!crit.ascending) cmp = -cmp;
                if (cmp !== 0) return cmp;
            }
            return 0;
        });

        // Write back sorted rows
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

        this._undoManager.push({
            type: 'sort',
            sheetIndex: this.activeSheet,
            previousCells: previousCells
        });
        this.render();
    }

    showSortDialog() {
        const sheet = this._sheet();
        if (!sheet) return;

        const overlay = document.createElement('div');
        overlay.className = 'modal-overlay show';
        const modal = document.createElement('div');
        modal.className = 'modal';
        modal.style.minWidth = '450px';

        const criteria = [{ col: this.selectedCell.col, ascending: true }];

        const renderCriteria = () => {
            const container = modal.querySelector('#sortCriteria');
            container.innerHTML = '';
            criteria.forEach((crit, idx) => {
                const row = document.createElement('div');
                row.style.cssText = 'display:flex;align-items:center;gap:8px;margin-bottom:8px;';

                const label = document.createElement('span');
                label.style.cssText = 'font-size:12px;color:#5f6368;min-width:60px;';
                label.textContent = idx === 0 ? 'Sort by' : 'Then by';

                const colSelect = document.createElement('select');
                colSelect.style.cssText = 'flex:1;padding:6px 8px;border:1px solid #dadce0;border-radius:4px;font-size:13px;';
                for (let c = 0; c <= Math.max(sheet.maxCol, 25); c++) {
                    const opt = document.createElement('option');
                    opt.value = c;
                    // Use header row value if available, otherwise column letter
                    const headerCell = sheet.getCell(c, 0);
                    opt.textContent = this.getCellA1Col(c) + (headerCell ? ' (' + String(headerCell.value ?? '').substring(0, 20) + ')' : '');
                    if (c === crit.col) opt.selected = true;
                    colSelect.appendChild(opt);
                }
                colSelect.addEventListener('change', () => { crit.col = parseInt(colSelect.value, 10); });

                const orderSelect = document.createElement('select');
                orderSelect.style.cssText = 'padding:6px 8px;border:1px solid #dadce0;border-radius:4px;font-size:13px;min-width:90px;';
                orderSelect.innerHTML = '<option value="asc"' + (crit.ascending ? ' selected' : '') + '>A to Z</option><option value="desc"' + (!crit.ascending ? ' selected' : '') + '>Z to A</option>';
                orderSelect.addEventListener('change', () => { crit.ascending = orderSelect.value === 'asc'; });

                const removeBtn = document.createElement('button');
                removeBtn.style.cssText = 'border:none;background:transparent;cursor:pointer;color:#d93025;font-size:18px;padding:2px 4px;border-radius:4px;';
                removeBtn.innerHTML = '<span class="msi">delete</span>';
                removeBtn.title = 'Remove this sort level';
                removeBtn.addEventListener('click', () => {
                    if (criteria.length > 1) {
                        criteria.splice(idx, 1);
                        renderCriteria();
                    }
                });

                row.appendChild(label);
                row.appendChild(colSelect);
                row.appendChild(orderSelect);
                if (criteria.length > 1) row.appendChild(removeBtn);
                container.appendChild(row);
            });
        };

        modal.innerHTML = '<h3>Sort</h3>'
            + '<div id="sortCriteria" style="padding:12px 0;"></div>'
            + '<div style="margin-bottom:12px;"><button id="sortAddLevel" style="border:1px solid #dadce0;background:#fff;padding:4px 12px;border-radius:4px;font-size:12px;cursor:pointer;">+ Add level</button></div>'
            + '<div style="margin-bottom:12px;"><label style="display:flex;align-items:center;gap:6px;font-size:13px;"><input type="checkbox" id="sortHasHeader" checked> My data has headers</label></div>'
            + '<div class="modal-actions"><button class="ss-modal-cancel">Cancel</button><button class="ss-modal-ok primary">Sort</button></div>';
        overlay.appendChild(modal);
        document.body.appendChild(overlay);

        renderCriteria();

        modal.querySelector('#sortAddLevel').addEventListener('click', () => {
            criteria.push({ col: 0, ascending: true });
            renderCriteria();
        });

        const close = () => { document.body.removeChild(overlay); };
        modal.querySelector('.ss-modal-cancel').onclick = close;
        modal.querySelector('.ss-modal-ok').onclick = () => {
            const hasHeader = modal.querySelector('#sortHasHeader').checked;
            this.sortMulti(criteria, hasHeader);
            close();
        };
        overlay.onclick = (e) => { if (e.target === overlay) close(); };
    }

    // ─── Remove Duplicates (S2.6) ───────────────────
    showRemoveDuplicatesDialog() {
        const sheet = this._sheet();
        if (!sheet) return;

        const overlay = document.createElement('div');
        overlay.className = 'modal-overlay show';
        const modal = document.createElement('div');
        modal.className = 'modal';
        modal.style.minWidth = '400px';

        let colCheckboxes = '';
        for (let c = 0; c <= sheet.maxCol; c++) {
            const headerCell = sheet.getCell(c, 0);
            const label = this.getCellA1Col(c) + (headerCell ? ' (' + String(headerCell.value ?? '').substring(0, 20) + ')' : '');
            colCheckboxes += '<label style="display:flex;align-items:center;gap:8px;font-size:13px;padding:4px 0;cursor:pointer;">'
                + '<input type="checkbox" class="rd-col-check" data-col="' + c + '" checked> ' + this._xmlEsc(label) + '</label>';
        }

        modal.innerHTML = '<h3>Remove Duplicates</h3>'
            + '<div style="padding:8px 0;font-size:13px;color:#5f6368;">Select columns to compare for duplicates:</div>'
            + '<div style="max-height:200px;overflow-y:auto;padding:4px 0;">' + colCheckboxes + '</div>'
            + '<div id="rdPreview" style="padding:8px 0;font-size:13px;color:#5f6368;"></div>'
            + '<div class="modal-actions"><button class="ss-modal-cancel">Cancel</button><button class="ss-modal-ok primary" id="rdRemoveBtn">Remove</button></div>';
        overlay.appendChild(modal);
        document.body.appendChild(overlay);

        const updatePreview = () => {
            const checks = modal.querySelectorAll('.rd-col-check:checked');
            const cols = Array.from(checks).map(cb => parseInt(cb.dataset.col, 10));
            const count = this._countDuplicates(cols);
            modal.querySelector('#rdPreview').textContent = count + ' duplicate row(s) found.';
        };
        updatePreview();

        modal.querySelectorAll('.rd-col-check').forEach(cb => cb.addEventListener('change', updatePreview));

        const close = () => { document.body.removeChild(overlay); };
        modal.querySelector('.ss-modal-cancel').onclick = close;
        modal.querySelector('#rdRemoveBtn').onclick = () => {
            const checks = modal.querySelectorAll('.rd-col-check:checked');
            const cols = Array.from(checks).map(cb => parseInt(cb.dataset.col, 10));
            const removed = this._removeDuplicates(cols);
            this._showToast(removed + ' duplicate(s) removed.');
            close();
        };
        overlay.onclick = (e) => { if (e.target === overlay) close(); };
    }

    _countDuplicates(cols) {
        const sheet = this._sheet();
        if (!sheet) return 0;
        const seen = new Set();
        let count = 0;
        for (let r = 1; r <= sheet.maxRow; r++) {
            const key = cols.map(c => {
                const cell = sheet.getCell(c, r);
                return cell ? String(cell.value ?? '') : '';
            }).join('\x00');
            if (seen.has(key)) count++;
            else seen.add(key);
        }
        return count;
    }

    _removeDuplicates(cols) {
        const sheet = this._sheet();
        if (!sheet) return 0;

        // Save state for undo
        const previousCells = JSON.parse(JSON.stringify(sheet.cells));

        const seen = new Set();
        const keepRows = [0]; // Always keep header
        for (let r = 1; r <= sheet.maxRow; r++) {
            const key = cols.map(c => {
                const cell = sheet.getCell(c, r);
                return cell ? String(cell.value ?? '') : '';
            }).join('\x00');
            if (!seen.has(key)) {
                seen.add(key);
                keepRows.push(r);
            }
        }

        const removed = (sheet.maxRow + 1) - keepRows.length;
        if (removed === 0) return 0;

        // Rebuild the sheet with only kept rows
        const newCells = {};
        let newMaxRow = 0;
        for (let i = 0; i < keepRows.length; i++) {
            const srcRow = keepRows[i];
            for (let c = 0; c <= sheet.maxCol; c++) {
                const cell = sheet.getCell(c, srcRow);
                if (cell) {
                    newCells[c + ',' + i] = { ...cell };
                    if (i > newMaxRow) newMaxRow = i;
                }
            }
        }

        sheet.cells = newCells;
        sheet.maxRow = newMaxRow;

        this._undoManager.push({
            type: 'sort',
            sheetIndex: this.activeSheet,
            previousCells: previousCells
        });

        this.render();
        return removed;
    }

    // ─── Sheet Management (S2.8) ────────────────────
    duplicateSheet(index) {
        if (!this.workbook || !this.workbook.sheets[index]) return;
        const original = this.workbook.sheets[index];
        const copy = new Sheet(original.name + ' (copy)');
        copy.cells = JSON.parse(JSON.stringify(original.cells));
        copy.colWidths = { ...original.colWidths };
        copy.rowHeights = { ...original.rowHeights };
        copy.merges = JSON.parse(JSON.stringify(original.merges));
        copy.maxCol = original.maxCol;
        copy.maxRow = original.maxRow;
        if (original.conditionalRules) {
            copy.conditionalRules = JSON.parse(JSON.stringify(original.conditionalRules));
        }
        if (original.namedRanges) {
            copy.namedRanges = { ...original.namedRanges };
        }
        if (original.images) {
            copy.images = JSON.parse(JSON.stringify(original.images));
            // Reset IDs for copied images
            for (const img of copy.images) img.id = Date.now() + Math.random();
        }
        this.workbook.sheets.splice(index + 1, 0, copy);
        this.activeSheet = index + 1;
        this.updateSheetTabs();
        this.render();
    }

    moveSheet(fromIndex, toIndex) {
        if (!this.workbook) return;
        if (fromIndex < 0 || fromIndex >= this.workbook.sheets.length) return;
        if (toIndex < 0 || toIndex >= this.workbook.sheets.length) return;
        const sheet = this.workbook.sheets.splice(fromIndex, 1)[0];
        this.workbook.sheets.splice(toIndex, 0, sheet);
        this.activeSheet = toIndex;
        this.updateSheetTabs();
        this.render();
    }

    hideSheet(index) {
        if (!this.workbook || this.workbook.sheets.length <= 1) return;
        if (!this._hiddenSheets) this._hiddenSheets = new Set();
        this._hiddenSheets.add(index);
        if (this.activeSheet === index) {
            // Find next visible sheet
            for (let i = 0; i < this.workbook.sheets.length; i++) {
                if (!this._hiddenSheets.has(i)) {
                    this.activeSheet = i;
                    break;
                }
            }
        }
        this.updateSheetTabs();
        this.render();
    }

    unhideSheet(index) {
        if (this._hiddenSheets) this._hiddenSheets.delete(index);
        this.updateSheetTabs();
        this.render();
    }

    setSheetTabColor(index, color) {
        if (!this.workbook || !this.workbook.sheets[index]) return;
        this.workbook.sheets[index].tabColor = color;
        this.updateSheetTabs();
    }

    // ─── Chart Integration (S3) ─────────────────────

    /** Get the current selection range (normalized). */
    getSelectionRange() {
        if (this.selectionRange) {
            return this._normalizeRange(this.selectionRange);
        }
        return {
            startCol: this.selectedCell.col,
            startRow: this.selectedCell.row,
            endCol: this.selectedCell.col,
            endRow: this.selectedCell.row
        };
    }

    /**
     * Insert a chart for the current selection.
     * @param {string} chartType - 'column', 'bar', 'line', 'area', 'pie', 'doughnut'
     */
    async insertChart(chartType) {
        const sheet = this._sheet();
        if (!sheet) return;

        const { parseChartData, createChartElement } = await import('./spreadsheet-charts.js');
        const range = this.getSelectionRange();
        const data = parseChartData(sheet, range);
        if (!data) return;

        // Position the chart near the selection
        const sx = this._cellScreenX(range.endCol + 1);
        const sy = this._cellScreenY(range.startRow);

        const chartObj = createChartElement(this.canvasWrap, chartType, data, {
            title: chartType.charAt(0).toUpperCase() + chartType.slice(1) + ' Chart',
        }, {
            x: Math.max(60, sx + 20),
            y: Math.max(40, sy),
            width: 480,
            height: 320,
            range: { ...range },
            sheetIndex: this.activeSheet,
        });

        // Remove handler
        chartObj.onRemove = (c) => {
            const idx = this._charts.indexOf(c);
            if (idx !== -1) this._charts.splice(idx, 1);
            const sIdx = sheet.charts.indexOf(c);
            if (sIdx !== -1) sheet.charts.splice(sIdx, 1);
        };

        this._charts.push(chartObj);
        sheet.charts.push(chartObj);
    }

    /** Re-render all charts whose data range overlaps the changed cells (debounced). */
    _refreshCharts() {
        if (this._chartRefreshTimer) clearTimeout(this._chartRefreshTimer);
        this._chartRefreshTimer = setTimeout(() => {
            this._doRefreshCharts();
        }, 100);
    }

    _doRefreshCharts() {
        const sheet = this._sheet();
        if (!sheet || sheet.charts.length === 0) return;

        import('./spreadsheet-charts.js').then(({ parseChartData }) => {
            for (const chartObj of sheet.charts) {
                if (!chartObj.range) continue;
                const newData = parseChartData(sheet, chartObj.range);
                if (newData) {
                    chartObj.data = newData;
                    chartObj.renderer.render(chartObj.type, newData, chartObj.options);
                }
            }
        });
    }

    /** Remove all chart DOM elements (used when switching sheets or destroying). */
    _removeAllChartElements() {
        for (const c of this._charts) {
            if (c.renderer) c.renderer.destroy();
            if (c.container && c.container.parentNode) c.container.remove();
        }
        this._charts = [];
    }

    /** Show chart DOM elements for the active sheet. */
    _showActiveSheetCharts() {
        this._removeAllChartElements();
        const sheet = this._sheet();
        if (!sheet) return;
        for (const chartObj of sheet.charts) {
            if (chartObj.container && !chartObj.container.parentNode) {
                this.canvasWrap.appendChild(chartObj.container);
            }
            this._charts.push(chartObj);
        }
    }

    // ─── Named Ranges (S2.7) ────────────────────────
    showNamedRangeDialog() {
        const sheet = this._sheet();
        if (!sheet) return;

        const overlay = document.createElement('div');
        overlay.className = 'modal-overlay show';
        const modal = document.createElement('div');
        modal.className = 'modal';
        modal.style.minWidth = '480px';

        const renderList = () => {
            const listEl = modal.querySelector('#nrList');
            listEl.innerHTML = '';
            const entries = Object.entries(sheet.namedRanges || {});
            if (entries.length === 0) {
                listEl.innerHTML = '<div style="padding:8px;color:#5f6368;font-size:13px;">No named ranges defined.</div>';
            } else {
                for (const [name, range] of entries) {
                    const row = document.createElement('div');
                    row.style.cssText = 'display:flex;align-items:center;justify-content:space-between;padding:6px 8px;border-bottom:1px solid #e8eaed;font-size:13px;';
                    row.innerHTML = '<span><strong>' + this._xmlEsc(name) + '</strong> = ' + this._xmlEsc(range) + '</span>';
                    const delBtn = document.createElement('button');
                    delBtn.style.cssText = 'border:none;background:transparent;cursor:pointer;color:#d93025;font-size:13px;padding:2px 8px;';
                    delBtn.textContent = 'Delete';
                    delBtn.title = 'Delete named range "' + name + '"';
                    delBtn.addEventListener('click', () => {
                        delete sheet.namedRanges[name];
                        renderList();
                    });
                    row.appendChild(delBtn);
                    listEl.appendChild(row);
                }
            }
        };

        const selRange = this.getSelectionRange();
        const defaultRange = this.getCellA1(selRange.startCol, selRange.startRow) + ':' + this.getCellA1(selRange.endCol, selRange.endRow);

        modal.innerHTML = '<h3>Named Ranges</h3>'
            + '<div id="nrList" style="max-height:160px;overflow-y:auto;border:1px solid #dadce0;border-radius:4px;margin-bottom:12px;"></div>'
            + '<div style="display:flex;flex-direction:column;gap:8px;">'
            + '<div style="font-size:12px;color:#5f6368;font-weight:500;">Add New Named Range</div>'
            + '<div style="display:flex;gap:8px;">'
            + '<input type="text" id="nrName" placeholder="Name (e.g. TotalSales)" style="flex:1;padding:6px 8px;border:1px solid #dadce0;border-radius:4px;font-size:13px;">'
            + '<input type="text" id="nrRange" placeholder="Range (e.g. A1:A100)" value="' + defaultRange + '" style="flex:1;padding:6px 8px;border:1px solid #dadce0;border-radius:4px;font-size:13px;">'
            + '<button id="nrAddBtn" style="padding:6px 16px;background:#1a73e8;color:#fff;border:none;border-radius:4px;cursor:pointer;font-size:13px;">Add</button>'
            + '</div></div>'
            + '<div class="modal-actions" style="margin-top:12px;"><button class="ss-modal-ok primary">Close</button></div>';
        overlay.appendChild(modal);
        document.body.appendChild(overlay);

        renderList();

        modal.querySelector('#nrAddBtn').addEventListener('click', () => {
            const name = modal.querySelector('#nrName').value.trim();
            const range = modal.querySelector('#nrRange').value.trim();
            if (!name || !range) return;
            if (!/^[A-Za-z_]\w*$/.test(name)) {
                this._showToast('Invalid name. Use letters, numbers, and underscores. Start with a letter.');
                return;
            }
            if (!sheet.namedRanges) sheet.namedRanges = {};
            sheet.namedRanges[name] = range;
            modal.querySelector('#nrName').value = '';
            renderList();
        });

        const close = () => { document.body.removeChild(overlay); };
        modal.querySelector('.ss-modal-ok').onclick = close;
        overlay.onclick = (e) => { if (e.target === overlay) close(); };
    }

    /** Show named range selector dropdown when clicking cell ref label. */
    _setupNameBoxDropdown() {
        if (!this.cellRefLabel) return;
        this.cellRefLabel.style.cursor = 'pointer';
        this.cellRefLabel.title = 'Click to select named range or type cell reference';
        this.cellRefLabel.addEventListener('click', () => {
            const sheet = this._sheet();
            if (!sheet || !sheet.namedRanges || Object.keys(sheet.namedRanges).length === 0) return;

            // Remove any existing dropdown
            if (this._nameBoxDropdown) {
                this._nameBoxDropdown.remove();
                this._nameBoxDropdown = null;
                return;
            }

            const dd = document.createElement('div');
            dd.className = 'ss-validation-dropdown';
            dd.style.left = '0px';
            dd.style.top = '100%';
            dd.style.minWidth = '150px';

            for (const [name, range] of Object.entries(sheet.namedRanges)) {
                const opt = document.createElement('div');
                opt.className = 'ss-validation-option';
                opt.innerHTML = '<strong>' + this._xmlEsc(name) + '</strong> <span style="color:#5f6368;font-size:11px;">' + this._xmlEsc(range) + '</span>';
                opt.addEventListener('click', () => {
                    // Select the named range
                    const parsed = this._parseA1Range(range);
                    if (parsed) {
                        this.selectRange(parsed.startCol, parsed.startRow, parsed.endCol, parsed.endRow);
                        this._ensureVisible(parsed.startCol, parsed.startRow);
                    }
                    dd.remove();
                    this._nameBoxDropdown = null;
                    this.render();
                });
                dd.appendChild(opt);
            }

            this.cellRefLabel.parentElement.style.position = 'relative';
            this.cellRefLabel.parentElement.appendChild(dd);
            this._nameBoxDropdown = dd;

            const closeHandler = (e) => {
                if (!dd.contains(e.target) && e.target !== this.cellRefLabel) {
                    dd.remove();
                    this._nameBoxDropdown = null;
                    document.removeEventListener('mousedown', closeHandler);
                }
            };
            setTimeout(() => document.addEventListener('mousedown', closeHandler), 0);
        });
    }

    // ─── Insert Images (S3.5) ────────────────────────
    insertImage() {
        const input = document.createElement('input');
        input.type = 'file';
        input.accept = 'image/*';
        input.style.display = 'none';
        input.addEventListener('change', () => {
            const file = input.files[0];
            if (!file) return;
            const reader = new FileReader();
            reader.onload = (e) => {
                const data = e.target.result;
                this._addImageOverlay(data);
            };
            reader.readAsDataURL(file);
            input.remove();
        });
        document.body.appendChild(input);
        input.click();
    }

    _addImageOverlay(dataUrl) {
        const sheet = this._sheet();
        if (!sheet) return;

        const id = Date.now();
        const imgMeta = { data: dataUrl, x: 100, y: 100, width: 200, height: 150, id };
        if (!sheet.images) sheet.images = [];
        sheet.images.push(imgMeta);

        this._createImageElement(imgMeta);
    }

    _createImageElement(imgMeta) {
        const container = document.createElement('div');
        container.className = 'ss-image-overlay';
        container.style.cssText = 'position:absolute;left:' + imgMeta.x + 'px;top:' + imgMeta.y + 'px;width:' + imgMeta.width + 'px;height:' + imgMeta.height + 'px;cursor:move;border:2px solid transparent;z-index:50;';
        container.dataset.imgId = imgMeta.id;

        const img = document.createElement('img');
        img.src = imgMeta.data;
        img.style.cssText = 'width:100%;height:100%;object-fit:contain;pointer-events:none;';
        container.appendChild(img);

        // Resize handle
        const resizeHandle = document.createElement('div');
        resizeHandle.style.cssText = 'position:absolute;right:-4px;bottom:-4px;width:10px;height:10px;background:#1a73e8;cursor:se-resize;border-radius:2px;';
        container.appendChild(resizeHandle);

        // Remove button
        const removeBtn = document.createElement('button');
        removeBtn.style.cssText = 'position:absolute;top:-8px;right:-8px;width:20px;height:20px;border-radius:50%;background:#d93025;color:#fff;border:none;cursor:pointer;font-size:12px;line-height:20px;text-align:center;display:none;z-index:51;';
        removeBtn.textContent = 'x';
        removeBtn.title = 'Remove image';
        container.appendChild(removeBtn);

        container.addEventListener('mouseenter', () => {
            container.style.borderColor = '#1a73e8';
            removeBtn.style.display = '';
        });
        container.addEventListener('mouseleave', () => {
            container.style.borderColor = 'transparent';
            removeBtn.style.display = 'none';
        });

        // Drag to move
        let dragState = null;
        container.addEventListener('mousedown', (e) => {
            if (e.target === resizeHandle) return;
            e.preventDefault();
            dragState = { startX: e.clientX, startY: e.clientY, origX: imgMeta.x, origY: imgMeta.y };
            const onMove = (me) => {
                if (!dragState) return;
                imgMeta.x = Math.max(0, dragState.origX + (me.clientX - dragState.startX));
                imgMeta.y = Math.max(0, dragState.origY + (me.clientY - dragState.startY));
                container.style.left = imgMeta.x + 'px';
                container.style.top = imgMeta.y + 'px';
            };
            const onUp = () => {
                dragState = null;
                document.removeEventListener('mousemove', onMove);
                document.removeEventListener('mouseup', onUp);
            };
            document.addEventListener('mousemove', onMove);
            document.addEventListener('mouseup', onUp);
        });

        // Resize handle
        let resizeState = null;
        resizeHandle.addEventListener('mousedown', (e) => {
            e.preventDefault();
            e.stopPropagation();
            resizeState = { startX: e.clientX, startY: e.clientY, origW: imgMeta.width, origH: imgMeta.height };
            const onMove = (me) => {
                if (!resizeState) return;
                imgMeta.width = Math.max(40, resizeState.origW + (me.clientX - resizeState.startX));
                imgMeta.height = Math.max(30, resizeState.origH + (me.clientY - resizeState.startY));
                container.style.width = imgMeta.width + 'px';
                container.style.height = imgMeta.height + 'px';
            };
            const onUp = () => {
                resizeState = null;
                document.removeEventListener('mousemove', onMove);
                document.removeEventListener('mouseup', onUp);
            };
            document.addEventListener('mousemove', onMove);
            document.addEventListener('mouseup', onUp);
        });

        // Remove
        removeBtn.addEventListener('click', (e) => {
            e.stopPropagation();
            const sheet = this._sheet();
            if (sheet && sheet.images) {
                const idx = sheet.images.findIndex(im => im.id === imgMeta.id);
                if (idx >= 0) sheet.images.splice(idx, 1);
            }
            container.remove();
        });

        this.canvasWrap.appendChild(container);
        imgMeta._el = container;
    }

    /** Recreate image DOM elements for the active sheet. */
    _showActiveSheetImages() {
        // Remove old image elements
        this.canvasWrap.querySelectorAll('.ss-image-overlay').forEach(el => el.remove());
        const sheet = this._sheet();
        if (!sheet || !sheet.images) return;
        for (const imgMeta of sheet.images) {
            this._createImageElement(imgMeta);
        }
    }

    // ─── Shapes & Text Boxes (S5.6) ─────────────────────
    showInsertShapeDialog() {
        const overlay = document.createElement('div');
        overlay.className = 'modal-overlay show';
        const modal = document.createElement('div');
        modal.className = 'modal';
        modal.style.maxWidth = '380px';

        const h3 = document.createElement('h3');
        h3.innerHTML = '<span class="msi modal-icon">shapes</span> Insert Shape';
        modal.appendChild(h3);

        const desc = document.createElement('p');
        desc.style.cssText = 'font-size:13px;color:#5f6368;margin:0 0 8px';
        desc.textContent = 'Select a shape type to insert:';
        modal.appendChild(desc);

        const grid = document.createElement('div');
        grid.className = 'shape-type-grid';
        grid.style.cssText = 'display:grid;grid-template-columns:repeat(4,1fr);gap:8px;padding:12px 0';

        const shapeTypes = [
            { type: 'rectangle', label: 'Rectangle', svg: '<svg viewBox="0 0 48 36" width="48" height="36"><rect x="4" y="4" width="40" height="28" rx="0" fill="#e8f0fe" stroke="#1a73e8" stroke-width="2"/></svg>' },
            { type: 'rounded-rect', label: 'Rounded Rect', svg: '<svg viewBox="0 0 48 36" width="48" height="36"><rect x="4" y="4" width="40" height="28" rx="8" fill="#e8f0fe" stroke="#1a73e8" stroke-width="2"/></svg>' },
            { type: 'ellipse', label: 'Ellipse', svg: '<svg viewBox="0 0 48 36" width="48" height="36"><ellipse cx="24" cy="18" rx="20" ry="14" fill="#e8f0fe" stroke="#1a73e8" stroke-width="2"/></svg>' },
            { type: 'textbox', label: 'Text Box', svg: '<svg viewBox="0 0 48 36" width="48" height="36"><rect x="4" y="4" width="40" height="28" rx="4" fill="#fff" stroke="#dadce0" stroke-width="2"/><text x="24" y="22" text-anchor="middle" font-size="11" fill="#5f6368" font-family="Arial">Abc</text></svg>' },
        ];

        for (const st of shapeTypes) {
            const btn = document.createElement('button');
            btn.style.cssText = 'display:flex;flex-direction:column;align-items:center;justify-content:center;gap:4px;padding:12px 6px;border:2px solid #dadce0;border-radius:6px;background:#fff;cursor:pointer;font-size:12px;font-family:Arial,sans-serif;color:#202124;transition:border-color 0.15s,background 0.15s;';
            btn.title = 'Insert ' + st.label;
            btn.innerHTML = st.svg;
            const lbl = document.createElement('span');
            lbl.textContent = st.label;
            btn.appendChild(lbl);
            btn.addEventListener('click', () => {
                this._insertShape(st.type);
                close();
            });
            btn.addEventListener('mouseenter', () => { btn.style.borderColor = '#1a73e8'; btn.style.background = '#f0f6ff'; });
            btn.addEventListener('mouseleave', () => { btn.style.borderColor = '#dadce0'; btn.style.background = '#fff'; });
            grid.appendChild(btn);
        }
        modal.appendChild(grid);

        const actionsDiv = document.createElement('div');
        actionsDiv.className = 'modal-actions';
        const cancelBtn = document.createElement('button');
        cancelBtn.className = 'ss-modal-cancel';
        cancelBtn.textContent = 'Cancel';
        actionsDiv.appendChild(cancelBtn);
        modal.appendChild(actionsDiv);

        overlay.appendChild(modal);
        document.body.appendChild(overlay);

        function close() { if (overlay.parentNode) overlay.parentNode.removeChild(overlay); }
        cancelBtn.addEventListener('click', close);
        overlay.addEventListener('click', (e) => { if (e.target === overlay) close(); });
    }

    _insertShape(type) {
        const sheet = this._sheet();
        if (!sheet) return;
        if (!sheet.shapes) sheet.shapes = [];

        const defaults = {
            'rectangle': { fill: '#e8f0fe', stroke: '#1a73e8', strokeWidth: 2, borderRadius: 0 },
            'rounded-rect': { fill: '#e8f0fe', stroke: '#1a73e8', strokeWidth: 2, borderRadius: 8 },
            'ellipse': { fill: '#e8f0fe', stroke: '#1a73e8', strokeWidth: 2, borderRadius: 50 },
            'textbox': { fill: '#ffffff', stroke: '#dadce0', strokeWidth: 1, borderRadius: 4 },
        };

        const style = { fontSize: 13, textAlign: 'center', ...(defaults[type] || defaults['rectangle']) };
        const shape = {
            id: Date.now() + Math.floor(Math.random() * 1000),
            type: type,
            x: 120 + Math.floor(Math.random() * 60),
            y: 120 + Math.floor(Math.random() * 60),
            width: type === 'textbox' ? 180 : 160,
            height: type === 'textbox' ? 80 : 120,
            text: type === 'textbox' ? 'Text' : '',
            style: style,
        };

        sheet.shapes.push(shape);
        this._createShapeElement(shape);
    }

    _createShapeElement(shape) {
        const el = document.createElement('div');
        el.className = 'ss-shape-overlay';
        el.dataset.shapeId = shape.id;
        el.style.position = 'absolute';
        el.style.left = shape.x + 'px';
        el.style.top = shape.y + 'px';
        el.style.width = shape.width + 'px';
        el.style.height = shape.height + 'px';

        // Shape-specific rendering
        const s = shape.style || {};
        if (shape.type === 'rectangle') {
            el.style.background = s.fill || '#e8f0fe';
            el.style.border = (s.strokeWidth || 2) + 'px solid ' + (s.stroke || '#1a73e8');
            el.style.borderRadius = '0';
        } else if (shape.type === 'rounded-rect') {
            el.style.background = s.fill || '#e8f0fe';
            el.style.border = (s.strokeWidth || 2) + 'px solid ' + (s.stroke || '#1a73e8');
            el.style.borderRadius = (s.borderRadius || 8) + 'px';
        } else if (shape.type === 'ellipse') {
            el.style.background = s.fill || '#e8f0fe';
            el.style.border = (s.strokeWidth || 2) + 'px solid ' + (s.stroke || '#1a73e8');
            el.style.borderRadius = '50%';
        } else if (shape.type === 'textbox') {
            el.style.background = s.fill || '#fff';
            el.style.border = (s.strokeWidth || 1) + 'px solid ' + (s.stroke || '#dadce0');
            el.style.borderRadius = (s.borderRadius || 4) + 'px';
        }

        // Text content (editable on double-click)
        const textDiv = document.createElement('div');
        textDiv.className = 'ss-shape-text';
        textDiv.textContent = shape.text || '';
        textDiv.style.cssText = 'padding:8px;font-size:' + (s.fontSize || 13) + 'px;text-align:' + (s.textAlign || 'center') + ';outline:none;height:100%;display:flex;align-items:center;justify-content:center;overflow:hidden;word-wrap:break-word;box-sizing:border-box;color:#202124;user-select:none;';
        el.appendChild(textDiv);

        // Double-click to edit text
        textDiv.addEventListener('dblclick', (e) => {
            e.stopPropagation();
            textDiv.contentEditable = 'true';
            textDiv.style.userSelect = 'auto';
            textDiv.style.cursor = 'text';
            textDiv.focus();
            // Select all text on edit start
            const range = document.createRange();
            range.selectNodeContents(textDiv);
            const sel = window.getSelection();
            sel.removeAllRanges();
            sel.addRange(range);
        });
        textDiv.addEventListener('blur', () => {
            textDiv.contentEditable = 'false';
            textDiv.style.userSelect = 'none';
            textDiv.style.cursor = '';
            shape.text = textDiv.textContent;
        });
        textDiv.addEventListener('keydown', (e) => {
            if (e.key === 'Escape') { textDiv.blur(); }
            e.stopPropagation();
        });

        // Resize handle (bottom-right corner)
        const resizeHandle = document.createElement('div');
        resizeHandle.style.cssText = 'position:absolute;right:-4px;bottom:-4px;width:10px;height:10px;background:#1a73e8;cursor:se-resize;border-radius:2px;opacity:0;transition:opacity 0.15s;';
        el.appendChild(resizeHandle);

        // Remove button (top-right)
        const removeBtn = document.createElement('button');
        removeBtn.style.cssText = 'position:absolute;top:-8px;right:-8px;width:20px;height:20px;border-radius:50%;background:#d93025;color:#fff;border:none;cursor:pointer;font-size:12px;line-height:20px;text-align:center;display:none;z-index:26;';
        removeBtn.textContent = 'x';
        removeBtn.title = 'Remove shape';
        el.appendChild(removeBtn);

        // Show controls on hover
        el.addEventListener('mouseenter', () => {
            el.style.outline = '2px solid #1a73e8';
            el.style.outlineOffset = '-1px';
            removeBtn.style.display = '';
            resizeHandle.style.opacity = '1';
        });
        el.addEventListener('mouseleave', () => {
            el.style.outline = '';
            el.style.outlineOffset = '';
            removeBtn.style.display = 'none';
            resizeHandle.style.opacity = '0';
        });

        // Drag to move
        let dragState = null;
        el.addEventListener('mousedown', (e) => {
            if (e.target === resizeHandle || e.target === removeBtn) return;
            // Don't drag if editing text
            if (textDiv.contentEditable === 'true') return;
            e.preventDefault();
            dragState = { startX: e.clientX, startY: e.clientY, origX: shape.x, origY: shape.y };
            const onMove = (me) => {
                if (!dragState) return;
                shape.x = Math.max(0, dragState.origX + (me.clientX - dragState.startX));
                shape.y = Math.max(0, dragState.origY + (me.clientY - dragState.startY));
                el.style.left = shape.x + 'px';
                el.style.top = shape.y + 'px';
            };
            const onUp = () => {
                dragState = null;
                document.removeEventListener('mousemove', onMove);
                document.removeEventListener('mouseup', onUp);
            };
            document.addEventListener('mousemove', onMove);
            document.addEventListener('mouseup', onUp);
        });

        // Resize
        let resizeState = null;
        resizeHandle.addEventListener('mousedown', (e) => {
            e.preventDefault();
            e.stopPropagation();
            resizeState = { startX: e.clientX, startY: e.clientY, origW: shape.width, origH: shape.height };
            const onMove = (me) => {
                if (!resizeState) return;
                shape.width = Math.max(40, resizeState.origW + (me.clientX - resizeState.startX));
                shape.height = Math.max(30, resizeState.origH + (me.clientY - resizeState.startY));
                el.style.width = shape.width + 'px';
                el.style.height = shape.height + 'px';
            };
            const onUp = () => {
                resizeState = null;
                document.removeEventListener('mousemove', onMove);
                document.removeEventListener('mouseup', onUp);
            };
            document.addEventListener('mousemove', onMove);
            document.addEventListener('mouseup', onUp);
        });

        // Remove
        removeBtn.addEventListener('click', (e) => {
            e.stopPropagation();
            const sheet = this._sheet();
            if (sheet && sheet.shapes) {
                const idx = sheet.shapes.findIndex(sh => sh.id === shape.id);
                if (idx >= 0) sheet.shapes.splice(idx, 1);
            }
            el.remove();
        });

        this.canvasWrap.appendChild(el);
        shape._el = el;
    }

    /** Recreate shape DOM elements for the active sheet. */
    _showActiveSheetShapes() {
        this.canvasWrap.querySelectorAll('.ss-shape-overlay').forEach(el => el.remove());
        const sheet = this._sheet();
        if (!sheet || !sheet.shapes) return;
        for (const shape of sheet.shapes) {
            this._createShapeElement(shape);
        }
    }

    // ─── Sparklines (S3.6) ───────────────────────────
    _renderSparkline(ctx, x, y, w, h, sparkInfo, sheet) {
        const rangeStr = sparkInfo.range;
        const type = sparkInfo.type; // 'line' or 'bar'
        const values = parseRange(rangeStr, sheet);
        if (values.length === 0) return;

        const padding = 3;
        const innerW = w - padding * 2;
        const innerH = h - padding * 2;
        const minVal = Math.min(...values);
        const maxVal = Math.max(...values);
        const range = maxVal - minVal || 1;

        ctx.save();
        ctx.beginPath();
        ctx.rect(x, y, w, h);
        ctx.clip();

        if (type === 'bar') {
            // Bar sparkline
            const barW = Math.max(1, innerW / values.length - 1);
            for (let i = 0; i < values.length; i++) {
                const ratio = (values[i] - minVal) / range;
                const barH = Math.max(1, ratio * innerH);
                const bx = x + padding + i * (innerW / values.length);
                const by = y + padding + innerH - barH;
                const slColors = this._getThemeColors();
                ctx.fillStyle = values[i] >= 0 ? slColors.sparklinePositive : slColors.sparklineNegative;
                ctx.fillRect(bx, by, barW, barH);
            }
        } else {
            // Line sparkline
            ctx.strokeStyle = this._getThemeColors().sparklinePositive;
            ctx.lineWidth = 1.5;
            ctx.beginPath();
            for (let i = 0; i < values.length; i++) {
                const px = x + padding + (i / (values.length - 1 || 1)) * innerW;
                const py = y + padding + innerH - ((values[i] - minVal) / range) * innerH;
                if (i === 0) ctx.moveTo(px, py);
                else ctx.lineTo(px, py);
            }
            ctx.stroke();

            // Area fill
            ctx.lineTo(x + padding + innerW, y + padding + innerH);
            ctx.lineTo(x + padding, y + padding + innerH);
            ctx.closePath();
            ctx.fillStyle = this._getThemeColors().sparklineArea;
            ctx.fill();
        }
        ctx.restore();
    }

    // ─── Hyperlinks (S5.5) ───────────────────────────
    setCellHyperlink(col, row, url) {
        const sheet = this._sheet();
        if (!sheet) return;
        let cell = sheet.getCell(col, row);
        if (!cell) {
            cell = { value: '', formula: null, display: '', type: 'string', style: null };
            sheet.setCell(col, row, cell);
        }
        if (url) {
            cell.hyperlink = url;
        } else {
            delete cell.hyperlink;
        }
        this.render();
    }

    // ─── Pivot Tables (S5.1) ─────────────────────────
    showPivotTableDialog() {
        const sheet = this._sheet();
        if (!sheet) return;

        const range = this.getSelectionRange();
        const defaultRange = this.getCellA1(range.startCol, range.startRow) + ':' + this.getCellA1(range.endCol, range.endRow);

        // Collect column headers from row 0
        const headers = [];
        for (let c = range.startCol; c <= range.endCol; c++) {
            const hc = sheet.getCell(c, 0);
            headers.push({ col: c, name: hc ? String(hc.value ?? this.getCellA1Col(c)) : this.getCellA1Col(c) });
        }

        const overlay = document.createElement('div');
        overlay.className = 'modal-overlay show';
        const modal = document.createElement('div');
        modal.className = 'modal';
        modal.style.minWidth = '500px';

        const buildOptions = () => headers.map(h => '<option value="' + h.col + '">' + this._xmlEsc(h.name) + '</option>').join('');
        const aggOptions = '<option value="sum">Sum</option><option value="count">Count</option><option value="average">Average</option>';

        modal.innerHTML = '<h3>Pivot Table</h3>'
            + '<div style="display:flex;flex-direction:column;gap:10px;padding:8px 0;">'
            + '<div class="modal-field"><label style="font-size:12px;color:#5f6368;margin-bottom:4px;display:block;">Source Range</label>'
            + '<input type="text" id="ptRange" value="' + defaultRange + '" style="width:100%;padding:6px 8px;border:1px solid #dadce0;border-radius:4px;font-size:13px;"></div>'
            + '<div class="modal-field"><label style="font-size:12px;color:#5f6368;margin-bottom:4px;display:block;">Row Field</label>'
            + '<select id="ptRowField" style="width:100%;padding:6px 8px;border:1px solid #dadce0;border-radius:4px;font-size:13px;">' + buildOptions() + '</select></div>'
            + '<div class="modal-field"><label style="font-size:12px;color:#5f6368;margin-bottom:4px;display:block;">Column Field (optional)</label>'
            + '<select id="ptColField" style="width:100%;padding:6px 8px;border:1px solid #dadce0;border-radius:4px;font-size:13px;"><option value="">None</option>' + buildOptions() + '</select></div>'
            + '<div class="modal-field"><label style="font-size:12px;color:#5f6368;margin-bottom:4px;display:block;">Value Field</label>'
            + '<select id="ptValField" style="width:100%;padding:6px 8px;border:1px solid #dadce0;border-radius:4px;font-size:13px;">' + buildOptions() + '</select></div>'
            + '<div class="modal-field"><label style="font-size:12px;color:#5f6368;margin-bottom:4px;display:block;">Aggregation</label>'
            + '<select id="ptAgg" style="width:100%;padding:6px 8px;border:1px solid #dadce0;border-radius:4px;font-size:13px;">' + aggOptions + '</select></div>'
            + '</div>'
            + '<div class="modal-actions"><button class="ss-modal-cancel">Cancel</button><button class="ss-modal-ok primary">Create</button></div>';
        overlay.appendChild(modal);
        document.body.appendChild(overlay);

        const close = () => { document.body.removeChild(overlay); };
        modal.querySelector('.ss-modal-cancel').onclick = close;
        modal.querySelector('.ss-modal-ok').onclick = () => {
            const srcRange = this._parseA1Range(modal.querySelector('#ptRange').value);
            if (!srcRange) { close(); return; }
            const rowField = parseInt(modal.querySelector('#ptRowField').value, 10);
            const colFieldVal = modal.querySelector('#ptColField').value;
            const colField = colFieldVal !== '' ? parseInt(colFieldVal, 10) : null;
            const valField = parseInt(modal.querySelector('#ptValField').value, 10);
            const agg = modal.querySelector('#ptAgg').value;

            this._generatePivotTable(srcRange, rowField, colField, valField, agg);
            close();
        };
        overlay.onclick = (e) => { if (e.target === overlay) close(); };
    }

    _generatePivotTable(srcRange, rowField, colField, valField, agg) {
        const sheet = this._sheet();
        if (!sheet) return;

        // Gather source data (skip header row)
        const data = [];
        for (let r = srcRange.startRow + 1; r <= srcRange.endRow; r++) {
            const rowVal = sheet.getCell(rowField, r);
            const valCell = sheet.getCell(valField, r);
            const colVal = colField !== null ? sheet.getCell(colField, r) : null;
            data.push({
                rowKey: rowVal ? String(rowVal.value ?? '') : '',
                colKey: colVal ? String(colVal.value ?? '') : '',
                value: valCell ? Number(valCell.value) : 0
            });
        }

        // Get unique row keys and col keys
        const rowKeys = [...new Set(data.map(d => d.rowKey))].sort();
        const colKeys = colField !== null ? [...new Set(data.map(d => d.colKey))].sort() : ['Value'];

        // Aggregate
        const pivotData = {};
        for (const d of data) {
            const rk = d.rowKey;
            const ck = colField !== null ? d.colKey : 'Value';
            const key = rk + '\x00' + ck;
            if (!pivotData[key]) pivotData[key] = [];
            if (!isNaN(d.value)) pivotData[key].push(d.value);
        }

        const aggregate = (values) => {
            if (values.length === 0) return 0;
            switch (agg) {
                case 'sum': return values.reduce((a, b) => a + b, 0);
                case 'count': return values.length;
                case 'average': return values.reduce((a, b) => a + b, 0) / values.length;
                default: return values.reduce((a, b) => a + b, 0);
            }
        };

        // Create a new sheet with the pivot table
        const pivotSheet = new Sheet('Pivot Table');

        // Header row
        const rowHeader = sheet.getCell(rowField, srcRange.startRow);
        pivotSheet.setCell(0, 0, { value: rowHeader ? String(rowHeader.value ?? '') : 'Row', formula: null, display: rowHeader ? String(rowHeader.value ?? '') : 'Row', type: 'string', style: { bold: true } });
        for (let ci = 0; ci < colKeys.length; ci++) {
            pivotSheet.setCell(ci + 1, 0, { value: colKeys[ci], formula: null, display: colKeys[ci], type: 'string', style: { bold: true } });
        }

        // Data rows
        for (let ri = 0; ri < rowKeys.length; ri++) {
            pivotSheet.setCell(0, ri + 1, { value: rowKeys[ri], formula: null, display: rowKeys[ri], type: 'string', style: null });
            for (let ci = 0; ci < colKeys.length; ci++) {
                const key = rowKeys[ri] + '\x00' + colKeys[ci];
                const vals = pivotData[key] || [];
                const result = aggregate(vals);
                pivotSheet.setCell(ci + 1, ri + 1, { value: result, formula: null, display: String(Math.round(result * 100) / 100), type: 'number', style: null });
            }
        }

        this.workbook.sheets.push(pivotSheet);
        this.activeSheet = this.workbook.sheets.length - 1;
        this.selectedCell = { col: 0, row: 0 };
        this.selectionRange = null;
        this.scrollX = 0;
        this.scrollY = 0;
        this.updateSheetTabs();
        this.render();
    }

    // ─── Text to Columns (S5.2) ──────────────────────
    showTextToColumnsDialog() {
        const sheet = this._sheet();
        if (!sheet) return;

        const range = this.getSelectionRange();
        const col = range.startCol;
        const startRow = range.startRow;
        const endRow = range.endRow;

        // Collect text data from the selected column
        const textData = [];
        for (let r = startRow; r <= endRow; r++) {
            const cell = sheet.getCell(col, r);
            textData.push(cell ? String(cell.value ?? '') : '');
        }

        const overlay = document.createElement('div');
        overlay.className = 'modal-overlay show';
        const modal = document.createElement('div');
        modal.className = 'modal';
        modal.style.minWidth = '500px';

        modal.innerHTML = '<h3>Text to Columns</h3>'
            + '<div style="display:flex;flex-direction:column;gap:10px;padding:8px 0;">'
            + '<div class="modal-field"><label style="font-size:12px;color:#5f6368;margin-bottom:4px;display:block;">Delimiter</label>'
            + '<div style="display:flex;gap:12px;flex-wrap:wrap;">'
            + '<label style="font-size:13px;cursor:pointer;"><input type="radio" name="ttcDelim" value="," checked> Comma</label>'
            + '<label style="font-size:13px;cursor:pointer;"><input type="radio" name="ttcDelim" value=";"> Semicolon</label>'
            + '<label style="font-size:13px;cursor:pointer;"><input type="radio" name="ttcDelim" value="\t"> Tab</label>'
            + '<label style="font-size:13px;cursor:pointer;"><input type="radio" name="ttcDelim" value=" "> Space</label>'
            + '<label style="font-size:13px;cursor:pointer;"><input type="radio" name="ttcDelim" value="custom"> Custom:</label>'
            + '<input type="text" id="ttcCustomDelim" maxlength="5" style="width:60px;padding:4px 6px;border:1px solid #dadce0;border-radius:4px;font-size:13px;" placeholder="|">'
            + '</div></div>'
            + '<div class="modal-field"><label style="font-size:12px;color:#5f6368;margin-bottom:4px;display:block;">Preview</label>'
            + '<div id="ttcPreview" style="max-height:140px;overflow:auto;border:1px solid #dadce0;border-radius:4px;padding:8px;font-family:monospace;font-size:12px;background:#f8f9fa;white-space:pre;"></div></div>'
            + '</div>'
            + '<div class="modal-actions"><button class="ss-modal-cancel">Cancel</button><button class="ss-modal-ok primary">OK</button></div>';
        overlay.appendChild(modal);
        document.body.appendChild(overlay);

        const previewEl = modal.querySelector('#ttcPreview');
        const customInput = modal.querySelector('#ttcCustomDelim');

        const getDelimiter = () => {
            const checked = modal.querySelector('input[name="ttcDelim"]:checked');
            if (!checked) return ',';
            if (checked.value === 'custom') return customInput.value || ',';
            if (checked.value === '\\t') return '\t';
            return checked.value;
        };

        const updatePreview = () => {
            const delim = getDelimiter();
            let html = '';
            for (let i = 0; i < Math.min(textData.length, 5); i++) {
                const parts = textData[i].split(delim);
                html += parts.map(p => '[' + p.trim() + ']').join('  ') + '\n';
            }
            if (textData.length > 5) html += '... and ' + (textData.length - 5) + ' more rows\n';
            previewEl.textContent = html;
        };

        updatePreview();
        modal.querySelectorAll('input[name="ttcDelim"]').forEach(r => r.addEventListener('change', updatePreview));
        customInput.addEventListener('input', updatePreview);

        const close = () => { document.body.removeChild(overlay); };
        modal.querySelector('.ss-modal-cancel').onclick = close;
        modal.querySelector('.ss-modal-ok').onclick = () => {
            const delim = getDelimiter();
            for (let r = startRow; r <= endRow; r++) {
                const cell = sheet.getCell(col, r);
                const text = cell ? String(cell.value ?? '') : '';
                const parts = text.split(delim);
                for (let i = 0; i < parts.length; i++) {
                    this._setCellValue(col + i, r, parts[i].trim());
                }
            }
            this.render();
            close();
        };
        overlay.onclick = (e) => { if (e.target === overlay) close(); };
    }

    // S4.1 ── Real-time Collaboration ──────────────
    startCollab(room, name, relayUrl) {
        this.stopCollab();
        this._collabRoom = room; this._collabName = name || 'Anonymous';
        this._collabPeerId = 'ss-' + Date.now() + '-' + Math.random().toString(36).slice(2, 8);
        this._collabConnected = false; this._collabReconnectAttempt = 0;
        this._collabOfflineBuffer = []; this._collabApplyingRemote = false;
        this._collabPeers = new Map(); this._collabRelayUrl = relayUrl;
        var PC = ['#4285f4','#ea4335','#34a853','#fbbc04','#9c27b0','#00bcd4','#ff5722','#607d8b','#e91e63','#3f51b5'];
        this._collabColor = PC[Math.abs(this._hc(this._collabPeerId)) % PC.length];
        this._collabConnect();
        this._collabCursorTimer = setInterval(() => { if (this._collabConnected && this._collabWs) this._collabSendOp({ action:'ssCursor', sheet:this.activeSheet, col:this.selectedCell.col, row:this.selectedCell.row, name:this._collabName, color:this._collabColor }); }, 500);
    }
    _hc(s) { var h=0; for(var i=0;i<s.length;i++){h=((h<<5)-h)+s.charCodeAt(i);h|=0;} return h; }
    _collabConnect() {
        var DL=[2000,4000,8000,16000,30000];
        try { this._collabWs = new WebSocket(this._collabRelayUrl+(this._collabRelayUrl.includes('?')?'&':'?')+'room='+encodeURIComponent(this._collabRoom)+'&peer='+encodeURIComponent(this._collabPeerId)+'&name='+encodeURIComponent(this._collabName)); } catch(e){ return; }
        var self = this;
        var connectTimer = setTimeout(function() {
            if (self._collabWs && self._collabWs.readyState !== WebSocket.OPEN) {
                self._collabWs.close();
            }
        }, 8000);
        this._collabWs.onopen = () => { clearTimeout(connectTimer); this._collabConnected=true; this._collabReconnectAttempt=0; /* Bug C25: Flush offline buffer with error handling */ var buf=this._collabOfflineBuffer||[]; this._collabOfflineBuffer=[]; for(var i=0;i<buf.length;i++){try{if(this._collabWs&&this._collabWs.readyState===WebSocket.OPEN){this._collabWs.send(buf[i]);}else{this._collabOfflineBuffer.push(buf[i]);break;}}catch(e){this._collabOfflineBuffer=buf.slice(i);break;}} this.broadcastSheetSync(); };
        this._collabWs.onmessage = (ev) => { try { var msg=JSON.parse(ev.data), data=msg.data?(typeof msg.data==='string'?JSON.parse(msg.data):msg.data):msg, from=msg.peerId||data.peerId; if(from===this._collabPeerId) return; this._collabApplyingRemote=true; this._applyRemoteSSop(data,from); this._collabApplyingRemote=false; } catch(e){} };
        this._collabWs.onclose = () => { this._collabConnected=false; if(this._collabRoom&&this._collabReconnectAttempt<5){var d=DL[Math.min(this._collabReconnectAttempt,DL.length-1)];this._collabReconnectAttempt++;this._collabReconnectTimer=setTimeout(()=>this._collabConnect(),d);} };
        this._collabWs.onerror = () => {};
    }
    stopCollab() { if(this._collabWs){this._collabWs.close();this._collabWs=null;} if(this._collabCursorTimer){clearInterval(this._collabCursorTimer);this._collabCursorTimer=null;} if(this._collabReconnectTimer){clearTimeout(this._collabReconnectTimer);this._collabReconnectTimer=null;} this._collabConnected=false;this._collabRoom=null;this._collabPeers=new Map(); }
    isCollabActive() { return !!this._collabConnected; }
    _collabSendOp(op) { op.peerId=this._collabPeerId; var j=JSON.stringify(op); if(this._collabWs&&this._collabConnected) { this._collabWs.send(j); } else { if(!this._collabOfflineBuffer) this._collabOfflineBuffer=[]; if(this._collabOfflineBuffer.length<5000) { this._collabOfflineBuffer.push(j); } } }
    broadcastCellEdit(sh,col,row,val,style) { if(!this._collabConnected||this._collabApplyingRemote) return; this._collabSendOp({action:'ssSetCell',sheet:sh,col:col,row:row,value:val,style:style||null}); }
    broadcastFormatChange(sh,col,row,style) { if(!this._collabConnected||this._collabApplyingRemote) return; this._collabSendOp({action:'ssFormat',sheet:sh,col:col,row:row,style:style}); }
    broadcastSheetSync() { if(!this._collabConnected||!this.workbook) return; try { this._collabSendOp({action:'ssSync',workbookData:JSON.stringify({sheets:this.workbook.sheets.map(s=>({name:s.name,cells:s.cells,colWidths:s.colWidths,rowHeights:s.rowHeights,merges:s.merges,maxCol:s.maxCol,maxRow:s.maxRow}))})}); } catch(e){} }
    _applyRemoteSSop(data,fromPeer) {
        switch(data.action) {
            case 'ssSetCell': this.applyRemoteCellEdit(data); break;
            case 'ssFormat': { var sh=this.workbook&&this.workbook.sheets[data.sheet]; if(!sh) break; var c=sh.getCell(data.col,data.row); if(!c){c={value:'',formula:null,display:'',type:'string',style:{}};sh.setCell(data.col,data.row,c);} Object.assign(c.style||(c.style={}),data.style||{}); this.render(); break; }
            case 'ssSync': { try { var wb=JSON.parse(data.workbookData); if(wb&&wb.sheets){this.workbook=new Workbook();this.workbook.sheets=wb.sheets.map(sd=>{var s=new Sheet(sd.name);s.cells=sd.cells||{};s.colWidths=sd.colWidths||{};s.rowHeights=sd.rowHeights||{};s.merges=sd.merges||[];s.maxCol=sd.maxCol||0;s.maxRow=sd.maxRow||0;return s;});if(this.activeSheet>=this.workbook.sheets.length)this.activeSheet=0;this.updateSheetTabs();this._updateFormulaBar();this.render();} } catch(e){} break; }
            case 'ssCursor': { if(!fromPeer) break; var p=this._collabPeers.get(fromPeer)||{}; p.name=data.name||'Anonymous'; p.color=data.color||'#4285f4'; p.selection={sheet:data.sheet,col:data.col,row:data.row}; p.lastSeen=Date.now(); this._collabPeers.set(fromPeer,p); for(var[pid,pp] of this._collabPeers){if(Date.now()-pp.lastSeen>10000)this._collabPeers.delete(pid);} this.render(); break; }
        }
    }
    // Note: Spreadsheet collab uses last-write-wins (no CRDT).
    // Concurrent edits to the same cell will overwrite each other.
    applyRemoteCellEdit(data) { var sh=this.workbook&&this.workbook.sheets[data.sheet]; if(!sh) return; if(data.value===''||data.value===null||data.value===undefined){sh.deleteCell(data.col,data.row);} else{var nc=this._parseRawValue(String(data.value));if(nc.formula)nc.display=String(evaluateFormula(nc.formula,sh));if(data.style)nc.style=data.style;sh.setCell(data.col,data.row,nc);} this.render(); }
    _renderPeerCursors(ctx) {
        if(!this._collabPeers||this._collabPeers.size===0) return;
        for(var[,peer] of this._collabPeers) {
            if(!peer.selection||peer.selection.sheet!==this.activeSheet) continue;
            var col=peer.selection.col,row=peer.selection.row,sx=this._cellScreenX(col),sy=this._cellScreenY(row),sw=this.getColumnWidth(col),sh=this.getRowHeight(row);
            ctx.strokeStyle=peer.color||'#4285f4';ctx.lineWidth=2;ctx.strokeRect(sx,sy,sw,sh);
            ctx.font='500 10px Arial, sans-serif';var tw=ctx.measureText(peer.name).width,lw=tw+8,lh=16,lx=sx,ly=sy-lh-2;
            ctx.fillStyle=peer.color||'#4285f4';ctx.fillRect(lx,ly,lw,lh);ctx.fillStyle=this._getThemeColors().peerCursorLabel;ctx.textAlign='left';ctx.textBaseline='middle';ctx.fillText(peer.name,lx+4,ly+lh/2);
        }
    }

    // S4.3 ── Zoom Support ────────────────────────
    get zoomLevel() { return this._zoomLevel||1.0; }
    set zoomLevel(v) { this._zoomLevel=Math.max(0.5,Math.min(2.0,v)); this._applyZoom(); try{localStorage.setItem('ss-zoom',String(this._zoomLevel));}catch(e){} }
    _initZoom() { try{var s=localStorage.getItem('ss-zoom');if(s)this._zoomLevel=Math.max(0.5,Math.min(2.0,parseFloat(s)));}catch(e){} this._applyZoom(); }
    _applyZoom() { if(this.canvasWrap){this.canvasWrap.style.transform='scale('+this._zoomLevel+')';this.canvasWrap.style.transformOrigin='top left';this.canvasWrap.style.width=(100/this._zoomLevel)+'%';this.canvasWrap.style.height=(100/this._zoomLevel)+'%';} }
    zoomIn() { this.zoomLevel=(this._zoomLevel||1.0)+0.1; }
    zoomOut() { this.zoomLevel=(this._zoomLevel||1.0)-0.1; }
    zoomTo(pct) { this.zoomLevel=pct/100; }

    // S5.7 ── Touch Event Handlers (Mobile/Tablet) ──

    _handleTouchStart(e) {
        if (e.touches.length === 1) {
            e.preventDefault();
            const touch = e.touches[0];
            const rect = this.canvas.getBoundingClientRect();
            const x = touch.clientX - rect.left;
            const y = touch.clientY - rect.top;
            this._touchStartTime = Date.now();
            this._touchStartPos = { x, y };
            this._touchMoved = false;
            // Simulate mousedown for cell selection
            this.handleMouseDown({
                clientX: touch.clientX,
                clientY: touch.clientY,
                button: 0,
                preventDefault() {},
                shiftKey: false
            });
        } else if (e.touches.length === 2) {
            // Pinch-to-zoom start
            e.preventDefault();
            const dx = e.touches[0].clientX - e.touches[1].clientX;
            const dy = e.touches[0].clientY - e.touches[1].clientY;
            this._pinchStartDist = Math.sqrt(dx * dx + dy * dy);
            this._pinchStartZoom = this._zoomLevel || 1.0;
        }
    }

    _handleTouchMove(e) {
        if (e.touches.length === 1 && this._touchStartPos) {
            e.preventDefault();
            const touch = e.touches[0];
            const rect = this.canvas.getBoundingClientRect();
            const x = touch.clientX - rect.left;
            const y = touch.clientY - rect.top;
            const dx = this._touchStartPos.x - x;
            const dy = this._touchStartPos.y - y;
            // Only start scrolling after a small threshold to avoid jitter
            if (!this._touchMoved && Math.abs(dx) < 4 && Math.abs(dy) < 4) return;
            this._touchMoved = true;
            this.scrollX = Math.max(0, this.scrollX + dx);
            this.scrollY = Math.max(0, this.scrollY + dy);
            this._touchStartPos = { x, y };
            this.render();
        } else if (e.touches.length === 2 && this._pinchStartDist) {
            // Pinch-to-zoom
            e.preventDefault();
            const dx = e.touches[0].clientX - e.touches[1].clientX;
            const dy = e.touches[0].clientY - e.touches[1].clientY;
            const dist = Math.sqrt(dx * dx + dy * dy);
            const scale = dist / this._pinchStartDist;
            this.zoomLevel = this._pinchStartZoom * scale;
        }
    }

    _handleTouchEnd(e) {
        if (e.changedTouches.length === 1 && this._touchStartTime) {
            const elapsed = Date.now() - this._touchStartTime;
            const touch = e.changedTouches[0];
            if (elapsed < 300 && !this._touchMoved) {
                // Short tap without movement — select cell
                this.handleMouseDown({
                    clientX: touch.clientX,
                    clientY: touch.clientY,
                    button: 0,
                    preventDefault() {},
                    shiftKey: false
                });
                this.handleMouseUp({
                    clientX: touch.clientX,
                    clientY: touch.clientY,
                    button: 0,
                    preventDefault() {}
                });
                // Detect double-tap for editing
                if (this._lastTapTime && Date.now() - this._lastTapTime < 400) {
                    this.handleDoubleClick({
                        clientX: touch.clientX,
                        clientY: touch.clientY,
                        preventDefault() {}
                    });
                    this._lastTapTime = null;
                } else {
                    this._lastTapTime = Date.now();
                }
            }
        }
        this._touchStartPos = null;
        this._touchStartTime = null;
        this._touchMoved = false;
        this._pinchStartDist = null;
    }

    // S4.5 ── Formula Bar Syntax Highlighting ──────
    _updateFormulaHighlight() {
        if (!this._formulaHighlight) return;
        const input = document.activeElement === this.formulaInput ? this.formulaInput
            : (this.editingCell ? this.editInput : null);
        const val = input ? input.value : (this.formulaInput ? this.formulaInput.value : '');

        if (!val || (!val.startsWith('=') && !val.startsWith('{='))) {
            this._formulaHighlight.style.display = 'none';
            if (this.formulaInput) this.formulaInput.style.color = '';
            return;
        }

        // Make input text transparent so overlay shows through
        if (this.formulaInput && document.activeElement === this.formulaInput) {
            this.formulaInput.style.color = 'transparent';
            this.formulaInput.style.caretColor = this._getThemeColors().text;
        }

        const colors = this._formulaRefColors;
        let html = '';
        let lastIdx = 0;
        let colorIdx = 0;

        // Find ranges and single refs, color them
        const rangeRegex = /([A-Z]{1,3}\d{1,7}):([A-Z]{1,3}\d{1,7})/gi;
        const allRefs = [];
        let m;

        while ((m = rangeRegex.exec(val)) !== null) {
            allRefs.push({ start: m.index, end: m.index + m[0].length, text: m[0] });
        }

        const singleRegex = /\b([A-Z]{1,3})(\d{1,7})\b/gi;
        while ((m = singleRegex.exec(val)) !== null) {
            // Skip if overlaps with a range ref
            if (allRefs.some(r => m.index >= r.start && m.index < r.end)) continue;
            // Skip function names
            if (/^[A-Z]{2,}$/i.test(m[1])) {
                const fnNames = ['SUM','AVERAGE','COUNT','COUNTA','MIN','MAX','IF','IFERROR','AND','OR','NOT','VLOOKUP','HLOOKUP','INDEX','MATCH','LEFT','RIGHT','MID','LEN','TRIM','UPPER','LOWER','CONCATENATE','FIND','SEARCH','SUBSTITUTE','TEXT','VALUE','ROUND','ROUNDUP','ROUNDDOWN','CEILING','FLOOR','ABS','INT','MOD','POWER','SQRT','SIN','COS','TAN','ASIN','ACOS','ATAN','LOG','LOG10','LN','EXP','PI','RAND','RANDBETWEEN','COUNTIF','SUMIF','AVERAGEIF','NOW','TODAY','DATE','YEAR','MONTH','DAY','SPARKLINE'];
                if (fnNames.includes(m[1].toUpperCase())) continue;
            }
            allRefs.push({ start: m.index, end: m.index + m[0].length, text: m[0] });
        }

        // Sort by position
        allRefs.sort((a, b) => a.start - b.start);

        // Build highlighted HTML
        const esc = (s) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
        for (const ref of allRefs) {
            if (ref.start > lastIdx) {
                html += esc(val.slice(lastIdx, ref.start));
            }
            const color = colors[colorIdx % colors.length];
            html += '<span style="color:' + color + ';font-weight:600;">' + esc(ref.text) + '</span>';
            colorIdx++;
            lastIdx = ref.end;
        }
        if (lastIdx < val.length) {
            html += esc(val.slice(lastIdx));
        }

        this._formulaHighlight.innerHTML = html;
        this._formulaHighlight.style.display = 'block';

        // Sync scroll position
        if (this.formulaInput && document.activeElement === this.formulaInput) {
            this._formulaHighlight.scrollLeft = this.formulaInput.scrollLeft;
        }
    }

    // S4.4 ── Formula Autocomplete ────────────────
    _initFormulaAutocomplete() {
        this._acDrop=null; this._acIdx=-1; this._sigTip=null;
        var self=this;
        var doAC=function(e){var inp=e.target,v=inp.value;if(!v.startsWith('=')||v.length<3){self._hideAC();self._hideSig();return;}var m=v.match(/([A-Z]{2,})$/i);if(m){var pf=m[1].toUpperCase(),ms=FORMULA_CATALOG.filter(function(f){return f.name.startsWith(pf);});if(ms.length>0&&pf!==ms[0].name)self._showAC(inp,ms);else self._hideAC();}else{self._hideAC();var fm=v.match(/([A-Z]+)\([^)]*$/i);if(fm){var fn=FORMULA_CATALOG.find(function(f){return f.name===fm[1].toUpperCase();});if(fn)self._showSig(inp,fn);else self._hideSig();}else self._hideSig();}};
        this.editInput.addEventListener('input',doAC);
        this.formulaInput.addEventListener('input',doAC);
        var doKeys=function(e){if(!self._acDrop||self._acDrop.style.display==='none')return;var it=self._acDrop.querySelectorAll('.ss-ac-item');if(e.key==='ArrowDown'){e.preventDefault();e.stopPropagation();self._acIdx=Math.min(self._acIdx+1,it.length-1);self._hlAC(it);}else if(e.key==='ArrowUp'){e.preventDefault();e.stopPropagation();self._acIdx=Math.max(self._acIdx-1,0);self._hlAC(it);}else if(e.key==='Tab'||e.key==='Enter'){if(self._acIdx>=0&&self._acIdx<it.length){e.preventDefault();e.stopPropagation();it[self._acIdx].click();}}else if(e.key==='Escape')self._hideAC();};
        this.editInput.addEventListener('keydown',doKeys,true);
        this.formulaInput.addEventListener('keydown',doKeys,true);
    }
    _showAC(inp,ms) {
        if(!this._acDrop){this._acDrop=document.createElement('div');this._acDrop.className='ss-autocomplete-dropdown';this.canvasWrap.appendChild(this._acDrop);}
        this._acDrop.innerHTML='';this._acIdx=0;var self=this;
        for(var i=0;i<Math.min(ms.length,10);i++){var f=ms[i],it=document.createElement('div');it.className='ss-ac-item'+(i===0?' active':'');it.innerHTML='<span class="ss-ac-name">'+f.name+'</span><span class="ss-ac-desc">'+f.desc+'</span>';(function(fn){it.addEventListener('click',function(){var v=inp.value,pm=v.match(/([A-Z]{2,})$/i);if(pm){inp.value=v.slice(0,v.length-pm[1].length)+fn.name+'(';inp.focus();inp.selectionStart=inp.selectionEnd=inp.value.length;self._showSig(inp,fn);}self._hideAC();});})(f);this._acDrop.appendChild(it);}
        var r=inp.getBoundingClientRect(),w=this.canvasWrap.getBoundingClientRect();
        this._acDrop.style.display='block';this._acDrop.style.left=(r.left-w.left)+'px';this._acDrop.style.top=(r.bottom-w.top+2)+'px';this._acDrop.style.minWidth='280px';
    }
    _hideAC() { if(this._acDrop)this._acDrop.style.display='none'; this._acIdx=-1; }
    _hlAC(it) { for(var i=0;i<it.length;i++){if(i===this._acIdx)it[i].classList.add('active');else it[i].classList.remove('active');} }
    _showSig(inp,fn) {
        if(!this._sigTip){this._sigTip=document.createElement('div');this._sigTip.className='ss-signature-tooltip';this.canvasWrap.appendChild(this._sigTip);}
        this._sigTip.textContent=fn.name+'('+fn.signature+')';
        var r=inp.getBoundingClientRect(),w=this.canvasWrap.getBoundingClientRect();
        this._sigTip.style.display='block';this._sigTip.style.left=(r.left-w.left)+'px';this._sigTip.style.top=(r.bottom-w.top+2)+'px';
    }
    _hideSig() { if(this._sigTip)this._sigTip.style.display='none'; }

    // S4.6 ── Keyboard Shortcuts Help (Ctrl+/) ────
    showKeyboardShortcutsHelp() {
        var ov=document.createElement('div');ov.className='modal-overlay show';
        var md=document.createElement('div');md.className='modal';md.style.minWidth='520px';md.style.maxHeight='80vh';md.style.overflow='auto';
        var secs=[{t:'Navigation',s:[['Arrow keys','Move to adjacent cell'],['Tab / Shift+Tab','Move right / left'],['Enter / Shift+Enter','Move down / up'],['Page Up / Page Down','Scroll up / down one screen'],['Ctrl+Home','Go to cell A1'],['Ctrl+End','Go to last used cell'],['Ctrl+Arrow','Jump to edge of data region']]},{t:'Editing',s:[['F2','Edit selected cell'],['Delete / Backspace','Clear cell contents'],['Escape','Cancel editing / clear selection'],['Ctrl+Z','Undo'],['Ctrl+Y / Ctrl+Shift+Z','Redo'],['Ctrl+B','Toggle bold'],['Ctrl+I','Toggle italic'],['Ctrl+U','Toggle underline']]},{t:'Selection',s:[['Shift+Arrow','Extend selection'],['Ctrl+Space','Select entire column'],['Shift+Space','Select entire row'],['Ctrl+A','Select all cells'],['Shift+Click','Extend selection to clicked cell']]},{t:'Data and View',s:[['Ctrl+F','Find in sheet'],['Ctrl+H','Find and replace'],['Ctrl+/','Show this shortcuts help'],['Ctrl+Mousewheel','Zoom in / out']]}];
        var h='<h3>Keyboard Shortcuts</h3><div style="padding:8px 0;">';
        for(var i=0;i<secs.length;i++){h+='<div style="margin-bottom:16px;"><div style="font-weight:600;font-size:13px;color:#202124;margin-bottom:8px;">'+secs[i].t+'</div><table style="width:100%;font-size:13px;border-collapse:collapse;">';for(var j=0;j<secs[i].s.length;j++){h+='<tr><td style="padding:3px 8px 3px 0;white-space:nowrap;"><kbd style="display:inline-block;padding:2px 6px;background:#f1f3f4;border:1px solid #dadce0;border-radius:3px;font-family:monospace;font-size:11px;">'+secs[i].s[j][0]+'</kbd></td><td style="padding:3px 0;color:#5f6368;">'+secs[i].s[j][1]+'</td></tr>';}h+='</table></div>';}
        h+='</div><div class="modal-actions"><button class="ss-modal-ok primary">Close</button></div>';
        md.innerHTML=h;ov.appendChild(md);document.body.appendChild(ov);
        var self=this,cl=function(){document.body.removeChild(ov);self.canvas.focus();};
        md.querySelector('.ss-modal-ok').onclick=cl;ov.onclick=function(e){if(e.target===ov)cl();};
    }

    // ─── Destroy ─────────────────────────────────
    destroy() {
        this.stopCollab();
        this._removeAllChartElements();
        this._hideAC(); this._hideSig();
        this.canvasWrap?.querySelectorAll('.ss-image-overlay').forEach(el => el.remove());
        this.canvasWrap?.querySelectorAll('.ss-shape-overlay').forEach(el => el.remove());
        if (this._resizeObserver) { this._resizeObserver.disconnect(); this._resizeObserver = null; }
        if (this._rafId) { cancelAnimationFrame(this._rafId); this._rafId = null; }
        this.container.innerHTML = '';
    }

    // ── AI Integration ──────────────────────────────

    /** Cached AI module import — avoids dynamic import on every call (Bug 31) */
    async _getAIModule() {
        if (!this._aiModuleCache) this._aiModuleCache = await import('./ai.js');
        return this._aiModuleCache;
    }

    /** Show AI hint below formula bar when user types "=" and pauses */
    _showFormulaAIHint() {
        if (!window.S1_CONFIG?.enableAI) return;
        let hint = this.container.querySelector('.ss-ai-formula-hint');
        if (!hint) {
            hint = document.createElement('div');
            hint.className = 'ss-ai-formula-hint';
            hint.innerHTML = '<span class="ss-ai-formula-hint-text">Describe what you need — AI can write the formula</span>';
            // Insert after formula bar
            const formulaBar = this.container.querySelector('.ss-formula-bar');
            if (formulaBar && formulaBar.parentNode) {
                formulaBar.parentNode.insertBefore(hint, formulaBar.nextSibling);
            }
            hint.onclick = async () => {
                hint.style.display = 'none';
                const description = await ssPrompt('Describe the formula you need:', '');
                if (!description) return;
                const loadingOverlay = document.createElement('div');
                loadingOverlay.className = 'modal-overlay show';
                loadingOverlay.innerHTML = '<div class="modal"><h3>AI is thinking...</h3><div style="text-align:center;padding:20px"><div class="ai-inline-loading" style="justify-content:center">Processing your request</div></div><div class="modal-actions"><button class="ss-modal-cancel ai-loading-cancel">Cancel</button></div></div>';
                document.body.appendChild(loadingOverlay);
                loadingOverlay.querySelector('.ai-loading-cancel')?.addEventListener('click', () => {
                    import('./ai.js').then(m => m.abortAI()).catch(() => {});
                    if (loadingOverlay.parentNode) loadingOverlay.parentNode.removeChild(loadingOverlay);
                });
                try {
                    const { aiFormula } = await this._getAIModule();
                    const result = await aiFormula(description);
                    if (result) {
                        this.formulaInput.value = result.trim();
                        this.formulaInput.focus();
                    }
                } catch (err) {
                    await ssAlert('AI Error', err.message || 'AI is not available');
                } finally {
                    if (loadingOverlay.parentNode) loadingOverlay.parentNode.removeChild(loadingOverlay);
                }
            };
        }
        hint.style.display = 'flex';
    }

    /** Ask AI a free-form question about the current cell */
    async _showAIPromptForCell(cell) {
        const cellRef = this.getCellA1(cell.col, cell.row);
        const sheet = this._sheet();
        const cellData = sheet ? sheet.getCell(cell.col, cell.row) : null;
        const cellValue = cellData?.display ?? cellData?.value ?? '';
        const cellFormula = cellData?.formula || '';

        const question = await ssPrompt(
            `Ask AI about cell ${cellRef}${cellFormula ? ' (formula: ' + cellFormula + ')' : ''}:`,
            ''
        );
        if (!question) return;

        const loadingOverlay = document.createElement('div');
        loadingOverlay.className = 'modal-overlay show';
        loadingOverlay.innerHTML = '<div class="modal"><h3>AI is thinking...</h3><div style="text-align:center;padding:20px"><div class="ai-inline-loading" style="justify-content:center">Processing your request</div></div><div class="modal-actions"><button class="ss-modal-cancel ai-loading-cancel">Cancel</button></div></div>';
        document.body.appendChild(loadingOverlay);
        loadingOverlay.querySelector('.ai-loading-cancel')?.addEventListener('click', () => {
            import('./ai.js').then(m => m.abortAI()).catch(() => {});
            if (loadingOverlay.parentNode) loadingOverlay.parentNode.removeChild(loadingOverlay);
        });
        try {
            const { aiComplete } = await this._getAIModule();
            const prompt = `Cell ${cellRef} contains: ${cellFormula ? 'formula=' + cellFormula + ', result=' : ''}${cellValue}\n\nUser question: ${question}`;
            const result = await aiComplete('formula', prompt, { maxTokens: 512 });
            await ssAlert('AI Response', result);
        } catch (err) {
            await ssAlert('AI Error', err.message || 'AI is not available');
        } finally {
            if (loadingOverlay.parentNode) loadingOverlay.parentNode.removeChild(loadingOverlay);
        }
    }

    /** Explain a formula in a cell */
    async _explainFormula(cell, formula) {
        const cellRef = this.getCellA1(cell.col, cell.row);
        const loadingOverlay = document.createElement('div');
        loadingOverlay.className = 'modal-overlay show';
        loadingOverlay.innerHTML = '<div class="modal"><h3>AI is thinking...</h3><div style="text-align:center;padding:20px"><div class="ai-inline-loading" style="justify-content:center">Processing your request</div></div><div class="modal-actions"><button class="ss-modal-cancel ai-loading-cancel">Cancel</button></div></div>';
        document.body.appendChild(loadingOverlay);
        loadingOverlay.querySelector('.ai-loading-cancel')?.addEventListener('click', () => {
            import('./ai.js').then(m => m.abortAI()).catch(() => {});
            if (loadingOverlay.parentNode) loadingOverlay.parentNode.removeChild(loadingOverlay);
        });
        try {
            const { aiComplete } = await this._getAIModule();
            const prompt = `Explain this spreadsheet formula step by step in plain language:\nCell: ${cellRef}\nFormula: ${formula}`;
            const result = await aiComplete('formula', prompt, {
                systemPrompt: 'You are a spreadsheet formula expert. Explain the formula step by step in plain language. Be concise.',
                maxTokens: 512
            });
            await ssAlert(`Formula explanation for ${cellRef}`, result);
        } catch (err) {
            await ssAlert('AI Error', err.message || 'AI is not available');
        } finally {
            if (loadingOverlay.parentNode) loadingOverlay.parentNode.removeChild(loadingOverlay);
        }
    }

    /** Analyze selected range with AI */
    async _analyzeWithAI() {
        const sel = this.selectionRange || {
            startCol: this.selectedCell.col,
            startRow: this.selectedCell.row,
            endCol: this.selectedCell.col,
            endRow: this.selectedCell.row
        };
        const sheet = this._sheet();
        if (!sheet) return;

        const r0 = Math.min(sel.startRow, sel.endRow);
        const r1 = Math.max(sel.startRow, sel.endRow);
        const c0 = Math.min(sel.startCol, sel.endCol);
        const c1 = Math.max(sel.startCol, sel.endCol);

        // Build CSV from selected range
        const rows = [];
        for (let r = r0; r <= r1; r++) {
            const cells = [];
            for (let c = c0; c <= c1; c++) {
                const key = `${c},${r}`;
                const cellData = sheet.cells?.[key];
                cells.push(cellData?.display ?? cellData?.value ?? '');
            }
            rows.push(cells.join(','));
        }
        const csv = rows.join('\n');

        if (!csv.trim()) {
            await ssAlert('AI Analysis', 'No data in the selected range.');
            return;
        }

        const loadingOverlay = document.createElement('div');
        loadingOverlay.className = 'modal-overlay show';
        loadingOverlay.innerHTML = '<div class="modal"><h3>AI is thinking...</h3><div style="text-align:center;padding:20px"><div class="ai-inline-loading" style="justify-content:center">Processing your request</div></div><div class="modal-actions"><button class="ss-modal-cancel ai-loading-cancel">Cancel</button></div></div>';
        document.body.appendChild(loadingOverlay);
        loadingOverlay.querySelector('.ai-loading-cancel')?.addEventListener('click', () => {
            import('./ai.js').then(m => m.abortAI()).catch(() => {});
            if (loadingOverlay.parentNode) loadingOverlay.parentNode.removeChild(loadingOverlay);
        });
        try {
            const { aiAnalyzeData } = await this._getAIModule();
            const rangeLabel = `${this.getCellA1(c0, r0)}:${this.getCellA1(c1, r1)}`;
            const result = await aiAnalyzeData(`Range ${rangeLabel}:\n${csv}`);
            await ssAlert(`AI Analysis (${rangeLabel})`, result);
        } catch (err) {
            await ssAlert('AI Error', err.message || 'AI is not available');
        } finally {
            if (loadingOverlay.parentNode) loadingOverlay.parentNode.removeChild(loadingOverlay);
        }
    }
}

// S4.4 ── Formula catalog for autocomplete ────
var FORMULA_CATALOG = [
    {name:'SUM',desc:'Add numbers',signature:'number1, [number2], ...'},
    {name:'AVERAGE',desc:'Average of numbers',signature:'number1, [number2], ...'},
    {name:'COUNT',desc:'Count numbers',signature:'value1, [value2], ...'},
    {name:'COUNTA',desc:'Count non-empty cells',signature:'value1, [value2], ...'},
    {name:'MIN',desc:'Smallest value',signature:'number1, [number2], ...'},
    {name:'MAX',desc:'Largest value',signature:'number1, [number2], ...'},
    {name:'IF',desc:'Conditional value',signature:'condition, value_if_true, [value_if_false]'},
    {name:'IFERROR',desc:'Value if error',signature:'value, value_if_error'},
    {name:'AND',desc:'All conditions true',signature:'logical1, [logical2], ...'},
    {name:'OR',desc:'Any condition true',signature:'logical1, [logical2], ...'},
    {name:'NOT',desc:'Reverse a logical value',signature:'logical'},
    {name:'VLOOKUP',desc:'Vertical lookup',signature:'lookup_value, table_array, col_index, [range_lookup]'},
    {name:'HLOOKUP',desc:'Horizontal lookup',signature:'lookup_value, table_array, row_index, [range_lookup]'},
    {name:'INDEX',desc:'Value at position',signature:'array, row_num, [col_num]'},
    {name:'MATCH',desc:'Position of value',signature:'lookup_value, lookup_array, [match_type]'},
    {name:'LEFT',desc:'Left characters',signature:'text, [num_chars]'},
    {name:'RIGHT',desc:'Right characters',signature:'text, [num_chars]'},
    {name:'MID',desc:'Middle characters',signature:'text, start_num, num_chars'},
    {name:'LEN',desc:'Length of text',signature:'text'},
    {name:'TRIM',desc:'Remove extra spaces',signature:'text'},
    {name:'UPPER',desc:'Convert to uppercase',signature:'text'},
    {name:'LOWER',desc:'Convert to lowercase',signature:'text'},
    {name:'CONCATENATE',desc:'Join text strings',signature:'text1, [text2], ...'},
    {name:'FIND',desc:'Find text (case-sensitive)',signature:'find_text, within_text, [start_num]'},
    {name:'SEARCH',desc:'Find text (case-insensitive)',signature:'find_text, within_text, [start_num]'},
    {name:'SUBSTITUTE',desc:'Replace text',signature:'text, old_text, new_text, [instance_num]'},
    {name:'TEXT',desc:'Format number as text',signature:'value, format_text'},
    {name:'VALUE',desc:'Convert text to number',signature:'text'},
    {name:'ROUND',desc:'Round to digits',signature:'number, num_digits'},
    {name:'ROUNDUP',desc:'Round up',signature:'number, num_digits'},
    {name:'ROUNDDOWN',desc:'Round down',signature:'number, num_digits'},
    {name:'CEILING',desc:'Round up to multiple',signature:'number, significance'},
    {name:'FLOOR',desc:'Round down to multiple',signature:'number, significance'},
    {name:'ABS',desc:'Absolute value',signature:'number'},
    {name:'INT',desc:'Integer part',signature:'number'},
    {name:'MOD',desc:'Remainder',signature:'number, divisor'},
    {name:'POWER',desc:'Raise to power',signature:'number, power'},
    {name:'SQRT',desc:'Square root',signature:'number'},
    {name:'SIN',desc:'Sine',signature:'number'},
    {name:'COS',desc:'Cosine',signature:'number'},
    {name:'TAN',desc:'Tangent',signature:'number'},
    {name:'ASIN',desc:'Arcsine',signature:'number'},
    {name:'ACOS',desc:'Arccosine',signature:'number'},
    {name:'ATAN',desc:'Arctangent',signature:'number'},
    {name:'LOG',desc:'Logarithm',signature:'number, [base]'},
    {name:'LOG10',desc:'Base-10 logarithm',signature:'number'},
    {name:'LN',desc:'Natural logarithm',signature:'number'},
    {name:'EXP',desc:'e raised to power',signature:'number'},
    {name:'PI',desc:'Value of Pi',signature:''},
    {name:'RAND',desc:'Random 0 to 1',signature:''},
    {name:'RANDBETWEEN',desc:'Random integer',signature:'bottom, top'},
    {name:'COUNTIF',desc:'Count matching cells',signature:'range, criteria'},
    {name:'SUMIF',desc:'Sum matching cells',signature:'range, criteria, [sum_range]'},
    {name:'AVERAGEIF',desc:'Average matching cells',signature:'range, criteria, [average_range]'},
    {name:'NOW',desc:'Current date/time',signature:''},
    {name:'TODAY',desc:'Current date',signature:''},
    {name:'DATE',desc:'Create a date',signature:'year, month, day'},
    {name:'YEAR',desc:'Year from date',signature:'serial_number'},
    {name:'MONTH',desc:'Month from date',signature:'serial_number'},
    {name:'DAY',desc:'Day from date',signature:'serial_number'},
];
