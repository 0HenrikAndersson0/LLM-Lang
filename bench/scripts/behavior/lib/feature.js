import * as Helpers from "./Fixtures/Helpers.js";


export function Report(n) { return Helpers.Double(n)+Helpers.Trip(n); }

export function ReportMany(xs) { return xs.map(x=>Helpers.Double(x)); }
