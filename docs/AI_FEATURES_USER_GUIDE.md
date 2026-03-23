# AI Features User Guide

**Last updated:** 2026-03-23

This guide explains what the Rudra Office AI features can access, what they **do not** automatically access, and how to use the current AI workflows in documents and spreadsheets.

## 1. What data does the AI panel have access to?

### By default, the AI panel does **not** send the full document automatically

The current implementation mainly sends:

- the text you type into the AI panel
- the current text selection, if you have one selected
- a **small paragraph-context snippet** from the current paragraph for better prompting
- recent AI conversation history from the same session

This means the AI panel is **selection-first**, not full-document-first.

## 2. What gets sent in each AI flow?

### A. AI panel chat

When you use the AI panel:

- If you type a request **and** have text selected, the request includes your typed instruction plus the selected text.
- If you leave the input blank but have text selected, the panel builds a mode-specific request such as:
  - "Improve this text"
  - "Check grammar"
  - "Summarize this"
- The panel also builds a context-aware system prompt using the current mode and a short paragraph preview.

### B. Floating selection toolbar

When you highlight text in the document editor, the floating AI toolbar can:

- improve writing
- shorten
- expand
- fix grammar
- translate

These actions use the **selected text** as the main payload.

### C. Inline AI prompt and slash commands

The inline AI features also work from the current selection or paragraph:

- `/ai`
- `/ai-improve`
- `/ai-grammar`
- `/ai-summarize`
- `/ai-translate`
- `/ai-fill`
- `/ai-format`
- `/ai-table`
- `/ai-formula`

If there is no selection, the inline flow falls back to the **current paragraph text**.

### D. Spreadsheet AI

In spreadsheet mode, the AI features use the current selected cell range, collected as tab/newline-delimited data.

This powers:

- formula suggestions
- formula explanation prompts
- data analysis actions

## 3. What the AI does **not** currently do automatically

The current implementation does **not** automatically:

- read the entire open document and summarize it in the background
- scan every page silently
- ingest the whole spreadsheet workbook by default
- analyze PDFs as a full document corpus through the panel

To get best results, select the content you want AI to work with first.

## 4. How to use AI features

## Document editor

### Option 1: AI panel

1. Open **Tools → AI Assistant** or press **Ctrl/Cmd + Shift + A**.
2. Pick a mode:
   - Write & rewrite
   - Grammar & clarity
   - Summarize
   - Translate
   - Spreadsheet formula
   - Data analysis
3. Select text first if you want targeted output.
4. Type your instruction and press **Enter** or click **Send**.
5. Use the response action buttons:
   - **Copy**
   - **Replace**
   - **Insert below**

### Option 2: Floating toolbar on selected text

1. Select text in the document.
2. Wait for the floating AI toolbar.
3. Choose:
   - Improve
   - Shorten
   - Expand
   - Grammar
   - Translate
4. Review the inline diff.
5. Accept, reject, or retry.

### Option 3: Slash commands

1. Type `/` in a document paragraph.
2. Choose an AI command from the slash menu.
3. Confirm or refine the prompt.
4. Review the generated suggestion inline.

## Spreadsheet editor

### Formula help

1. Select a target cell or range.
2. Open the AI panel or use spreadsheet AI entry points.
3. Use **Spreadsheet formula** mode.
4. Describe the formula you want in plain language.

Example:

> Write a formula that totals column B when column A equals "Paid".

### Data analysis

1. Select the cells you want analyzed.
2. Open the AI panel.
3. Choose **Data analysis**.
4. Ask for trends, totals, outliers, or summaries.

## 5. Best-practice guide for users

- **Select first, ask second.** The AI is strongest when you explicitly select the text or cells you want it to use.
- **Use narrow prompts.** "Rewrite this introduction for clarity" works better than "fix my document."
- **Use the floating toolbar for quick edits.** It is the fastest flow for rewrite/grammar/translate tasks.
- **Use the panel for multi-turn work.** The side panel keeps short session history.
- **Use slash commands for drafting inside the editor.** They fit better into writing flow than opening the side panel.

## 6. Privacy and endpoint behavior

- AI requests are sent to the configured AI endpoint from `S1_CONFIG.aiUrl`, or to the auto-detected local sidecar.
- If a non-local AI endpoint is configured, the app shows a one-time notice that document content will be transmitted for AI processing.
- Because the app sends selected text and nearby context, users should avoid selecting sensitive content unless they intend to send it.

## 7. Current limitations

- Full-document AI workflows are not yet first-class; most current flows are selection-based.
- Panel Replace still depends on reconstructing the selected text location, so duplicate phrases in a paragraph can be ambiguous.
- AI behavior depends on the configured sidecar/API supporting the expected OpenAI-compatible chat completion format.

## 8. Recommended user workflow

If you're unsure how to use AI in Rudra Office, start with this:

1. Select one paragraph.
2. Use the floating AI toolbar to improve or summarize it.
3. Accept the change inline.
4. For larger work, open the AI panel and continue iteratively section by section.

That matches the current product architecture much better than treating the AI panel like a full-document agent.
