# frozen_string_literal: true

require "json"

ROOT = File.expand_path(__dir__)
NBSP = "\u00A0"
QUOTES = {
  "ascii" => "\"",
  "left" => "\u201C",
  "right" => "\u201D",
  "jaws" => "\uE121"
}.freeze

cases = []

def add_case(cases, name:, family:, expected_java_valid:, axes:, source:)
  path = File.join(ROOT, "#{name}.puml")
  File.write(path, source, mode: "w:UTF-8")
  cases << {
    "name" => name,
    "source" => File.basename(path),
    "family" => family,
    "expected_java_valid" => expected_java_valid,
    "axes" => axes
  }
end

def document(family, token, command, code, invalid: false, command_last: false)
  if family == "component"
    anchor = "CAnchor#{token}"
    peer = "CPeer#{token}"
    late = "CLate#{token}"
    declaration = %(component "Anchor #{token}" as #{anchor})
    peer_declaration = %(component "Peer #{token}" as #{peer})
    late_declaration = %(component "Late #{token}" as #{late})
  else
    anchor = "DAnchor#{token}"
    peer = "DPeer#{token}"
    late = "DLate#{token}"
    declaration = %(node "Anchor #{token}" as #{anchor})
    peer_declaration = %(artifact "Peer #{token}" as #{peer})
    late_declaration = %(cloud "Late #{token}" as #{late})
  end

  lines = ["@startuml", declaration]
  if command_last
    lines.concat([peer_declaration, "#{anchor} --> #{peer} : before", command])
  else
    lines << command
    lines << peer_declaration
    lines << "#{code} --> #{anchor} : ownership" unless code.nil?
    lines << late_declaration
    lines << "#{anchor} --> #{late} : survives" if invalid
    lines << "#{anchor} --> #{peer} : order"
  end
  lines << "@enduml"
  lines.join("\n") + "\n"
end

decorations = [
  "",
  " $tag",
  " <<Reviewed>>",
  " #LightBlue",
  " $first $second <<Reviewed>> #PaleGreen",
  " $nbsp#{NBSP}$second <<Mixed Case>> #LightBlue/LightGreen",
  " #LightBlue-LightGreen",
  " $audit <<Terminal>> #LightBlue|LightGreen"
].freeze

# Exhaust every independent opening/closing pair in Pattern2's %g alphabet for
# both Description families. Whitespace, case, decorations, declaration order,
# and relation count vary independently across the matrix.
QUOTES.each_with_index do |(open_name, opener), open_index|
  QUOTES.each_with_index do |(close_name, closer), close_index|
    ["component", "deployment"].each_with_index do |family, family_index|
      index = open_index * QUOTES.length + close_index
      token = format("V%02d%s", index, family_index.zero? ? "C" : "D")
      code = family_index.zero? ? "Memo.C#{token}" : "Memo.D#{token}"
      note_keyword = ["note", "NoTe", "NOTE", "nOtE"][index % 4]
      as_keyword = ["as", "AS", "As", "aS"][(index + family_index) % 4]
      gap = [" ", "\t", NBSP][(index + family_index) % 3]
      body = index.even? ? "matrix #{token}" : "matrix\\n#{token}"
      decoration = decorations[(index + family_index) % decorations.length]
      command = "#{note_keyword}#{gap}#{opener}#{body}#{closer}#{gap}#{as_keyword}#{gap}#{code}#{decoration}"
      name = "valid_#{family}_pair_#{open_name}_#{close_name}"
      add_case(
        cases,
        name: name,
        family: family,
        expected_java_valid: true,
        axes: [
          "inline",
          "%g opener #{open_name}",
          "%g closer #{close_name}",
          "#{gap == NBSP ? 'NBSP' : gap == "\t" ? 'tab' : 'ASCII'} command whitespace",
          "mixed keyword case",
          "decoration variant #{(index + family_index) % decorations.length}",
          "renamed labels",
          "later declarations and relations"
        ],
        source: document(family, token, command, code)
      )
    end
  end
end

# Valid multiline commands vary optional suffixes, terminator spelling, NBSP,
# nesting pressure, and later relation reuse without relying on quote syntax.
["component", "deployment"].each do |family|
  4.times do |index|
    token = "M#{index}#{family[0].upcase}"
    code = family == "component" ? "Multi.C#{token}" : "Multi.D#{token}"
    gap = index == 1 ? NBSP : " "
    note_keyword = index.even? ? "note" : "NoTe"
    as_keyword = index < 2 ? "as" : "AS"
    suffix = ["", " $multi", " <<Long Form>>", " $multi <<Long Form>> #Lavender"][index]
    terminator = ["endnote", "end note", "ENDNOTE", "EnD NoTe"][index]
    command = [
      "#{note_keyword}#{gap}#{as_keyword}#{gap}#{code}#{suffix}",
      "multiline #{token}",
      "second row #{3 - index}",
      terminator
    ].join("\n")
    add_case(
      cases,
      name: "valid_#{family}_multiline_variant_#{index}",
      family: family,
      expected_java_valid: true,
      axes: ["multiline", "terminator #{terminator}", "optional decoration variant #{index}", "case/NBSP", "later relation reuse"],
      source: document(family, token, command, code)
    )
  end
end

# Every %g member appears as an embedded premature closer in both families,
# while the actual opener and trailing delimiter rotate through the alphabet.
quote_entries = QUOTES.to_a
quote_entries.each_with_index do |(embedded_name, embedded), index|
  ["component", "deployment"].each_with_index do |family, family_index|
    opener_name, opener = quote_entries[(index + family_index) % quote_entries.length]
    trailing_name, trailing = quote_entries[(index + family_index + 2) % quote_entries.length]
    token = "E#{index}#{family_index}"
    code = family_index.zero? ? "Escape.C#{token}" : "Escape.D#{token}"
    command = "note #{opener}left#{embedded}right#{trailing} as #{code} $embedded"
    add_case(
      cases,
      name: "invalid_#{family}_embedded_#{embedded_name}_open_#{opener_name}_trail_#{trailing_name}",
      family: family,
      expected_java_valid: false,
      axes: ["inline malformed", "%g opener #{opener_name}", "embedded %g #{embedded_name}", "trailing %g #{trailing_name}", "would-be endpoint relation", "later declaration/order"],
      source: document(family, token, command, code, invalid: true)
    )
  end
end

# Missing closers cover each possible opener in each family.
QUOTES.each_with_index do |(open_name, opener), index|
  ["component", "deployment"].each_with_index do |family, family_index|
    token = "U#{index}#{family_index}"
    code = family_index.zero? ? "Unclosed.C#{token}" : "Unclosed.D#{token}"
    command = "NOTE#{index.odd? ? NBSP : ' '}#{opener}unterminated #{token} as #{code} $open"
    add_case(
      cases,
      name: "invalid_#{family}_missing_closer_after_#{open_name}",
      family: family,
      expected_java_valid: false,
      axes: ["inline malformed", "%g opener #{open_name}", "missing closer", "case/NBSP", "would-be endpoint relation"],
      source: document(family, token, command, code, invalid: true)
    )
  end
end

suffix_mutations = {
  "missing_as" => ->(q, code) { "note #{q}missing as#{q} #{code}" },
  "extra_as" => ->(q, code) { "note #{q}extra as#{q} as as #{code}" },
  "missing_code" => ->(q, _code) { "note #{q}missing code#{q} as" },
  "extra_code" => ->(q, code) { "note #{q}extra code#{q} as #{code} TrailingCode" },
  "hyphen_code" => ->(q, code) { "note #{q}hyphen code#{q} as #{code}-bad" },
  "extra_color" => ->(q, code) { "note #{q}extra color#{q} as #{code} #LightBlue #PaleGreen" },
  "color_before_stereo" => ->(q, code) { "note #{q}wrong order#{q} as #{code} #LightBlue <<Late>>" },
  "tag_after_stereo" => ->(q, code) { "note #{q}wrong tag order#{q} as #{code} <<First>> $late" },
  "duplicate_stereo" => ->(q, code) { "note #{q}duplicate stereo#{q} as #{code} <<One>> <<Two>>" },
  "unknown_color" => ->(q, code) { "note #{q}unknown color#{q} as #{code} #HeldoutColor#{code.delete('.')}" }
}.freeze

suffix_mutations.each_with_index do |(mutation_name, mutation), index|
  ["component", "deployment"].each_with_index do |family, family_index|
    quote_name, quote = quote_entries[(index + family_index) % quote_entries.length]
    token = "S#{index}#{family_index}"
    code = family_index.zero? ? "Suffix.C#{token}" : "Suffix.D#{token}"
    add_case(
      cases,
      name: "invalid_#{family}_suffix_#{mutation_name}_#{quote_name}",
      family: family,
      expected_java_valid: mutation_name == "duplicate_stereo",
      axes: [mutation_name == "duplicate_stereo" ? "inline Java-valid combined stereotype payload" : "inline malformed", "%g delimiter #{quote_name}", mutation_name.tr("_", " "), "would-be endpoint relation", "later declarations and relations"],
      source: document(family, token, mutation.call(quote, code), code, invalid: true)
    )
  end
end

# Pattern2 expands %g inside Stereotag.pattern too. This matrix attacks whether
# Rust's successful decoration parser mistakenly accepts a quote-bearing tag.
QUOTES.each_with_index do |(quote_name, quote), index|
  ["component", "deployment"].each_with_index do |family, family_index|
    outer_name, outer = quote_entries[(index + family_index + 1) % quote_entries.length]
    token = "T#{index}#{family_index}"
    code = family_index.zero? ? "Tag.C#{token}" : "Tag.D#{token}"
    command = "note #{outer}tag quote #{token}#{outer} as #{code} $audit#{quote}tail <<TagProbe>> #LightBlue"
    add_case(
      cases,
      name: "invalid_#{family}_tag_contains_#{quote_name}_outer_#{outer_name}",
      family: family,
      expected_java_valid: false,
      axes: ["inline malformed decoration", "outer %g #{outer_name}", "tag contains %g #{quote_name}", "would-be endpoint relation", "later declarations and relations"],
      source: document(family, token, command, code, invalid: true)
    )
  end
end

multiline_mutations = {
  "missing_code" => ->(_code) { "note as" },
  "extra_as" => ->(code) { "note as as #{code}" },
  "hyphen_code" => ->(code) { "note as #{code}-bad" },
  "decoration_order" => ->(code) { "note as #{code} #LightBlue <<Late>>" },
  "unknown_color" => ->(code) { "note as #{code} #HeldoutMultiline#{code.delete('.')}" },
  "unterminated" => ->(code) { "note as #{code}\nbody without a terminator" }
}.freeze

multiline_mutations.each_with_index do |(mutation_name, mutation), index|
  ["component", "deployment"].each_with_index do |family, family_index|
    token = "L#{index}#{family_index}"
    code = family_index.zero? ? "Long.C#{token}" : "Long.D#{token}"
    command = mutation.call(code)
    if mutation_name == "unterminated"
      swallowed = family == "component" ? "Swallowed.C#{token}" : "Swallowed.D#{token}"
      declaration = if family == "component"
                      %(component "Swallowed #{token}" as #{swallowed})
                    else
                      %(database "Swallowed #{token}" as #{swallowed})
                    end
      command += "\n#{declaration}\n#{code} --> #{swallowed} : swallowed relation"
    end
    command += "\nbody #{token}\nendnote" unless mutation_name == "unterminated" || mutation_name == "missing_code"
    add_case(
      cases,
      name: "invalid_#{family}_multiline_#{mutation_name}",
      family: family,
      expected_java_valid: false,
      axes: ["multiline malformed", mutation_name.tr("_", " "), "later declarations and relations", "renamed labels/count"],
      source: document(family, token, command, code, invalid: true, command_last: mutation_name == "unterminated")
    )
  end
end

File.write(File.join(ROOT, "cases.json"), JSON.pretty_generate({
  "schema_version" => 1,
  "quote_alphabet" => QUOTES.transform_values { |value| format("U+%04X", value.ord) },
  "case_count" => cases.length,
  "cases" => cases
}) + "\n")

warn "generated #{cases.length} fresh cases in #{ROOT}"
