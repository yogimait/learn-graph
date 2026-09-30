import { useMemo, useState } from "react";
import type { DayCount } from "../lib/types";

const CELL = 13;
const GAP = 4;
const ROWS = 7;
const BATCH_WEEKS = 26;

export const PROJECT_START = "2026-09-01";

function dateToIso(d: Date): string {
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${d.getFullYear()}-${m}-${day}`;
}

export function level(count: number): number {
  if (count <= 0) return 0;
  if (count <= 2) return 1;
  if (count <= 4) return 2;
  if (count <= 6) return 3;
  return 4;
}

function levelLabel(l: number): string {
  if (l === 1) return "1-2 topics";
  if (l === 2) return "3-4 topics";
  if (l === 3) return "5-6 topics";
  return "7+ topics";
}

interface Props {
  counts: DayCount[];
  start?: string;
  color?: string;
}

interface Cell {
  date: Date;
  level: number;
  count: number;
}

const month = (d: Date) => d.toLocaleString("en", { month: "short" });

function batchLabel(b: Cell[][]): string {
  const first = b[0]?.[0];
  const lastWeek = b[b.length - 1];
  const last = lastWeek && lastWeek.length ? lastWeek[lastWeek.length - 1] : undefined;
  if (!first || !last) return "";
  const a = `${month(first.date)} ${first.date.getFullYear()}`;
  const z = `${month(last.date)} ${last.date.getFullYear()}`;
  return a === z ? a : `${a} – ${z}`;
}

/**
 * GitHub-style contribution calendar, paged in batches instead of scrolled.
 * Weeks run from the project start (Sep 2026) to today; each batch shows
 * BATCH_WEEKS weeks, navigated with older/newer arrows.
 * Intensity = distinct canonical topics per day. Pure SVG, no chart dependency.
 */
export default function Heatmap({ counts, start = PROJECT_START, color = "#39d353" }: Props) {
  const batches = useMemo(() => {
    const [y, m, d] = start.split("-").map(Number);
    const today = new Date();
    today.setHours(0, 0, 0, 0);
    // first week starts on the start date itself (partial); cells are
    // positioned by weekday so no padding days leak in from the previous month
    const cur = new Date(y, m - 1, d);

    const byDate = new Map(counts.map((c) => [c.date, c.count]));
    const weeks: Cell[][] = [];
    let week: Cell[] = [];
    while (cur <= today) {
      const count = byDate.get(dateToIso(cur)) ?? 0;
      week.push({ date: new Date(cur), level: level(count), count });
      if (cur.getDay() === 6 || cur.getTime() >= today.getTime()) {
        weeks.push(week);
        week = [];
      }
      cur.setDate(cur.getDate() + 1);
    }

    const batches: Cell[][][] = [];
    for (let i = 0; i < weeks.length; i += BATCH_WEEKS) {
      batches.push(weeks.slice(i, i + BATCH_WEEKS));
    }
    return batches;
  }, [counts, start]);

  const [page, setPage] = useState<number | null>(null);
  const idx = Math.min(page ?? batches.length - 1, batches.length - 1);
  const cur = batches[idx] ?? [];

  const monthLabels: { x: number; label: string }[] = [];
  let lastMonth = -1;
  cur.forEach((w, i) => {
    const m = w[0].date.getMonth();
    if (m !== lastMonth) {
      monthLabels.push({ x: i, label: month(w[0].date) });
      lastMonth = m;
    }
  });

  const width = cur.length * (CELL + GAP);
  const height = ROWS * (CELL + GAP);
  const base: [number, number, number] = [
    parseInt(color.slice(1, 3), 16),
    parseInt(color.slice(3, 5), 16),
    parseInt(color.slice(5, 7), 16),
  ];
  const mix = (f: number): string => {
    const c = base.map((v) => Math.round(v * f + 12 * (1 - f)));
    return `rgb(${c[0]},${c[1]},${c[2]})`;
  };
  // empty cells blend toward the pitch-black background, not white (GitHub-style)
  const shades = ["#171717", mix(0.3), mix(0.52), mix(0.74), mix(1)];

  return (
    <div className="heatmap">
      <div className="hm-head">
        <span className="hm-range">{batchLabel(cur)}</span>
        {batches.length > 1 && (
          <div className="hm-pager">
            <button title="Older" disabled={idx === 0} onClick={() => setPage(idx - 1)}>
              ‹
            </button>
            <span className="hm-page">
              {idx + 1}/{batches.length}
            </span>
            <button title="Newer" disabled={idx === batches.length - 1} onClick={() => setPage(idx + 1)}>
              ›
            </button>
          </div>
        )}
      </div>
      <svg width={width} height={height + 16}>
        {monthLabels.map(({ x, label }) => (
          <text key={x} x={x * (CELL + GAP)} y={10} fontSize={9} fill="#8a8a8a">
            {label}
          </text>
        ))}
        {cur.map((w, wi) =>
          w.map((d, di) => (
            <rect
              key={`${wi}-${di}`}
              x={wi * (CELL + GAP)}
              y={16 + d.date.getDay() * (CELL + GAP)}
              width={CELL}
              height={CELL}
              rx={2}
              fill={shades[d.level]}
            >
              <title>
                {dateToIso(d.date)}: {d.level === 0 ? "no activity" : levelLabel(d.level)}
              </title>
            </rect>
          )),
        )}
      </svg>
      <div className="heatmap-legend">
        <span>Less</span>
        {shades.map((s) => (
          <span key={s} className="legend-cell" style={{ background: s }} />
        ))}
        <span>More</span>
      </div>
    </div>
  );
}

export { dateToIso };