// The model templates the guide teaches with. Their `.axm` text lives in
// fixtures/models/templates/, which an axiom-model test also reads, so the
// text shown here is the text the engine is tested on.
import broken from "../../../fixtures/models/templates/broken.axm?raw";
import flip from "../../../fixtures/models/templates/flip.axm?raw";
import linksParam from "../../../fixtures/models/templates/links-param.axm?raw";
import links from "../../../fixtures/models/templates/links.axm?raw";
import retry from "../../../fixtures/models/templates/retry.axm?raw";
import server from "../../../fixtures/models/templates/server.axm?raw";
import slowRepair from "../../../fixtures/models/templates/slow-repair.axm?raw";

export interface Template {
  id: string;
  title: string;
  description: string;
  // Plain sentences, each followed by the model line it becomes.
  breakdown: { plain: string; line: string }[];
  // What to look for once it runs.
  note: string | null;
  source: string;
  // Shown in the row of examples above the editor.
  preset: string | null;
}

export const TEMPLATES: Template[] = [
  {
    id: "links",
    title: "Alice's links",
    description: "A network link that fails now and then and gets repaired.",
    breakdown: [
      { plain: "A link is either up or down.", line: "states up, down" },
      { plain: "A link that is up fails 1 time in 10.", line: "from up:   up 9/10, down 1/10" },
      { plain: "A link that is down is repaired 3 times in 10.", line: "from down: up 3/10, down 7/10" },
    ],
    note: null,
    source: links,
    preset: "Alice's links: settles at 1/4",
  },
  {
    id: "slow-repair",
    title: "Slow repair",
    description: "The same link, but repair is as rare as failure.",
    breakdown: [
      { plain: "A link that is up fails 1 time in 10.", line: "from up:   up 9/10, down 1/10" },
      { plain: "A link that is down is repaired only 1 time in 10.", line: "from down: up 1/10, down 9/10" },
    ],
    note: "It settles at 1/2, but takes longer to get within 1/1000.",
    source: slowRepair,
    preset: "Slow repair: settles at 1/2, slowly",
  },
  {
    id: "flip",
    title: "Flips every step",
    description: "A link that always changes state, so it never settles.",
    breakdown: [
      { plain: "An up link always goes down.", line: "from up:   down 1" },
      { plain: "A down link always comes back up.", line: "from down: up 1" },
    ],
    note: "Look for the missing convergence plot: the chance of down alternates between 0 and 1 forever.",
    source: flip,
    preset: "Flips every step: never settles",
  },
  {
    id: "server",
    title: "Server with three states",
    description: "A server that is up, degraded or down.",
    breakdown: [
      { plain: "A server is up, degraded or down.", line: "states up, degraded, down" },
      { plain: "An up server degrades 1 time in 10 and never goes straight down.", line: "from up:       up 9/10, degraded 1/10" },
      { plain: "A degraded server recovers half the time and goes down 1 time in 5.", line: "from degraded: up 1/2, degraded 3/10, down 1/5" },
    ],
    note: "With more than two states the browser shows only the Mass claims: each row adds up to 1.",
    source: server,
    preset: null,
  },
  {
    id: "retry",
    title: "Retry 3/10",
    description: "A request that is tried again until it succeeds.",
    breakdown: [{ plain: "A request succeeds 3 times in 10; otherwise it is tried again.", line: "retry request { success 3/10 }" }],
    note: null,
    source: retry,
    preset: "Retry 3/10: 10/3 attempts expected",
  },
  {
    id: "links-param",
    title: "Links with parameters",
    description: "Alice's links for every failure chance a and repair chance b.",
    breakdown: [
      { plain: "a is any failure chance strictly between 0 and 1.", line: "param a in ]0, 1[" },
      { plain: "An up link stays up with chance 1 - a.", line: "from up:   up 1 - a, down a" },
    ],
    note: "The browser checks this model but does not analyse parameters yet.",
    source: linksParam,
    preset: null,
  },
  {
    id: "broken",
    title: "A broken model",
    description: "The row for up adds up to 11/10, so the engine rejects it.",
    breakdown: [{ plain: "9/10 + 2/10 is 11/10, not 1.", line: "from up:   up 9/10, down 2/10" }],
    note: "Read the error: its code names the fault, and the line and column point at the row.",
    source: broken,
    preset: null,
  },
];
