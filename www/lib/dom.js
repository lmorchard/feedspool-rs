export const $ = (sel, context = document.body) =>
  typeof sel === "string" ? context.querySelector(sel) : sel;

export const $$ = (sel, context = document) =>
  Array.from(context.querySelectorAll(sel));

export function clearChildren(sel, context = document.body) {
  let parentNode = $(sel, context);
  while (parentNode.firstChild) {
    parentNode.removeChild(parentNode.firstChild);
  }
  return parentNode;
}

export function replaceChildren(sel, childNodes, context = document.body) {
  let parentNode = clearChildren(sel, context);
  for (let node of childNodes) {
    parentNode.appendChild(node);
  }
  return parentNode;
}

export function updateElements(changes = [], context = document.body) {
  for (const sel in changes) {
    for (const el of $$(sel, context)) {
      updateElement(el, changes[sel]);
    }
  }
}

export function updateElement(el, changeSet) {
  if (typeof changeSet === "function") {
    changeSet(el);
  } else {
    for (const name in changeSet) {
      const value = changeSet[name];
      if (name.startsWith("@")) {
        el.setAttribute(name.substring(1), value);
      } if (name === "children") {
        replaceChildren(el, value);
      } else {
        el[name] = value;
      }
    }
  }
  return el;
}

export function createElement(name, changeSet = {}) {
  return updateElement(document.createElement(name), changeSet);
}

export function html(strings, ...values) {
  const src = strings
    .reduce(
      (result, string, idx) =>
        result + string + (values[idx] ? values[idx] : ""),
      ""
    )
    .trim();

  const frag = document
    .createRange()
    .createContextualFragment(src).firstElementChild;

  return document.adoptNode(frag);
}

export class BaseElement extends HTMLElement {
  constructor() {
    super();
    this._props = {};
    this.attachShadow({ mode: "open" }).appendChild(
      this.template().content.cloneNode(true)
    );
  }

  template() {
    return this.constructor.template;
  }

  get props() {
    return this._props;
  }

  set props(newProps) {
    const oldProps = this._props;
    this._props = newProps;
    this.propsChanged(newProps, oldProps);
  }

  propsChanged(newProps, oldProps) {}

  $(sel) {
    return $(sel, this.shadowRoot);
  }

  $$(sel) {
    return $$(sel, this.shadowRoot);
  }

  clearChildren(sel) {
    return clearChildren(sel, this.shadowRoot);
  }

  replaceChildren(sel, childNodes) {
    return replaceChildren(sel, childNodes, this.shadowRoot);
  }

  updateElements(changes) {
    return updateElements(changes, this.shadowRoot);
  }
}
