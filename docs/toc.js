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
        this.innerHTML = '<ol class="chapter"><li class="chapter-item expanded "><a href="intro.html"><strong aria-hidden="true">1.</strong> Intro to Graphix</a></li><li class="chapter-item expanded "><a href="install.html"><strong aria-hidden="true">2.</strong> Installing Graphix</a></li><li class="chapter-item expanded "><a href="core/overview.html"><strong aria-hidden="true">3.</strong> Core Language</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="core/fundamental_types.html"><strong aria-hidden="true">3.1.</strong> Fundamental Types</a></li><li class="chapter-item expanded "><a href="core/let_binds.html"><strong aria-hidden="true">3.2.</strong> Let Binds</a></li><li class="chapter-item expanded "><a href="core/connect.html"><strong aria-hidden="true">3.3.</strong> Connect</a></li><li class="chapter-item expanded "><a href="core/block.html"><strong aria-hidden="true">3.4.</strong> Blocks</a></li><li class="chapter-item expanded "><a href="core/use.html"><strong aria-hidden="true">3.5.</strong> Use</a></li><li class="chapter-item expanded "><a href="core/select.html"><strong aria-hidden="true">3.6.</strong> Select</a></li><li class="chapter-item expanded "><a href="core/error.html"><strong aria-hidden="true">3.7.</strong> Error Handling</a></li></ol></li><li class="chapter-item expanded "><a href="functions/overview.html"><strong aria-hidden="true">4.</strong> Functions</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="functions/semantics.html"><strong aria-hidden="true">4.1.</strong> Detailed Semantics</a></li><li class="chapter-item expanded "><a href="functions/closures.html"><strong aria-hidden="true">4.2.</strong> Closures and First Class Functions</a></li><li class="chapter-item expanded "><a href="functions/polymorphism.html"><strong aria-hidden="true">4.3.</strong> Polymorphism</a></li><li class="chapter-item expanded "><a href="functions/recursion.html"><strong aria-hidden="true">4.4.</strong> Recursion</a></li></ol></li><li class="chapter-item expanded "><a href="udf/overview.html"><strong aria-hidden="true">5.</strong> User Defined Types</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="udf/structs.html"><strong aria-hidden="true">5.1.</strong> Structs</a></li><li class="chapter-item expanded "><a href="udf/variants.html"><strong aria-hidden="true">5.2.</strong> Variants</a></li><li class="chapter-item expanded "><a href="udf/tuples.html"><strong aria-hidden="true">5.3.</strong> Tuples</a></li><li class="chapter-item expanded "><a href="udf/named.html"><strong aria-hidden="true">5.4.</strong> Named Types</a></li><li class="chapter-item expanded "><a href="udf/polymorphic.html"><strong aria-hidden="true">5.5.</strong> Parametric Polymorphism</a></li><li class="chapter-item expanded "><a href="udf/recursive.html"><strong aria-hidden="true">5.6.</strong> Recursive Types</a></li></ol></li><li class="chapter-item expanded "><a href="modules/overview.html"><strong aria-hidden="true">6.</strong> Modules</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="modules/inline.html"><strong aria-hidden="true">6.1.</strong> Inline Modules</a></li><li class="chapter-item expanded "><a href="modules/external.html"><strong aria-hidden="true">6.2.</strong> External Modules</a></li><li class="chapter-item expanded "><a href="modules/dynamic.html"><strong aria-hidden="true">6.3.</strong> Dynamic Modules</a></li></ol></li><li class="chapter-item expanded "><a href="stdlib/overview.html"><strong aria-hidden="true">7.</strong> The Standard Library</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="stdlib/core.html"><strong aria-hidden="true">7.1.</strong> core</a></li><li class="chapter-item expanded "><a href="stdlib/net.html"><strong aria-hidden="true">7.2.</strong> net</a></li><li class="chapter-item expanded "><a href="stdlib/array.html"><strong aria-hidden="true">7.3.</strong> array</a></li><li class="chapter-item expanded "><a href="stdlib/str.html"><strong aria-hidden="true">7.4.</strong> str</a></li><li class="chapter-item expanded "><a href="stdlib/re.html"><strong aria-hidden="true">7.5.</strong> re</a></li><li class="chapter-item expanded "><a href="stdlib/time.html"><strong aria-hidden="true">7.6.</strong> time</a></li><li class="chapter-item expanded "><a href="stdlib/rand.html"><strong aria-hidden="true">7.7.</strong> rand</a></li></ol></li><li class="chapter-item expanded "><a href="ui/overview.html"><strong aria-hidden="true">8.</strong> Building UIs With Graphix</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="ui/tui/overview.html"><strong aria-hidden="true">8.1.</strong> TUIs</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="ui/tui/barchart.html"><strong aria-hidden="true">8.1.1.</strong> barchart</a></li><li class="chapter-item expanded "><a href="ui/tui/block.html"><strong aria-hidden="true">8.1.2.</strong> block</a></li><li class="chapter-item expanded "><a href="ui/tui/browser.html"><strong aria-hidden="true">8.1.3.</strong> browser</a></li><li class="chapter-item expanded "><a href="ui/tui/calendar.html"><strong aria-hidden="true">8.1.4.</strong> calendar</a></li><li class="chapter-item expanded "><a href="ui/tui/canvas.html"><strong aria-hidden="true">8.1.5.</strong> canvas</a></li><li class="chapter-item expanded "><a href="ui/tui/chart.html"><strong aria-hidden="true">8.1.6.</strong> chart</a></li><li class="chapter-item expanded "><a href="ui/tui/text.html"><strong aria-hidden="true">8.1.7.</strong> text</a></li><li class="chapter-item expanded "><a href="ui/tui/paragraph.html"><strong aria-hidden="true">8.1.8.</strong> paragraph</a></li><li class="chapter-item expanded "><a href="ui/tui/gauge.html"><strong aria-hidden="true">8.1.9.</strong> gauge</a></li><li class="chapter-item expanded "><a href="ui/tui/linegauge.html"><strong aria-hidden="true">8.1.10.</strong> linegauge</a></li><li class="chapter-item expanded "><a href="ui/tui/layout.html"><strong aria-hidden="true">8.1.11.</strong> layout</a></li><li class="chapter-item expanded "><a href="ui/tui/list.html"><strong aria-hidden="true">8.1.12.</strong> list</a></li><li class="chapter-item expanded "><a href="ui/tui/scroll.html"><strong aria-hidden="true">8.1.13.</strong> scroll</a></li><li class="chapter-item expanded "><a href="ui/tui/sparkline.html"><strong aria-hidden="true">8.1.14.</strong> sparkline</a></li><li class="chapter-item expanded "><a href="ui/tui/table.html"><strong aria-hidden="true">8.1.15.</strong> table</a></li><li class="chapter-item expanded "><a href="ui/tui/tabs.html"><strong aria-hidden="true">8.1.16.</strong> tabs</a></li></ol></li></ol></li><li class="chapter-item expanded "><a href="embedding/overview.html"><strong aria-hidden="true">9.</strong> Embedding Graphix</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="embedding/builtins.html"><strong aria-hidden="true">9.1.</strong> Writing Built in Functions</a></li><li class="chapter-item expanded "><a href="embedding/shell.html"><strong aria-hidden="true">9.2.</strong> Using graphix-shell</a></li><li class="chapter-item expanded "><a href="embedding/rt.html"><strong aria-hidden="true">9.3.</strong> Using graphix-rt</a></li></ol></li></ol>';
        // Set the current, active page, and reveal it if it's hidden
        let current_page = document.location.href.toString().split("#")[0].split("?")[0];
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
