// Rudra Office — AI Panel Controller
// Context-aware AI assistant with floating selection toolbar,
// proactive hints, and side panel chat.

import { state, $ } from './state.js';
import { initAI, checkAIHealth, aiComplete, abortAI } from './ai.js';
import { getSelectionInfo } from './selection.js';

let _panel, _messages, _input, _sendBtn, _stopBtn, _status, _modeSelect;
let _floatingBar, _contextChip;
let _selectionTimer = null;
let _lastSelectedText = '';
let _hintTimer = null;
let _lastFloatingMode = null;
let _aiInlineModule = null;   // cached dynamic import for ai-inline.js (Bug 30)
let _lastHealthCheck = 0;     // timestamp of last health check (Bug 32)
let _lastHealthResult = false; // cached health check result (Bug 32)
let _savedSelectionRange = null; // saved selection range for Replace action (Bug 60)

const AI_MODE_OPTIONS = [
  { value: 'writer', label: 'Write & rewrite' },
  { value: 'grammar', label: 'Grammar & clarity' },
  { value: 'summarize', label: 'Summarize' },
  { value: 'translate', label: 'Translate' },
  { value: 'formula', label: 'Spreadsheet formula' },
  { value: 'data_analysis', label: 'Data analysis' },
];

// ── Context Detection (exported for ai-inline.js) ────

/** Detect what the user is currently working on */
export function detectContext() {
  const view = state.currentView;
  const format = state.currentFormat || '';

  if (view === 'pdf') {
    return { mode: 'viewer', type: 'pdf', label: 'PDF Viewer' };
  }

  if (state.spreadsheetView) {
    return { mode: 'spreadsheet', type: format.toLowerCase(), label: 'Spreadsheet' };
  }

  // Document editor
  const sel = window.getSelection();
  let paraContext = '';
  let nodeType = 'paragraph';

  if (sel && sel.rangeCount > 0) {
    let node = sel.anchorNode;
    while (node && node !== $('pageContainer')) {
      if (node.nodeType === 1) {
        const tag = node.tagName?.toLowerCase();
        if (/^h[1-6]$/.test(tag)) { nodeType = 'heading'; break; }
        if (tag === 'table' || tag === 'td' || tag === 'th') { nodeType = 'table'; break; }
        if (tag === 'li') { nodeType = 'list'; break; }
        if (node.classList?.contains('equation-inline') || node.classList?.contains('equation-block')) { nodeType = 'equation'; break; }
      }
      node = node.parentNode;
    }

    const anchorPara = sel.anchorNode?.parentElement?.closest?.('[data-node-id]');
    if (anchorPara) {
      paraContext = anchorPara.textContent?.slice(0, 300) || '';
    }
  }

  return {
    mode: 'document',
    type: format.toLowerCase() || 'docx',
    label: format || 'Document',
    nodeType,
    paraContext
  };
}

/** Build a context-enriched system prompt (exported for ai-inline.js) */
export function buildContextPrompt(mode, context) {
  let base = '';

  if (context.mode === 'spreadsheet') {
    base = 'You are an AI assistant embedded in Rudra Office Sheets. The user is working on a spreadsheet. ';
    if (mode === 'formula') {
      base += 'Help them write spreadsheet formulas. Use standard A1 notation. Support SUM, AVERAGE, VLOOKUP, IF, COUNTIF, INDEX/MATCH, and other standard functions. Return only the formula unless asked for explanation.';
    } else if (mode === 'data_analysis') {
      base += 'Analyze the data they provide. Identify trends, outliers, and provide summary statistics. Be concise and use numbers.';
    } else {
      base += 'Help them with their spreadsheet content. Be concise and direct.';
    }
  } else if (context.mode === 'document') {
    base = 'You are an AI assistant embedded in Rudra Office, a document editor. ';
    if (context.nodeType === 'heading') {
      base += 'The user is working on a heading. ';
    } else if (context.nodeType === 'table') {
      base += 'The user is working within a table. ';
    } else if (context.nodeType === 'list') {
      base += 'The user is working on a list. ';
    } else if (context.nodeType === 'equation') {
      base += 'The user is working with a mathematical equation. ';
    }

    if (context.paraContext) {
      base += `\n\nCurrent paragraph context: "${context.paraContext.slice(0, 200)}"\n\n`;
    }

    if (mode === 'writer') {
      base += 'Help them write, edit, and improve their content. Be concise. Return only the improved text unless asked for explanation.';
    } else if (mode === 'grammar') {
      base += 'Check for grammar, spelling, punctuation, and clarity. Return a JSON array: [{"original":"...","corrected":"...","reason":"..."}]. Empty array if no issues.';
    } else if (mode === 'summarize') {
      base += 'Summarize the content clearly. Bullet points for key takeaways. Under 200 words.';
    } else if (mode === 'translate') {
      base += 'Translate to the requested language. Preserve formatting and tone.';
    } else {
      base += 'Help them with their document. Be concise and direct.';
    }
  } else {
    base = 'You are an AI assistant in Rudra Office. Help the user with their task. Be concise.';
  }

  return base;
}

// ── Selection Handling ─────────────────────────────

function getSelectedText() {
  if (state.spreadsheetView) {
    return getSpreadsheetSelection();
  }

  const sel = window.getSelection();
  if (sel && sel.toString().trim()) return sel.toString().trim();

  try {
    const info = getSelectionInfo();
    if (info && info.text) return info.text;
  } catch { /* ignore */ }

  return '';
}

function getSpreadsheetSelection() {
  const sv = state.spreadsheetView;
  if (!sv || !sv.selection) return '';

  try {
    const { startRow, startCol, endRow, endCol } = sv.selection;
    const sheet = sv.workbook?.sheets?.[sv.activeSheet];
    if (!sheet) return '';

    const rows = [];
    const r0 = Math.min(startRow, endRow);
    const r1 = Math.max(startRow, endRow);
    const c0 = Math.min(startCol, endCol);
    const c1 = Math.max(startCol, endCol);

    for (let r = r0; r <= r1; r++) {
      const cells = [];
      for (let c = c0; c <= c1; c++) {
        const key = `${c},${r}`;
        const cell = sheet.cells?.[key];
        cells.push(cell?.display ?? cell?.value ?? '');
      }
      rows.push(cells.join('\t'));
    }
    return rows.join('\n');
  } catch { return ''; }
}

// ── Floating Selection Toolbar ─────────────────────

function showFloatingBar(selectedText) {
  if (!_floatingBar || !state.aiAvailable) return;
  if (state.aiPanelOpen) return;
  if (state.readOnlyMode) return;

  const sel = window.getSelection();
  if (!sel || sel.isCollapsed || !sel.rangeCount) return;

  const range = sel.getRangeAt(0);
  const rect = range.getBoundingClientRect();
  if (!rect || rect.width === 0) return;

  // Update buttons based on context (only rebuild when mode changes)
  const context = detectContext();
  const actionsContainer = $('aiFloatingActions');
  if (actionsContainer && context.mode !== _lastFloatingMode) {
    _lastFloatingMode = context.mode;
    if (context.mode === 'spreadsheet') {
      actionsContainer.innerHTML =
        '<button class="ai-floating-btn" data-ai-action="analyze" title="Analyze selected data">Analyze</button>' +
        '<button class="ai-floating-btn" data-ai-action="explain" title="Explain formula">Explain</button>' +
        '<button class="ai-floating-btn" data-ai-action="formula" title="Suggest formula">Formula</button>';
    } else {
      actionsContainer.innerHTML =
        '<button class="ai-floating-btn" data-ai-action="improve" title="Improve writing">Improve</button>' +
        '<button class="ai-floating-btn" data-ai-action="shorter" title="Make more concise">Shorten</button>' +
        '<button class="ai-floating-btn" data-ai-action="longer" title="Expand with more detail">Expand</button>' +
        '<button class="ai-floating-btn" data-ai-action="grammar" title="Fix grammar and spelling">Grammar</button>' +
        '<button class="ai-floating-btn" data-ai-action="translate" title="Translate text">Translate</button>';
    }
  }

  // Position above the selection, centered
  const barW = _floatingBar.offsetWidth || 300;
  let left = rect.left + rect.width / 2 - barW / 2;
  let top = rect.top - 44;

  left = Math.max(8, Math.min(left, window.innerWidth - barW - 8));
  if (top < 8) top = rect.bottom + 8;

  _floatingBar.style.left = left + 'px';
  _floatingBar.style.top = top + 'px';
  _floatingBar.style.display = 'flex';
  state.aiFloatingBarVisible = true;

  // Update context chip
  if (_contextChip) {
    const words = selectedText.split(/\s+/).length;
    _contextChip.textContent = `${words} word${words !== 1 ? 's' : ''} selected`;
  }

  // Proactive hint: first text selection
  if (!state.aiHintsShown.selectionHint && selectedText.length > 20) {
    const dismissed = localStorage.getItem('rudra_ai_hint_selection');
    if (!dismissed) {
      state.aiHintsShown.selectionHint = true;
      showHint('Select text and use the AI toolbar above to improve your writing');
    }
  }
}

function hideFloatingBar() {
  if (_floatingBar) _floatingBar.style.display = 'none';
  state.aiFloatingBarVisible = false;
}

/** Show a small language picker dropdown near the floating bar.
 *  Returns the selected language string, or null if cancelled. */
function showLanguagePicker(anchorEl) {
  return new Promise(resolve => {
    const languages = ['Spanish', 'French', 'German', 'Chinese', 'Japanese', 'Hindi', 'Arabic', 'Portuguese', 'Russian', 'Korean', 'Italian', 'Dutch', 'Turkish', 'Polish', 'Swedish'];
    const overlay = document.createElement('div');
    overlay.style.cssText = 'position:fixed;inset:0;z-index:300;';
    const dropdown = document.createElement('div');
    dropdown.className = 'ai-lang-picker';
    dropdown.style.cssText = 'position:fixed;z-index:301;background:var(--bg-surface,#fff);border:1px solid var(--border-color,#dadce0);border-radius:8px;box-shadow:0 4px 16px rgba(0,0,0,.15);padding:4px 0;max-height:260px;overflow-y:auto;min-width:160px;';
    const title = document.createElement('div');
    title.textContent = 'Translate to...';
    title.style.cssText = 'padding:6px 12px;font-size:11px;color:var(--text-muted,#5f6368);font-weight:500;border-bottom:1px solid var(--border-color,#dadce0);margin-bottom:2px;';
    dropdown.appendChild(title);
    languages.forEach(lang => {
      const btn = document.createElement('button');
      btn.textContent = lang;
      btn.style.cssText = 'display:block;width:100%;text-align:left;padding:6px 12px;border:none;background:none;font-size:13px;color:var(--text-primary,#202124);cursor:pointer;font-family:var(--font-ui);';
      btn.addEventListener('mouseenter', () => { btn.style.background = 'var(--accent-light,#e8f0fe)'; });
      btn.addEventListener('mouseleave', () => { btn.style.background = 'none'; });
      btn.addEventListener('click', (e) => {
        e.stopPropagation();
        cleanup();
        resolve(lang);
      });
      dropdown.appendChild(btn);
    });
    // Position near the anchor element (floating bar or button)
    if (anchorEl) {
      const rect = anchorEl.getBoundingClientRect();
      let top = rect.bottom + 4;
      let left = rect.left;
      if (top + 260 > window.innerHeight) top = Math.max(8, rect.top - 260);
      if (left + 160 > window.innerWidth) left = Math.max(8, window.innerWidth - 168);
      dropdown.style.top = top + 'px';
      dropdown.style.left = left + 'px';
    } else {
      dropdown.style.top = '50%';
      dropdown.style.left = '50%';
      dropdown.style.transform = 'translate(-50%, -50%)';
    }
    const cleanup = () => {
      overlay.remove();
      dropdown.remove();
    };
    overlay.addEventListener('click', () => { cleanup(); resolve(null); });
    document.addEventListener('keydown', function escHandler(e) {
      if (e.key === 'Escape') {
        e.preventDefault();
        document.removeEventListener('keydown', escHandler);
        cleanup();
        resolve(null);
      }
    });
    document.body.appendChild(overlay);
    document.body.appendChild(dropdown);
  });
}

/** Handle quick action from floating bar — delegates to ai-inline.js */
async function floatingAction(action) {
  const text = getSelectedText();
  if (!text) return;

  // For translate, ask for target language first
  if (action === 'translate') {
    const anchorEl = _floatingBar || null;
    const lang = await showLanguagePicker(anchorEl);
    if (!lang) return; // cancelled
    hideFloatingBar();
    try {
      if (!_aiInlineModule) _aiInlineModule = await import('./ai-inline.js');
      _aiInlineModule.triggerAIAction(action, null, lang);
    } catch (err) {
      console.warn('[ai-panel] Inline module unavailable for translate:', err);
    }
    return;
  }

  hideFloatingBar();

  // Delegate to inline suggestion system for visual diff (cached import — Bug 30)
  try {
    if (!_aiInlineModule) _aiInlineModule = await import('./ai-inline.js');
    _aiInlineModule.triggerAIAction(action);
  } catch (err) {
    console.warn('[ai-panel] Inline module unavailable, falling back to panel:', err);
    // Fallback: open panel and submit
    if (!state.aiPanelOpen) toggleAIPanel();
    await new Promise(r => setTimeout(r, 300));
    const actionMap = {
      improve: { mode: 'writer', prompt: `Improve this text for clarity and flow:\n\n${text}` },
      grammar: { mode: 'grammar', prompt: text },
      summarize: { mode: 'summarize', prompt: text },
      shorter: { mode: 'writer', prompt: `Make this more concise while keeping the key points:\n\n${text}` },
      longer: { mode: 'writer', prompt: `Expand on this with more detail:\n\n${text}` },
      translate: { mode: 'translate', prompt: `Translate to English:\n\n${text}` },
      explain: { mode: 'writer', prompt: `Explain this in simple terms:\n\n${text}` },
      formula: { mode: 'formula', prompt: text },
      analyze: { mode: 'data_analysis', prompt: `Analyze this data:\n\n${text}` },
    };
    const config = actionMap[action];
    if (config) {
      _modeSelect.value = config.mode;
      addMessage('user', `[${action}] ${text.slice(0, 80)}${text.length > 80 ? '...' : ''}`);
      await runAI(config.mode, config.prompt);
    }
  }
}

// ── Proactive Hints ────────────────────────────────

function showHint(message) {
  if (!state.aiAvailable) return;

  const hint = $('aiHintBubble');
  if (!hint) return;

  hint.textContent = message;
  hint.style.display = 'block';
  hint.style.opacity = '0';
  requestAnimationFrame(() => { hint.style.opacity = '1'; });

  clearTimeout(_hintTimer);
  _hintTimer = setTimeout(() => dismissHint(), 6000);
}

function dismissHint() {
  const hint = $('aiHintBubble');
  if (hint) {
    hint.style.opacity = '0';
    setTimeout(() => { hint.style.display = 'none'; }, 200);
  }
}

function getWelcomeMarkup() {
  return '<div class="ai-welcome">' +
    '<p>How can I help with your document?</p>' +
    '<p class="ai-welcome-hint">Pick a prompt below or type your own request.</p>' +
    '<p class="ai-access-note">AI works best on selected text. It sends your selection plus limited local context, not the entire document by default.</p>' +
    '<div class="ai-suggestions" id="aiSuggestions"></div>' +
    '</div>';
}

function getSuggestedPrompts() {
  const context = detectContext();
  const hasSelection = !!getSelectedText();

  if (context.mode === 'spreadsheet') {
    return [
      { label: 'Explain this formula', mode: 'formula', prompt: 'Explain this formula step by step and suggest a simpler version if possible.' },
      { label: 'Write a formula', mode: 'formula', prompt: 'Write a formula that:' },
      { label: 'Analyze selected data', mode: 'data_analysis', prompt: 'Analyze the selected data and call out trends, outliers, and totals.' },
    ];
  }

  if (hasSelection) {
    return [
      { label: 'Improve selected text', mode: 'writer', prompt: 'Improve this text for clarity and flow.' },
      { label: 'Fix grammar', mode: 'grammar', prompt: '' },
      { label: 'Summarize selection', mode: 'summarize', prompt: '' },
    ];
  }

  return [
    { label: 'Draft an introduction', mode: 'writer', prompt: 'Draft a concise introduction for this document.' },
    { label: 'Create an outline', mode: 'writer', prompt: 'Create a clear outline for this document.' },
    { label: 'Summarize the document', mode: 'summarize', prompt: '' },
  ];
}

function renderPromptSuggestions() {
  const container = $('aiSuggestions');
  if (!container) return;

  const suggestions = getSuggestedPrompts();
  container.innerHTML = '';

  suggestions.forEach(({ label, mode, prompt }) => {
    const button = document.createElement('button');
    button.className = 'ai-suggestion-btn';
    button.type = 'button';
    button.textContent = label;
    button.title = label;
    button.addEventListener('click', () => {
      if (_modeSelect) _modeSelect.value = mode;
      if (_input) {
        _input.value = prompt;
        _input.focus();
        _input.dispatchEvent(new Event('input'));
      }
      if (prompt) {
        sendMessage();
      }
    });
    container.appendChild(button);
  });
}

function populateModeOptions() {
  if (!_modeSelect || _modeSelect.options.length) return;

  AI_MODE_OPTIONS.forEach(({ value, label }) => {
    const option = document.createElement('option');
    option.value = value;
    option.textContent = label;
    _modeSelect.appendChild(option);
  });
}

function checkForHints() {
  if (!state.aiAvailable || state.aiPanelOpen) return;

  const context = detectContext();

  // Hint on first spreadsheet use
  if (context.mode === 'spreadsheet' && !state.aiHintsShown.spreadsheetHint) {
    const dismissed = localStorage.getItem('rudra_ai_hint_spreadsheet');
    if (!dismissed) {
      state.aiHintsShown.spreadsheetHint = true;
      showHint('AI can help with formulas and data analysis');
      localStorage.setItem('rudra_ai_hint_spreadsheet', '1');
    }
    return;
  }

  // Hint on long paragraph
  if (context.paraContext && context.paraContext.length > 200 && !state.aiHintsShown.selectionHint) {
    const dismissed = localStorage.getItem('rudra_ai_hint_selection');
    if (!dismissed) {
      showHint('Select text and let AI help improve your writing');
    }
  }
}

// ── Core AI Execution ──────────────────────────────

async function runAI(mode, userMessage) {
  if (state.aiGenerating) return;

  const context = detectContext();
  const systemPrompt = buildContextPrompt(mode, context);

  state.aiGenerating = true;
  // Track AI panel usage (Enhancement 88)
  import('./analytics.js').then(m => m.trackEvent('ai_panel', mode)).catch(() => {});
  _sendBtn.style.display = 'none';
  _stopBtn.style.display = 'flex';
  showTyping();

  try {
    let responseDiv = null;
    let fullText = '';

    await aiComplete(mode, userMessage, {
      systemPrompt,
      context: state.aiConversation,
      onChunk(chunk) {
        hideTyping();
        if (!responseDiv) {
          responseDiv = document.createElement('div');
          responseDiv.className = 'ai-msg ai-msg-ai';
          _messages.appendChild(responseDiv);
        }
        fullText += chunk;
        responseDiv.innerHTML = formatAIResponse(fullText);
        _messages.scrollTop = _messages.scrollHeight;
      }
    });

    hideTyping();

    if (!responseDiv && fullText) {
      addMessage('ai', fullText);
    } else if (responseDiv) {
      appendActionButtons(responseDiv, fullText);
    }

    state.aiConversation.push({ role: 'user', content: userMessage });
    state.aiConversation.push({ role: 'assistant', content: fullText });
    if (state.aiConversation.length > 12) state.aiConversation = state.aiConversation.slice(-12);
    try { sessionStorage.setItem('rudra_ai_conversation', JSON.stringify(state.aiConversation)); } catch (_) {}

  } catch (err) {
    hideTyping();
    if (err.name !== 'AbortError') {
      addMessage('ai', `Error: ${err.message}`, { isError: true });
    }
  } finally {
    state.aiGenerating = false;
    _sendBtn.style.display = 'flex';
    _stopBtn.style.display = 'none';
  }
}

// ── Message Rendering ──────────────────────────────

function addMessage(role, content, opts = {}) {
  const welcome = _messages.querySelector('.ai-welcome');
  if (welcome) welcome.remove();

  const div = document.createElement('div');
  div.className = `ai-msg ai-msg-${role}`;

  if (role === 'ai' && !opts.isError) {
    div.innerHTML = formatAIResponse(content);
    if (!opts.noActions) appendActionButtons(div, content);
  } else if (opts.isError) {
    div.className = 'ai-msg ai-msg-error';
    div.textContent = content;
  } else {
    div.textContent = content;
  }

  _messages.appendChild(div);
  _messages.scrollTop = _messages.scrollHeight;
  return div;
}

function appendActionButtons(div, text) {
  const actions = document.createElement('div');
  actions.className = 'ai-msg-actions';

  const btns = [
    { label: 'Copy', icon: 'content_copy', action: () => {
      navigator.clipboard.writeText(text).then(() => {
        btns[0].el.textContent = 'Copied';
        setTimeout(() => { btns[0].el.innerHTML = '<span class="msi" style="font-size:14px">content_copy</span> Copy'; }, 1500);
      });
    }},
    { label: 'Replace', icon: 'find_replace', action: () => replaceSelection(text) },
    { label: 'Insert below', icon: 'vertical_align_bottom', action: () => insertBelow(text) },
  ];

  btns.forEach((b, i) => {
    const btn = document.createElement('button');
    btn.className = 'ai-msg-action-btn';
    btn.innerHTML = `<span class="msi" style="font-size:14px">${b.icon}</span> ${b.label}`;
    btn.title = b.label;
    btn.onclick = b.action;
    b.el = btn;
    actions.appendChild(btn);
  });

  div.appendChild(actions);
}

/** Replace selection via WASM ops for undo/redo and collaboration support */
function replaceSelection(text) {
  let sel = window.getSelection();

  // If current selection is collapsed, try saved range (Bug 60, Bug 75)
  if ((!sel || sel.isCollapsed) && _savedSelectionRange) {
    if (_savedSelectionRange.nodeId) {
      // Reconstruct selection from saved nodeId + text (Bug 75 — stale-resistant)
      const savedNode = $('editorCanvas')?.querySelector(`[data-node-id="${_savedSelectionRange.nodeId}"]`);
      if (savedNode) {
        const paraText = savedNode.textContent || '';
        const idx = paraText.indexOf(_savedSelectionRange.text);
        if (idx !== -1) {
          const range = document.createRange();
          const textNode = savedNode.firstChild;
          if (textNode) {
            range.setStart(textNode, idx);
            range.setEnd(textNode, idx + _savedSelectionRange.text.length);
            sel = window.getSelection();
            sel.removeAllRanges();
            sel.addRange(range);
          }
        }
      }
    } else {
      // Legacy: DOM Range fallback
      sel = window.getSelection();
      sel.removeAllRanges();
      sel.addRange(_savedSelectionRange);
    }
  }

  if (!sel || sel.isCollapsed || !sel.rangeCount) return;

  // Find the target paragraph node
  let node = sel.anchorNode;
  while (node && node.nodeType !== 1) node = node.parentNode;
  while (node && !node.dataset?.nodeId) node = node.parentNode;

  const nodeId = node?.dataset?.nodeId;
  const doc = state.doc;

  if (nodeId && doc) {
    try {
      // Replace via WASM
      const el = node;
      const fullText = el.textContent || '';
      const selectedText = sel.toString();
      const startIdx = fullText.indexOf(selectedText);

      if (startIdx !== -1) {
        // Use replace_text which preserves formatting of untouched runs (Bug 1 fix).
        // WASM expects Unicode codepoint offsets, not JS UTF-16 code units (Bug 42 fix).
        const cpOffset = Array.from(fullText.slice(0, startIdx)).length;
        const cpLength = Array.from(selectedText).length;
        doc.replace_text(nodeId, cpOffset, cpLength, text);

        const before = fullText.slice(0, startIdx);
        const after = fullText.slice(startIdx + selectedText.length);
        el.textContent = before + text + after;

        state.dirty = true;
        return;
      }
    } catch (err) {
      console.warn('[ai-panel] WASM replace failed:', err);
    }
  }

  // No DOM fallback — would create WASM/DOM divergence (Bug 5 fix)
  console.error('[ai-panel] Replace failed — no WASM path available');
  // Show user-visible error (Bug 45 fix)
  import('./toolbar-handlers.js').then(m => m.showToast('Replace failed — please try again', 'error')).catch(() => {});
}

/** Insert text below current paragraph via WASM ops */
function insertBelow(text) {
  const sel = window.getSelection();
  if (!sel || !sel.rangeCount) return;

  let para = sel.getRangeAt(0).endContainer;
  while (para && para.nodeType !== 1) para = para.parentNode;
  while (para && !para.dataset?.nodeId) para = para.parentNode;

  const nodeId = para?.dataset?.nodeId;
  const doc = state.doc;

  if (nodeId && doc) {
    try {
      // Insert via WASM — add paragraph after current node
      doc.insert_paragraph_after(nodeId, text);
      // Don't manually create DOM <p> — it would be malformed without proper
      // attributes. Mark dirty and let the render pipeline build correct DOM
      // on the next render cycle (Bug 6 fix).
      state.dirty = true;
      // Trigger re-render so the new paragraph appears in DOM (Bug 46 fix)
      try {
        const editorCanvas = $('editorCanvas');
        if (editorCanvas) editorCanvas.dispatchEvent(new Event('input', { bubbles: true }));
      } catch (_) {}
      return;
    } catch (err) {
      console.warn('[ai-panel] WASM insert failed:', err);
    }
  }

  // No DOM fallback — would create WASM/DOM divergence (Bug 6 fix)
  console.error('[ai-panel] Insert below failed — no WASM path available');
  // Show user-visible error (Bug 45 fix)
  import('./toolbar-handlers.js').then(m => m.showToast('Insert failed — please try again', 'error')).catch(() => {});
}

function formatAIResponse(text) {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/```(\w*)\n([\s\S]*?)```/g, '<pre><code>$2</code></pre>')
    .replace(/`([^`]+)`/g, '<code style="background:rgba(0,0,0,.06);padding:1px 4px;border-radius:3px;font-size:12px">$1</code>')
    .replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
    .replace(/\n/g, '<br>');
}

function showTyping() {
  const div = document.createElement('div');
  div.className = 'ai-typing';
  div.id = 'aiTyping';
  div.innerHTML = '<span class="ai-typing-dot"></span><span class="ai-typing-dot"></span><span class="ai-typing-dot"></span>';
  _messages.appendChild(div);
  _messages.scrollTop = _messages.scrollHeight;
}

function hideTyping() {
  const el = document.getElementById('aiTyping');
  if (el) el.remove();
}

// ── Send Message ───────────────────────────────────

async function sendMessage() {
  const text = _input.value.trim();
  const selectedText = getSelectedText();
  if (!text && !selectedText) return;
  if (state.aiGenerating) return;

  const mode = _modeSelect.value;

  let userMessage = text;
  if (selectedText && text) {
    userMessage = `${text}\n\nSelected text:\n${selectedText}`;
  } else if (selectedText && !text) {
    const modeLabels = {
      writer: 'Improve this text:',
      grammar: 'Check grammar:',
      summarize: 'Summarize this:',
      translate: 'Translate to English:',
      formula: 'Write a formula for:',
      data_analysis: 'Analyze this data:'
    };
    userMessage = `${modeLabels[mode] || 'Help with:'}\n\n${selectedText}`;
  }

  if (!userMessage) return;

  // Save selection for later Replace action (Bug 60, Bug 75 — store nodeId + text instead of DOM Range)
  const currentSel = window.getSelection();
  if (currentSel && currentSel.rangeCount > 0 && !currentSel.isCollapsed) {
    // Store nodeId + text instead of DOM Range (stale-resistant)
    const selText = currentSel.toString();
    let selNode = currentSel.anchorNode;
    while (selNode && selNode.nodeType !== 1) selNode = selNode.parentNode;
    while (selNode && !selNode.dataset?.nodeId) selNode = selNode.parentNode;
    if (selNode?.dataset?.nodeId) {
      _savedSelectionRange = { nodeId: selNode.dataset.nodeId, text: selText };
    } else {
      _savedSelectionRange = currentSel.getRangeAt(0).cloneRange();
    }
  }

  const displayText = text || `[${_modeSelect.options[_modeSelect.selectedIndex].text}]`;
  if (selectedText && !text) {
    addMessage('user', displayText + '\n' + selectedText.slice(0, 100) + (selectedText.length > 100 ? '...' : ''), { noActions: true });
  } else {
    addMessage('user', displayText, { noActions: true });
  }

  _input.value = '';
  _input.style.height = '';

  await runAI(mode, userMessage);
}

// ── Panel Toggle ───────────────────────────────────

export function toggleAIPanel() {
  state.aiPanelOpen = !state.aiPanelOpen;

  if (state.aiPanelOpen) {
    _panel.style.display = 'flex';
    _panel.offsetHeight; // force reflow
    _panel.classList.add('show');
    _input.focus();
    hideFloatingBar();
    dismissHint();
    updateContextIndicator();
    renderPromptSuggestions();

    // One-time warning for external AI endpoints (Bug 66)
    const aiUrl = window.S1_CONFIG?.aiUrl || '';
    const isExternal = aiUrl && !aiUrl.includes('localhost') && !aiUrl.includes('127.0.0.1');
    if (isExternal && !localStorage.getItem('rudra_ai_external_consent')) {
      let host;
      try { host = new URL(aiUrl).hostname; } catch { host = aiUrl; }
      addMessage('ai', `Note: AI features will send selected text to ${host}. Your document content is transmitted to process AI requests. This is a one-time notice.`, { isError: false, noActions: true });
      localStorage.setItem('rudra_ai_external_consent', '1');
    }

    // Cache health check result for 30 seconds (Bug 32)
    const now = Date.now();
    if (now - _lastHealthCheck > 30000) {
      checkAIHealth().then(ok => {
        _lastHealthCheck = Date.now();
        _lastHealthResult = ok;
        state.aiAvailable = ok;
        _status.className = 'ai-panel-status ' + (ok ? 'connected' : 'error');
        _status.title = ok ? 'AI connected' : 'AI not reachable — start the sidecar with: docker compose up rudra-ai';
        if (!ok) {
          _lastFloatingMode = null; // Reset so buttons rebuild when AI comes back (Bug 56)
          addMessage('ai', 'AI sidecar is not running. Start it with:\ndocker compose up rudra-ai', { isError: true, noActions: true });
        }
      });
    } else {
      // Use cached result
      _status.className = 'ai-panel-status ' + (_lastHealthResult ? 'connected' : 'error');
    }
  } else {
    _panel.classList.remove('show');
    setTimeout(() => { if (!state.aiPanelOpen) _panel.style.display = 'none'; }, 260);
  }
}

function updateContextIndicator() {
  const ctx = detectContext();
  const chip = $('aiContextLabel');
  if (chip) {
    chip.textContent = ctx.label;
    chip.title = `Context: ${ctx.mode} (${ctx.type})`;
  }

  // Auto-switch mode based on context
  if (ctx.mode === 'spreadsheet') {
    _modeSelect.value = 'formula';
  } else {
    if (_modeSelect.value === 'formula' || _modeSelect.value === 'data_analysis') {
      _modeSelect.value = 'writer';
    }
  }

  renderPromptSuggestions();
}

// ── Initialization ─────────────────────────────────

export function initAIPanel() {
  const cfg = window.S1_CONFIG || {};
  if (!cfg.enableAI) {
    // Show "not configured" message if user opens the AI panel
    if (!window.S1_CONFIG?.aiUrl) {
      const messages = document.getElementById('aiMessages');
      if (messages) {
        messages.innerHTML = '<div style="padding:24px;text-align:center;color:var(--text-muted);">' +
          '<span class="msi" style="font-size:32px;display:block;margin-bottom:8px;">smart_toy</span>' +
          '<p style="font-weight:500;margin-bottom:4px;">AI Assistant Not Configured</p>' +
          '<p style="font-size:12px;">Set <code>aiUrl</code> and <code>enableAI</code> in S1_CONFIG to enable AI features.</p></div>';
      }
    }
    return;
  }
  if (!initAI()) return;

  _panel = $('aiPanel');
  _messages = $('aiMessages');
  _input = $('aiInput');
  _sendBtn = $('aiSendBtn');
  _stopBtn = $('aiStopBtn');
  _status = $('aiPanelStatus');
  _modeSelect = $('aiModeSelect');
  _floatingBar = $('aiFloatingBar');
  _contextChip = $('aiFloatingContext');

  if (!_panel) return;

  // Drag-to-resize handle (Enhancement 86)
  populateModeOptions();

  const resizeHandle = _panel.querySelector('.ai-panel-resize-handle') || document.createElement('div');
  if (!resizeHandle.parentNode) {
    resizeHandle.className = 'ai-panel-resize-handle';
    resizeHandle.title = 'Drag to resize';
    _panel.insertBefore(resizeHandle, _panel.firstChild);
  }

  let _resizing = false;
  let _startX = 0;
  let _startWidth = 0;

  resizeHandle.addEventListener('mousedown', (e) => {
    e.preventDefault();
    _resizing = true;
    _startX = e.clientX;
    _startWidth = _panel.offsetWidth;
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';
  });

  document.addEventListener('mousemove', (e) => {
    if (!_resizing) return;
    const delta = _startX - e.clientX; // Left edge drag = invert
    const newWidth = Math.max(280, Math.min(600, _startWidth + delta));
    _panel.style.width = newWidth + 'px';
    _panel.style.minWidth = newWidth + 'px';
  });

  document.addEventListener('mouseup', () => {
    if (_resizing) {
      _resizing = false;
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
    }
  });

  // Restore conversation from session (Enhancement 68)
  try {
    const saved = sessionStorage.getItem('rudra_ai_conversation');
    if (saved) state.aiConversation = JSON.parse(saved);
  } catch (_) {}

  if (_messages && !_messages.children.length) {
    _messages.innerHTML = getWelcomeMarkup();
  }
  renderPromptSuggestions();

  // Close
  $('aiPanelClose')?.addEventListener('click', toggleAIPanel);

  // Send
  _sendBtn.addEventListener('click', sendMessage);

  // Stop
  _stopBtn.addEventListener('click', () => {
    abortAI();
    state.aiGenerating = false;
    _sendBtn.style.display = 'flex';
    _stopBtn.style.display = 'none';
    hideTyping();
  });

  // Enter = send, Shift+Enter = newline
  _input.addEventListener('keydown', (e) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      sendMessage();
    }
  });

  // Auto-resize
  _input.addEventListener('input', () => {
    _input.style.height = '';
    _input.style.height = Math.min(_input.scrollHeight, 120) + 'px';
  });

  // Clear conversation
  $('aiClearBtn')?.addEventListener('click', () => {
    _messages.innerHTML = getWelcomeMarkup();
    state.aiConversation = [];
    sessionStorage.removeItem('rudra_ai_conversation');
    renderPromptSuggestions();
  });

  // Menu item
  $('menuAIAssist')?.addEventListener('click', toggleAIPanel);

  // Hint dismiss
  $('aiHintBubble')?.addEventListener('click', () => {
    localStorage.setItem('rudra_ai_hint_selection', '1');
    dismissHint();
    toggleAIPanel();
  });

  // Keyboard shortcut: Ctrl/Cmd+Shift+A
  document.addEventListener('keydown', (e) => {
    if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key === 'A') {
      e.preventDefault();
      toggleAIPanel();
    }
  });

  // Escape dismisses floating bar
  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape' && state.aiFloatingBarVisible) {
      hideFloatingBar();
    }
  });

  // ── Selection listener — show floating bar on text selection ──
  document.addEventListener('selectionchange', () => {
    clearTimeout(_selectionTimer);
    _selectionTimer = setTimeout(() => {
      const text = getSelectedText();
      if (text && text.length >= 15 && text !== _lastSelectedText) {
        _lastSelectedText = text;
        state.aiLastSelectedText = text;
        showFloatingBar(text);
      } else if (!text || text.length < 15) {
        _lastSelectedText = '';
        state.aiLastSelectedText = '';
        hideFloatingBar();
      }
    }, 400);
  });

  // Hide floating bar on scroll (Bug 58)
  const editorCanvas = $('editorCanvas');
  if (editorCanvas) {
    editorCanvas.addEventListener('scroll', () => {
      if (state.aiFloatingBarVisible) hideFloatingBar();
    }, { passive: true });
  }

  // Hide floating bar on click outside
  document.addEventListener('mousedown', (e) => {
    if (_floatingBar && !_floatingBar.contains(e.target)) {
      hideFloatingBar();
    }
  });

  // Floating bar quick actions — delegates to inline system
  _floatingBar?.addEventListener('click', (e) => {
    const btn = e.target.closest('[data-ai-action]');
    if (btn) {
      e.preventDefault();
      e.stopPropagation();
      floatingAction(btn.dataset.aiAction);
    }
  });

  // Update context when view changes
  const viewObserver = new MutationObserver(() => {
    if (state.aiPanelOpen) updateContextIndicator();
  });
  const toolbar = $('toolbar');
  if (toolbar) viewObserver.observe(toolbar, { attributes: true, subtree: true });

  // Check AI health at startup (silently)
  checkAIHealth().then(ok => {
    state.aiAvailable = ok;
    if (!ok) _lastFloatingMode = null; // Reset so buttons rebuild when AI comes back (Bug 56)
    if (ok) {
      setTimeout(() => checkForHints(), 10000);
    }
  });
}
