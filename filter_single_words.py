#!/usr/bin/env python3
"""Filter neword.txt to keep only single-word entries."""

import sys

def filter_single_words(input_file, output_file=None):
    """
    Read input_file, filter for clean single-word entries, write to output_file.
    Removes entries with spaces, hyphens, periods, commas, equals, backticks, or apostrophes.
    """
    single_words = []

    with open(input_file, 'r', encoding='latin-1') as f:
        for line in f:
            word = line.rstrip('\r\n')
            # Check if it's a single word (no spaces, hyphens, or special chars)
            if word and not any(c in word for c in [' ', '-', '.', ',', '=', '`', "'"]):
                single_words.append(word)

    # Write to output file or stdout
    if output_file:
        with open(output_file, 'w', encoding='latin-1') as f:
            f.write('\n'.join(single_words) + '\n')
        print(f"Filtered to {len(single_words)} clean words (removed special chars: . , = ` ')")
        print(f"Output: {output_file}")
    else:
        for line in single_words:
            print(line)
        print(f"\nTotal: {len(single_words)} clean words", file=sys.stderr)

if __name__ == '__main__':
    input_file = 'src-tauri/resources/indonesian-wordlist.txt'
    output_file = 'src-tauri/resources/indonesian-wordlist-clean.txt'

    try:
        filter_single_words(input_file, output_file)
    except FileNotFoundError:
        print(f"Error: {input_file} not found", file=sys.stderr)
        sys.exit(1)
