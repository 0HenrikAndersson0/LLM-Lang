import "./domhandler.js";
import "./domutils.js";
import "./js.js";


export function getParse(parser) {
  return (content, options, isDocument, context)=>{ if (typeof Buffer!=="undefined"&&Buffer.isBuffer(content)) { content=content.toString(); } if (typeof content==="string") { return parser(content, options, isDocument, context); } doc=content"opaque:AnyNode[]"|Document; if (!Array.isArray(doc)&&checkIsDocument(doc)) { return doc; } root=new Document([]); update(doc, root); return root; };
}

export function update(newChilds, parent) {
  const arr = Array.isArray(newChilds)?newChilds: [newChilds];
  if (parent) {
    parent.children=arr;
  } else { parent=null; }
  for (let i = 0; i < arr.length; i++) {
    const node = arr[i];
    if (node.parent&&node.parent.children!==arr) { removeElement(node); }
    if (parent) {
      node.prev=arr[i-1]||null;
      node.next=arr[i+1]||null;
    } else { node.prev=node.next=null; }
    node.parent=parent;
  }
  return parent;
}
