// PDF Form Filling — detect and render interactive form fields over PDF pages
import { state, $ } from './state.js';
import { showToast } from './toolbar-handlers.js';

let _formMode = false;
let _formFieldElements = [];

/**
 * Toggle form fill mode — detects and renders form fields.
 */
export function toggleFormMode() {
  _formMode = !_formMode;
  if (_formMode) {
    detectAndRenderForms();
  } else {
    clearFormFields();
  }
}

/**
 * Detect form fields from WASM PDF editor and render HTML controls.
 */
async function detectAndRenderForms() {
  if (!state._wasmPdfEditor) {
    // Try to detect form fields from PDF.js annotations
    await detectFormsFromPdfJs();
    return;
  }

  try {
    const fieldsJson = state._wasmPdfEditor.get_form_fields();
    const fields = JSON.parse(fieldsJson);
    state.pdfFormFields = fields;
    renderFormFields(fields);
  } catch (e) {
    showToast('No form fields detected', 'error');
    _formMode = false;
  }
}

/**
 * Fallback: detect form fields using PDF.js annotation data.
 */
async function detectFormsFromPdfJs() {
  const viewer = state.pdfViewer;
  if (!viewer) return;
  const pdfDoc = viewer.getPdfDocument();
  if (!pdfDoc) return;

  const fields = [];
  const pageCount = pdfDoc.numPages;

  for (let i = 1; i <= pageCount; i++) {
    const page = await pdfDoc.getPage(i);
    const annotations = await page.getAnnotations();
    const viewport = page.getViewport({ scale: state.pdfZoom || 1.0 });

    for (const annot of annotations) {
      if (!annot.fieldType) continue;

      const rect = annot.rect;
      // Convert PDF coordinates to viewport coordinates
      const [x1, y1] = viewport.convertToViewportPoint(rect[0], rect[1]);
      const [x2, y2] = viewport.convertToViewportPoint(rect[2], rect[3]);

      const field = {
        name: annot.fieldName || annot.id || `field_${fields.length}`,
        field_type: mapFieldType(annot.fieldType),
        page: i,
        rect: {
          x: Math.min(x1, x2),
          y: Math.min(y1, y2),
          width: Math.abs(x2 - x1),
          height: Math.abs(y2 - y1),
        },
        value: annot.fieldValue || '',
        options: annot.options?.map(o => o.displayValue || o.exportValue) || [],
      };
      fields.push(field);
    }
  }

  if (fields.length === 0) {
    showToast('No form fields detected in this PDF');
    _formMode = false;
    return;
  }

  state.pdfFormFields = fields;
  renderFormFields(fields);
  showToast(`${fields.length} form field(s) detected`);
}

function mapFieldType(pdfJsType) {
  switch (pdfJsType) {
    case 'Tx': return 'Text';
    case 'Btn': return 'Checkbox';
    case 'Ch': return 'Dropdown';
    case 'Sig': return 'Signature';
    default: return 'Text';
  }
}

/**
 * Render HTML form controls over the PDF pages at field positions.
 */
function renderFormFields(fields) {
  clearFormFields();

  for (const field of fields) {
    const overlayLayer = state.pdfViewer?.getOverlayLayer(field.page);
    if (!overlayLayer) continue;

    overlayLayer.style.pointerEvents = 'auto';

    const wrapper = document.createElement('div');
    wrapper.className = 'pdf-form-field';
    wrapper.style.left = field.rect.x + 'px';
    wrapper.style.top = field.rect.y + 'px';
    wrapper.style.width = field.rect.width + 'px';
    wrapper.style.height = field.rect.height + 'px';
    wrapper.title = field.name;

    let control;

    switch (field.field_type) {
      case 'Text': {
        if (field.rect.height > 40) {
          control = document.createElement('textarea');
        } else {
          control = document.createElement('input');
          control.type = 'text';
        }
        control.value = field.value;
        control.placeholder = field.name;
        control.addEventListener('change', () => {
          field.value = control.value;
          state.pdfModified = true;
          // Update WASM editor if available
          if (state._wasmPdfEditor) {
            try { state._wasmPdfEditor.set_form_field_value(field.name, control.value); } catch (_) {}
          }
        });
        break;
      }

      case 'Checkbox': {
        control = document.createElement('input');
        control.type = 'checkbox';
        control.checked = field.value === 'Yes' || field.value === 'true' || field.value === 'On';
        control.style.width = 'auto';
        control.style.height = 'auto';
        control.style.margin = '4px';
        control.addEventListener('change', () => {
          field.value = control.checked ? 'Yes' : 'Off';
          state.pdfModified = true;
          if (state._wasmPdfEditor) {
            try { state._wasmPdfEditor.set_form_field_value(field.name, field.value); } catch (_) {}
          }
        });
        break;
      }

      case 'Radio': {
        control = document.createElement('input');
        control.type = 'radio';
        control.name = field.name;
        control.checked = !!field.value;
        control.style.width = 'auto';
        control.style.height = 'auto';
        control.style.margin = '4px';
        control.addEventListener('change', () => {
          field.value = control.checked ? 'On' : 'Off';
          state.pdfModified = true;
          if (state._wasmPdfEditor) {
            try { state._wasmPdfEditor.set_form_field_value(field.name, field.value); } catch (_) {}
          }
        });
        break;
      }

      case 'Dropdown': {
        control = document.createElement('select');
        for (const opt of field.options) {
          const option = document.createElement('option');
          option.value = opt;
          option.textContent = opt;
          if (opt === field.value) option.selected = true;
          control.appendChild(option);
        }
        control.addEventListener('change', () => {
          field.value = control.value;
          state.pdfModified = true;
          if (state._wasmPdfEditor) {
            try { state._wasmPdfEditor.set_form_field_value(field.name, control.value); } catch (_) {}
          }
        });
        break;
      }

      case 'Signature': {
        control = document.createElement('button');
        control.textContent = 'Sign';
        control.style.cssText = 'width:100%;height:100%;cursor:pointer;background:var(--accent-light);border:1px dashed var(--accent);font-size:11px;color:var(--accent);border-radius:2px;';
        control.addEventListener('click', async () => {
          try {
            const { openSignatureModal } = await import('./pdf-signatures.js');
            openSignatureModal();
          } catch (_) {}
        });
        break;
      }

      default: {
        control = document.createElement('input');
        control.type = 'text';
        control.value = field.value;
        break;
      }
    }

    wrapper.appendChild(control);
    overlayLayer.appendChild(wrapper);
    _formFieldElements.push(wrapper);
  }

  // Show form indicator in status bar
  const info = $('statusInfo');
  if (info) {
    info.textContent = `Form mode: ${fields.length} field(s) — Tab to navigate`;
  }

  // Enable tab navigation between fields
  enableTabNavigation();
}

/**
 * Clear all rendered form field overlays.
 */
function clearFormFields() {
  _formFieldElements.forEach(el => el.remove());
  _formFieldElements = [];
  state.pdfFormFields = [];

  // Reset overlay pointer events
  const viewer = state.pdfViewer;
  if (viewer) {
    const pageCount = viewer.getPageCount();
    for (let i = 1; i <= pageCount; i++) {
      const overlay = viewer.getOverlayLayer(i);
      if (overlay && overlay.children.length === 0) {
        overlay.style.pointerEvents = 'none';
      }
    }
  }
}

/**
 * Enable Tab key navigation between form fields.
 */
function enableTabNavigation() {
  const inputs = _formFieldElements
    .map(w => w.querySelector('input,select,textarea,button'))
    .filter(Boolean);

  inputs.forEach((input, idx) => {
    input.addEventListener('keydown', (e) => {
      if (e.key === 'Tab') {
        e.preventDefault();
        const nextIdx = e.shiftKey ? idx - 1 : idx + 1;
        if (nextIdx >= 0 && nextIdx < inputs.length) {
          inputs[nextIdx].focus();
          // Scroll the page into view if needed
          const field = state.pdfFormFields[nextIdx];
          if (field) {
            state.pdfViewer?.goToPage(field.page);
          }
        }
      }
    });
  });
}

/**
 * Flatten form fields — converts form data to static content.
 * Requires WASM PDF editor.
 */
export async function flattenForm() {
  if (!state._wasmPdfEditor) {
    showToast('Form flattening requires WASM PDF editor', 'error');
    return;
  }
  try {
    state._wasmPdfEditor.flatten_form();
    state.pdfModified = true;
    // Reload the PDF
    const bytes = state._wasmPdfEditor.save();
    state.pdfBytes = bytes;
    await state.pdfViewer.open(bytes);
    clearFormFields();
    _formMode = false;
    showToast('Form flattened — fields are now static content');
  } catch (e) {
    showToast('Flatten failed: ' + e.message, 'error');
  }
}
