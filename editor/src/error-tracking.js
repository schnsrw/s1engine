// s1 Editor — Lightweight global error tracking
// Captures uncaught errors and unhandled promise rejections, surfaces them in the status bar.

let _lastError = null;
let _errorCount = 0;

/**
 * Record an error and update the status bar indicator.
 * @param {*} error - The error object, string, or rejection reason
 */
export function recordError(error) {
  _lastError = error;
  _errorCount++;
  const indicator = document.getElementById('errorIndicator');
  if (indicator) {
    indicator.style.display = 'inline-flex';
    indicator.title = `${_errorCount} error(s) occurred \u2014 click for details`;
    const badge = indicator.querySelector('.error-badge');
    if (badge) badge.textContent = _errorCount;
  }
}

/**
 * Get the last error details.
 * @returns {{ error: *, count: number }}
 */
export function getLastError() {
  return { error: _lastError, count: _errorCount };
}

/**
 * Clear all recorded errors and hide the status bar indicator.
 */
export function clearErrors() {
  _lastError = null;
  _errorCount = 0;
  const indicator = document.getElementById('errorIndicator');
  if (indicator) indicator.style.display = 'none';
}
