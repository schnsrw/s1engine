// Populate the sidebar
//
// This is a script, and not included directly in the page, to control the total size of the book.
// The TOC contains an entry for each page, so if each page includes a copy of the TOC,
// the total size of the page becomes O(n**2).
class MDBookSidebarScrollbox extends HTMLElement {
    constructor() {
        super();
    }
    connectedCallback() {
        this.innerHTML = '<ol class="chapter"><li class="chapter-item expanded affix "><a href="introduction.html">Introduction</a></li><li class="chapter-item expanded affix "><li class="part-title">Getting Started</li><li class="chapter-item expanded "><a href="getting-started/quick-start.html"><strong aria-hidden="true">1.</strong> Quick Start</a></li><li class="chapter-item expanded "><a href="getting-started/installation.html"><strong aria-hidden="true">2.</strong> Installation</a><a class="toggle"><div>❱</div></a></li><li><ol class="section"><li class="chapter-item "><a href="getting-started/rust.html"><strong aria-hidden="true">2.1.</strong> Rust Library</a></li><li class="chapter-item "><a href="getting-started/npm.html"><strong aria-hidden="true">2.2.</strong> npm / WASM</a></li></ol></li><li class="chapter-item expanded "><li class="part-title">Guides</li><li class="chapter-item expanded "><a href="guides/react.html"><strong aria-hidden="true">3.</strong> Embed in React</a></li><li class="chapter-item expanded "><a href="guides/vue.html"><strong aria-hidden="true">4.</strong> Embed in Vue</a></li><li class="chapter-item expanded "><a href="guides/conversion.html"><strong aria-hidden="true">5.</strong> Format Conversion</a></li><li class="chapter-item expanded "><a href="guides/collaboration.html"><strong aria-hidden="true">6.</strong> Collaboration Setup</a></li><li class="chapter-item expanded "><a href="guides/white-label.html"><strong aria-hidden="true">7.</strong> White-Labeling</a></li><li class="chapter-item expanded "><a href="guides/configuration.html"><strong aria-hidden="true">8.</strong> Configuration</a></li><li class="chapter-item expanded affix "><li class="part-title">Deployment</li><li class="chapter-item expanded "><a href="getting-started/docker.html"><strong aria-hidden="true">9.</strong> Docker</a></li><li class="chapter-item expanded "><a href="guides/self-hosting.html"><strong aria-hidden="true">10.</strong> Self-Hosting</a></li><li class="chapter-item expanded affix "><li class="part-title">API Reference</li><li class="chapter-item expanded "><a href="api/rust.html"><strong aria-hidden="true">11.</strong> Rust API</a></li><li class="chapter-item expanded "><a href="api/wasm.html"><strong aria-hidden="true">12.</strong> WASM / JavaScript API</a></li><li class="chapter-item expanded "><a href="api/c-ffi.html"><strong aria-hidden="true">13.</strong> C FFI API</a></li><li class="chapter-item expanded "><a href="api/rest.html"><strong aria-hidden="true">14.</strong> REST API</a></li><li class="chapter-item expanded "><a href="api/websocket.html"><strong aria-hidden="true">15.</strong> WebSocket Protocol</a></li><li class="chapter-item expanded affix "><li class="part-title">Architecture</li><li class="chapter-item expanded "><a href="architecture/overview.html"><strong aria-hidden="true">16.</strong> Overview</a></li><li class="chapter-item expanded "><a href="architecture/model.html"><strong aria-hidden="true">17.</strong> Document Model</a></li><li class="chapter-item expanded "><a href="architecture/operations.html"><strong aria-hidden="true">18.</strong> Operations &amp; CRDT</a></li><li class="chapter-item expanded "><a href="architecture/formats.html"><strong aria-hidden="true">19.</strong> Format Crates</a></li><li class="chapter-item expanded "><a href="architecture/layout.html"><strong aria-hidden="true">20.</strong> Layout Engine</a></li><li class="chapter-item expanded affix "><li class="part-title">Contributing</li><li class="chapter-item expanded "><a href="contributing/setup.html"><strong aria-hidden="true">21.</strong> Development Setup</a></li><li class="chapter-item expanded "><a href="contributing/rules.html"><strong aria-hidden="true">22.</strong> Architecture Rules</a></li><li class="chapter-item expanded "><a href="contributing/testing.html"><strong aria-hidden="true">23.</strong> Testing</a></li></ol>';
        // Set the current, active page, and reveal it if it's hidden
        let current_page = document.location.href.toString().split("#")[0];
        if (current_page.endsWith("/")) {
            current_page += "index.html";
        }
        var links = Array.prototype.slice.call(this.querySelectorAll("a"));
        var l = links.length;
        for (var i = 0; i < l; ++i) {
            var link = links[i];
            var href = link.getAttribute("href");
            if (href && !href.startsWith("#") && !/^(?:[a-z+]+:)?\/\//.test(href)) {
                link.href = path_to_root + href;
            }
            // The "index" page is supposed to alias the first chapter in the book.
            if (link.href === current_page || (i === 0 && path_to_root === "" && current_page.endsWith("/index.html"))) {
                link.classList.add("active");
                var parent = link.parentElement;
                if (parent && parent.classList.contains("chapter-item")) {
                    parent.classList.add("expanded");
                }
                while (parent) {
                    if (parent.tagName === "LI" && parent.previousElementSibling) {
                        if (parent.previousElementSibling.classList.contains("chapter-item")) {
                            parent.previousElementSibling.classList.add("expanded");
                        }
                    }
                    parent = parent.parentElement;
                }
            }
        }
        // Track and set sidebar scroll position
        this.addEventListener('click', function(e) {
            if (e.target.tagName === 'A') {
                sessionStorage.setItem('sidebar-scroll', this.scrollTop);
            }
        }, { passive: true });
        var sidebarScrollTop = sessionStorage.getItem('sidebar-scroll');
        sessionStorage.removeItem('sidebar-scroll');
        if (sidebarScrollTop) {
            // preserve sidebar scroll position when navigating via links within sidebar
            this.scrollTop = sidebarScrollTop;
        } else {
            // scroll sidebar to current active section when navigating via "next/previous chapter" buttons
            var activeSection = document.querySelector('#sidebar .active');
            if (activeSection) {
                activeSection.scrollIntoView({ block: 'center' });
            }
        }
        // Toggle buttons
        var sidebarAnchorToggles = document.querySelectorAll('#sidebar a.toggle');
        function toggleSection(ev) {
            ev.currentTarget.parentElement.classList.toggle('expanded');
        }
        Array.from(sidebarAnchorToggles).forEach(function (el) {
            el.addEventListener('click', toggleSection);
        });
    }
}
window.customElements.define("mdbook-sidebar-scrollbox", MDBookSidebarScrollbox);
