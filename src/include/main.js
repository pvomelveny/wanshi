// Mobile TOC: responsive open / close behavior for the table of contents.
//
// Listen for the breakpoint actually flipping, not for every resize: on
// mobile, collapsing the URL bar and opening the keyboard both fire `resize`,
// and forcing the state each time snapped shut a TOC the reader had just
// opened.
const tocBreakpoint = window.matchMedia("(max-width: 1000px)");

function toggleDetailsOpen() {
  const details = document.querySelector("#toc>div>details");
  if (!details) return;

  if (tocBreakpoint.matches) {
    details.removeAttribute("open");
  } else {
    details.setAttribute("open", "");
  }
}

document.addEventListener("DOMContentLoaded", toggleDetailsOpen);
tocBreakpoint.addEventListener("change", toggleDetailsOpen);

// Theme: option selection and persistence. The SPSA color-invert solver that
// used to live here served a `.color-invert` class nothing emits — wanshi
// ships one locked theme — and cost every visitor 1500+ solver iterations on
// first load. Deleted with the owner's sign-off; it lives in git history and
// upstream kodama if themes ever return.
const WANSHI_THEME_KEY = `wanshi-theme`;

function storeSelectedTheme(name) {
  localStorage.setItem(WANSHI_THEME_KEY, name);
}

function getCurrentTheme() {
  return localStorage.getItem(WANSHI_THEME_KEY) || window.primaryTheme;
}

function selectTheme(themeName) {
  storeSelectedTheme(themeName);
}

function applyFavorTheme() {
  const favoredTheme = getCurrentTheme();
  if (favoredTheme) {
    const selector = `input[type='radio'][id='${favoredTheme}']`;
    const favoredOption = window.themeOptions.querySelector(selector);

    if (favoredOption) {
      favoredOption.checked = true;
    } else if (window.primaryOption) {
      // Fallback to primary theme, if the favored theme is not found
      // e.g., when the theme options have changed, and the stored theme is no longer available.
      window.primaryOption.checked = true;
    }
  }
}

document.addEventListener("DOMContentLoaded", function () {
  window.themeOptions = document.getElementById("theme-options");
  const templateContent = document.getElementById("theme-option-template").content;

  customElements.define(
    "theme-option",
    class extends HTMLElement {
      constructor() {
        super();

        const node = templateContent.cloneNode(true);
        const themeName = this.getAttribute("name");

        const input = node.querySelector("input");
        input.setAttribute("id", themeName);
        input.setAttribute("value", themeName);

        const label = node.querySelector("label");
        label.setAttribute("for", themeName);
        label.addEventListener("click", () => selectTheme(themeName));

        while (this.firstChild) {
          label.appendChild(this.firstChild);
        }
        this.appendChild(node);
      }
    },
  );

  window.primaryOption = window.themeOptions.querySelector("input[type='radio']");
  window.primaryTheme = primaryOption?.value;

  applyFavorTheme();
});

// Search: token-prefix matching over the inverted index in wanshi.search.json.
//
// The index maps tokens to section ids rather than storing text, so body
// matches tell us *which* notes matched but not where. Titles, taxons and slugs
// ship as text and are matched directly, which is also what drives ranking:
// a title hit outranks a body hit.
(function () {
  const MAX_RESULTS = 30;

  let index = null;
  let loading = null;

  function tokenize(text) {
    return text
      .toLowerCase()
      .split(/[^\p{L}\p{N}]+/u)
      .filter((t) => t.length > 0);
  }

  function loadIndex(url) {
    if (index) return Promise.resolve(index);
    if (loading) return loading;
    loading = fetch(url)
      .then((r) => (r.ok ? r.json() : Promise.reject(new Error(r.status))))
      .then((data) => {
        index = data;
        return index;
      })
      .catch((err) => {
        console.warn("wanshi: could not load search index", err);
        index = { docs: [], tokens: {} };
        return index;
      });
    return loading;
  }

  // Every token must match somewhere (AND), by prefix, so "mono" finds
  // "monoid" and "free mono" narrows rather than widens.
  function search(query) {
    const terms = tokenize(query);
    if (terms.length === 0) return [];

    const docs = index.docs || [];
    const tokens = index.tokens || {};
    const tokenKeys = Object.keys(tokens);

    let candidates = null;
    const scores = new Map();

    for (const term of terms) {
      const hits = new Set();

      docs.forEach((doc, id) => {
        const title = (doc.title || "").toLowerCase();
        const slug = (doc.slug || "").toLowerCase();
        const taxon = (doc.taxon || "").toLowerCase();
        let weight = 0;
        if (title.startsWith(term)) weight = 6;
        else if (title.includes(term)) weight = 4;
        else if (taxon.startsWith(term)) weight = 2;
        else if (slug.includes(term)) weight = 2;
        if (weight > 0) {
          hits.add(id);
          scores.set(id, (scores.get(id) || 0) + weight);
        }
      });

      for (const key of tokenKeys) {
        if (!key.startsWith(term)) continue;
        // An exact token is a stronger signal than a longer word it prefixes.
        const weight = key === term ? 2 : 1;
        for (const id of tokens[key]) {
          hits.add(id);
          scores.set(id, (scores.get(id) || 0) + weight);
        }
      }

      candidates =
        candidates === null
          ? hits
          : new Set([...candidates].filter((id) => hits.has(id)));
      if (candidates.size === 0) break;
    }

    return [...(candidates || [])]
      .sort((a, b) => {
        const diff = (scores.get(b) || 0) - (scores.get(a) || 0);
        if (diff !== 0) return diff;
        return (docs[a].title || "").localeCompare(docs[b].title || "");
      })
      .slice(0, MAX_RESULTS)
      .map((id) => docs[id]);
  }

  function render(results, query, container) {
    if (query.trim() === "") {
      container.innerHTML = "";
      return;
    }
    if (results.length === 0) {
      container.innerHTML = '<p class="search-empty">No matches.</p>';
      return;
    }

    const list = document.createElement("ul");
    list.className = "block";
    for (const doc of results) {
      const item = document.createElement("li");
      item.className = "entry";

      if (doc.date) {
        const date = document.createElement("span");
        date.className = "date";
        date.textContent = doc.date;
        item.appendChild(date);
      }

      const link = document.createElement("a");
      link.className = "link local";
      link.href = doc.url;
      link.title = doc.title + " [" + doc.slug + "]";

      if (doc.taxon) {
        const taxon = document.createElement("span");
        taxon.className = "taxon";
        taxon.textContent = doc.taxon + ". ";
        link.appendChild(taxon);
      }
      const title = document.createElement("span");
      title.className = "title";
      title.textContent = doc.title;
      link.appendChild(title);

      item.appendChild(link);
      list.appendChild(item);
    }
    container.innerHTML = "";
    container.appendChild(list);
  }

  document.addEventListener("DOMContentLoaded", function () {
    const root = document.getElementById("search");
    if (!root) return;

    const input = document.getElementById("search-input");
    const results = document.getElementById("search-results");
    const catalog = document.querySelector("#toc>div.block");
    const url = root.getAttribute("data-index");
    if (!input || !results || !url) return;

    // Only now is the control usable, so only now does it appear.
    root.removeAttribute("hidden");

    const update = function () {
      const query = input.value;
      // While searching, results stand in for the page's own contents.
      if (catalog) catalog.hidden = query.trim() !== "";
      if (query.trim() === "") {
        render([], query, results);
        return;
      }
      loadIndex(url).then(() => render(search(query), query, results));
    };

    input.addEventListener("input", update);

    input.addEventListener("keydown", function (event) {
      if (event.key === "Escape") {
        input.value = "";
        update();
        input.blur();
      }
    });

    // "/" is the conventional focus-search key; ignore it while typing.
    document.addEventListener("keydown", function (event) {
      if (event.key !== "/" || event.metaKey || event.ctrlKey || event.altKey) return;
      const active = document.activeElement;
      const tag = active ? active.tagName : "";
      if (tag === "INPUT" || tag === "TEXTAREA" || (active && active.isContentEditable)) return;
      event.preventDefault();
      // Ensure the sidebar is open before focusing, on narrow screens.
      const details = document.querySelector("#toc>div>details");
      if (details) details.setAttribute("open", "");
      input.focus();
      input.select();
    });

    // Warm the index on first focus so the first keystroke feels instant.
    input.addEventListener("focus", function () {
      loadIndex(url);
    }, { once: true });
  });
})();

// Pinning: hold the hovered containment path open with a click.
//
// The bars normally trace whatever the pointer is inside and vanish when it
// leaves. A click freezes the path so it survives looking away — reading a
// deeply nested note while keeping sight of what holds it. A second click on
// the same section releases it, as does a click on prose belonging to no
// nested section.
(function () {
  const PIN_SELECTOR = "article section.block section.block";
  let pinned = null;

  function article() {
    return document.querySelector("article");
  }

  function clearPin() {
    document
      .querySelectorAll("section.block.pinned")
      .forEach((section) => section.classList.remove("pinned"));
    const root = article();
    if (root) root.classList.remove("has-pin");
    pinned = null;
  }

  function setPin(section) {
    clearPin();
    const root = article();
    if (!root) return;
    // The whole chain, so the brackets a click freezes are the ones the pointer
    // was already drawing. The page's own section is marked too and no rule
    // matches it, which is the same way it stays out of the hover.
    for (let node = section; node && node !== root; node = node.parentElement) {
      if (node.matches("section.block")) node.classList.add("pinned");
    }
    root.classList.add("has-pin");
    pinned = section;
  }

  document.addEventListener("click", function (event) {
    const root = article();
    if (!root || !root.contains(event.target)) return;

    // A click that ends a drag is a text selection, not a pick.
    const selection = window.getSelection();
    if (selection && selection.toString().length > 0) return;

    // Everything here already means something: a summary opens and closes its
    // section, and the rest navigate. None of them should also move the pin.
    if (event.target.closest("summary, a, button, input, [onclick]")) return;

    const section = event.target.closest(PIN_SELECTOR);
    if (!section || section === pinned) {
      clearPin();
      return;
    }
    setPin(section);
  });

  // Escape releases, the way it does for the search box. Registered separately
  // rather than folded into that handler, which only runs when a search index
  // is configured.
  document.addEventListener("keydown", function (event) {
    if (event.key === "Escape" && pinned) clearPin();
  });
})();
