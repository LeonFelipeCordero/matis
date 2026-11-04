const namespaces = { svg: "http://www.w3.org/2000/svg", xmlns: "http://www.w3.org/2000/xmlns/" };

export function createFragment({ children }) {
  const fragment = document.createDocumentFragment();
  fragment.append(...children.flat(Infinity));
  return fragment;
}

export function createElement(tag, properties, ...children) {
  if (typeof tag === "function") return tag({ ...properties, children });
  const prefix = tag.includes(":") ? tag.split(":")[0] : null;
  const element = prefix ? document.createElementNS(namespaces[prefix], tag) : document.createElement(tag);
  for (const [name, value] of Object.entries(properties || {})) {
    if (name.includes(":")) element.setAttributeNS(namespaces[name.split(":")[0]], name, value);
    else element.setAttribute(name, value);
  }
  element.append(...children.flat(Infinity));
  return element;
}
