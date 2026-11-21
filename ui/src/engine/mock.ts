import { absQ, add, div, eq, le, mul, parseQ, q, sub, toFraction, toNumber, type Q } from "./rational";
import type { AnalyzeResult, Claim, Diagnostic, Engine, SeriesPoint, Transition } from "./types";

export const LINKS_MODEL = `chain links {
  states up, down
  from up:   up 9/10, down 1/10
  from down: up 3/10, down 7/10
}
`;

const EPSILON = q(1n, 1000n);
const RETRY_SUCCESS = q(3n, 10n);
const MAX_STEPS = 200;

interface Parsed {
  name: string;
  states: string[];
  rows: Map<string, Map<string, Q>>;
}

function parse(source: string): Parsed | Diagnostic[] {
  const errors: Diagnostic[] = [];
  let name = "";
  let states: string[] = [];
  const rows = new Map<string, Map<string, Q>>();
  const lines = source.split("\n");
  lines.forEach((raw, i) => {
    const line = raw.trim();
    const at = i + 1;
    if (line === "" || line === "}") {
      return;
    }
    const header = /^chain\s+(\w+)\s*\{$/.exec(line);
    if (header?.[1] !== undefined) {
      name = header[1];
      return;
    }
    const decl = /^states\s+(.+)$/.exec(line);
    if (decl?.[1] !== undefined) {
      states = decl[1].split(",").map((s) => s.trim());
      return;
    }
    const row = /^from\s+(\w+)\s*:\s*(.+)$/.exec(line);
    if (row?.[1] !== undefined && row[2] !== undefined) {
      const entries = new Map<string, Q>();
      for (const part of row[2].split(",")) {
        const [to, p] = part.trim().split(/\s+/);
        const value = p === undefined ? undefined : parseQ(p);
        if (to === undefined || value === undefined) {
          errors.push({ line: at, message: `Cannot read "${part.trim()}". Write each entry as a state and a fraction, such as "up 9/10".` });
          continue;
        }
        entries.set(to, value);
      }
      rows.set(row[1], entries);
      return;
    }
    errors.push({ line: at, message: `Unexpected "${line}". Lines start with chain, states or from.` });
  });
  if (name === "") {
    errors.push({ line: 1, message: "No chain header found. Start the model with \"chain <name> {\"." });
  }
  if (states.length !== 2) {
    errors.push({ line: 1, message: "Declare exactly two states, such as \"states up, down\"; the preview engine handles two-state chains only." });
  }
  for (const s of states) {
    const r = rows.get(s);
    if (r === undefined) {
      errors.push({ line: 1, message: `State ${s} has no row. Add a line starting with \"from ${s}:\".` });
      continue;
    }
    const total = [...r.values()].reduce(add, q(0n));
    if (!eq(total, q(1n))) {
      errors.push({ line: 1, message: `Row ${s} sums to ${total.n}/${total.d}. Adjust its probabilities so they sum to 1.` });
    }
  }
  return errors.length > 0 ? errors : { name, states, rows };
}

function analyzeSync(source: string): AnalyzeResult {
  const parsed = parse(source);
  if (Array.isArray(parsed)) {
    return { ok: false, diagnostics: parsed };
  }
  const { name, states, rows } = parsed;
  const [first, tracked] = states as [string, string];
  const p = (from: string, to: string): Q => rows.get(from)?.get(to) ?? q(0n);
  const transitions: Transition[] = states.flatMap((from) =>
    states.map((to) => ({ from, to, p: toFraction(p(from, to)) })),
  );

  // d' = P(first -> tracked) (1 - d) + P(tracked -> tracked) d = a d + b.
  const b = p(first, tracked);
  const a = sub(p(tracked, tracked), b);
  if (eq(absQ(a), q(1n))) {
    return {
      ok: false,
      diagnostics: [{ line: 1, message: "This chain never settles to one distribution. Give at least one state a transition probability strictly between 0 and 1." }],
    };
  }
  const fixed = div(b, sub(q(1n), a));
  const initial = q(1n);
  const series: SeriesPoint[] = [];
  let d = initial;
  let horizon = -1;
  for (let t = 0; t <= MAX_STEPS; t += 1) {
    series.push({ t, value: toFraction(d), approx: toNumber(d) });
    if (horizon < 0 && le(absQ(sub(d, fixed)), EPSILON)) {
      horizon = t;
    }
    if (horizon >= 0 && t >= horizon + 7) {
      break;
    }
    d = add(mul(a, d), b);
  }

  const fx = `${fixed.n}/${fixed.d}`;
  const claims: Claim[] = [
    {
      id: "steady-state",
      statement: `P(${tracked}) converges to ${fx}`,
      status: "proven",
      detail: `Fixed point of d' = ${a.n}/${a.d} d + ${b.n}/${b.d}, checked by the Lean kernel.`,
    },
    {
      id: "horizon",
      statement: `|d_t - ${fx}| ≤ 1/1000 for all t ≥ ${horizon}`,
      status: "proven",
      detail: "Geometric contraction bound with exact arithmetic.",
    },
    {
      id: "horizon-tight",
      statement: `|d_t - ${fx}| ≤ 1/1000 for all t ≥ ${horizon - 1}`,
      status: "rejected",
      detail: `Counterexample at t = ${horizon - 1}.`,
    },
    {
      id: "retry-success",
      statement: "Retry with success 3/10 eventually succeeds with probability 1",
      status: "proven",
      detail: "Least fixed point of the retry recursion.",
    },
    {
      id: "retry-attempts",
      statement: "Expected attempts of the retry equal 10/3",
      status: "proven",
      detail: "Exact solution of E = 1 + 7/10 E.",
    },
    {
      id: "mean-outage",
      statement: `Mean outage length is about 3.3 steps`,
      status: "estimated",
      detail: "Monte Carlo over 10000 runs; not checked by the kernel.",
    },
  ];

  return {
    ok: true,
    analysis: {
      chain: { name, states, transitions },
      convergence: {
        state: tracked,
        a: toFraction(a),
        b: toFraction(b),
        initial: toFraction(initial),
        fixedPoint: toFraction(fixed),
        epsilon: toFraction(EPSILON),
        horizon,
        series,
      },
      retry: {
        success: toFraction(RETRY_SUCCESS),
        successProbability: toFraction(q(1n)),
        expectedAttempts: toFraction(div(q(1n), RETRY_SUCCESS)),
      },
      certificate: { version: 1, claims },
    },
  };
}

export const mockEngine: Engine = {
  analyze: (source) => Promise.resolve(analyzeSync(source)),
};
