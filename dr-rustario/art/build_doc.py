#!/usr/bin/env python3
"""Assemble the network's feature reference: one page, every input, drawn and measured.

    ga dr explain 1500 2 10 > work/explain.txt
    cargo run -p dr-rustario-vs-rustris --example feature_shots -- work/shots nes
    python3 dr-rustario/art/crop.py work/shots work/shots-cropped
    python3 dr-rustario/art/build_doc.py work

The values come from `manifest.json`, which the shot renderer writes in the same pass that
draws the pictures, and never from the report - the two producers drifted apart once and every
label ended up on the wrong picture. The report is read only for the influence table, keyed by
name so no ordering can corrupt it.
"""
import base64, html, json, pathlib, re, sys

HERE = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else ".")
SHOTS = HERE / "shots-cropped"
OUT = HERE / "dr-rustario-features.html"

BLOCKS = [
    ("bottle", "How the bottle moved", 0, 7,
     "Seven readings of the bottle the placement leaves behind, each as a <em>change</em> from "
     "the bottle before the pill. They say what the placement did to the stack as a whole."),
    ("runs", "Runs that have not gone yet", 7, 13,
     "Six counts of runs one and two short of a match, split by axis and by whether a virus is "
     "in them. A run walled in at both ends is not counted, and neither is one whose only gap "
     "sits under an overhang - room always means room a pill could actually reach."),
    ("placement", "What the placement itself did", 13, 26,
     "Thirteen readings of the two cells the pill filled and the lines through them. This is "
     "where the deterministic ai keeps almost all of its weight, and where the trained model "
     "keeps most of its too."),
    ("context", "What kind of bottle this is", 26, 32,
     "Six readings of the bottle <em>before</em> the pill. They are identical for every "
     "candidate by construction, so they cannot rank anything - they are there to tell the "
     "network which half of the game it is in."),
]


def shot(index, name, which):
    path = SHOTS / f"{index:02d}-{name.replace('.', '-')}-{which}.png"
    return base64.b64encode(path.read_bytes()).decode()


def frame(index, name, which, caption, value, cls):
    val = f'<b class="val">{value}</b>' if value is not None else ""
    return (f'<figure class="frame {cls}"><img alt="{html.escape(name)} {which}" '
            f'src="data:image/png;base64,{shot(index, name, which)}">'
            f'<figcaption><span>{caption}</span>{val}</figcaption></figure>')


def bar(value, most, colour):
    width = 0 if most == 0 else 100 * value / most
    return f'<span class="bar"><span style="width:{width:.1f}%;background:{colour}"></span></span>'


def gather():
    """The inputs, their purposes, what the model does with each, and the drawn scenarios."""
    source = pathlib.Path(__file__).parent.parent / "src/game/ai/explain.rs"
    block = source.read_text()
    block = block[block.index("pub const INPUTS"):]
    block = block[:block.index("];")]
    named = re.findall(r'Input::(comparative|context)\("([^"]+)", "((?:[^"\\]|\\.)*)"\)', block)
    data = [{"kind": k, "name": n, "purpose": p.replace('\\"', '"')} for k, n, p in named]

    report = (HERE / "explain.txt").read_text()
    influence = {
        m.group(1): dict(weight=float(m.group(2)), spread=float(m.group(3)),
                         changes=float(m.group(4)), viruses=int(m.group(5)))
        for m in re.finditer(r"^(\S+)\s+([\d.]+)\s+([\d.]+)\s+([\d.]+)%\s+(\d+)$", report, re.M)
    }
    baseline = int(re.search(r"(\d+) viruses, \d+ bottles", report).group(1))
    manifest = {r["name"]: r for r in json.loads((HERE / "shots/manifest.json").read_text())}
    assert len(manifest) == len(data), "the manifest and the inputs disagree"

    for index, item in enumerate(data):
        item.update(influence[item["name"]])
        drawn = manifest[item["name"]]
        assert drawn["input"] == index, f"{item['name']} is at a different index in the manifest"
        item["value"] = drawn["value"]
        item["cells_popped"] = drawn["cells_popped"]
        item["viruses_popped"] = drawn["viruses_popped"]
        item["rounds"] = drawn["rounds"]
        item["found"] = drawn["found"]
        item["viruses_gone"] = [drawn["viruses_before"] - v for v in drawn["viruses_after"]]
    return data, baseline


def main():
    inputs, baseline = gather()
    most = max(i["changes"] for i in inputs)
    ranked = sorted(inputs, key=lambda i: -i["changes"])
    colour = {"bottle": "var(--blue)", "runs": "var(--blue)",
              "placement": "var(--red)", "context": "var(--yellow)"}

    def block_of(index):
        for key, _, lo, hi, _ in BLOCKS:
            if lo <= index < hi:
                return key
        return "bottle"

    parts = []
    add = parts.append

    # ---- ranked chart
    rows = []
    for item in ranked:
        i = inputs.index(item)
        key = block_of(i)
        delta = item["viruses"] - baseline
        sign = "+" if delta > 0 else ""
        rows.append(
            f'<a class="rank" href="#f{i:02d}">'
            f'<span class="rank-name">{html.escape(item["name"])}</span>'
            f'{bar(item["changes"], most, colour[key])}'
            f'<span class="rank-num">{item["changes"]:.1f}%</span>'
            f'<span class="rank-delta {"up" if delta > 0 else "down"}">{sign}{delta}</span>'
            f"</a>"
        )
    chart = "\n".join(rows)

    # ---- index rail
    nav = []
    for key, title, lo, hi, _ in BLOCKS:
        links = "".join(
            f'<a href="#f{i:02d}"><b>{i:02d}</b>{html.escape(inputs[i]["name"].split(".", 1)[1])}</a>'
            for i in range(lo, hi)
        )
        nav.append(f'<div class="nav-block"><h4>{title}</h4>{links}</div>')
    rail = "\n".join(nav)

    # ---- one card per input
    cards = []
    for key, title, lo, hi, blurb in BLOCKS:
        cards.append(
            f'<section class="block" id="b-{key}"><header class="block-head">'
            f'<h2>{title}</h2><p>{blurb}</p>'
            f'<p class="block-range">inputs {lo:02d}&ndash;{hi - 1:02d} of <code>raw_inputs</code></p>'
            f"</header>"
        )
        for i in range(lo, hi):
            item = inputs[i]
            name = item["name"]
            high, low = item["value"]
            delta = item["viruses"] - baseline
            dead = item["weight"] == 0 or item["changes"] == 0
            same = high == low
            def lands(at):
                cells = item["cells_popped"][at]
                if cells == 0:
                    return "lands, nothing clears"
                viruses = item["viruses_popped"][at]
                return f"lands, {cells} go" + (f" &mdash; {viruses} viral" if viruses else "")

            def settles(at):
                gone = item["viruses_gone"][at]
                cascade = " after a cascade" if item["rounds"][at] > 1 else ""
                if gone == 0:
                    return "settles, no virus gone" + cascade
                return f"settles, {gone} virus{'es' if gone != 1 else ''} gone{cascade}"

            frames = "".join([
                frame(i, name, "before", "before the pill", None, "f-before"),
                frame(i, name, "a-landed", lands(0), f"{high:g}", "f-a"),
                frame(i, name, "a-after", settles(0), None, "f-a"),
                frame(i, name, "b-landed", lands(1), f"{low:g}", "f-b"),
                frame(i, name, "b-after", settles(1), None, "f-b"),
            ])
            note = ""
            if dead:
                note = ('<p class="flag flag-dead">Provably inert. Every weight the first layer '
                        'gives it is zero, it never differs between candidates, and silencing it '
                        'changes nothing the model does.</p>')
            elif same:
                note = ('<p class="flag">No placement in any of the sample bottles separates '
                        'this input, so the two shots below show the same value.</p>')
            elif item.get("found"):
                note = ('<p class="flag">Found in a real game rather than drawn. Every other '
                        'input here has a bottle built for it, holding only the pieces it needs; '
                        'this one describes a bottle that is one half away from a clear which '
                        '<em>cascades</em>, and every compact way of setting that up either '
                        'clears on the spot or buries the very cell the clear needs. It is far '
                        'easier arrived at than built.</p>')
            elif max(item["rounds"]) > 1:
                at = 0 if item["rounds"][0] > 1 else 1
                note = (f'<p class="flag">This placement <strong>cascades</strong>. The dashed '
                        f'ring marks the first clear only &mdash; '
                        f'{item["viruses_popped"][at]} of the '
                        f'{item["viruses_gone"][at]} viruses this input counts. The rest are '
                        f'taken by later rounds, once what the first one left unsupported has '
                        f'fallen, so by then those cells are somewhere else and there is nothing '
                        f'in the landing shot to ring them on.</p>')
            elif delta > 0:
                note = (f'<p class="flag">Silencing this input made the model play <em>better</em> '
                        f'on this sample &mdash; {item["viruses"]} viruses against {baseline}. '
                        f'Two games is a thin sample, but it is a candidate for cutting.</p>')

            cards.append(f"""
<article class="card {key}" id="f{i:02d}">
  <header class="card-head">
    <span class="idx">{i:02d}</span>
    <h3><code>{html.escape(name)}</code></h3>
    <span class="chip chip-{key}">{"context" if key == "context" else "comparative"}</span>
  </header>
  <p class="purpose">{html.escape(item["purpose"])}</p>
  {note}
  <div class="stats">
    <div class="stat"><span>changes the model&rsquo;s mind</span><b>{item["changes"]:.1f}<i>%</i></b>
      {bar(item["changes"], most, colour[key])}</div>
    <div class="stat"><span>viruses without it</span><b>{item["viruses"]}<i> / {baseline}</i></b>
      {bar(min(item["viruses"], baseline * 2), baseline * 2, colour[key])}</div>
    <div class="stat"><span>first-layer weight</span><b>{item["weight"]:.2f}</b>
      {bar(item["weight"], 13, colour[key])}</div>
    <div class="stat"><span>spread between candidates</span><b>{item["spread"]:.3f}</b>
      {bar(item["spread"], 0.35, colour[key])}</div>
  </div>
  <div class="screen">{frames}</div>
</article>""")
        cards.append("</section>")

    body = "\n".join(cards)

    OUT.write_text(TEMPLATE.format(chart=chart, rail=rail, cards=body, baseline=baseline))
    print(f"{OUT}  {OUT.stat().st_size / 1024 / 1024:.2f} MB")


TEMPLATE = """<title>Thirty Two Numbers</title>
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Chakra+Petch:wght@600;700&family=IBM+Plex+Mono:wght@400;600&family=IBM+Plex+Sans:wght@400;500;600&display=swap">
<style>
:root {{
  --screen: #000000;
  --blue: #1FA5FE;
  --yellow: #C4B200;
  --red: #D41E41;
  --ground: #EDF1F6;
  --surface: #FFFFFF;
  --sunken: #E3E9F0;
  --ink: #0D141E;
  --muted: #5B6878;
  --line: #D2DAE4;
  --display: "Chakra Petch", "Trebuchet MS", sans-serif;
  --body: "IBM Plex Sans", system-ui, sans-serif;
  --mono: "IBM Plex Mono", ui-monospace, monospace;
}}
@media (prefers-color-scheme: dark) {{
  :root:not([data-theme="light"]) {{
    --ground: #0A0E14;
    --surface: #131A23;
    --sunken: #0E141B;
    --ink: #E2E8F0;
    --muted: #8A97A8;
    --line: #232D3A;
  }}
}}
:root[data-theme="dark"] {{
  --ground: #0A0E14;
  --surface: #131A23;
  --sunken: #0E141B;
  --ink: #E2E8F0;
  --muted: #8A97A8;
  --line: #232D3A;
}}
* {{ box-sizing: border-box; }}
body {{
  background: var(--ground);
  color: var(--ink);
  font-family: var(--body);
  font-size: 16px;
  line-height: 1.6;
  -webkit-font-smoothing: antialiased;
}}
.page {{ display: grid; grid-template-columns: 1fr; gap: 0; }}
@media (min-width: 1120px) {{
  .page {{ grid-template-columns: 232px minmax(0, 1fr); }}
}}
.wrap {{ max-width: 940px; margin: 0 auto; padding: 0 24px 96px; width: 100%; }}

/* ---- masthead */
.masthead {{ padding: 72px 0 40px; border-bottom: 2px solid var(--ink); margin-bottom: 40px; }}
.eyebrow {{
  font-family: var(--mono); font-size: 12px; font-weight: 600;
  letter-spacing: .14em; text-transform: uppercase; color: var(--blue); margin: 0 0 18px;
}}
h1 {{
  font-family: var(--display); font-weight: 700; font-size: clamp(38px, 6vw, 62px);
  line-height: 1.02; letter-spacing: -.015em; margin: 0 0 20px; text-wrap: balance;
}}
.standfirst {{ font-size: 19px; line-height: 1.55; color: var(--muted); max-width: 62ch; margin: 0; }}
.standfirst strong {{ color: var(--ink); font-weight: 600; }}

h2 {{
  font-family: var(--display); font-weight: 700; font-size: 27px; letter-spacing: -.01em;
  margin: 0 0 8px; text-wrap: balance;
}}
h3 {{ font-family: var(--display); font-weight: 600; font-size: 20px; margin: 0; }}
h4 {{
  font-family: var(--mono); font-size: 11px; font-weight: 600; letter-spacing: .1em;
  text-transform: uppercase; color: var(--muted); margin: 0 0 8px;
}}
p {{ margin: 0 0 14px; }}
code {{ font-family: var(--mono); font-size: .92em; }}
a {{ color: inherit; }}
:focus-visible {{ outline: 2px solid var(--blue); outline-offset: 3px; border-radius: 2px; }}

/* ---- how to read */
.method {{ display: grid; gap: 18px; margin: 0 0 52px; }}
@media (min-width: 720px) {{ .method {{ grid-template-columns: repeat(3, 1fr); }} }}
.method div {{
  background: var(--surface); border: 1px solid var(--line); border-top: 3px solid var(--blue);
  padding: 18px 18px 16px;
}}
.method h4 {{ color: var(--ink); font-size: 12px; }}
.method p {{ font-size: 14.5px; color: var(--muted); margin: 0; }}

/* ---- ranked chart */
.ranked {{ margin: 0 0 60px; }}
.rank {{
  display: grid; grid-template-columns: minmax(130px, 200px) 1fr 54px 54px;
  align-items: center; gap: 12px; padding: 5px 8px; text-decoration: none;
  border-bottom: 1px solid var(--line);
}}
.rank:hover {{ background: var(--surface); }}
.rank-name {{ font-family: var(--mono); font-size: 12.5px; overflow: hidden; text-overflow: ellipsis; }}
.rank-num, .rank-delta {{
  font-family: var(--mono); font-size: 12.5px; text-align: right;
  font-variant-numeric: tabular-nums;
}}
.rank-delta.up {{ color: var(--blue); }}
.rank-delta.down {{ color: var(--muted); }}
.bar {{ display: block; height: 7px; background: var(--sunken); overflow: hidden; }}
.bar > span {{ display: block; height: 100%; }}

/* ---- index rail */
.rail {{ display: none; }}
@media (min-width: 1120px) {{
  .rail {{
    display: block; position: sticky; top: 0; align-self: start; max-height: 100vh;
    overflow-y: auto; padding: 72px 0 48px 24px; border-right: 1px solid var(--line);
  }}
}}
.nav-block {{ margin-bottom: 22px; }}
.nav-block a {{
  display: grid; grid-template-columns: 26px 1fr; gap: 4px; text-decoration: none;
  font-family: var(--mono); font-size: 12px; color: var(--muted); padding: 2.5px 0;
}}
.nav-block a b {{ color: var(--line); font-weight: 400; }}
.nav-block a:hover {{ color: var(--blue); }}
.nav-block a:hover b {{ color: var(--blue); }}

/* ---- blocks and cards */
.block {{ margin: 0 0 20px; }}
.block-head {{ padding: 42px 0 20px; }}
.block-head p {{ color: var(--muted); max-width: 64ch; }}
.block-range {{
  font-family: var(--mono); font-size: 11.5px; letter-spacing: .06em;
  text-transform: uppercase; color: var(--muted); margin: 0;
}}
.card {{
  background: var(--surface); border: 1px solid var(--line); padding: 22px;
  margin: 0 0 18px; scroll-margin-top: 16px;
}}
.card-head {{ display: flex; align-items: baseline; gap: 12px; margin-bottom: 12px; flex-wrap: wrap; }}
.idx {{
  font-family: var(--mono); font-size: 12px; font-weight: 600; color: var(--surface);
  background: var(--ink); padding: 2px 7px; letter-spacing: .04em;
}}
.card.bottle .idx, .card.runs .idx {{ background: var(--blue); color: #04121C; }}
.card.placement .idx {{ background: var(--red); color: #fff; }}
.card.context .idx {{ background: var(--yellow); color: #1A1600; }}
.chip {{
  margin-left: auto; font-family: var(--mono); font-size: 10.5px; letter-spacing: .09em;
  text-transform: uppercase; color: var(--muted); border: 1px solid var(--line); padding: 2px 8px;
}}
.purpose {{ color: var(--muted); max-width: 66ch; font-size: 15px; }}
.flag {{
  font-size: 14px; border-left: 3px solid var(--yellow); padding: 8px 0 8px 12px;
  background: var(--sunken); color: var(--ink); margin-bottom: 14px;
}}
.flag-dead {{ border-left-color: var(--red); }}

.stats {{ display: grid; gap: 14px 22px; margin: 16px 0 20px; }}
@media (min-width: 620px) {{ .stats {{ grid-template-columns: repeat(4, 1fr); }} }}
.stat span {{
  display: block; font-family: var(--mono); font-size: 10.5px; letter-spacing: .05em;
  text-transform: uppercase; color: var(--muted); margin-bottom: 3px;
}}
.stat b {{
  display: block; font-family: var(--display); font-weight: 700; font-size: 22px;
  line-height: 1.1; margin-bottom: 6px; font-variant-numeric: tabular-nums;
}}
.stat i {{ font-style: normal; font-size: 13px; font-weight: 600; color: var(--muted); }}

/* ---- the shots, always on the console's own black */
.screen {{
  background: var(--screen); border: 1px solid var(--line); padding: 16px;
  display: grid; grid-template-columns: repeat(5, 1fr); gap: 10px; overflow-x: auto;
}}
@media (max-width: 700px) {{ .screen {{ grid-template-columns: repeat(3, 1fr); }} }}
.frame {{ margin: 0; display: flex; flex-direction: column; gap: 7px; }}
/* the two frames of one placement share a rule above them, so a reader can see which pair is
   which without a caption saying so */
.frame.f-a img {{ border-top: 2px solid var(--red); }}
.frame.f-b img {{ border-top: 2px solid var(--blue); }}
.frame.f-before img {{ border-top: 2px solid #3B4654; }}
.frame img {{
  width: 100%; height: auto; display: block; image-rendering: pixelated;
  align-self: start;
}}
figcaption {{
  display: flex; justify-content: space-between; align-items: baseline; gap: 8px;
  font-family: var(--mono); font-size: 10.5px; letter-spacing: .04em; color: #7E8B9B;
  border-top: 1px solid #232D3A; padding-top: 6px;
}}
.val {{ color: var(--blue); font-size: 13px; font-variant-numeric: tabular-nums; }}

.legend {{ border: 1px solid var(--line); background: var(--surface); padding: 18px 20px; margin: 0 0 40px; }}
.legend-rows {{ display: grid; gap: 8px; }}
@media (min-width: 760px) {{ .legend-rows {{ grid-template-columns: 1fr 1fr; gap: 8px 26px; }} }}
.legend p {{ margin: 0; font-size: 14.5px; color: var(--muted); display: flex; gap: 10px; align-items: baseline; }}
.key {{ flex: none; width: 16px; height: 16px; display: inline-block; position: relative; top: 3px; }}
.key-solid {{ border: 2px solid var(--ink); }}
.key-dash {{ border: 2px dashed var(--ink); }}
.key-a {{ background: var(--red); }}
.key-b {{ background: var(--blue); }}

.closing {{ border-top: 2px solid var(--ink); margin-top: 48px; padding-top: 36px; }}
.closing h2 {{ margin-bottom: 16px; }}
.closing ul {{ padding-left: 20px; color: var(--muted); max-width: 66ch; }}
.closing li {{ margin-bottom: 12px; }}
.closing strong {{ color: var(--ink); font-weight: 600; }}
@media (prefers-reduced-motion: reduce) {{ * {{ transition: none !important; }} }}
</style>

<div class="page">
<nav class="rail">{rail}</nav>
<main class="wrap">

<header class="masthead">
  <p class="eyebrow">Dr. Rustario &middot; feature reference</p>
  <h1>Thirty two numbers</h1>
  <p class="standfirst">Everything the network knows about a bottle is these thirty two
  readings of it. Each one below is drawn twice on the NES theme &mdash; the placement that
  moves it <strong>furthest from zero</strong> and the one at the other end of its range, found
  by running the real placement search over a bottle built to hold only what that input needs
  &mdash; and measured three ways against the model that is embedded today. Every strip shows
  where the pill&rsquo;s halves landed and what the clear took with them.</p>
</header>

<section class="legend">
  <h4>Reading a strip</h4>
  <div class="legend-rows">
    <p><span class="key key-solid"></span> a solid ring is where the pill&rsquo;s two halves came
    to rest</p>
    <p><span class="key key-dash"></span> a dashed ring is a cell the <em>first</em> clear is
    taking, drawn in the theme&rsquo;s own pop sprite. A cascade takes more after it.</p>
    <p><span class="key key-a"></span> the placement that moves the input furthest from zero,
    where it lands and once it settles</p>
    <p><span class="key key-b"></span> the other end of the range, the same two ways</p>
  </div>
</section>

<section class="method">
  <div>
    <h4>Changes the model&rsquo;s mind</h4>
    <p>The share of {baseline} pills where the model picks a different placement with that input
    silenced. The strongest of the three measures: it is the model&rsquo;s own answer, over real
    games.</p>
  </div>
  <div>
    <h4>Viruses without it</h4>
    <p>What it then destroys over two whole games, against {baseline} with everything. A thin
    sample, so read it as a direction and not a figure.</p>
  </div>
  <div>
    <h4>Weight and spread</h4>
    <p>The size of the first layer&rsquo;s weight column, and how far the input moves between the
    placements of one pill. Zero on either is proof of nothing; anything else is weak evidence.</p>
  </div>
</section>

<section class="ranked">
  <h2>What the model would miss</h2>
  <p style="color:var(--muted);max-width:64ch">Every input, by how often silencing it changes the
  model&rsquo;s choice. The right hand column is the change in viruses destroyed &mdash; positive
  means the model played <em>better</em> without it.</p>
  {chart}
</section>

{cards}

<section class="closing">
  <h2>What this says about the feature set</h2>
  <ul>
    <li><strong>Three inputs are the spine.</strong> <code>place.reach</code>,
    <code>place.patterns_cleared</code> and <code>place.touching</code> change the model&rsquo;s
    mind on a third of all pills each, and silencing any one of them collapses it from
    {baseline} viruses to under 350. They are all readings of the run the pill landed in.</li>
    <li><strong>The context block earns its place after all, and not the way it was meant to.</strong>
    Those five inputs are identical for every candidate &mdash; their spread is exactly zero, so
    they cannot rank anything &mdash; yet silencing <code>context.max_height</code> changes the
    model&rsquo;s mind on a fifth of pills. A constant cannot separate candidates, but it does move
    the network&rsquo;s operating point, and the layers past the first then weigh the comparative
    inputs differently. A linear fit says this block adds nothing; the network says otherwise.</li>
    <li><strong><code>context.held</code> is inert and provably so.</strong> Zero weights, zero
    spread, zero mind changes, and the same virus count with and without it. It is the hold
    input, silenced after teaching and never trained because hold is off &mdash; it costs
    thirty two first-layer weights to say nothing.</li>
    <li><strong>The row-and-column split looks one sided.</strong> Down a column,
    <code>delta.virus_2_col</code> and <code>delta.block_3_col</code> both move the model;
    along a row, <code>delta.virus_3_row</code> and <code>delta.block_3_row</code> change its
    mind on about one pill in a hundred. The bottle is eight wide and sixteen tall, so a
    vertical run is the easier thing to build and the model appears to have noticed.</li>
    <li><strong>A handful of inputs may be worth their weights in noise.</strong> Silencing
    <code>delta.entrance_height</code>, <code>delta.block_3_col</code> or <code>delta.holes</code>
    left the model playing <em>better</em> on this sample. Two games is far too thin to act on,
    but they are where a trimming experiment should start.</li>
  </ul>
  <p style="color:var(--muted);font-size:14px;margin-top:22px">
  Generated from <code>ga dr explain</code> and
  <code>cargo run --example feature_shots -- out nes</code>. Every number and every picture comes
  from the same scenarios, so neither can drift from the other.</p>
</section>

</main>
</div>
"""

if __name__ == "__main__":
    main()
