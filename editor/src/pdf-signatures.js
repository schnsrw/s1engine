// PDF Signatures — visual signature creation/placement + digital signing UI
import { state, $ } from './state.js';
import { showToast } from './toolbar-handlers.js';

let _sigCanvas = null;
let _sigCtx = null;
let _sigDrawing = false;
let _sigColor = '#000000';
let _sigImageData = null; // data URL of the final signature image
let _placementMode = false;

// ─── Signature Modal ─────────────────────────────────

export function openSignatureModal() {
  const modal = $('pdfSignatureModal');
  if (!modal) return;
  modal.classList.add('show');
  _sigImageData = null;
  initDrawTab();
  wireSignatureModalEvents();
}

function closeSignatureModal() {
  const modal = $('pdfSignatureModal');
  if (modal) modal.classList.remove('show');
}

function initDrawTab() {
  _sigCanvas = $('sigDrawCanvas');
  if (!_sigCanvas) return;

  // Set actual canvas resolution (account for hi-DPI)
  const rect = _sigCanvas.getBoundingClientRect();
  const dpr = window.devicePixelRatio || 1;
  const scale = Math.max(dpr, 2); // at least 2x for quality
  _sigCanvas.width = rect.width * scale;
  _sigCanvas.height = rect.height * scale;
  _sigCtx = _sigCanvas.getContext('2d');
  _sigCtx.scale(scale, scale);
  _sigCtx.fillStyle = '#fff';
  _sigCtx.fillRect(0, 0, rect.width, rect.height);
  _sigCtx.strokeStyle = _sigColor;
  _sigCtx.lineWidth = 2;
  _sigCtx.lineCap = 'round';
  _sigCtx.lineJoin = 'round';

  // Drawing events
  _sigCanvas.addEventListener('mousedown', sigMouseDown);
  _sigCanvas.addEventListener('mousemove', sigMouseMove);
  _sigCanvas.addEventListener('mouseup', sigMouseUp);
  _sigCanvas.addEventListener('mouseleave', sigMouseUp);

  // Touch support
  _sigCanvas.addEventListener('touchstart', sigTouchStart, { passive: false });
  _sigCanvas.addEventListener('touchmove', sigTouchMove, { passive: false });
  _sigCanvas.addEventListener('touchend', sigMouseUp);
}

function sigMouseDown(e) {
  _sigDrawing = true;
  const rect = _sigCanvas.getBoundingClientRect();
  _sigCtx.beginPath();
  _sigCtx.moveTo(e.clientX - rect.left, e.clientY - rect.top);
}

function sigMouseMove(e) {
  if (!_sigDrawing) return;
  const rect = _sigCanvas.getBoundingClientRect();
  _sigCtx.lineTo(e.clientX - rect.left, e.clientY - rect.top);
  _sigCtx.stroke();
  _sigCtx.beginPath();
  _sigCtx.moveTo(e.clientX - rect.left, e.clientY - rect.top);
}

function sigMouseUp() {
  _sigDrawing = false;
}

function sigTouchStart(e) {
  e.preventDefault();
  const touch = e.touches[0];
  const rect = _sigCanvas.getBoundingClientRect();
  _sigDrawing = true;
  _sigCtx.beginPath();
  _sigCtx.moveTo(touch.clientX - rect.left, touch.clientY - rect.top);
}

function sigTouchMove(e) {
  e.preventDefault();
  if (!_sigDrawing) return;
  const touch = e.touches[0];
  const rect = _sigCanvas.getBoundingClientRect();
  _sigCtx.lineTo(touch.clientX - rect.left, touch.clientY - rect.top);
  _sigCtx.stroke();
  _sigCtx.beginPath();
  _sigCtx.moveTo(touch.clientX - rect.left, touch.clientY - rect.top);
}

function wireSignatureModalEvents() {
  // Tab switching
  document.querySelectorAll('.sig-tab').forEach(tab => {
    tab.addEventListener('click', () => {
      document.querySelectorAll('.sig-tab').forEach(t => t.classList.remove('active'));
      tab.classList.add('active');
      $('sigDrawTab').style.display = tab.dataset.tab === 'draw' ? '' : 'none';
      $('sigTypeTab').style.display = tab.dataset.tab === 'type' ? '' : 'none';
      $('sigUploadTab').style.display = tab.dataset.tab === 'upload' ? '' : 'none';
    });
  });

  // Color selection
  document.querySelectorAll('.sig-color').forEach(btn => {
    btn.addEventListener('click', () => {
      document.querySelectorAll('.sig-color').forEach(b => b.classList.remove('active'));
      btn.classList.add('active');
      _sigColor = btn.dataset.color;
      if (_sigCtx) _sigCtx.strokeStyle = _sigColor;
    });
  });

  // Clear drawing
  $('sigDrawClear')?.addEventListener('click', () => {
    if (!_sigCtx || !_sigCanvas) return;
    const rect = _sigCanvas.getBoundingClientRect();
    _sigCtx.fillStyle = '#fff';
    _sigCtx.fillRect(0, 0, rect.width, rect.height);
    _sigCtx.strokeStyle = _sigColor;
  });

  // Type tab — live preview
  const typeInput = $('sigTypeInput');
  const fontSelect = $('sigFontSelect');
  const preview = $('sigFontPreview');

  function updateTypePreview() {
    if (!preview || !typeInput) return;
    const text = typeInput.value || 'Your Name';
    const font = fontSelect?.value || 'cursive';
    preview.textContent = text;
    preview.style.fontFamily = font;
    preview.style.color = _sigColor;
  }

  typeInput?.addEventListener('input', updateTypePreview);
  fontSelect?.addEventListener('change', updateTypePreview);
  updateTypePreview();

  // Upload tab
  const uploadArea = $('sigUploadArea');
  const uploadInput = $('sigUploadInput');
  const uploadPreview = $('sigUploadPreview');

  uploadArea?.addEventListener('click', () => uploadInput?.click());
  uploadArea?.addEventListener('dragover', (e) => { e.preventDefault(); uploadArea.style.borderColor = 'var(--accent)'; });
  uploadArea?.addEventListener('dragleave', () => { uploadArea.style.borderColor = ''; });
  uploadArea?.addEventListener('drop', (e) => {
    e.preventDefault();
    uploadArea.style.borderColor = '';
    const file = e.dataTransfer.files[0];
    if (file && file.type.startsWith('image/')) handleUploadFile(file);
  });
  uploadInput?.addEventListener('change', (e) => {
    const file = e.target.files[0];
    if (file) handleUploadFile(file);
  });

  function handleUploadFile(file) {
    const reader = new FileReader();
    reader.onload = () => {
      _sigImageData = reader.result;
      if (uploadPreview) {
        uploadPreview.innerHTML = `<img src="${reader.result}" alt="Signature preview">`;
      }
    };
    reader.readAsDataURL(file);
  }

  // Cancel
  $('sigCancelBtn')?.addEventListener('click', closeSignatureModal);

  // Apply — capture signature image and enter placement mode
  $('sigApplyBtn')?.addEventListener('click', () => {
    const activeTab = document.querySelector('.sig-tab.active')?.dataset.tab || 'draw';

    if (activeTab === 'draw') {
      if (!_sigCanvas) return;
      _sigImageData = _sigCanvas.toDataURL('image/png');
    } else if (activeTab === 'type') {
      // Render typed text to canvas for capture
      const text = typeInput?.value || 'Signature';
      const font = fontSelect?.value || 'cursive';
      const canvas = document.createElement('canvas');
      canvas.width = 400;
      canvas.height = 120;
      const ctx = canvas.getContext('2d');
      ctx.fillStyle = '#fff';
      ctx.fillRect(0, 0, 400, 120);
      ctx.font = `36px ${font}`;
      ctx.fillStyle = _sigColor;
      ctx.textBaseline = 'middle';
      ctx.fillText(text, 20, 60);
      _sigImageData = canvas.toDataURL('image/png');
    }
    // 'upload' tab already sets _sigImageData

    if (!_sigImageData) {
      showToast('No signature to place');
      return;
    }

    closeSignatureModal();
    enterPlacementMode();
  });
}

// ─── Signature Placement ─────────────────────────────

function enterPlacementMode() {
  _placementMode = true;
  const container = $('pdfCanvasContainer');
  if (container) {
    container.style.cursor = 'crosshair';
    container.addEventListener('mousedown', onPlacementClick);
  }
  showToast('Click and drag on a page to place signature');
}

function exitPlacementMode() {
  _placementMode = false;
  const container = $('pdfCanvasContainer');
  if (container) {
    container.style.cursor = '';
    container.removeEventListener('mousedown', onPlacementClick);
  }
}

let _placementStart = null;
let _placementPreview = null;

function onPlacementClick(e) {
  const pageEl = e.target.closest('.pdf-page');
  if (!pageEl) return;
  const pageNum = parseInt(pageEl.dataset.pageNum, 10);
  const rect = pageEl.getBoundingClientRect();
  const x = e.clientX - rect.left;
  const y = e.clientY - rect.top;

  _placementStart = { pageNum, x, y, pageEl };

  // Create preview overlay
  const overlay = state.pdfViewer?.getOverlayLayer(pageNum);
  if (!overlay) return;
  overlay.style.pointerEvents = 'auto';

  _placementPreview = document.createElement('div');
  _placementPreview.className = 'pdf-stamp-overlay';
  _placementPreview.style.left = x + 'px';
  _placementPreview.style.top = y + 'px';
  _placementPreview.style.width = '0px';
  _placementPreview.style.height = '0px';
  _placementPreview.style.border = '2px dashed var(--accent)';
  _placementPreview.style.opacity = '0.7';
  _placementPreview.style.pointerEvents = 'none';

  const img = document.createElement('img');
  img.src = _sigImageData;
  img.style.width = '100%';
  img.style.height = '100%';
  img.style.objectFit = 'contain';
  img.draggable = false;
  _placementPreview.appendChild(img);
  overlay.appendChild(_placementPreview);

  // Listen for drag and release
  const onMove = (ev) => {
    if (!_placementStart) return;
    const nx = ev.clientX - rect.left;
    const ny = ev.clientY - rect.top;
    const left = Math.min(_placementStart.x, nx);
    const top = Math.min(_placementStart.y, ny);
    const width = Math.abs(nx - _placementStart.x);
    const height = Math.abs(ny - _placementStart.y);
    _placementPreview.style.left = left + 'px';
    _placementPreview.style.top = top + 'px';
    _placementPreview.style.width = width + 'px';
    _placementPreview.style.height = height + 'px';
  };

  const onUp = async (ev) => {
    document.removeEventListener('mousemove', onMove);
    document.removeEventListener('mouseup', onUp);

    if (!_placementStart) return;
    const nx = ev.clientX - rect.left;
    const ny = ev.clientY - rect.top;
    const left = Math.min(_placementStart.x, nx);
    const top = Math.min(_placementStart.y, ny);
    let width = Math.abs(nx - _placementStart.x);
    let height = Math.abs(ny - _placementStart.y);

    // Minimum size
    if (width < 40) width = 150;
    if (height < 20) height = 60;

    // Finalize the placement — create annotation
    _placementPreview.style.border = '';
    _placementPreview.style.opacity = '';
    _placementPreview.style.left = left + 'px';
    _placementPreview.style.top = top + 'px';
    _placementPreview.style.width = width + 'px';
    _placementPreview.style.height = height + 'px';

    // Store as stamp annotation (use shared class from pdf-annotations)
    const { PdfAnnotation } = await import('./pdf-annotations.js');
    const ann = new PdfAnnotation('stamp', pageNum, {
      x: left,
      y: top,
      width,
      height,
      imageData: _sigImageData,
      isSignature: true,
    });
    state.pdfAnnotations.push(ann);
    state.pdfModified = true;

    // Refresh annotations panel
    import('./pdf-annotations.js').then(m => {
      m.renderAnnotationsForPage(pageNum);
      m.refreshAnnotationsPanel();
    });

    _placementStart = null;
    _placementPreview = null;
    exitPlacementMode();
    showToast('Signature placed');
  };

  document.addEventListener('mousemove', onMove);
  document.addEventListener('mouseup', onUp);
}

// ─── Digital Signature Modal ─────────────────────────

export function openDigitalSignModal() {
  const modal = $('pdfDigitalSignModal');
  if (!modal) return;
  modal.classList.add('show');
  wireDigitalSignEvents();
}

function closeDigitalSignModal() {
  const modal = $('pdfDigitalSignModal');
  if (modal) modal.classList.remove('show');
}

function wireDigitalSignEvents() {
  const certInput = $('digSignCertInput');
  const passwordInput = $('digSignPassword');
  const certInfo = $('digSignCertInfo');

  // Parse certificate when file is selected
  certInput?.addEventListener('change', async (e) => {
    const file = e.target.files[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = async () => {
      const pfxBytes = new Uint8Array(reader.result);
      const password = passwordInput?.value || '';
      try {
        if (state._wasmPdfEditor) {
          const infoJson = state._wasmPdfEditor.get_cert_info(pfxBytes, password);
          const info = JSON.parse(infoJson);
          if (certInfo) {
            certInfo.style.display = 'block';
            certInfo.innerHTML = `
              <strong>Subject:</strong> ${escapeHtml(info.subject)}<br>
              <strong>Issuer:</strong> ${escapeHtml(info.issuer)}<br>
              <strong>Valid:</strong> ${info.valid_from} to ${info.valid_to}<br>
              <strong>Serial:</strong> ${info.serial}
            `;
          }
        }
      } catch (err) {
        if (certInfo) {
          certInfo.style.display = 'block';
          certInfo.innerHTML = `<span style="color:var(--danger)">Could not read certificate: ${escapeHtml(err.message)}</span>`;
        }
      }
    };
    reader.readAsArrayBuffer(file);
  });

  // Re-check cert when password changes
  passwordInput?.addEventListener('change', () => {
    if (certInput?.files[0]) {
      certInput.dispatchEvent(new Event('change'));
    }
  });

  $('digSignCancelBtn')?.addEventListener('click', closeDigitalSignModal);

  $('digSignApplyBtn')?.addEventListener('click', async () => {
    const file = certInput?.files[0];
    if (!file) {
      showToast('Please select a certificate file', 'error');
      return;
    }
    const password = passwordInput?.value || '';
    const reason = $('digSignReason')?.value || 'Approval';

    const reader = new FileReader();
    reader.onload = async () => {
      const pfxBytes = new Uint8Array(reader.result);
      try {
        if (state._wasmPdfEditor) {
          const signedBytes = state._wasmPdfEditor.sign_pdf(pfxBytes, password, 'Signature1', reason);
          state.pdfBytes = signedBytes;
          state.pdfModified = false;

          // Download signed PDF
          const blob = new Blob([signedBytes], { type: 'application/pdf' });
          const url = URL.createObjectURL(blob);
          const a = document.createElement('a');
          a.href = url;
          a.download = ($('docName').value || 'document') + '_signed.pdf';
          a.click();
          URL.revokeObjectURL(url);

          closeDigitalSignModal();
          showToast('Document signed and saved');
        } else {
          showToast('Digital signatures require WASM PDF editor', 'error');
        }
      } catch (err) {
        showToast('Signing failed: ' + err.message, 'error');
      }
    };
    reader.readAsArrayBuffer(file);
  });
}

function escapeHtml(str) {
  const div = document.createElement('div');
  div.textContent = str || '';
  return div.innerHTML;
}
