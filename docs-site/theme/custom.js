// Rudra Code — Custom mdbook enhancements

(function() {
    'use strict';

    // ─── Admonition Styling ──────────────────────────
    // Converts blockquotes starting with **Note:**, **Warning:**, **Tip:**, **Important:**
    // into styled admonition boxes
    function styleAdmonitions() {
        var blockquotes = document.querySelectorAll('.content main blockquote');
        blockquotes.forEach(function(bq) {
            var firstP = bq.querySelector('p');
            if (!firstP) return;
            var strong = firstP.querySelector('strong');
            if (!strong) return;
            var text = strong.textContent.trim().toLowerCase().replace(':', '');
            var types = {
                'note': { color: '#1a73e8', bg: '#e8f0fe', icon: 'info', darkBg: '#1a2744', darkColor: '#8ab4f8' },
                'info': { color: '#1a73e8', bg: '#e8f0fe', icon: 'info', darkBg: '#1a2744', darkColor: '#8ab4f8' },
                'tip': { color: '#34a853', bg: '#e6f4ea', icon: 'lightbulb', darkBg: '#1a3a2a', darkColor: '#81c995' },
                'warning': { color: '#f9ab00', bg: '#fef7e0', icon: 'warning', darkBg: '#3a3018', darkColor: '#fdd663' },
                'caution': { color: '#f9ab00', bg: '#fef7e0', icon: 'warning', darkBg: '#3a3018', darkColor: '#fdd663' },
                'important': { color: '#ea4335', bg: '#fce8e6', icon: 'error', darkBg: '#3a1a1a', darkColor: '#f28b82' },
                'danger': { color: '#ea4335', bg: '#fce8e6', icon: 'error', darkBg: '#3a1a1a', darkColor: '#f28b82' }
            };
            var cfg = types[text];
            if (!cfg) return;
            bq.setAttribute('data-admonition', text);
            bq.style.borderLeftColor = cfg.color;
            bq.style.background = cfg.bg;
            bq.style.borderRadius = '0 8px 8px 0';
            bq.style.padding = '12px 20px 12px 20px';
            // Style the label
            strong.style.color = cfg.color;
            strong.style.textTransform = 'uppercase';
            strong.style.fontSize = '12px';
            strong.style.letterSpacing = '0.05em';
        });
    }

    // ─── External Link Indicators ────────────────────
    // Add arrow icon to external links
    function markExternalLinks() {
        var links = document.querySelectorAll('.content main a[href^="http"]');
        links.forEach(function(a) {
            if (a.hostname === window.location.hostname) return;
            a.setAttribute('target', '_blank');
            a.setAttribute('rel', 'noopener noreferrer');
            if (!a.querySelector('.external-icon')) {
                var icon = document.createElement('span');
                icon.className = 'external-icon';
                icon.textContent = ' \u2197';
                icon.style.fontSize = '0.75em';
                icon.style.opacity = '0.5';
                a.appendChild(icon);
            }
        });
    }

    // ─── Table of Contents Progress ──────────────────
    // Highlight current section in sidebar based on scroll position
    function initScrollSpy() {
        var headings = document.querySelectorAll('.content main h2[id], .content main h3[id]');
        if (headings.length === 0) return;
        var ticking = false;
        window.addEventListener('scroll', function() {
            if (!ticking) {
                requestAnimationFrame(function() {
                    var scrollTop = window.scrollY + 100;
                    var current = null;
                    headings.forEach(function(h) {
                        if (h.offsetTop <= scrollTop) current = h;
                    });
                    // Could be used for a TOC sidebar highlight
                    ticking = false;
                });
                ticking = true;
            }
        });
    }

    // ─── Keyboard Shortcuts ──────────────────────────
    // / to focus search, Esc to close
    document.addEventListener('keydown', function(e) {
        if (e.key === '/' && !isInputFocused()) {
            e.preventDefault();
            var searchbar = document.getElementById('searchbar');
            if (searchbar) searchbar.focus();
        }
    });

    function isInputFocused() {
        var tag = document.activeElement ? document.activeElement.tagName : '';
        return tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT';
    }

    // ─── Back to Top Button ──────────────────────────
    function initBackToTop() {
        var btn = document.createElement('button');
        btn.id = 'back-to-top';
        btn.setAttribute('aria-label', 'Back to top');
        btn.textContent = '\u2191';
        btn.style.cssText = 'position:fixed;bottom:32px;right:32px;width:40px;height:40px;border-radius:50%;border:1px solid #dadce0;background:#fff;color:#5f6368;font-size:18px;cursor:pointer;opacity:0;transition:opacity 0.2s ease,transform 0.2s ease;transform:translateY(8px);z-index:100;box-shadow:0 2px 8px rgba(0,0,0,0.1);display:flex;align-items:center;justify-content:center;';
        document.body.appendChild(btn);
        btn.addEventListener('click', function() {
            window.scrollTo({ top: 0, behavior: 'smooth' });
        });
        var visible = false;
        window.addEventListener('scroll', function() {
            var show = window.scrollY > 400;
            if (show !== visible) {
                visible = show;
                btn.style.opacity = show ? '1' : '0';
                btn.style.transform = show ? 'translateY(0)' : 'translateY(8px)';
                btn.style.pointerEvents = show ? 'auto' : 'none';
            }
        });
    }

    // ─── Init ────────────────────────────────────────
    function init() {
        styleAdmonitions();
        markExternalLinks();
        initScrollSpy();
        initBackToTop();
    }

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', init);
    } else {
        init();
    }
})();
