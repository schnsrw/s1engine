// Rudra Office — AI Inline Suggestion Engine
// Inline prompt at cursor, streaming diff rendering, accept/reject via document ops.

import { state, $ } from './state.js';
import { aiComplete, abortAI } from './ai.js';
import { detectContext, buildContextPrompt } from './ai-panel.js';

let _promptEl, _promptInput, _suggestionEl, _diffEl;
let _lastAction = null;
let _lastPrompt = '';
let _diffRAF = null;
let _lastRequestTime = 0; // rate limiting cooldown (Enhancement 39)

// ── Word-level diff ─────────────────────────────────

function computeWordDiff(original, suggested) {
  const origWords = original.split(/(\s+)/);
  const sugWords = suggested.split(/(\s+)/);

  // Guard: fall back to simple before/after for very large texts
  if (origWords.length * sugWords.length > 100000) {
    return [
      { type: 'del', text: original },
      { type: 'ins', text: suggested }
    ];
  }

  // Simple LCS-based diff on words
  const m = origWords.length, n = sugWords.length;
  const dp = Array.from({ length: m + 1 }, () => new Uint32Array(n + 1));

  for (let i = 1; i <= m; i++) {
    for (let j = 1; j <= n; j++) {
      if (origWords[i - 1] === sugWords[j - 1]) {
        dp[i][j] = dp[i - 1][j - 1] + 1;
      } else {
        dp[i][j] = Math.max(dp[i - 1][j], dp[i][j - 1]);
      }
    }
  }

  // Backtrack to build diff
  let i = m, j = n;
  const ops = [];
  while (i > 0 || j > 0) {
    if (i > 0 && j > 0 && origWords[i - 1] === sugWords[j - 1]) {
      ops.push({ type: 'keep', text: origWords[i - 1] });
      i--; j--;
    } else if (j > 0 && (i === 0 || dp[i][j - 1] >= dp[i - 1][j])) {
      ops.push({ type: 'ins', text: sugWords[j - 1] });
      j--;
    } else {
      ops.push({ type: 'del', text: origWords[i - 1] });
      i--;
    }
  }
  ops.reverse();
  return ops;
}

function renderDiffHTML(original, suggested) {
  if (!original && suggested) {
    return `<span class="ai-diff-ins">${escapeHTML(suggested)}</span>`;
  }
  const ops = computeWordDiff(original, suggested);
  return ops.map(op => {
    const text = escapeHTML(op.text);
    if (op.type === 'del') return `<span class="ai-diff-del">${text}</span>`;
    if (op.type === 'ins') return `<span class="ai-diff-ins">${text}</span>`;
    return text;
  }).join('');
}

function escapeHTML(s) {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

// ── Inline AI Prompt (at cursor) ────────────────────

export function showInlineAIPrompt() {
  if (!state.aiAvailable) return;
  if (state.readOnlyMode) return;

  _promptEl = $('aiInlinePrompt');
  _promptInput = $('aiInlinePromptInput');
  if (!_promptEl || !_promptInput) return;

  // Position at cursor using fixed positioning with viewport coords (Bug 3 fix)
  const sel = window.getSelection();
  if (sel && sel.rangeCount > 0) {
    const range = sel.getRangeAt(0);
    const rect = range.getBoundingClientRect();
    _promptEl.style.top = rect.bottom + 4 + 'px';
    _promptEl.style.left = Math.max(8, rect.left) + 'px';
  }

  _promptEl.style.display = 'block';
  _promptInput.value = '';
  _promptInput.focus();
}

export function hideInlinePrompt() {
  if (_promptEl) _promptEl.style.display = 'none';
}

// ── Inline Suggestion (diff view) ───────────────────

function positionSuggestionNearNode(nodeId) {
  _suggestionEl = $('aiInlineSuggestion');
  if (!_suggestionEl) return;

  const canvas = $('editorCanvas');
  if (!canvas) return;

  // Position near the target node using fixed positioning with viewport coords (Bug 3 fix)
  const nodeEl = canvas.querySelector(`[data-node-id="${nodeId}"]`);
  if (nodeEl) {
    const rect = nodeEl.getBoundingClientRect();
    _suggestionEl.style.top = rect.bottom + 4 + 'px';
    _suggestionEl.style.left = Math.max(8, rect.left) + 'px';
  }
}

export function showInlineSuggestion(nodeId, original, suggested, action, startOffset, multiNodeInfo) {
  _suggestionEl = $('aiInlineSuggestion');
  _diffEl = $('aiInlineDiff');
  if (!_suggestionEl || !_diffEl) return;

  state.aiInlineSuggestion = { nodeId, original, suggested, action, startOffset: startOffset ?? 0, multiNodeInfo: multiNodeInfo || null };

  // Render word-level diff
  _diffEl.innerHTML = renderDiffHTML(original, suggested);

  positionSuggestionNearNode(nodeId);
  _suggestionEl.style.display = 'block';
}

export function hideInlineSuggestion() {
  if (_suggestionEl) _suggestionEl.style.display = 'none';
  state.aiInlineSuggestion = null;
}

// ── Core AI Action Trigger ──────────────────────────

export async function triggerAIAction(action, customPrompt) {
  if (state.aiGenerating) return;
  if (!state.aiAvailable) return;
  // Rate limiting: 1 second cooldown between requests (Enhancement 39)
  if (Date.now() - _lastRequestTime < 1000) return;

  const sel = window.getSelection();
  let selectedText = sel ? sel.toString().trim() : '';
  let targetNodeId = null;

  // Find the target paragraph node
  if (sel && sel.rangeCount > 0) {
    let node = sel.anchorNode;
    while (node && node !== $('editorCanvas')) {
      if (node.nodeType === 1 && node.dataset?.nodeId) {
        targetNodeId = node.dataset.nodeId;
        break;
      }
      node = node.parentNode;
    }
  }

  // Detect multi-paragraph selection (Bug #35)
  let multiNodeInfo = null;
  if (sel && sel.rangeCount > 0 && targetNodeId) {
    const range = sel.getRangeAt(0);
    const canvas = $('editorCanvas');
    if (canvas) {
      const allParas = canvas.querySelectorAll('[data-node-id]');
      const selectedParas = [];
      for (const para of allParas) {
        if (range.intersectsNode(para)) {
          selectedParas.push({
            nodeId: para.dataset.nodeId,
            text: para.textContent || ''
          });
        }
      }
      if (selectedParas.length > 1) {
        multiNodeInfo = selectedParas;
      }
    }
  }

  // If no selection, use the entire paragraph text
  if (!selectedText && targetNodeId) {
    const el = $('editorCanvas')?.querySelector(`[data-node-id="${targetNodeId}"]`);
    if (el) selectedText = el.textContent?.trim() || '';
  }

  if (!selectedText) return;

  // Capture the offset of the selected text within the paragraph (Bug 2 fix)
  let selStartOffset = 0;
  if (targetNodeId) {
    const el = $('editorCanvas')?.querySelector(`[data-node-id="${targetNodeId}"]`);
    const paraText = el?.textContent || '';
    const idx = paraText.indexOf(selectedText);
    selStartOffset = idx >= 0 ? idx : 0;
  }

  const context = detectContext();
  const actionPrompts = {
    improve: `Improve this text for clarity, flow, and professionalism. Return only the improved text:\n\n${selectedText}`,
    shorter: `Make this more concise while keeping the key points. Return only the shortened text:\n\n${selectedText}`,
    longer: `Expand on this with more detail and supporting points. Return only the expanded text:\n\n${selectedText}`,
    grammar: `Fix any grammar, spelling, and punctuation issues. Return only the corrected text:\n\n${selectedText}`,
    translate: `Detect the language of this text. If it's English, translate to Spanish. If it's any other language, translate to English. Return only the translated text, nothing else:\n\n${selectedText}`,
    custom: customPrompt ? `${customPrompt}\n\nText:\n${selectedText}` : selectedText,
  };

  const prompt = actionPrompts[action] || actionPrompts.custom;
  const mode = action === 'grammar' ? 'writer' : 'writer';
  const systemPrompt = buildContextPrompt(mode, context);

  _lastAction = action;
  _lastPrompt = customPrompt || '';

  state.aiGenerating = true;

  // Show loading indicator
  _suggestionEl = $('aiInlineSuggestion');
  _diffEl = $('aiInlineDiff');
  if (_suggestionEl && _diffEl) {
    _diffEl.innerHTML = '<span class="ai-inline-loading">AI is thinking...</span>';
    // Position near the target node
    if (targetNodeId) positionSuggestionNearNode(targetNodeId);
    _suggestionEl.style.display = 'block';
  }

  try {
    let fullText = '';

    const result = await aiComplete(mode, prompt, {
      systemPrompt,
      context: state.aiConversation.slice(-12),
      noAutoAbort: true,
      onChunk(chunk) {
        fullText += chunk;
        if (targetNodeId && _diffEl && !_diffRAF) {
          _diffRAF = requestAnimationFrame(() => {
            _diffEl.innerHTML = renderDiffHTML(selectedText, fullText.trim()) + '<span class="ai-stream-cursor"></span>';
            if (_suggestionEl) _suggestionEl.style.display = 'block';
            _diffRAF = null;
          });
        }
      }
    });

    fullText = (fullText || result).trim();

    if (fullText && targetNodeId) {
      showInlineSuggestion(targetNodeId, selectedText, fullText, action, selStartOffset, multiNodeInfo);
    }

    // Add to conversation context
    state.aiConversation.push(
      { role: 'user', content: `[${action}] ${selectedText.slice(0, 150)}` },
      { role: 'assistant', content: fullText.slice(0, 200) }
    );
    if (state.aiConversation.length > 12) {
      state.aiConversation = state.aiConversation.slice(-12);
    }

  } catch (err) {
    if (err.name !== 'AbortError') {
      console.error('[ai-inline] Error:', err);
    }
    hideInlineSuggestion();
  } finally {
    state.aiGenerating = false;
    _lastRequestTime = Date.now(); // rate limiting cooldown (Enhancement 39)
  }
}

// ── Accept / Reject ─────────────────────────────────

export function acceptSuggestion() {
  const suggestion = state.aiInlineSuggestion;
  if (!suggestion) return;

  const { nodeId, original, suggested, startOffset, multiNodeInfo } = suggestion;
  const doc = state.doc;
  if (!doc) { hideInlineSuggestion(); return; }

  try {
    if (multiNodeInfo && multiNodeInfo.length > 1) {
      // ── Multi-paragraph accept (Bug #35) ──────────────────────
      // Split the AI-suggested text into paragraph-sized chunks by newlines.
      // Apply each chunk to its corresponding paragraph via set_paragraph_text.
      const suggestedParas = suggested.split('\n');

      for (let i = 0; i < multiNodeInfo.length; i++) {
        const para = multiNodeInfo[i];
        const newText = i < suggestedParas.length ? suggestedParas[i] : '';
        try {
          doc.set_paragraph_text(para.nodeId, newText);
          // Update DOM to reflect the change
          const el = $('editorCanvas')?.querySelector(`[data-node-id="${para.nodeId}"]`);
          if (el) el.textContent = newText;
        } catch (e) {
          console.warn('[ai-inline] Multi-para replace error for node', para.nodeId, e);
        }
      }

      // Mark dirty and record undo
      state.dirty = true;
      state.undoHistory = state.undoHistory || [];
      state.undoHistory.unshift({ label: 'AI suggestion accepted (multi-paragraph)', timestamp: Date.now() });
      if (state.undoHistory.length > 50) state.undoHistory.length = 50;

    } else {
      // ── Single paragraph accept (existing logic) ──────────────
      // Find the DOM element
      const el = $('editorCanvas')?.querySelector(`[data-node-id="${nodeId}"]`);
      if (!el) { hideInlineSuggestion(); return; }

      // Use stored startOffset (captured at action trigger time) to avoid
      // indexOf failures with duplicate text (Bug 2 fix).
      const offset = startOffset ?? 0;

      // Replace via WASM replace_text which preserves formatting of untouched runs (Bug 1 fix).
      // Signature: replace_text(node_id, offset, length, replacement)
      try {
        doc.replace_text(nodeId, offset, original.length, suggested);
      } catch (wasmErr) {
        console.warn('[ai-inline] WASM replace_text failed:', wasmErr);
      }

      // Update DOM to reflect the change
      const fullText = el.textContent || '';
      const before = fullText.slice(0, offset);
      const after = fullText.slice(offset + original.length);
      el.textContent = before + suggested + after;

      // Mark dirty and record undo (Enhancement 34)
      state.dirty = true;
      state.undoHistory = state.undoHistory || [];
      state.undoHistory.unshift({ label: 'AI suggestion accepted', timestamp: Date.now() });
      if (state.undoHistory.length > 50) state.undoHistory.length = 50;

      // Restore cursor to end of replaced text (Enhancement 37)
      try {
        const sel = window.getSelection();
        const range = document.createRange();
        const textNode = el.firstChild;
        if (textNode) {
          const endPos = offset + suggested.length;
          const safePos = Math.min(endPos, textNode.textContent?.length || 0);
          range.setStart(textNode, safePos);
          range.collapse(true);
          sel.removeAllRanges();
          sel.addRange(range);
        }
      } catch (_) { /* cursor restore is non-critical */ }

      // Collaboration: WASM replace_text integrates with the CRDT model,
      // so collab peers will receive the change via the next sync cycle.
    }

  } catch (err) {
    console.error('[ai-inline] Accept error:', err);
  }

  hideInlineSuggestion();
}

export function rejectSuggestion() {
  hideInlineSuggestion();
}

export function retrySuggestion() {
  const suggestion = state.aiInlineSuggestion;
  if (!suggestion) return;

  hideInlineSuggestion();

  // Re-trigger with the same action
  if (_lastAction) {
    triggerAIAction(_lastAction, _lastPrompt || undefined);
  }
}

// ── Initialization ──────────────────────────────────

export function initAIInline() {
  const cfg = window.S1_CONFIG || {};
  if (!cfg.enableAI) return;

  _promptEl = $('aiInlinePrompt');
  _promptInput = $('aiInlinePromptInput');
  _suggestionEl = $('aiInlineSuggestion');
  _diffEl = $('aiInlineDiff');

  if (!_promptEl) return;

  // Inline prompt submit
  _promptInput?.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      const text = _promptInput.value.trim();
      if (text) {
        hideInlinePrompt();
        triggerAIAction('custom', text);
      }
    }
    if (e.key === 'Escape') {
      e.preventDefault();
      hideInlinePrompt();
    }
  });

  // Suggestion accept/reject buttons
  $('aiInlineAccept')?.addEventListener('click', acceptSuggestion);
  $('aiInlineReject')?.addEventListener('click', rejectSuggestion);
  $('aiInlineRetry')?.addEventListener('click', retrySuggestion);

  // Keyboard: Escape=reject when suggestion is visible
  document.addEventListener('keydown', (e) => {
    if (!state.aiInlineSuggestion) return;
    if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA' || e.target.isContentEditable) return;

    if (e.key === 'Escape') {
      e.preventDefault();
      rejectSuggestion();
    }
  });

  // Click outside inline prompt or suggestion — single combined handler
  document.addEventListener('mousedown', (e) => {
    // Close inline prompt if clicking outside it
    if (_promptEl?.style.display === 'block' && !_promptEl.contains(e.target)) {
      hideInlinePrompt();
    }
    // Reject suggestion if clicking outside it
    if (state.aiInlineSuggestion && _suggestionEl && !_suggestionEl.contains(e.target)) {
      rejectSuggestion();
    }
  });
}
