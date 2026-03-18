// s1 Editor — Analytics stubs
//
// These are no-op stubs. Replace with your own analytics provider
// (e.g., Plausible, PostHog, Mixpanel) by implementing trackEvent().
//
// Example:
//   export function trackEvent(category, action) {
//     posthog.capture(`${category}:${action}`);
//   }

/**
 * Track a feature usage event.
 * @param {string} _category - Event category (e.g., 'toolbar', 'insert')
 * @param {string} _action - Specific action (e.g., 'bold', 'save-docx')
 */
export function trackEvent(_category, _action) {
  // No-op — replace with your analytics provider
}

/**
 * Get usage statistics.
 * @returns {Object} Empty object (stub)
 */
export function getStats() {
  return {};
}

/**
 * Get session duration in seconds.
 * @returns {number}
 */
export function getSessionDuration() {
  return 0;
}

/**
 * Clear analytics data.
 */
export function clearStats() {
  // No-op
}
