import argparse
import json
import os
from collections import defaultdict
from typing import Any


def load_records(path: str) -> list[dict[str, Any]]:
    with open(path, "r", encoding="utf-8") as f:
        if path.endswith(".jsonl"):
            return [json.loads(line) for line in f if line.strip()]
        data = json.load(f)

    if isinstance(data, dict):
        if "grades" in data:
            return data["grades"]
        if "results" in data:
            return data["results"]
        return []
    return data


def main() -> None:
    parser = argparse.ArgumentParser(description="Statistics for Cortex Memory LoCoMo evaluation results")
    parser.add_argument("--input", required=True, help="Path to answers/grades JSON or JSONL file")
    args = parser.parse_args()

    if not os.path.exists(args.input):
        raise FileNotFoundError(f"Input file not found: {args.input}")

    rows = load_records(args.input)
    total = len(rows)
    correct = sum(1 for row in rows if row.get("grade") is True)
    wrong = sum(1 for row in rows if row.get("grade") is False)
    accuracy = correct / (correct + wrong) if (correct + wrong) > 0 else 0.0

    total_time = 0.0
    total_prompt_tokens = 0
    total_completion_tokens = 0
    total_tokens = 0
    category_stats: dict[str, dict[str, float]] = defaultdict(lambda: {"correct": 0, "wrong": 0, "total": 0})

    for row in rows:
        total_time += float(row.get("time_cost", 0.0) or 0.0)
        usage = row.get("token_usage") or {}
        total_prompt_tokens += int(usage.get("prompt_tokens", 0) or 0)
        total_completion_tokens += int(usage.get("completion_tokens", 0) or 0)
        total_tokens += int(usage.get("total_tokens", 0) or 0)
        category = str(row.get("category", "unknown"))
        category_stats[category]["total"] += 1
        if row.get("grade") is True:
            category_stats[category]["correct"] += 1
        elif row.get("grade") is False:
            category_stats[category]["wrong"] += 1

    avg_time = total_time / total if total > 0 else 0.0
    lines = [
        "=== Cortex Memory LoCoMo Evaluation Statistics ===",
        f"Total rows: {total}",
        f"Correct: {correct}",
        f"Wrong: {wrong}",
        f"Accuracy: {accuracy:.2%}",
        f"Average time cost: {avg_time:.2f}s",
        "",
        "Token usage:",
        f"  Total prompt tokens: {total_prompt_tokens}",
        f"  Total completion tokens: {total_completion_tokens}",
        f"  Total tokens: {total_tokens}",
        "",
        "Per-category:",
    ]

    for category in sorted(category_stats):
        stats = category_stats[category]
        pct = stats["correct"] / stats["total"] if stats["total"] > 0 else 0.0
        lines.append(
            f"  Category {category}: {int(stats['correct'])}/{int(stats['total'])} ({pct:.2%})"
        )

    for line in lines:
        print(line)

    summary_path = os.path.join(os.path.dirname(args.input), "summary.txt")
    with open(summary_path, "w", encoding="utf-8") as f:
        f.write("\n".join(lines) + "\n")
    print(f"\nSummary saved to {summary_path}")


if __name__ == "__main__":
    main()
