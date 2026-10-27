




/** @interface Window */
export class Window {}

/** @interface Staff */
export class Staff {}



/** @interface Shift */
export class Shift {}

/** @interface Roster */
export class Roster {}

export function timeOf(minutes) {
  const wrapped = ((minutes%1440)+1440)%1440;
  return `${String(Math.floor(wrapped/60)).padStart(2, "0")}:${String(wrapped%60).padStart(2, "0")}`;
}

export function overlapQuarters(a, b) {
  if (a.day!==b.day) {
    return 0;
  }
  const a0 = minutes(a.from);
  const a1 = minutes(a.to);
  const b0 = minutes(b.from);
  const b1 = minutes(b.to);
  const lo = Math.max(a0, b0);
  const hi = Math.min(a1, b1);
  return Math.max(0, Math.floor((hi-lo)/15));
}

function minutes(t) {
  const m = /^([01]\d|2[0-3]):([0-5]\d)$/.exec(t);
  if (!m) {
    throw new RangeError(`bad time: ${JSON.stringify(t)}`);
  }
  return Number(m[1])*60+Number(m[2]);
}

export function buildRoster(demand, staff) {
  const shifts = [];
  const remaining = new Map(staff.map((s)=>[s.id, s.weeklyQuarters]));
  const unfilled = [];
  const ordered = [...demand].sort((x, y)=>x.day-y.day||minutes(x.start)-minutes(y.start));
  for (const slot of ordered) {
    const candidates = staff.filter((s)=>s.skills.includes(slot.skill)).map((s)=>({staff: s, cover: Math.max(0, ...s.windows.map((w)=>overlapQuarters(w, slotOf(slot))))})).filter((c)=>c.cover>0&&(remaining.get(c.staff.id)??0)>0).sort((x, y)=>Math.min(remaining.get(y.staff.id)??0, y.cover)-Math.min(remaining.get(x.staff.id)??0, x.cover)||x.staff.id.localeCompare(y.staff.id));
    if (candidates.length===0) {
      unfilled.push(`${slot.day}/${slot.skill}`);
      "opaque:continue;";
    }
    const pick = candidates[0];
    const booked = Math.min(slot.quarters, pick.cover, remaining.get(pick.staff.id)??0);
    shifts.push({personId: pick.staff.id, day: slot.day, start: slot.start, quarters: booked, skill: slot.skill});
    remaining.set(pick.staff.id, (remaining.get(pick.staff.id)??0)-booked);
  }
  const unsatisfied = [...remaining.entries()].filter(([, q])=>q>0).map(([id])=>id).sort();
  return {shifts: shifts.sort((x, y)=>x.day-y.day||minutes(x.start)-minutes(y.start)), unsatisfied, unfilled: [...new Set(unfilled)]};
}

function slotOf(slot) {
  const s = minutes(slot.start);
  return {day: slot.day, from :slot.start, to: timeOf(s+slot.quarters*15)};
}

export function weeklyTotals(roster, staff) {
  const totals = new Map(staff.map((s)=>[s.id, 0]));
  for (const shift of roster.shifts) { totals.set(shift.personId, (totals.get(shift.personId)??0)+shift.quarters); }
  return totals;
}
