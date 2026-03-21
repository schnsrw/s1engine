// Rudra Office — AI Assistant Client
// Connects to the llama.cpp sidecar (OpenAI-compatible API)

const SYSTEM_PROMPTS = {
  writer: 'You are a writing assistant embedded in Rudra Office, a document editor. Help the user write, edit, and improve their document content. Be concise and direct. Return only the improved text unless asked for explanation.',
  grammar: 'You are a grammar checker in Rudra Office. Check the given text for grammar, spelling, punctuation, and clarity issues. Return a JSON array: [{"original":"...","corrected":"...","reason":"..."}]. If no issues, return [].',
  summarize: 'You are a summarization assistant in Rudra Office. Summarize clearly and concisely. Use bullet points for key takeaways. Keep summaries under 200 words.',
  formula: 'You are a spreadsheet formula assistant in Rudra Office. Help users write formulas. Support standard functions (SUM, AVERAGE, VLOOKUP, IF, COUNTIF, etc.). Return only the formula unless asked for explanation.',
  translate: 'You are a translation assistant in Rudra Office. Translate to the requested language. Preserve formatting and tone. Return only the translated text.',
  data_analysis: 'You are a data analysis assistant in Rudra Office Sheets. Analyze the provided spreadsheet data and provide insights: trends, outliers, summary statistics. Be concise.'
};

let _baseUrl = '';
let _abortController = null;

/** Initialize AI module with the sidecar URL */
export function initAI() {
  const cfg = window.S1_CONFIG || {};
  if (!cfg.enableAI) return false;

  // Auto-detect: same host, port 8081
  _baseUrl = cfg.aiUrl || `${window.location.protocol}//${window.location.hostname}:8081`;
  return true;
}

/** Check if the AI sidecar is reachable */
export async function checkAIHealth() {
  try {
    const res = await fetch(`${_baseUrl}/health`, { signal: AbortSignal.timeout(3000) });
    return res.ok;
  } catch {
    return false;
  }
}

/** Abort any in-flight AI request */
export function abortAI() {
  if (_abortController) {
    _abortController.abort();
    _abortController = null;
  }
}

/**
 * Send a chat completion request to the AI sidecar.
 * @param {string} mode - One of: writer, grammar, summarize, formula, translate, data_analysis
 * @param {string} userMessage - The user's input/selected text
 * @param {object} [opts] - Optional overrides
 * @param {string} [opts.systemPrompt] - Override the system prompt
 * @param {number} [opts.maxTokens] - Max tokens to generate (default: 1024)
 * @param {number} [opts.temperature] - Temperature (default: 0.3)
 * @param {function} [opts.onChunk] - Streaming callback(text) for partial results
 * @param {boolean} [opts.noAutoAbort] - If true, skip aborting previous requests (for concurrent usage)
 * @returns {Promise<string>} The AI response text
 */
export async function aiComplete(mode, userMessage, opts = {}) {
  if (!opts.noAutoAbort) abortAI();
  _abortController = new AbortController();

  const systemPrompt = opts.systemPrompt || SYSTEM_PROMPTS[mode] || SYSTEM_PROMPTS.writer;
  const maxTokens = opts.maxTokens || 1024;
  const temperature = opts.temperature ?? 0.3;
  const stream = typeof opts.onChunk === 'function';

  const contextMessages = Array.isArray(opts.context) ? opts.context : [];

  const body = {
    messages: [
      { role: 'system', content: systemPrompt },
      ...contextMessages,
      { role: 'user', content: userMessage }
    ],
    max_tokens: maxTokens,
    temperature,
    stream
  };

  // Auto-abort after timeout (Bug 33)
  const timeoutMs = opts.timeout || 60000;
  const timeoutId = setTimeout(() => {
    if (_abortController) _abortController.abort(new Error('AI request timed out'));
  }, timeoutMs);

  let res;
  try {
    res = await fetch(`${_baseUrl}/v1/chat/completions`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
      signal: _abortController.signal
    });
  } catch (fetchErr) {
    clearTimeout(timeoutId);
    throw fetchErr;
  }

  if (!res.ok) {
    clearTimeout(timeoutId);
    const err = await res.text().catch(() => 'Unknown error');
    throw new Error(`AI request failed (${res.status}): ${err}`);
  }

  // Streaming response
  if (stream) {
    const reader = res.body.getReader();
    const decoder = new TextDecoder();
    let fullText = '';
    let buffer = '';

    try {
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;

        buffer += decoder.decode(value, { stream: true });
        const lines = buffer.split('\n');
        buffer = lines.pop() || '';

        for (const line of lines) {
          if (!line.startsWith('data: ') || line === 'data: [DONE]') continue;
          try {
            const chunk = JSON.parse(line.slice(6));
            const delta = chunk.choices?.[0]?.delta?.content;
            if (delta) {
              fullText += delta;
              opts.onChunk(delta);
            }
          } catch { /* skip malformed chunks */ }
        }
      }
    } finally {
      clearTimeout(timeoutId);
    }

    _abortController = null;
    return fullText;
  }

  // Non-streaming response
  const data = await res.json();
  clearTimeout(timeoutId);
  _abortController = null;
  return data.choices?.[0]?.message?.content || '';
}

// ── Convenience helpers ──────────────────────────────

/** Improve/rewrite selected text */
export function aiRewrite(text, instruction) {
  const prompt = instruction
    ? `${instruction}\n\nText:\n${text}`
    : `Improve this text for clarity and grammar:\n\n${text}`;
  return aiComplete('writer', prompt);
}

/** Grammar check — returns array of corrections */
export async function aiGrammarCheck(text) {
  const result = await aiComplete('grammar', text, { temperature: 0.1 });
  try {
    return JSON.parse(result);
  } catch {
    return [];
  }
}

/** Summarize text */
export function aiSummarize(text) {
  return aiComplete('summarize', text);
}

/** Generate a spreadsheet formula */
export function aiFormula(description) {
  return aiComplete('formula', description, { maxTokens: 256 });
}

/** Translate text */
export function aiTranslate(text, targetLang) {
  return aiComplete('translate', `Translate to ${targetLang}:\n\n${text}`);
}

/** Analyze spreadsheet data */
export function aiAnalyzeData(csvData) {
  return aiComplete('data_analysis', `Analyze this data:\n\n${csvData}`);
}
