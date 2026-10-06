# Fixture: every Blueprints component

- Feature Name: `fixture`
- Start Date: 2026-10-04
- Pinned at: `4fcfb4adc85524243c5e392becd4f490e3e42252`
- Status: proposed

A test fixture, never published: scripts/check-legibility.ts renders it to
hold each component to its rules, and the responsive gate lays it out at
every width and taps what is interactive. Its claims and its experiment are
syntax, not findings.

## Summary

<claim id="compose-method" basis="observed">The stage that measures is a method called compose [lib.rs:250](../../src/lib.rs#L250 "pub fn compose(&self, doc: &mut Document)")</claim>, <claim basis="inferred" likelihood="likely">a fixture predicts a likely outcome</claim>, and <claim id="fixture-assumes" basis="assumed">a fixture assumes something</claim>.

<pragmatics>

<ask>

Read the fixture.

</ask>

<will>

- Render it.

</will>

<needs>

- Nothing.

</needs>

<wont>

- Publish it.

</wont>

<silence>

Nothing happens.

</silence>

</pragmatics>

<changed date="2026-10-04" since="the fixture's first draft">

<was>

The stage was called measure.

</was>

<now>

It is called compose. [lib.rs:250](../../src/lib.rs#L250 "pub fn compose")

</now>

</changed>

| Task | Rust | Python |
|---|---|---|
| Measure | <claim basis="observed">a method [lib.rs:250](../../src/lib.rs#L250 "pub fn compose")</claim> | none |

<sketch-figure id="fixture" seed="3" title="A fixture's structure">

```text
width 400
state today shipped "the fixture today"
box a "A" 4 20 96 34
box b "B" 150 20 96 34
edge a b
state next proposed "the fixture proposed"
box a "A" 4 20 96 34
hex c "C\nport" 150 4 120 70
box b "B" 300 20 96 34
edge a c
edge c b dashed
```

</sketch-figure>

<decision id="fixture" title="1. Does the fixture render?" reversible="yes" grounds="compose-method fixture-assumes">

<choice key="a" title="Yes">

It renders.

</choice>

<choice key="b" title="No">

It does not.

</choice>

<recommendation choice="a">

It should.

</recommendation>

<against>

Fixtures rot.

</against>

</decision>

<decision id="fixture-decided" title="2. Is a decided decision shown as decided?" reversible="costly" depends="fixture">

<choice key="a" title="Yes">

Yes.

</choice>

<choice key="b" title="No">

No.

</choice>

<recommendation choice="a">

Yes.

</recommendation>

<against>

None.

</against>

<ruling response="accept-with-reservation" date="2026-10-04">

Accepted, with a reservation recorded here.

</ruling>

</decision>

<assumptions />

<experiment id="fixture" title="A fixture experiment">

<hypothesis recorded="2026-10-04" commit="4fcfb4adc85524243c5e392becd4f490e3e42252">

A fixture predicts its runs land between 0.7 and 0.9.

</hypothesis>

<method>

Five runs of nothing.

</method>

<outcomes values="0.81 0.79 0.84 0.79 0.88" unit="F1" label="Fixture F1" min="0.7" max="0.9" />

<result>

Every run landed between 0.79 and 0.88.

</result>

<provenance>

Inputs: none. matra: none.

</provenance>

<limits>

Nothing: it is a fixture.

</limits>

</experiment>

<awaiting />

<record-index kind="proposals" />

| Proposal | Title | Status | Tracking issue |
|---|---|---|---|
| EPR-0000 | The template | not a record | none |
| EPR-9999 | A fixture | proposed | none |
