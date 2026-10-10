import "./domhandler.js";
import "./js.js";


export function isCheerio(maybeCheerio) {
  return ((maybeCheerio)?.cheerio==="[cheerio object]");
}

export function camelCase(str) {
  return str.replace(/[._-](\w|$)/g, (_, x)=>(x).toUpperCase());
}

export function cssCase(str) {
  return str.replace(/[A-Z]/g, "-$&").toLowerCase();
}

export function domEach(array, fn) {
  const len = array.length;
  for (let i = 0; i < len; i++) { fn (array[i], i); }
  return array;
}

const CharacterCode = {LowerA: 97, LowerZ: 122, UpperA: 65, UpperZ: 90, Exclamation: 33};

export function isHtml(str) {
  if (typeof str!=="string") {
    return false;
  }
  const tagStart = str.indexOf("<");
  if (tagStart===-1||tagStart>str.length-3) { return false; }
  const tagChar = str.charCodeAt(tagStart+1);
  return (((tagChar>=CharacterCode.LowerA&&tagChar<=CharacterCode.LowerZ)||(tagChar>=CharacterCode.UpperA&&tagChar<=CharacterCode.UpperZ)||tagChar===CharacterCode.Exclamation)&&str.includes(">", tagStart+2));
}
