// Mobile TOC: responsive open / close behavior for the table of contents.
function toggleDetailsOpen() {
  const details = document.querySelector("#toc>div>details");
  if (!details) return;

  if (window.matchMedia("(max-width: 1000px)").matches) {
    details.removeAttribute("open");
  } else {
    details.setAttribute("open", "");
  }
}

document.addEventListener("DOMContentLoaded", toggleDetailsOpen);
window.addEventListener("resize", toggleDetailsOpen);

// Theme: option selection, persistence, and dynamic color invert logic.
const WANSHI_THEME_KEY = `wanshi-theme`;

function storeSelectedTheme(name) {
  localStorage.setItem(WANSHI_THEME_KEY, name);
}

function getCurrentTheme() {
  return localStorage.getItem(WANSHI_THEME_KEY) || window.primaryTheme;
}

function selectTheme(themeName) {
  storeSelectedTheme(themeName);
  requestAnimationFrame(applyDynamicColorInvert);
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
  applyDynamicColorInvert();
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

/// SPSA filter solver
/// 
/// Also see: 
/// - https://en.wikipedia.org/wiki/Simultaneous_perturbation_stochastic_approximation
/// - https://github.com/pranjalworm/css-filter-generator
/// 
/// Principle
/// 
/// invert → sepia → saturate → hue-rotate → brightness → contrast
/// loss = |ΔR| + |ΔG| + |ΔB| + |ΔH| + |ΔS| + |ΔL|
/// 

const clamp = (value) => Math.max(0, Math.min(255, value));

const multiply = ([r, g, b], matrix) => {
  const nr = clamp(r * matrix[0] + g * matrix[1] + b * matrix[2]);
  const ng = clamp(r * matrix[3] + g * matrix[4] + b * matrix[5]);
  const nb = clamp(r * matrix[6] + g * matrix[7] + b * matrix[8]);
  return [nr, ng, nb];
};

const hueRotate = ([r, g, b], angle = 0) => {
  const rad = (angle / 180) * Math.PI;
  const sin = Math.sin(rad);
  const cos = Math.cos(rad);
  const matrix = [
    0.213 + cos * 0.787 - sin * 0.213,
    0.715 - cos * 0.715 - sin * 0.715,
    0.072 - cos * 0.072 + sin * 0.928,
    0.213 - cos * 0.213 + sin * 0.143,
    0.715 + cos * 0.285 + sin * 0.140,
    0.072 - cos * 0.072 - sin * 0.283,
    0.213 - cos * 0.213 - sin * 0.787,
    0.715 - cos * 0.715 + sin * 0.715,
    0.072 + cos * 0.928 + sin * 0.072,
  ];
  return multiply([r, g, b], matrix);
};

const grayscale = ([r, g, b], value = 1) => {
  const matrix = [
    0.2126 + 0.7874 * (1 - value),
    0.7152 - 0.7152 * (1 - value),
    0.0722 - 0.0722 * (1 - value),
    0.2126 - 0.2126 * (1 - value),
    0.7152 + 0.2848 * (1 - value),
    0.0722 - 0.0722 * (1 - value),
    0.2126 - 0.2126 * (1 - value),
    0.7152 - 0.7152 * (1 - value),
    0.0722 + 0.9278 * (1 - value),
  ];
  return multiply([r, g, b], matrix);
};

const sepia = ([r, g, b], value = 1) => {
  const matrix = [
    0.393 + 0.607 * (1 - value),
    0.769 - 0.769 * (1 - value),
    0.189 - 0.189 * (1 - value),
    0.349 - 0.349 * (1 - value),
    0.686 + 0.314 * (1 - value),
    0.168 - 0.168 * (1 - value),
    0.272 - 0.272 * (1 - value),
    0.534 - 0.534 * (1 - value),
    0.131 + 0.869 * (1 - value),
  ];
  return multiply([r, g, b], matrix);
};

const saturate = ([r, g, b], value = 1) => {
  const matrix = [
    0.213 + 0.787 * value,
    0.715 - 0.715 * value,
    0.072 - 0.072 * value,
    0.213 - 0.213 * value,
    0.715 + 0.285 * value,
    0.072 - 0.072 * value,
    0.213 - 0.213 * value,
    0.715 - 0.715 * value,
    0.072 + 0.928 * value,
  ];
  return multiply([r, g, b], matrix);
};

const linear = ([r, g, b], slope = 1, intercept = 0) => {
  const scale = 255;
  return [
    clamp(r * slope + intercept * scale),
    clamp(g * slope + intercept * scale),
    clamp(b * slope + intercept * scale),
  ];
};

const brightness = (rgb, value = 1) => linear(rgb, value);
const contrast = (rgb, value = 1) => linear(rgb, value, -0.5 * value + 0.5);

const invert = ([r, g, b], value = 1) => {
  const nr = (value + r / 255 * (1 - 2 * value)) * 255;
  const ng = (value + g / 255 * (1 - 2 * value)) * 255;
  const nb = (value + b / 255 * (1 - 2 * value)) * 255;
  return [clamp(nr), clamp(ng), clamp(nb)];
};

// Code taken from https://stackoverflow.com/a/9493060/2688027, licensed under CC BY-SA.
const rgbToHsl = (r, g, b) => {
  r /= 255; g /= 255; b /= 255;
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  let h, s, l = (max + min) / 2;
  if (max === min) {
    h = s = 0;
  } else {
    const d = max - min;
    s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
    switch (max) {
      case r: h = (g - b) / d + (g < b ? 6 : 0); break;
      case g: h = (b - r) / d + 2; break;
      case b: h = (r - g) / d + 4; break;
    }
    h /= 6;
  }
  return { h: h * 100, s: s * 100, l: l * 100 };
};

const applyFilters = (rgb, [inv, sep, sat, hue, bri, con]) => {
  let color = [...rgb];
  color = invert(color, inv / 100);
  color = sepia(color, sep / 100);
  color = saturate(color, sat / 100);
  color = hueRotate(color, hue * 3.6);
  color = brightness(color, bri / 100);
  color = contrast(color, con / 100);
  return color;
};

const lossFn = (targetRgb, filters) => {
  const resultRgb = applyFilters([0, 0, 0], filters);
  const targetHsl = rgbToHsl(...targetRgb);
  const resultHsl = rgbToHsl(...resultRgb);
  return (
    Math.abs(resultRgb[0] - targetRgb[0]) +
    Math.abs(resultRgb[1] - targetRgb[1]) +
    Math.abs(resultRgb[2] - targetRgb[2]) +
    Math.abs(resultHsl.h - targetHsl.h) +
    Math.abs(resultHsl.s - targetHsl.s) +
    Math.abs(resultHsl.l - targetHsl.l)
  );
};

const fix = (value, idx) => {
  let max = 100;
  if (idx === 2) max = 7500;                  // saturate
  else if (idx === 4 || idx === 5) max = 200; // brightness, contrast
  if (idx === 3) { // hue-rotate
    if (value > max) return value % max;
    if (value < 0) return max + (value % max);
    return value;
  }
  return Math.max(0, Math.min(max, value));
};

const spsa = (targetRgb, A, a, c, initial, iters) => {
  const alpha = 1;
  const gamma = 1 / 6;
  let values = [...initial];
  let best = [...values];
  let bestLoss = Infinity;

  for (let k = 0; k < iters; k++) {
    const ck = c / Math.pow(k + 1, gamma);
    const deltas = Array(6).fill().map(() => (Math.random() > 0.5 ? 1 : -1));
    const highArgs = values.map((v, i) => v + ck * deltas[i]);
    const lowArgs = values.map((v, i) => v - ck * deltas[i]);
    const lossDiff = lossFn(targetRgb, highArgs) - lossFn(targetRgb, lowArgs);

    values = values.map((v, i) => {
      const g = (lossDiff / (2 * ck)) * deltas[i];
      const ak = a[i] / Math.pow(A + k + 1, alpha);
      return fix(v - ak * g, i);
    });

    const loss = lossFn(targetRgb, values);
    if (loss < bestLoss) {
      best = [...values];
      bestLoss = loss;
    }
  }
  return { values: best, loss: bestLoss };
};

// Threshold for acceptable color loss. This value determines when the SPSA algorithm stops early.
// A loss below this value is considered visually indistinguishable for most use cases.
// The value 25 was determined empirically to balance performance and color accuracy.
const ACCEPTABLE_COLOR_LOSS = 25;

const solveWide = (targetRgb) => {
  const A = 5;
  const c = 15;
  const a = [60, 180, 18000, 600, 1.2, 1.2];
  let best = { loss: Infinity, values: null };
  for (let i = 0; best.loss > ACCEPTABLE_COLOR_LOSS && i < 3; i++) {
    const initial = [50, 20, 3750, 50, 100, 100];
    const result = spsa(targetRgb, A, a, c, initial, 1000);
    if (result.loss < best.loss) best = result;
  }
  return best;
};

const solveNarrow = (targetRgb, wideResult) => {
  const A = wideResult.loss;
  const c = 2;
  const A1 = A + 1;
  const a = [0.25 * A1, 0.25 * A1, A1, 0.25 * A1, 0.2 * A1, 0.2 * A1];
  return spsa(targetRgb, A, a, c, wideResult.values, 500);
};

const cssFilterString = (filters) => {
  const [inv, sep, sat, hue, bri, con] = filters.map(Math.round);
  const hueDeg = Math.round(hue * 3.6);
  return `filter: invert(${inv}%) sepia(${sep}%) saturate(${sat}%) hue-rotate(${hueDeg}deg) brightness(${bri}%) contrast(${con}%);`;
};

const solveColor = (targetRgb) => {
  const wide = solveWide(targetRgb);
  const narrow = solveNarrow(targetRgb, wide);
  return {
    values: narrow.values,
    loss: narrow.loss,
    filter: cssFilterString(narrow.values),
  };
};

// Reuse a single canvas / context to avoid memory leaks
const colorParseCanvas = document.createElement('canvas');
colorParseCanvas.width = 1;
colorParseCanvas.height = 1;
const colorParseContext = colorParseCanvas.getContext('2d');

const toRGBArray = (cssColor) => {
  colorParseContext.fillStyle = cssColor;
  colorParseContext.fillRect(0, 0, 1, 1);

  // The `getImageData().data` returns a Uint8ClampedArray with 4 values per pixel (RGBA). 
  // also see: https://developer.mozilla.org/zh-CN/docs/Web/API/ImageData/data
  const rgba = colorParseContext.getImageData(0, 0, 1, 1).data;
  return Array.from(rgba.slice(0, 3));
};

const DYNAMIC_STYLE_ELEMENT_ID = 'dynamic-color-invert-style';

function memorizedSolvedFilter(color) {
  const colorKey = `solved-${color}`;
  const storedFilterOption = localStorage.getItem(colorKey);
  if (storedFilterOption) {
    return storedFilterOption;
  } else {
    const filter = solveColor(color).filter;
    localStorage.setItem(colorKey, filter);
    return filter;
  }
}

function applyDynamicColorInvert() {
  const textColor = getComputedStyle(document.body).getPropertyValue('--text-color');
  if (textColor && textColor.length > 0) {
    const rgb = toRGBArray(textColor);
    const filter = memorizedSolvedFilter(rgb);

    const existingStyle = document.getElementById(DYNAMIC_STYLE_ELEMENT_ID);
    if (existingStyle) {
      existingStyle.textContent = `.color-invert {${filter}}`;
    } else {
      const style = document.createElement('style');
      style.id = DYNAMIC_STYLE_ELEMENT_ID;
      style.textContent = `.color-invert {${filter}}`;
      document.head.appendChild(style);
    }
  }
}

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
