"""CLI for matra. Wraps the Rust engine via PyO3."""

from __future__ import annotations

import json
from pathlib import Path

import click
from rich.console import Console
from rich.table import Table

from matra._core import Matra
from matra.types import Document, Keyphrase, ScoredSentence

console = Console()


def _get_matra() -> Matra:
    """Create a Matra instance. Auto-downloads model on first use."""
    model_dir = str(Path.home() / ".matra" / "models")
    try:
        return Matra.english(model_dir)
    except Exception as e:
        console.print(f"[red]Failed to load model: {e}[/red]")
        raise SystemExit(1) from e


def _doc_metrics(result: Document) -> dict[str, float | int | None]:
    """Compute document-level metrics from a serialized Document."""
    sentences = [
        sent
        for sec in result["sections"]
        for para in sec["paragraphs"]
        for sent in para["sentences"]
    ]

    total = len(sentences)
    word_counts = [sum(1 for t in s["tokens"] if not t["is_punct"]) for s in sentences]
    total_words = sum(word_counts)
    passive = sum(
        1
        for s in sentences
        if any(t["dep"] in ("nsubj:pass", "nsubjpass", "aux:pass") for t in s["tokens"])
    )

    mean_len = total_words / total if total else 0.0
    passive_ratio = passive / total if total else 0.0

    return {
        "total_sentences": total,
        "total_words": total_words,
        "passive_ratio": passive_ratio,
        "mean_sentence_length": mean_len,
        "vocabulary_ttr": result["vocabulary_ttr"],
        "nominalization_ratio": result["nominalization_ratio"],
    }


@click.group()
def main() -> None:
    """matra -- NLP library: UDPipe parse, metrics, summarization, keyphrase extraction."""


@main.command()
@click.argument("path", type=click.Path(exists=True, path_type=Path))
@click.option("-n", default=3, show_default=True, help="Number of sentences")
@click.option("--json-output", "--json", is_flag=True, help="Output as JSON")
@click.option(
    "--method",
    type=click.Choice(["tfidf", "textrank"]),
    default="tfidf",
    show_default=True,
    help="Summarization algorithm",
)
def summarize(path: Path, n: int, json_output: bool, method: str) -> None:
    """Extract top-N sentences as an extractive summary."""
    v = _get_matra()
    text = path.read_text()

    result: list[ScoredSentence]
    if method == "textrank":
        result = v.textrank_summarize(text, n)
    else:
        result = v.tfidf_summarize(text, n)

    if json_output:
        click.echo(json.dumps(result, indent=2))
        return

    for i, sent in enumerate(result, 1):
        console.print(f"[cyan]{i}.[/cyan] {sent['text']}")
        console.print(f"   [dim]score={sent['score']:.3f}  position={sent['position']}[/dim]")


@main.command()
@click.argument("path", type=click.Path(exists=True, path_type=Path))
@click.option("-n", default=10, show_default=True, help="Max keyphrases")
@click.option("--json-output", "--json", is_flag=True, help="Output as JSON")
@click.option(
    "--method",
    type=click.Choice(["rake", "yake"]),
    default="rake",
    show_default=True,
    help="Keyphrase algorithm",
)
def keyphrases(path: Path, n: int, json_output: bool, method: str) -> None:
    """Extract ranked keyphrases from text."""
    v = _get_matra()
    text = path.read_text()

    result: list[Keyphrase]
    if method == "yake":
        result = v.yake_keyphrases(text, n)
    else:
        result = v.rake_keyphrases(text, n)

    if json_output:
        click.echo(json.dumps(result, indent=2))
        return

    table = Table(title=f"Keyphrases: {path.name}")
    table.add_column("#", style="dim")
    table.add_column("Phrase", style="cyan")
    table.add_column("Score", justify="right")
    for i, kp in enumerate(result, 1):
        table.add_row(str(i), kp["phrase"], f"{kp['score']:.2f}")
    console.print(table)


@main.command()
@click.argument("path", type=click.Path(exists=True, path_type=Path))
@click.option("--json-output", "--json", is_flag=True, help="Output as JSON")
@click.option("--sections", "-s", is_flag=True, help="Show per-section breakdown")
def analyze(path: Path, json_output: bool, sections: bool) -> None:
    """Analyze a file. Metrics only, no judgments."""
    v = _get_matra()
    text = path.read_text()

    result: Document
    if path.suffix == ".md":
        result = v.analyze_markdown(text)
    else:
        result = v.analyze(text)

    if json_output:
        click.echo(json.dumps(result, indent=2))
        return

    doc = _doc_metrics(result)

    table = Table(title=f"matra: {path.name}")
    table.add_column("Metric", style="cyan")
    table.add_column("Value")

    table.add_row("Sentences", str(doc["total_sentences"]))
    table.add_row("Words", str(doc["total_words"]))
    table.add_row("", "")
    table.add_row("Passive ratio", f"{doc['passive_ratio']:.1%}")
    table.add_row("Mean sentence length", f"{doc['mean_sentence_length']:.1f}")
    ttr = doc["vocabulary_ttr"]
    table.add_row("Vocabulary TTR", f"{ttr:.2f}" if ttr is not None else "-")
    nom = doc["nominalization_ratio"]
    table.add_row("Nominalization ratio", f"{nom:.1%}" if nom is not None else "-")

    console.print(table)

    if sections:
        st = Table(title="Sections")
        st.add_column("Level")
        st.add_column("Heading", ratio=1)
        st.add_column("Paragraphs", justify="right")
        st.add_column("Sentences", justify="right")
        st.add_column("Words", justify="right")

        for sec in result["sections"]:
            paras = sec["paragraphs"]
            sents = sum(len(p["sentences"]) for p in paras)
            words = sum(
                sum(1 for t in s["tokens"] if not t["is_punct"])
                for p in paras
                for s in p["sentences"]
            )
            st.add_row(
                f"h{sec['level']}",
                (sec["heading"] or "(intro)")[:45],
                str(len(paras)),
                str(sents),
                str(words),
            )
        console.print()
        console.print(st)
