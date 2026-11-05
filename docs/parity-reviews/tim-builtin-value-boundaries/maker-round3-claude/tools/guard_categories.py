"""Count parity_review_records_are_complete violations by message category.
Usage: python3 -I guard_categories.py <harness_guards.log>..."""
import collections, sys
for path in sys.argv[1:]:
    text = open(path).read()
    start = text.find("invalid T14 parity review evidence:")
    end = text.find("\nnote: run with", start)
    counts = collections.Counter()
    for line in text[start:end].splitlines()[1:]:
        line = line.strip()
        if ": " in line:
            counts[line.split(": ", 1)[1].split(" (")[0][:80]] += 1
    print("===", path, sum(counts.values()))
    for message, n in sorted(counts.items()):
        print(f"{n:6d}  {message}")
