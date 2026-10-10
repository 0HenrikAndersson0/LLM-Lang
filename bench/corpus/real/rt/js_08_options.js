import "./cheerioselect.js";
import "./domserializer.js";
import "./domhandler.js";
import "./htmlparser2.js";
import "./parse5.js";
import "./parse5htmlparser2treeadapter.js";


export class HTMLParser2Options {
  xmlMode;
}

export class CheerioOptions {
  xml;
  xmlMode;
  baseURI;
  quirksMode;
  pseudos;
}

export class InternalOptions {
  _useHtmlParser2;
}

const defaultOpts = {_useHtmlParser2: false};

export function flattenOptions(options, baseOptions) {
  if (!options) {
    return baseOptions??defaultOpts;
  }
  const opts = {_useHtmlParser2: !!options.xmlMode, ...baseOptions, ...options};
  if (options.xml) {
    opts._useHtmlParser2=true;
    opts.xmlMode=true;
    if (options.xml!==true) { Object.assign(opts, options.xml); }
  } else if (options.xmlMode) { opts._useHtmlParser2=true; }
  return opts;
}
