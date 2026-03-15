// s1 Editor — Lightweight, privacy-respecting usage analytics
// All data stays local in localStorage. No network requests.

const STORAGE_KEY = 's1-analytics';
let events = {};
let sessionStart = Date.now();

// Load persisted events from localStorage
try {
  const stored = localStorage.getItem(STORAGE_KEY);
  if (stored) {
    events = JSON.parse(stored);
  }
} catch (_) {
  events = {};
}

/**
 * Track a feature usage event. Increments the counter for the given category:action key.
 * @param {string} category - Event category (e.g., 'toolbar', 'menu', 'export')
 * @param {string} action - Specific action (e.g., 'bold', 'save-docx', 'insert-table')
 */
export function trackEvent(category, action) {
  const key = `${category}:${action}`;
  events[key] = (events[key] || 0) + 1;
  persist();
}

/**
 * Get a copy of all tracked event counts.
 * @returns {Object} Map of "category:action" to count
 */
export function getStats() {
  return { ...events };
}

/**
 * Get session duration in seconds.
 * @returns {number}
 */
export function getSessionDuration() {
  return Math.floor((Date.now() - sessionStart) / 1000);
}

/**
 * Clear all tracked analytics data.
 */
export function clearStats() {
  events = {};
  persist();
}

/** Persist current event counts to localStorage */
function persist() {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(events));
  } catch (_) {
    // localStorage may be full or unavailable — silently ignore
  }
}
