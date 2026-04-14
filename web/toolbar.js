/**
 * toolbar.js — Toolbar event wiring and state management
 *
 * Handles toolbar button clicks, status bar updates, image insertion,
 * and PDF export. Separated from index.html for maintainability.
 */

export function initToolbar(api) {
  // Status bar updates
  api.asc_registerCallback('asc_onCountPages', function(count) {
    var el = document.getElementById('sb-page');
    if (el) el.textContent = 'Pages: ' + count;
  });

  api.asc_registerCallback('asc_onCurrentPage', function(page) {
    var total = api.asc_getCountPages ? api.asc_getCountPages() : '?';
    var el = document.getElementById('sb-page');
    if (el) el.textContent = 'Page ' + (page + 1) + ' of ' + total;
  });

  // Track formatting state and update toolbar buttons
  api.asc_registerCallback('asc_onBold', function(isBold) {
    toggleActive('tb-bold', isBold);
  });
  api.asc_registerCallback('asc_onItalic', function(isItalic) {
    toggleActive('tb-italic', isItalic);
  });
  api.asc_registerCallback('asc_onUnderline', function(isUnderline) {
    toggleActive('tb-underline', isUnderline);
  });
  api.asc_registerCallback('asc_onStrikeout', function(isStrike) {
    toggleActive('tb-strike', isStrike);
  });
  api.asc_registerCallback('asc_onFontFamily', function(font) {
    var el = document.getElementById('tb-font');
    if (el && font && font.get_Name) el.value = font.get_Name();
  });
  api.asc_registerCallback('asc_onFontSize', function(size) {
    var el = document.getElementById('tb-size');
    if (el && size) el.value = size;
  });
  api.asc_registerCallback('asc_onPrAlign', function(align) {
    // Update alignment button states
    document.querySelectorAll('[data-align]').forEach(function(btn) {
      btn.classList.toggle('active', parseInt(btn.dataset.align) === align);
    });
  });

  // Image picker
  var imgPicker = document.getElementById('img-picker');
  if (imgPicker) {
    imgPicker.addEventListener('change', function(ev) {
      var file = ev.target.files[0];
      if (!file) return;
      var reader = new FileReader();
      reader.onload = function(e) {
        if (api.asc_addImage) {
          api.asc_addImage([e.target.result]);
        }
      };
      reader.readAsDataURL(file);
      ev.target.value = ''; // reset for re-selection
    });
  }
}

function toggleActive(id, state) {
  var el = document.getElementById(id);
  if (el) el.classList.toggle('active', !!state);
}

/**
 * Wire save button and keyboard shortcut
 */
export function initSave(saveFn) {
  window.rudra_save = saveFn;

  // Show save button after first doc open
  var btn = document.getElementById('save-btn');
  if (btn) btn.style.display = '';

  // Ctrl+S handler
  document.addEventListener('keydown', function(e) {
    if ((e.ctrlKey || e.metaKey) && e.key === 's') {
      e.preventDefault();
      saveFn();
    }
  });
}
