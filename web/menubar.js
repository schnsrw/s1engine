/**
 * menubar.js — Google Docs-style dropdown menu bar
 *
 * Creates File/Edit/View/Insert/Format/Tools/Help menus with
 * keyboard shortcuts and Material Icons.
 */

export function createMenuBar(api) {
  var menubar = document.getElementById('menubar');
  if (!menubar) return;

  var menus = {
    'File': [
      { label: 'New', icon: 'note_add', action: function() { location.reload(); } },
      { label: 'Open', icon: 'folder_open', shortcut: '⌘O', action: function() { document.getElementById('file-picker').click(); } },
      { label: 'Save as DOCX', icon: 'save', shortcut: '⌘S', action: function() { window.rudra_save && window.rudra_save(); } },
      { sep: true },
      { label: 'Download as PDF', icon: 'picture_as_pdf', action: function() { alert('PDF export coming soon'); } },
      { label: 'Download as ODT', icon: 'download', action: function() { alert('ODT export coming soon'); } },
      { label: 'Download as TXT', icon: 'text_snippet', action: function() { alert('TXT export coming soon'); } },
      { sep: true },
      { label: 'Page setup', icon: 'tune', action: function() { alert('Page setup coming soon'); } },
      { label: 'Print', icon: 'print', shortcut: '⌘P', action: function() { api.asc_Print && api.asc_Print(); } },
    ],
    'Edit': [
      { label: 'Undo', icon: 'undo', shortcut: '⌘Z', action: function() { api.Undo(); } },
      { label: 'Redo', icon: 'redo', shortcut: '⌘Y', action: function() { api.Redo(); } },
      { sep: true },
      { label: 'Cut', icon: 'content_cut', shortcut: '⌘X', action: function() { api.Cut(); } },
      { label: 'Copy', icon: 'content_copy', shortcut: '⌘C', action: function() { api.Copy(); } },
      { label: 'Paste', icon: 'content_paste', shortcut: '⌘V', action: function() { api.Paste(); } },
      { sep: true },
      { label: 'Select all', icon: 'select_all', shortcut: '⌘A', action: function() { api.asc_EditSelectAll(); } },
      { sep: true },
      { label: 'Find and replace', icon: 'find_replace', shortcut: '⌘⇧H', action: function() { api.asc_searchEnabled && api.asc_searchEnabled(true); } },
    ],
    'View': [
      { label: 'Show ruler', icon: 'straighten', action: function() { /* toggle ruler */ }, toggle: true },
      { label: 'Show non-printing', icon: 'space_bar', shortcut: '⌘⇧P', action: function() { api.ShowParaMarks = !api.ShowParaMarks; api.Resize(); } },
      { sep: true },
      { label: 'Zoom in', icon: 'zoom_in', shortcut: '⌘+', action: function() { if(api.zoomIn) api.zoomIn(); } },
      { label: 'Zoom out', icon: 'zoom_out', shortcut: '⌘-', action: function() { if(api.zoomOut) api.zoomOut(); } },
      { label: 'Fit to width', icon: 'fit_screen', action: function() { if(api.zoomFitToWidth) api.zoomFitToWidth(); } },
    ],
    'Insert': [
      { label: 'Image', icon: 'image', action: function() { document.getElementById('img-picker').click(); } },
      { label: 'Table', icon: 'table', action: function() { api.put_Table && api.put_Table(3, 3); } },
      { label: 'Link', icon: 'link', shortcut: '⌘K', action: function() { var u = prompt('URL:'); if (u) api.add_Hyperlink({ Value: u, Text: u }); } },
      { sep: true },
      { label: 'Comment', icon: 'comment', shortcut: '⌘⌥M', action: function() { api.asc_addComment && api.asc_addComment(); } },
      { label: 'Bookmark', icon: 'bookmark', action: function() { alert('Bookmark coming soon'); } },
      { sep: true },
      { label: 'Page break', icon: 'insert_page_break', action: function() { api.put_AddPageBreak && api.put_AddPageBreak(); } },
      { label: 'Horizontal line', icon: 'horizontal_rule', action: function() { alert('Horizontal line coming soon'); } },
      { sep: true },
      { label: 'Special characters', icon: 'special_character', action: function() { alert('Special characters coming soon'); } },
    ],
    'Format': [
      { label: 'Bold', icon: 'format_bold', shortcut: '⌘B', action: function() { api.put_TextPrBold(undefined); } },
      { label: 'Italic', icon: 'format_italic', shortcut: '⌘I', action: function() { api.put_TextPrItalic(undefined); } },
      { label: 'Underline', icon: 'format_underlined', shortcut: '⌘U', action: function() { api.put_TextPrUnderline(undefined); } },
      { label: 'Strikethrough', icon: 'strikethrough_s', action: function() { api.put_TextPrStrikeout(undefined); } },
      { sep: true },
      { label: 'Align left', icon: 'format_align_left', action: function() { api.put_PrAlign(1); } },
      { label: 'Center', icon: 'format_align_center', action: function() { api.put_PrAlign(2); } },
      { label: 'Align right', icon: 'format_align_right', action: function() { api.put_PrAlign(0); } },
      { label: 'Justify', icon: 'format_align_justify', action: function() { api.put_PrAlign(3); } },
      { sep: true },
      { label: 'Bulleted list', icon: 'format_list_bulleted', action: function() { api.put_ListType(0, 1); } },
      { label: 'Numbered list', icon: 'format_list_numbered', action: function() { api.put_ListType(1, 1); } },
      { sep: true },
      { label: 'Clear formatting', icon: 'format_clear', shortcut: '⌘\\', action: function() { api.ClearFormating && api.ClearFormating(); } },
    ],
    'Tools': [
      { label: 'Word count', icon: 'analytics', shortcut: '⌘⇧C', action: function() { alert('Word count coming soon'); } },
      { label: 'Spelling', icon: 'spellcheck', action: function() { alert('Spell check coming soon'); } },
      { sep: true },
      { label: 'Preferences', icon: 'settings', action: function() { alert('Preferences coming soon'); } },
    ],
    'Help': [
      { label: 'Keyboard shortcuts', icon: 'keyboard', shortcut: '⌘/', action: function() { alert('Keyboard shortcuts:\nCtrl+B: Bold\nCtrl+I: Italic\nCtrl+U: Underline\nCtrl+Z: Undo\nCtrl+Y: Redo\nCtrl+S: Save\nCtrl+P: Print'); } },
      { label: 'About Rudra Office', icon: 'info', action: function() { alert('Rudra Office\nPowered by s1engine + OnlyOffice sdkjs\nAGPL-3.0-or-later'); } },
    ]
  };

  // Build menu HTML
  for (var menuName in menus) {
    var menuBtn = document.createElement('div');
    menuBtn.className = 'menu-item';
    menuBtn.textContent = menuName;
    menuBtn.dataset.menu = menuName;

    var dropdown = document.createElement('div');
    dropdown.className = 'menu-dropdown';
    dropdown.style.display = 'none';

    menus[menuName].forEach(function(item) {
      if (item.sep) {
        var sep = document.createElement('div');
        sep.className = 'menu-sep';
        dropdown.appendChild(sep);
        return;
      }

      var row = document.createElement('div');
      row.className = 'menu-row';
      row.innerHTML = '<span class="mi" style="font-size:16px;width:20px">' + (item.icon || '') + '</span>' +
        '<span class="menu-label">' + item.label + '</span>' +
        (item.shortcut ? '<span class="menu-shortcut">' + item.shortcut + '</span>' : '');
      row.onclick = function(e) {
        e.stopPropagation();
        closeAllMenus();
        item.action();
      };
      dropdown.appendChild(row);
    });

    menuBtn.appendChild(dropdown);
    menuBtn.onclick = function(e) {
      e.stopPropagation();
      var dd = this.querySelector('.menu-dropdown');
      var isOpen = dd.style.display !== 'none';
      closeAllMenus();
      if (!isOpen) dd.style.display = 'block';
    };
    menuBtn.onmouseenter = function() {
      var anyOpen = document.querySelector('.menu-dropdown[style*="display: block"]');
      if (anyOpen) {
        closeAllMenus();
        this.querySelector('.menu-dropdown').style.display = 'block';
      }
    };

    menubar.appendChild(menuBtn);
  }

  // Close menus on click outside
  document.addEventListener('click', closeAllMenus);
}

function closeAllMenus() {
  document.querySelectorAll('.menu-dropdown').forEach(function(d) { d.style.display = 'none'; });
}
