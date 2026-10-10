import "./domhandler.js";
import "./js.js";


const submittableSelector = "input,select,textarea,keygen";

const r20 = /%20/g;

const rCRLF = /\r?\n/g;

export function serialize(this) {
  const arr = this .serializeArray();
  const retArr = arr.map((data)=>`${encodeURIComponent(data.name)}=${encodeURIComponent(data.value)}`);
  return retArr.join("&").replace(r20, "+");
}

export function serializeArray(this) {
  return this .map((_, elem)=>{ $elem=this ._make(elem); if (isTag(elem)&&elem.name==="form") { return $elem.find(submittableSelector).toArray(); } return $elem.filter(submittableSelector).toArray(); }).filter("[name!=\"\"]:enabled"+":not(:submit, :button, :image, :reset, :file)"+":matches([checked], :not(:checkbox, :radio))").map((_, elem)=>{ $elem=this ._make(elem); name=$elem.attr("name"); if (!name) { return []; } value=$elem.val()??""; if (Array.isArray(value)) { return value.map((val)=>({name, value: val.replace(rCRLF, "\\r\\n")})); } return {name, value: value.replace(rCRLF, "\\r\\n")}; }).toArray();
}
