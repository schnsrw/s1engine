// ─── Spreadsheet Chart Module ──────────────────────────
// Rudra Office — Renders bar, column, line, area, pie, doughnut charts
// from selected spreadsheet data using Canvas 2D. No external libraries.

const CHART_COLORS = [
    '#4285f4', '#ea4335', '#34a853', '#fbbc04',
    '#9c27b0', '#00bcd4', '#ff7043', '#8d6e63',
    '#78909c', '#7cb342', '#c0ca33', '#f4511e',
];

const LEGEND_BOX = 10;
const LEGEND_GAP = 6;
const LEGEND_ROW_HEIGHT = 18;

/**
 * Lightweight chart renderer for the Rudra Office spreadsheet.
 * Charts are drawn onto a dedicated <canvas> inside a draggable/resizable
 * container that floats above the grid.
 */
export class ChartRenderer {
    /**
     * @param {HTMLElement} container - The .ss-chart-container element
     */
    constructor(container) {
        this.container = container;
        this.canvas = document.createElement('canvas');
        this.canvas.className = 'ss-chart-canvas';
        this.ctx = this.canvas.getContext('2d');
        container.appendChild(this.canvas);

        this._resizeObserver = new ResizeObserver(() => this._syncSize());
        this._resizeObserver.observe(container);
        this._syncSize();
    }

    /** Keep canvas pixel buffer in sync with CSS size. */
    _syncSize() {
        const dpr = window.devicePixelRatio || 1;
        const rect = this.container.getBoundingClientRect();
        // Account for title bar height (~32px) — canvas fills remaining space
        const titleEl = this.container.querySelector('.ss-chart-title');
        const titleH = titleEl ? titleEl.offsetHeight : 0;
        const w = rect.width;
        const h = rect.height - titleH;
        if (w <= 0 || h <= 0) return;
        this.canvas.width = Math.round(w * dpr);
        this.canvas.height = Math.round(h * dpr);
        this.canvas.style.width = w + 'px';
        this.canvas.style.height = h + 'px';
        this.ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
        // Re-render if we have cached params
        if (this._lastType) {
            this.render(this._lastType, this._lastData, this._lastOptions);
        }
    }

    /**
     * Render a chart.
     * @param {'column'|'bar'|'line'|'area'|'pie'|'doughnut'} type
     * @param {{ labels: string[], series: { name: string, values: number[] }[] }} data
     * @param {{ title?: string, legendPos?: string, colors?: string[] }} options
     */
    render(type, data, options) {
        this._lastType = type;
        this._lastData = data;
        this._lastOptions = options || {};
        const ctx = this.ctx;
        const w = this.canvas.width / (window.devicePixelRatio || 1);
        const h = this.canvas.height / (window.devicePixelRatio || 1);
        ctx.clearRect(0, 0, w, h);
        ctx.fillStyle = '#fff';
        ctx.fillRect(0, 0, w, h);

        if (!data || !data.labels || !data.series || data.series.length === 0) {
            ctx.fillStyle = '#5f6368';
            ctx.font = '13px Arial, sans-serif';
            ctx.textAlign = 'center';
            ctx.fillText('No data to display', w / 2, h / 2);
            return;
        }

        switch (type) {
            case 'column': this._renderColumn(ctx, w, h, data, options); break;
            case 'bar':    this._renderBar(ctx, w, h, data, options); break;
            case 'line':   this._renderLine(ctx, w, h, data, options, false); break;
            case 'area':   this._renderLine(ctx, w, h, data, options, true); break;
            case 'pie':    this._renderPie(ctx, w, h, data, options, false); break;
            case 'doughnut': this._renderPie(ctx, w, h, data, options, true); break;
            default: break;
        }
    }

    // ─── Axis helpers ─────────────────────────────

    /** Compute nice Y-axis tick values. */
    _niceScale(minVal, maxVal, ticks) {
        if (maxVal === minVal) { maxVal = minVal + 1; }
        const range = maxVal - minVal;
        const roughStep = range / (ticks || 5);
        const mag = Math.pow(10, Math.floor(Math.log10(roughStep)));
        const residual = roughStep / mag;
        let niceStep;
        if (residual <= 1.5) niceStep = 1 * mag;
        else if (residual <= 3) niceStep = 2 * mag;
        else if (residual <= 7) niceStep = 5 * mag;
        else niceStep = 10 * mag;
        const niceMin = Math.floor(minVal / niceStep) * niceStep;
        const niceMax = Math.ceil(maxVal / niceStep) * niceStep;
        const steps = [];
        for (let v = niceMin; v <= niceMax + niceStep * 0.5; v += niceStep) {
            steps.push(parseFloat(v.toPrecision(12)));
        }
        return { min: niceMin, max: niceMax, steps };
    }

    _getColors(options) {
        return (options && options.colors && options.colors.length > 0)
            ? options.colors
            : CHART_COLORS;
    }

    /** S20: Auto-determine legend position — use bottom for narrow charts. */
    _effectiveLegendPos(options, chartWidth) {
        const requested = (options && options.legendPos) || 'bottom';
        // If chart is narrow and legend was set to 'right', force to 'bottom'
        if (requested === 'right' && chartWidth < 400) return 'bottom';
        return requested;
    }

    /** Draw Y-axis labels and horizontal gridlines, returns yScale info. */
    _drawYAxis(ctx, pad, chartH, allValues, formatFn) {
        const minV = Math.min(0, ...allValues);
        const maxV = Math.max(0, ...allValues);
        const scale = this._niceScale(minV, maxV, 5);
        const range = scale.max - scale.min || 1;

        ctx.save();
        ctx.font = '11px Arial, sans-serif';
        ctx.fillStyle = '#5f6368';
        ctx.textAlign = 'right';
        ctx.textBaseline = 'middle';
        ctx.strokeStyle = '#e8eaed';
        ctx.lineWidth = 1;

        for (const tick of scale.steps) {
            const y = pad.top + chartH - ((tick - scale.min) / range) * chartH;
            const label = formatFn ? formatFn(tick) : this._formatNumber(tick);
            ctx.fillText(label, pad.left - 6, y);
            ctx.beginPath();
            ctx.moveTo(pad.left, Math.round(y) + 0.5);
            ctx.lineTo(pad.left + (ctx.canvas.width / (window.devicePixelRatio || 1)) - pad.left - pad.right, Math.round(y) + 0.5);
            ctx.stroke();
        }
        ctx.restore();

        return { min: scale.min, max: scale.max, range };
    }

    /** Draw X-axis labels under the chart area. */
    _drawXLabels(ctx, labels, pad, chartW, chartH, barGroupWidth) {
        ctx.save();
        ctx.font = '11px Arial, sans-serif';
        ctx.fillStyle = '#5f6368';
        ctx.textAlign = 'center';
        ctx.textBaseline = 'top';
        for (let i = 0; i < labels.length; i++) {
            const x = pad.left + i * barGroupWidth + barGroupWidth / 2;
            const y = pad.top + chartH + 6;
            // Truncate long labels
            let label = String(labels[i]);
            if (label.length > 12) label = label.substring(0, 11) + '\u2026';
            ctx.fillText(label, x, y);
        }
        ctx.restore();
    }

    /** Draw a legend block. For small charts (width < 300), hide the legend. */
    _drawLegend(ctx, series, colors, w, h, pad, pos) {
        // Hide legend entirely on very small charts to prevent overlap
        if (w < 300) return;
        // Force legend to bottom on narrow charts instead of right
        if (pos === 'right' && w < 400) pos = 'bottom';

        ctx.save();
        ctx.font = '11px Arial, sans-serif';
        ctx.textBaseline = 'middle';

        if (pos === 'bottom' || !pos) {
            // Horizontal legend below the chart
            let totalWidth = 0;
            const items = series.map((s, i) => {
                const tw = ctx.measureText(s.name).width;
                totalWidth += LEGEND_BOX + LEGEND_GAP + tw + 16;
                return { name: s.name, color: colors[i % colors.length], tw };
            });
            let lx = (w - totalWidth) / 2;
            const ly = h - 14;
            for (const item of items) {
                ctx.fillStyle = item.color;
                ctx.fillRect(lx, ly - LEGEND_BOX / 2, LEGEND_BOX, LEGEND_BOX);
                ctx.fillStyle = '#202124';
                ctx.textAlign = 'left';
                ctx.fillText(item.name, lx + LEGEND_BOX + LEGEND_GAP, ly);
                lx += LEGEND_BOX + LEGEND_GAP + item.tw + 16;
            }
        } else if (pos === 'right') {
            const lx = w - pad.right + 10;
            let ly = pad.top + 4;
            for (let i = 0; i < series.length; i++) {
                ctx.fillStyle = colors[i % colors.length];
                ctx.fillRect(lx, ly, LEGEND_BOX, LEGEND_BOX);
                ctx.fillStyle = '#202124';
                ctx.textAlign = 'left';
                ctx.fillText(series[i].name, lx + LEGEND_BOX + LEGEND_GAP, ly + LEGEND_BOX / 2);
                ly += LEGEND_ROW_HEIGHT;
            }
        }
        ctx.restore();
    }

    _formatNumber(v) {
        if (Math.abs(v) >= 1e6) return (v / 1e6).toFixed(1) + 'M';
        if (Math.abs(v) >= 1e3) return (v / 1e3).toFixed(1) + 'K';
        if (Number.isInteger(v)) return String(v);
        return v.toFixed(1);
    }

    // ─── Column Chart ─────────────────────────────

    _renderColumn(ctx, w, h, data, options) {
        const colors = this._getColors(options);
        const legendPos = this._effectiveLegendPos(options, w);
        const pad = { top: 16, right: 20, bottom: 46, left: 52 };
        if (legendPos === 'right') pad.right = 100;
        if (data.series.length > 1) pad.bottom = 52;

        const chartW = w - pad.left - pad.right;
        const chartH = h - pad.top - pad.bottom;

        const allValues = data.series.flatMap(s => s.values);
        const yInfo = this._drawYAxis(ctx, pad, chartH, allValues);

        const barGroupWidth = chartW / data.labels.length;
        const numSeries = data.series.length;
        const barWidth = Math.max(4, (barGroupWidth * 0.7) / numSeries);
        const groupOffset = (barGroupWidth - barWidth * numSeries) / 2;

        // Draw bars
        const zeroY = pad.top + chartH - ((0 - yInfo.min) / yInfo.range) * chartH;

        for (let si = 0; si < numSeries; si++) {
            ctx.fillStyle = colors[si % colors.length];
            const series = data.series[si];
            for (let i = 0; i < series.values.length; i++) {
                const val = series.values[i] || 0;
                const x = pad.left + i * barGroupWidth + groupOffset + si * barWidth;
                const valY = pad.top + chartH - ((val - yInfo.min) / yInfo.range) * chartH;
                const barH = zeroY - valY;
                if (barH >= 0) {
                    ctx.fillRect(x, valY, barWidth - 1, barH);
                } else {
                    ctx.fillRect(x, zeroY, barWidth - 1, -barH);
                }
            }
        }

        // X-axis labels
        this._drawXLabels(ctx, data.labels, pad, chartW, chartH, barGroupWidth);

        // Baseline
        ctx.strokeStyle = '#dadce0';
        ctx.lineWidth = 1;
        ctx.beginPath();
        ctx.moveTo(pad.left, Math.round(zeroY) + 0.5);
        ctx.lineTo(pad.left + chartW, Math.round(zeroY) + 0.5);
        ctx.stroke();

        // Legend
        if (data.series.length > 1) {
            this._drawLegend(ctx, data.series, colors, w, h, pad, legendPos);
        }
    }

    // ─── Bar Chart (horizontal) ───────────────────

    _renderBar(ctx, w, h, data, options) {
        const colors = this._getColors(options);
        const legendPos = this._effectiveLegendPos(options, w);
        const pad = { top: 16, right: 30, bottom: 40, left: 70 };
        if (legendPos === 'right') pad.right = 100;
        if (data.series.length > 1) pad.bottom = 52;

        const chartW = w - pad.left - pad.right;
        const chartH = h - pad.top - pad.bottom;

        const allValues = data.series.flatMap(s => s.values);
        const minV = Math.min(0, ...allValues);
        const maxV = Math.max(0, ...allValues);
        const scale = this._niceScale(minV, maxV, 5);
        const range = scale.max - scale.min || 1;

        // X-axis (value axis) ticks at bottom
        ctx.save();
        ctx.font = '11px Arial, sans-serif';
        ctx.fillStyle = '#5f6368';
        ctx.textAlign = 'center';
        ctx.textBaseline = 'top';
        ctx.strokeStyle = '#e8eaed';
        ctx.lineWidth = 1;
        for (const tick of scale.steps) {
            const x = pad.left + ((tick - scale.min) / range) * chartW;
            ctx.fillText(this._formatNumber(tick), x, pad.top + chartH + 6);
            ctx.beginPath();
            ctx.moveTo(Math.round(x) + 0.5, pad.top);
            ctx.lineTo(Math.round(x) + 0.5, pad.top + chartH);
            ctx.stroke();
        }
        ctx.restore();

        // Y-axis labels (category axis)
        const barGroupHeight = chartH / data.labels.length;
        const numSeries = data.series.length;
        const barHeight = Math.max(4, (barGroupHeight * 0.7) / numSeries);
        const groupOffset = (barGroupHeight - barHeight * numSeries) / 2;

        ctx.save();
        ctx.font = '11px Arial, sans-serif';
        ctx.fillStyle = '#5f6368';
        ctx.textAlign = 'right';
        ctx.textBaseline = 'middle';
        for (let i = 0; i < data.labels.length; i++) {
            let label = String(data.labels[i]);
            if (label.length > 10) label = label.substring(0, 9) + '\u2026';
            const y = pad.top + i * barGroupHeight + barGroupHeight / 2;
            ctx.fillText(label, pad.left - 6, y);
        }
        ctx.restore();

        // Draw bars
        const zeroX = pad.left + ((0 - scale.min) / range) * chartW;

        for (let si = 0; si < numSeries; si++) {
            ctx.fillStyle = colors[si % colors.length];
            const series = data.series[si];
            for (let i = 0; i < series.values.length; i++) {
                const val = series.values[i] || 0;
                const y = pad.top + i * barGroupHeight + groupOffset + si * barHeight;
                const valX = pad.left + ((val - scale.min) / range) * chartW;
                const barW = valX - zeroX;
                if (barW >= 0) {
                    ctx.fillRect(zeroX, y, barW, barHeight - 1);
                } else {
                    ctx.fillRect(valX, y, -barW, barHeight - 1);
                }
            }
        }

        // Baseline
        ctx.strokeStyle = '#dadce0';
        ctx.lineWidth = 1;
        ctx.beginPath();
        ctx.moveTo(Math.round(zeroX) + 0.5, pad.top);
        ctx.lineTo(Math.round(zeroX) + 0.5, pad.top + chartH);
        ctx.stroke();

        // Legend
        if (data.series.length > 1) {
            this._drawLegend(ctx, data.series, colors, w, h, pad, legendPos);
        }
    }

    // ─── Line / Area Chart ────────────────────────

    _renderLine(ctx, w, h, data, options, fill) {
        const colors = this._getColors(options);
        const legendPos = this._effectiveLegendPos(options, w);
        const pad = { top: 16, right: 20, bottom: 46, left: 52 };
        if (legendPos === 'right') pad.right = 100;
        if (data.series.length > 1) pad.bottom = 52;

        const chartW = w - pad.left - pad.right;
        const chartH = h - pad.top - pad.bottom;

        const allValues = data.series.flatMap(s => s.values);
        const yInfo = this._drawYAxis(ctx, pad, chartH, allValues);

        const step = data.labels.length > 1
            ? chartW / (data.labels.length - 1)
            : chartW;

        const zeroY = pad.top + chartH - ((0 - yInfo.min) / yInfo.range) * chartH;

        for (let si = 0; si < data.series.length; si++) {
            const series = data.series[si];
            const color = colors[si % colors.length];
            const points = [];
            for (let i = 0; i < series.values.length; i++) {
                const val = series.values[i] || 0;
                const x = data.labels.length > 1
                    ? pad.left + i * step
                    : pad.left + chartW / 2;
                const y = pad.top + chartH - ((val - yInfo.min) / yInfo.range) * chartH;
                points.push({ x, y });
            }

            // Area fill
            if (fill && points.length > 1) {
                ctx.save();
                ctx.globalAlpha = 0.18;
                ctx.fillStyle = color;
                ctx.beginPath();
                ctx.moveTo(points[0].x, zeroY);
                for (const p of points) ctx.lineTo(p.x, p.y);
                ctx.lineTo(points[points.length - 1].x, zeroY);
                ctx.closePath();
                ctx.fill();
                ctx.restore();
            }

            // Line
            ctx.save();
            ctx.strokeStyle = color;
            ctx.lineWidth = 2;
            ctx.lineJoin = 'round';
            ctx.beginPath();
            for (let i = 0; i < points.length; i++) {
                if (i === 0) ctx.moveTo(points[i].x, points[i].y);
                else ctx.lineTo(points[i].x, points[i].y);
            }
            ctx.stroke();
            ctx.restore();

            // Data points
            for (const p of points) {
                ctx.beginPath();
                ctx.arc(p.x, p.y, 3.5, 0, Math.PI * 2);
                ctx.fillStyle = '#fff';
                ctx.fill();
                ctx.strokeStyle = color;
                ctx.lineWidth = 2;
                ctx.stroke();
            }
        }

        // X-axis labels
        ctx.save();
        ctx.font = '11px Arial, sans-serif';
        ctx.fillStyle = '#5f6368';
        ctx.textAlign = 'center';
        ctx.textBaseline = 'top';
        for (let i = 0; i < data.labels.length; i++) {
            const x = data.labels.length > 1
                ? pad.left + i * step
                : pad.left + chartW / 2;
            let label = String(data.labels[i]);
            if (label.length > 12) label = label.substring(0, 11) + '\u2026';
            ctx.fillText(label, x, pad.top + chartH + 6);
        }
        ctx.restore();

        // Baseline
        ctx.strokeStyle = '#dadce0';
        ctx.lineWidth = 1;
        ctx.beginPath();
        ctx.moveTo(pad.left, Math.round(zeroY) + 0.5);
        ctx.lineTo(pad.left + chartW, Math.round(zeroY) + 0.5);
        ctx.stroke();

        // Legend
        if (data.series.length > 1) {
            this._drawLegend(ctx, data.series, colors, w, h, pad, legendPos);
        }
    }

    // ─── Pie / Doughnut Chart ─────────────────────

    _renderPie(ctx, w, h, data, options, doughnut) {
        const colors = this._getColors(options);
        // Use first series only for pie
        const series = data.series[0];
        const values = series.values.map(v => Math.max(0, v || 0));
        const total = values.reduce((a, b) => a + b, 0);
        if (total === 0) {
            ctx.fillStyle = '#5f6368';
            ctx.font = '13px Arial, sans-serif';
            ctx.textAlign = 'center';
            ctx.fillText('All values are zero', w / 2, h / 2);
            return;
        }

        const legendH = Math.min(data.labels.length * LEGEND_ROW_HEIGHT + 10, h * 0.3);
        const availH = h - legendH - 10;
        const cx = w / 2;
        const cy = 8 + availH / 2;
        const radius = Math.min(availH / 2 - 10, w / 2 - 30);
        const innerRadius = doughnut ? radius * 0.55 : 0;

        let startAngle = -Math.PI / 2;

        for (let i = 0; i < values.length; i++) {
            const sliceAngle = (values[i] / total) * Math.PI * 2;
            const color = colors[i % colors.length];

            ctx.save();
            ctx.fillStyle = color;
            ctx.beginPath();
            ctx.moveTo(
                cx + innerRadius * Math.cos(startAngle),
                cy + innerRadius * Math.sin(startAngle)
            );
            ctx.arc(cx, cy, radius, startAngle, startAngle + sliceAngle);
            ctx.arc(cx, cy, innerRadius, startAngle + sliceAngle, startAngle, true);
            ctx.closePath();
            ctx.fill();

            // Percentage label
            if (sliceAngle > 0.18) {
                const midAngle = startAngle + sliceAngle / 2;
                const labelR = doughnut ? (radius + innerRadius) / 2 : radius * 0.65;
                const lx = cx + labelR * Math.cos(midAngle);
                const ly = cy + labelR * Math.sin(midAngle);
                const pct = ((values[i] / total) * 100).toFixed(1) + '%';
                ctx.fillStyle = '#fff';
                ctx.font = 'bold 11px Arial, sans-serif';
                ctx.textAlign = 'center';
                ctx.textBaseline = 'middle';
                ctx.fillText(pct, lx, ly);
            }

            ctx.restore();
            startAngle += sliceAngle;
        }

        // Doughnut center label
        if (doughnut) {
            ctx.fillStyle = '#202124';
            ctx.font = 'bold 14px Arial, sans-serif';
            ctx.textAlign = 'center';
            ctx.textBaseline = 'middle';
            ctx.fillText('Total', cx, cy - 8);
            ctx.font = '12px Arial, sans-serif';
            ctx.fillText(this._formatNumber(total), cx, cy + 8);
        }

        // Legend below the pie
        const legendY = availH + 14;
        ctx.save();
        ctx.font = '11px Arial, sans-serif';
        ctx.textBaseline = 'middle';
        // Arrange in rows, wrapping
        let lx = 16;
        let ly = legendY;
        for (let i = 0; i < data.labels.length; i++) {
            const label = String(data.labels[i]);
            const tw = ctx.measureText(label).width;
            const itemW = LEGEND_BOX + LEGEND_GAP + tw + 16;
            if (lx + itemW > w - 10 && lx > 16) {
                lx = 16;
                ly += LEGEND_ROW_HEIGHT;
            }
            ctx.fillStyle = colors[i % colors.length];
            ctx.fillRect(lx, ly - LEGEND_BOX / 2, LEGEND_BOX, LEGEND_BOX);
            ctx.fillStyle = '#202124';
            ctx.textAlign = 'left';
            ctx.fillText(label, lx + LEGEND_BOX + LEGEND_GAP, ly);
            lx += itemW;
        }
        ctx.restore();
    }

    /** Cleanup. */
    destroy() {
        if (this._resizeObserver) {
            this._resizeObserver.disconnect();
            this._resizeObserver = null;
        }
        if (this.container && this.canvas.parentNode === this.container) {
            this.container.removeChild(this.canvas);
        }
    }
}


// ─── Chart Manager ────────────────────────────────────
// Manages chart creation, positioning, drag/resize, and data binding
// for a SpreadsheetView instance.

let chartIdCounter = 0;

/**
 * Parse a selected range into chart-ready data.
 * Convention: first row = category labels, first column = series names,
 * remaining cells = numeric values.
 * If there is only one row of data, treat each column as a separate value.
 */
export function parseChartData(sheet, range) {
    const { startCol, startRow, endCol, endRow } = range;
    const numDataCols = endCol - startCol;
    const numDataRows = endRow - startRow;

    if (numDataCols < 1 && numDataRows < 1) return null;

    // Determine layout heuristic:
    // If selection has a header row (first row) and header column (first col):
    //   labels = first row values (col startCol+1 .. endCol)
    //   series names = first col values (row startRow+1 .. endRow)
    //   values = inner grid

    const getVal = (c, r) => {
        const cell = sheet.getCell(c, r);
        if (!cell) return '';
        return cell.display || String(cell.value ?? '');
    };

    const getNum = (c, r) => {
        const cell = sheet.getCell(c, r);
        if (!cell) return 0;
        const v = parseFloat(cell.value);
        return isNaN(v) ? 0 : v;
    };

    // Single row selection: labels from first row, one series from values
    if (numDataRows === 0) {
        const labels = [];
        const values = [];
        for (let c = startCol; c <= endCol; c++) {
            labels.push(getVal(c, startRow));
            values.push(getNum(c, startRow));
        }
        return { labels, series: [{ name: 'Series 1', values }] };
    }

    // Single column selection: labels from cells, one series from values
    if (numDataCols === 0) {
        const labels = [];
        const values = [];
        for (let r = startRow; r <= endRow; r++) {
            labels.push('Row ' + (r + 1));
            values.push(getNum(startCol, r));
        }
        return { labels, series: [{ name: getVal(startCol, startRow) || 'Series 1', values }] };
    }

    // Multi-row, multi-col: first row = labels, first col = series names
    const labels = [];
    for (let c = startCol + 1; c <= endCol; c++) {
        labels.push(getVal(c, startRow) || ('Col ' + (c - startCol)));
    }

    const series = [];
    for (let r = startRow + 1; r <= endRow; r++) {
        const name = getVal(startCol, r) || ('Series ' + (r - startRow));
        const values = [];
        for (let c = startCol + 1; c <= endCol; c++) {
            values.push(getNum(c, r));
        }
        series.push({ name, values });
    }

    // If only header + one data row, treat columns as individual values for pie
    if (series.length === 0) {
        const values = [];
        for (let c = startCol + 1; c <= endCol; c++) {
            values.push(getNum(c, startRow));
        }
        return { labels, series: [{ name: 'Series 1', values }] };
    }

    return { labels, series };
}

/**
 * Create a floating chart container as a DOM element.
 * Returns { id, container, renderer, type, range, sheetIndex }
 */
export function createChartElement(parentEl, chartType, data, options, position) {
    const id = 'ss-chart-' + (++chartIdCounter);

    const container = document.createElement('div');
    container.className = 'ss-chart-container';
    container.id = id;
    container.style.left = (position.x || 80) + 'px';
    container.style.top = (position.y || 80) + 'px';
    container.style.width = (position.width || 480) + 'px';
    container.style.height = (position.height || 320) + 'px';

    // Title bar (draggable)
    const titleBar = document.createElement('div');
    titleBar.className = 'ss-chart-title';
    titleBar.contentEditable = 'true';
    titleBar.spellcheck = false;
    titleBar.textContent = (options && options.title) || 'Chart';
    titleBar.title = 'Click to edit chart title';
    container.appendChild(titleBar);

    // Close button
    const closeBtn = document.createElement('button');
    closeBtn.className = 'ss-chart-close';
    closeBtn.textContent = '\u00D7';
    closeBtn.title = 'Remove chart';
    container.appendChild(closeBtn);

    parentEl.appendChild(container);

    const renderer = new ChartRenderer(container);
    renderer.render(chartType, data, options);

    // Make draggable by title bar
    _makeDraggable(container, titleBar);

    const chartObj = {
        id,
        container,
        renderer,
        type: chartType,
        data,
        options: options || {},
        range: position.range || null,
        sheetIndex: position.sheetIndex != null ? position.sheetIndex : 0,
    };

    // Close handler
    closeBtn.addEventListener('click', () => {
        renderer.destroy();
        container.remove();
        if (chartObj.onRemove) chartObj.onRemove(chartObj);
    });

    // Title edit
    titleBar.addEventListener('blur', () => {
        chartObj.options.title = titleBar.textContent;
    });
    titleBar.addEventListener('keydown', (e) => {
        if (e.key === 'Enter') {
            e.preventDefault();
            titleBar.blur();
        }
    });

    return chartObj;
}

function _makeDraggable(container, handle) {
    let dragging = false;
    let startX = 0, startY = 0;
    let origLeft = 0, origTop = 0;

    handle.addEventListener('mousedown', (e) => {
        // Don't drag while editing title text
        if (handle.isContentEditable && window.getSelection().toString().length > 0) return;
        dragging = true;
        startX = e.clientX;
        startY = e.clientY;
        origLeft = parseInt(container.style.left, 10) || 0;
        origTop = parseInt(container.style.top, 10) || 0;
        e.preventDefault();
    });

    document.addEventListener('mousemove', (e) => {
        if (!dragging) return;
        const dx = e.clientX - startX;
        const dy = e.clientY - startY;
        container.style.left = Math.max(0, origLeft + dx) + 'px';
        container.style.top = Math.max(0, origTop + dy) + 'px';
    });

    document.addEventListener('mouseup', () => {
        dragging = false;
    });
}
