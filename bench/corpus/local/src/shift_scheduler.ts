/**
 * Shift scheduling for small teams: the domain types plus the fold that
 * turns availability into weekly rosters. Deliberately framework-free —
 * the roster builder runs inside an edge worker.
 */

/** A wall-clock time as `HH:MM`, 24h, canon zero-padded. */
export type Time24 = `${number}:${number}`;

/** ISO weekday, 1 = Monday … 7 = Sunday. */
export type IsoWeekday = 1 | 2 | 3 | 4 | 5 | 6 | 7;

/** One availability window a person can staff. */
export interface Window {
  /** Weekday the window belongs to. */
  day: IsoWeekday;
  /** Earliest start — inclusive. */
  from: Time24;
  /** Latest end — exclusive. */
  to: Time24;
}

/** A person eligible for rostering. */
export interface Staff {
  /** Stable person id (used in rosters + audit). */
  id: string;
  /** Display name. */
  name: string;
  /** What this person may staff. */
  skills: readonly Skill[];
  /** Declared windows; an empty list means fully unavailable. */
  windows: readonly Window[];
  /** Contracted weekly hours in quarters of an hour. */
  weeklyQuarters: number;
}

export type Skill = "opening" | "closing" | "register" | "barista" | "stock";

/** A single booked shift on the roster. */
export interface Shift {
  /** Person id from {@link Staff.id}. */
  personId: string;
  day: IsoWeekday;
  start: Time24;
  /** Number of quarters booked. */
  quarters: number;
  /** Primary skill the shift covers. */
  skill: Skill;
}

/**
 * Result of one roster pass: bookings plus whoever never got their hours.
 * Unfilled demand is a scheduling failure the caller must resolve — the
 * builder never invents hours for people who cannot work them.
 */
export interface Roster {
  /** Booked shifts, canon sorted (day, then start). */
  shifts: Shift[];
  /** Person ids whose weekly quarter target stayed unmet. */
  unsatisfied: string[];
  /** Demand (per day/skill/hour) nobody could cover. */
  unfilled: string[];
}

/**
 * Turns minutes-from-midnight into canon `HH:MM`.
 * @param minutes anything in [0, 1439]; larger values wrap the clock.
 */
export function timeOf(minutes: number): Time24 {
  const wrapped = ((minutes % 1440) + 1440) % 1440;
  return `${String(Math.floor(wrapped / 60)).padStart(2, "0")}:${String(wrapped % 60).padStart(2, "0")}`;
}

/**
 * Overlap of two windows in quarters; adjacent windows touch but do not
 * overlap (open 09:00–12, 12:00–15 share no quarter).
 * @returns the covered quarter count, 0-safe.
 */
export function overlapQuarters(a: Window, b: Window): number {
  if (a.day !== b.day) {
    return 0;
  }
  const a0 = minutes(a.from), a1 = minutes(a.to);
  const b0 = minutes(b.from), b1 = minutes(b.to);
  const lo = Math.max(a0, b0), hi = Math.min(a1, b1);
  return Math.max(0, Math.floor((hi - lo) / 15));
}

/** Minutes-from-midnight; a bare `HH:` value fails loudly, canon never fakes. */
function minutes(t: Time24): number {
  const m = /^([01]\d|2[0-3]):([0-5]\d)$/.exec(t);
  if (!m) {
    throw new RangeError(`bad time: ${JSON.stringify(t)}`);
  }
  return Number(m[1]) * 60 + Number(m[2]);
}

/**
 * Builds a first-pass roster: walk unmet demand per day, book the eligible
 * person with the largest remaining allowance, longest first (fewest
 * fragments), ties broken by stable id order.
 *
 * @param demand slots that must be covered (day, skill)
 * @param staff the pool, order-independent
 */
export function buildRoster(
  demand: readonly { day: IsoWeekday; start: Time24; quarters: number; skill: Skill }[],
  staff: readonly Staff[],
): Roster {
  const shifts: Shift[] = [];
  const remaining = new Map<string, number>(staff.map((s) => [s.id, s.weeklyQuarters]));
  const unfilled: string[] = [];

  const ordered = [...demand].sort(
    (x, y) => x.day - y.day || minutes(x.start) - minutes(y.start),
  );
  for (const slot of ordered) {
    const candidates = staff
      .filter((s) => s.skills.includes(slot.skill))
      .map((s) => ({
        staff: s,
        cover: Math.max(
          0,
          ...s.windows.map((w) => overlapQuarters(w, slotOf(slot))),
        ),
      }))
      .filter((c) => c.cover > 0 && (remaining.get(c.staff.id) ?? 0) > 0)
      .sort(
        (x, y) =>
          Math.min(remaining.get(y.staff.id) ?? 0, y.cover) - Math.min(remaining.get(x.staff.id) ?? 0, x.cover) ||
          x.staff.id.localeCompare(y.staff.id),
      );
    if (candidates.length === 0) {
      unfilled.push(`${slot.day}/${slot.skill}`);
      continue;
    }
    const pick = candidates[0];
    const booked = Math.min(slot.quarters, pick.cover, remaining.get(pick.staff.id) ?? 0);
    shifts.push({
      personId: pick.staff.id,
      day: slot.day,
      start: slot.start,
      quarters: booked,
      skill: slot.skill,
    });
    remaining.set(pick.staff.id, (remaining.get(pick.staff.id) ?? 0) - booked);
  }

  const unsatisfied = [...remaining.entries()]
    .filter(([, q]) => q > 0)
    .map(([id]) => id)
    .sort();

  return {
    shifts: shifts.sort(
      (x, y) => x.day - y.day || minutes(x.start) - minutes(y.start),
    ),
    unsatisfied,
    unfilled: [...new Set(unfilled)],
  };
}

/** Adapts a demand slot to the window shape so overlap math is shared. */
function slotOf(slot: { day: IsoWeekday; start: Time24; quarters: number }): Window {
  const s = minutes(slot.start);
  return {
    day: slot.day,
    from: slot.start,
    to: timeOf(s + slot.quarters * 15),
  };
}

/**
 * Weekly totals per person — the payroll hand-off. Zero rows stay (staff
 * who ended up idle matter to audit).
 */
export function weeklyTotals(roster: Roster, staff: readonly Staff[]): Map<string, number> {
  const totals = new Map<string, number>(staff.map((s) => [s.id, 0]));
  for (const shift of roster.shifts) {
    totals.set(shift.personId, (totals.get(shift.personId) ?? 0) + shift.quarters);
  }
  return totals;
}